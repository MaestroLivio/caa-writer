<script lang="ts">
  import { api, isTauri } from "../api";
  import type {
    Alternative,
    LayoutDocument,
    LayoutElementSpec,
    LayoutSpecification,
    PlacedElement,
  } from "../types";
  import { language, strings } from "../i18n";

  let {
    doc,
    onSpeak,
    onClose,
    onApplySpec,
    onSave,
    editable = true,
  }: {
    doc: LayoutDocument;
    onSpeak: (text: string) => void;
    onClose: () => void;
    onApplySpec?: (spec: LayoutSpecification) => Promise<void>;
    onSave?: (spec: LayoutSpecification) => Promise<void>;
    editable?: boolean;
  } = $props();

  type MessageUnit = { key: string; label: string };

  let message = $state<MessageUnit[]>([]);
  /// «Componi frase» accumula i simboli; «Tocca per parlare» pronuncia subito.
  let mode = $state<"compose" | "touch">("compose");
  let pageIndex = $state(0);
  let history = $state<number[]>([]);
  let unitSeq = 0;

  // Modalità modifica: lavora su una copia della specifica, mai direttamente
  // sulle pagine calcolate. «Applica» ricostruisce la geometria dal backend.
  let editing = $state(false);
  let draft = $state<LayoutElementSpec[]>([]);
  let draftTitle = $state(doc.spec.title);
  let activeId = $state<string | null>(null);
  let searchQuery = $state("");
  let searchResults = $state<Alternative[]>([]);
  let searchBusy = $state(false);
  let applyBusy = $state(false);
  let saveBusy = $state(false);
  let editError = $state("");
  let editSeq = 0;

  const copy = $derived(strings[$language]);
  const currentPage = $derived(doc.pages[pageIndex] ?? doc.pages[0]);
  const cells = $derived((currentPage?.elements ?? []).filter((cell) => !cell.hidden));

  const draftPages = $derived(
    Math.max(
      1,
      doc.pages.length,
      ...draft.map((element) => Number(element.page ?? 0) + 1)
    )
  );
  const pageDraft = $derived(
    draft
      .filter((element) => Number(element.page ?? 0) === pageIndex && !element.hidden)
      .sort(
        (a, b) =>
          Number(a.row ?? 0) - Number(b.row ?? 0) ||
          Number(a.column ?? 0) - Number(b.column ?? 0)
      )
  );
  const activeElement = $derived(draft.find((element) => element.id === activeId) ?? null);
  const totalPages = $derived(editing ? draftPages : doc.pages.length);

  $effect(() => {
    // Fuori dalla modifica la copia segue il documento appena calcolato.
    if (editing) return;
    draftTitle = doc.spec.title;
    draft = (doc.spec.elements ?? []).map((element) => ({ ...element, action: element.action ? { ...element.action } : null }));
    if (pageIndex >= doc.pages.length) pageIndex = Math.max(0, doc.pages.length - 1);
  });

  function goTo(index: number) {
    if (index === pageIndex || index < 0 || index >= totalPages) return;
    history = [...history, pageIndex];
    pageIndex = index;
  }

  function back() {
    const previous = history[history.length - 1];
    if (previous === undefined) return;
    history = history.slice(0, -1);
    pageIndex = previous;
  }

  function home() {
    if (pageIndex === 0) return;
    history = [...history, pageIndex];
    pageIndex = 0;
  }

  function addCell(cell: PlacedElement) {
    unitSeq += 1;
    message = [...message, { key: `${cell.id}-${unitSeq}`, label: cell.spoken_text || cell.label || "?" }];
  }

  function removeLast() {
    message = message.slice(0, -1);
  }

  function removeAt(index: number) {
    message = message.filter((_, position) => position !== index);
  }

  function clearMessage() {
    message = [];
  }

  function speakMessage() {
    if (message.length === 0) return;
    const text = message
      .map((unit) => unit.label)
      .join(" ")
      .replace(/\s+([,.;:!?])/g, "$1")
      .trim();
    onSpeak(/[.!?]$/.test(text) ? text : `${text}.`);
  }

  function runAction(element: PlacedElement) {
    const action = element.action?.type ?? "append";
    const spoken = element.spoken_text || element.label || "";
    if (mode === "touch") {
      if (action === "navigate") {
        if (spoken) onSpeak(spoken);
        goTo(Number(element.action?.target_page ?? 0));
        return;
      }
      if (action === "back") {
        back();
        return;
      }
      if (action === "home") {
        home();
        return;
      }
      if (action === "clear") {
        clearMessage();
        return;
      }
      if (spoken) onSpeak(spoken);
      return;
    }
    if (action === "navigate") {
      goTo(Number(element.action?.target_page ?? 0));
      return;
    }
    if (action === "back") {
      back();
      return;
    }
    if (action === "home") {
      home();
      return;
    }
    if (action === "clear") {
      clearMessage();
      return;
    }
    if (action === "speak") {
      onSpeak(element.spoken_text || element.label);
      return;
    }
    addCell(element);
  }

  function handleKey(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.preventDefault();
      if (editing) {
        editing = false;
        return;
      }
      onClose();
    }
    if (!editing && event.key === "Backspace" && message.length > 0) {
      event.preventDefault();
      removeLast();
    }
  }

  // ------------------------------------------------------------- modifica
  function startEditing() {
    editSeq += 1;
    draft = (doc.spec.elements ?? []).map((element) => ({
      ...element,
      action: element.action ? { ...element.action } : { type: "append", target_page: null },
      spoken_text: element.spoken_text ?? element.label ?? "",
    }));
    draftTitle = doc.spec.title;
    pageIndex = 0;
    editing = true;
  }

  function setActionType(element: LayoutElementSpec, type: string) {
    element.action = {
      type,
      target_page:
        type === "navigate" ? Number(element.action?.target_page ?? 0) : null,
    };
  }

  function nextPosition(): { page: number; row: number; column: number } {
    const columns = Math.max(1, Number(doc.spec.layout.columns ?? 4));
    const rows = Math.max(1, Number(doc.spec.layout.rows ?? 3));
    const onPage = draft.filter((element) => Number(element.page ?? 0) === pageIndex);
    const index = onPage.length;
    const column = index % columns;
    const row = Math.floor(index / columns);
    if (row < rows) return { page: pageIndex, row, column };
    return { page: pageIndex + 1, row: 0, column: 0 };
  }

  function newId(): string {
    return `cell-${Date.now().toString(36)}-${(editSeq += 1)}`;
  }

  function addDraftCell() {
    const position = nextPosition();
    draft = [
      ...draft,
      {
        id: newId(),
        kind: "box",
        label: "",
        page: position.page,
        row: position.row,
        column: position.column,
        row_span: 1,
        column_span: 1,
        hidden: false,
        locked: false,
        spoken_text: "",
        action: { type: "append", target_page: null },
      },
    ];
    pageIndex = position.page;
    activeId = draft[draft.length - 1].id;
  }

  function removeDraftCell(id: string) {
    draft = draft.filter((element) => element.id !== id);
    if (activeId === id) activeId = null;
  }

  function addPage() {
    const next = draftPages;
    draft = [
      ...draft,
      {
        id: newId(),
        kind: "label",
        label: `Pagina ${next + 1}`,
        page: next,
        row: 0,
        column: 0,
        row_span: 1,
        column_span: 1,
        hidden: false,
        locked: false,
        spoken_text: `Pagina ${next + 1}`,
        action: { type: "navigate", target_page: next },
      },
    ];
    pageIndex = next;
    activeId = draft[draft.length - 1].id;
  }

  function applySpec(): LayoutSpecification {
    return {
      ...doc.spec,
      title: draftTitle,
      elements: draft.map((element) => ({ ...element })),
    };
  }

  async function applyDraft() {
    if (!onApplySpec) return;
    applyBusy = true;
    editError = "";
    try {
      await onApplySpec(applySpec());
      editing = false;
    } catch (error) {
      editError = error instanceof Error ? error.message : String(error);
    } finally {
      applyBusy = false;
    }
  }

  async function saveDraft() {
    if (!onSave) return;
    saveBusy = true;
    try {
      await onSave(applySpec());
    } finally {
      saveBusy = false;
    }
  }

  async function search() {
    const query = searchQuery.trim();
    if (!query) return;
    if (!isTauri()) {
      searchResults = [];
      return;
    }
    searchBusy = true;
    try {
      const page = await api.searchSymbols(query, 16, 0, null, false);
      searchResults = page.items;
    } finally {
      searchBusy = false;
    }
  }

  function applySymbol(alternative: Alternative) {
    if (!activeElement) return;
    activeElement.label = alternative.label;
    activeElement.symbol_id = alternative.symbol_id;
    activeElement.concept_id = alternative.concept_id;
    activeElement.svg = alternative.svg;
    activeElement.kind = "symbol";
    searchResults = [];
    searchQuery = "";
  }
</script>

<svelte:window on:keydown={handleKey} />

<section
  class="communicator"
  aria-label={copy.communicator.dialogLabel(doc.spec.title || copy.communicator.titleFallback)}
>
  <header class="communicator__bar">
    <button type="button" class="btn" onclick={home} disabled={pageIndex === 0}
      >{copy.communicator.home}</button
    >
    <button type="button" class="btn" onclick={back} disabled={history.length === 0}
      >{copy.communicator.back}</button
    >
    {#if editing}
      <input class="communicator__title" bind:value={draftTitle} aria-label="Titolo del comunicatore" />
    {:else}
      <h2>{doc.spec.title || copy.communicator.titleFallback}</h2>
    {/if}
    <span class="hint">{copy.communicator.page(pageIndex + 1, totalPages)}</span>
    {#if !editing}
      <div class="communicator__mode" role="group" aria-label="Modalità comunicazione">
        <button type="button" class="btn" class:btn--solid={mode === "compose"} onclick={() => (mode = "compose")}>
          Componi frase
        </button>
        <button type="button" class="btn" class:btn--solid={mode === "touch"} onclick={() => (mode = "touch")}>
          Tocca per parlare
        </button>
      </div>
    {/if}
    <span class="grow"></span>
    {#if editable && !editing}
      <button type="button" class="btn" onclick={startEditing}>Modifica</button>
    {:else if editing}
      <button type="button" class="btn" onclick={addDraftCell}>+ Cella</button>
      <button type="button" class="btn" onclick={addPage}>+ Pagina</button>
      <button type="button" class="btn" disabled={applyBusy} onclick={() => void applyDraft()}>
        {applyBusy ? "Applico…" : "Applica"}
      </button>
      <button type="button" class="btn btn--solid" disabled={saveBusy} onclick={() => void saveDraft()}>
        {saveBusy ? "Salvo…" : "Salva .caadoc"}
      </button>
      <button type="button" class="btn" onclick={() => (editing = false)}>Annulla modifiche</button>
    {/if}
    <button type="button" class="btn" onclick={onClose}>{copy.communicator.exit}</button>
  </header>

  {#if editing && pageDraft.length > 0}
    <div class="edit-tools">
      <label class="edit-search">
        <span class="hint">Pittogramma per la cella selezionata</span>
        <input
          type="search"
          bind:value={searchQuery}
          placeholder="Cerca un simbolo…"
          onkeydown={(event) => {
            if (event.key === "Enter") {
              event.preventDefault();
              void search();
            }
          }}
        />
      </label>
      <button class="btn" type="button" disabled={searchBusy || !activeElement} onclick={() => void search()}>
        {searchBusy ? "Cerco…" : "Cerca"}
      </button>
      <span class="hint">
        {activeElement ? `Cella selezionata: «${activeElement.label || "senza titolo"}»` : "Seleziona una cella per cambiarne il simbolo"}
      </span>
      {#if editError}
        <p class="warn" role="alert">{editError}</p>
      {/if}
      {#if searchResults.length > 0}
        <div class="edit-results">
          {#each searchResults as alternative (alternative.symbol_id)}
            <button
              type="button"
              class="edit-result"
              title={`${alternative.meaning || alternative.label} — ${alternative.license}`}
              onclick={() => applySymbol(alternative)}
            >
              {#if alternative.svg}
                <span class="edit-result__media">{@html alternative.svg}</span>
              {/if}
              <span>{alternative.label}</span>
            </button>
          {/each}
        </div>
      {/if}
    </div>
  {/if}

  <div class="communicator__grid" role="group" aria-label={copy.communicator.gridLabel}>
    {#if editing}
      {#each pageDraft as element (element.id)}
        {@const action = element.action ?? { type: "append", target_page: null }}
        <div class="cell cell--edit" class:cell--active={activeId === element.id}>
          <button
            type="button"
            class="cell__pick"
            aria-label={`Seleziona la cella ${element.label || "senza titolo"}`}
            onclick={() => (activeId = element.id)}
          >
            {#if element.svg}
              <span class="cell__symbol">{@html element.svg}</span>
            {:else}
              <span class="cell__placeholder">Scegli un simbolo</span>
            {/if}
          </button>
          <label class="edit-field">
            <span class="hint">Etichetta</span>
            <input bind:value={element.label} onfocus={() => (activeId = element.id)} />
          </label>
          <label class="edit-field">
            <span class="hint">Testo pronunciato</span>
            <input
              value={element.spoken_text ?? ""}
              placeholder={element.label}
              oninput={(event) => (element.spoken_text = event.currentTarget.value)}
            />
          </label>
          <label class="edit-field">
            <span class="hint">Azione</span>
            <select
              value={action.type}
              onchange={(event) => setActionType(element, event.currentTarget.value)}
            >
              <option value="append">Aggiungi alla frase</option>
              <option value="speak">Pronuncia la cella</option>
              <option value="navigate">Vai alla pagina</option>
              <option value="back">Indietro</option>
              <option value="home">Home</option>
              <option value="clear">Svuota la frase</option>
            </select>
          </label>
          {#if action.type === "navigate"}
            <label class="edit-field">
              <span class="hint">Pagina di destinazione</span>
              <select
                value={String(action.target_page ?? 0)}
                onchange={(event) => (action.target_page = Number(event.currentTarget.value))}
              >
                {#each Array.from({ length: draftPages }, (_, index) => index) as index (index)}
                  <option value={String(index)}>Pagina {index + 1}</option>
                {/each}
              </select>
            </label>
          {/if}
          <button
            type="button"
            class="mini mini--danger"
            aria-label={`Rimuovi la cella ${element.label || "senza titolo"}`}
            onclick={() => removeDraftCell(element.id)}
          >Rimuovi</button>
        </div>
      {/each}
    {:else}
      {#each cells as cell (cell.id)}
        <button
          type="button"
          class="cell"
          onclick={() => runAction(cell)}
          aria-label={mode === "touch" ? `Pronuncia ${cell.label || copy.communicator.unnamedSymbol}` : copy.communicator.add(cell.label || copy.communicator.unnamedSymbol)}
        >
          {#if cell.svg}
            <span class="cell__symbol">{@html cell.svg}</span>
          {/if}
          <span class="cell__label">{cell.label}</span>
          {#if cell.action?.type && cell.action.type !== "append"}
            <span class="cell__action">{cell.action.type}</span>
          {/if}
        </button>
      {/each}
      {#if cells.length === 0}
        <p class="hint">{copy.communicator.emptyPage}</p>
      {/if}
    {/if}
  </div>

  {#if mode === "compose"}
  <footer class="communicator__message">
    <div class="message" aria-live="polite" aria-label={copy.communicator.messageLabel}>
      {#if message.length === 0}
        <span class="hint">{copy.communicator.emptyMessage}</span>
      {:else}
        {#each message as unit, index (unit.key)}
          <button
            type="button"
            class="chip chip--removable"
            title="Rimuovi questa parola"
            onclick={() => removeAt(index)}
          >{unit.label} ×</button>
        {/each}
      {/if}
    </div>
    <div class="communicator__actions">
      <button type="button" class="btn" onclick={removeLast} disabled={message.length === 0}>
        {copy.communicator.removeLast}
      </button>
      <button type="button" class="btn" onclick={clearMessage} disabled={message.length === 0}>
        {copy.communicator.clear}
      </button>
      <button
        type="button"
        class="btn btn--solid"
        onclick={speakMessage}
        disabled={message.length === 0}
      >
        {copy.communicator.speak}
      </button>
    </div>
  </footer>
  {/if}
</section>

<style>
  .communicator {
    position: fixed;
    inset: 0;
    z-index: 80;
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 14px;
    background: var(--surface, #f7f7f8);
    color: var(--text, #16161a);
    overflow: auto;
  }

  .communicator__bar {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
  }

  .communicator__bar h2 {
    margin: 0 6px;
    font-size: 1.1rem;
  }

  .communicator__title {
    font-size: 1.05rem;
    font-weight: 600;
    min-width: 220px;
  }

  .grow {
    flex: 1 1 auto;
  }

  .edit-tools {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    gap: 8px;
    padding: 8px;
    border: 1px solid var(--border, #d3d3da);
    border-radius: 10px;
    background: var(--surface-2, #f2f3f7);
  }

  .edit-search {
    display: flex;
    flex-direction: column;
    min-width: 240px;
  }

  .edit-results {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    width: 100%;
    max-height: 150px;
    overflow: auto;
  }

  .edit-result {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 8px;
    border: 1px solid var(--border, #c9c9d2);
    border-radius: 8px;
    background: #fff;
    cursor: pointer;
  }

  .edit-result__media :global(svg) {
    width: 28px;
    height: 28px;
  }

  .communicator__grid {
    flex: 1 1 auto;
    min-height: 0;
    overflow: auto;
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(132px, 1fr));
    gap: 10px;
    align-content: start;
  }

  .cell {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
    min-height: 132px;
    padding: 10px;
    border: 2px solid var(--border, #b9b9c0);
    border-radius: 12px;
    background: #ffffff;
    color: #141414;
    cursor: pointer;
    font: inherit;
  }

  .cell:hover {
    border-color: var(--accent, #2463eb);
  }

  .cell:focus-visible {
    outline: 4px solid var(--accent, #2463eb);
    outline-offset: 2px;
  }

  .cell--edit {
    cursor: default;
    align-items: stretch;
    justify-content: flex-start;
    min-height: 0;
    gap: 4px;
  }

  .cell--active {
    border-color: var(--accent, #2463eb);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent, #2463eb) 22%, transparent);
  }

  .cell__pick {
    border: 0;
    background: transparent;
    cursor: pointer;
    padding: 4px;
    min-height: 56px;
  }

  .cell__placeholder {
    color: #667085;
    font-size: 0.78rem;
  }

  .cell__symbol {
    width: 72px;
    height: 72px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .cell__symbol :global(svg) {
    width: 100%;
    height: 100%;
  }

  .cell__label {
    font-size: 1rem;
    font-weight: 600;
    text-align: center;
  }

  .cell__action {
    font-size: 0.68rem;
    color: #475467;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .edit-field {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .edit-field input,
  .edit-field select {
    font-size: 0.8rem;
    padding: 3px 4px;
  }

  .mini {
    border: 1px solid #cfd6e2;
    border-radius: 6px;
    background: #f8fafc;
    cursor: pointer;
    font-size: 0.72rem;
    color: #1b2330;
    padding: 3px 6px;
  }

  .mini--danger:hover {
    background: #fdecec;
    border-color: #d64545;
  }

  .communicator__mode { display: inline-flex; gap: 6px; flex-wrap: wrap; }

  .communicator__message {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
    padding: 10px;
    border-top: 2px solid var(--border, #d3d3da);
    background: var(--surface, #fff);
  }

  .message {
    flex: 1 1 320px;
    min-height: 52px;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    padding: 8px;
    border: 1px dashed var(--border, #b9b9c0);
    border-radius: 10px;
  }

  .chip {
    padding: 4px 10px;
    border-radius: 999px;
    background: #e8eefc;
    color: #12234a;
    font-weight: 600;
  }

  .chip--removable {
    border: 0;
    cursor: pointer;
    font: inherit;
  }

  .communicator__actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  @media (max-width: 640px) {
    .communicator__grid {
      grid-template-columns: repeat(auto-fit, minmax(96px, 1fr));
    }
  }
</style>
