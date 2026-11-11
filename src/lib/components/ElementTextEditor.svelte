<script lang="ts">
  // Modifica del testo CAA dentro un elemento dell'editor universale.
  //
  // Non è un secondo editor: monta lo stesso `CaaEditor` del word processor,
  // quindi analisi, selezione, cursore, alternative, blocco e visibilità sono
  // esattamente quelli del testo libero. Al termine il testo e i token scelti
  // vengono salvati nell'elemento (`metadata.caa`), così restano modificabili
  // dopo il salvataggio e la riapertura.
  import { onMount, tick } from "svelte";
  import type { ComponentProps } from "svelte";
  import { api, isTauri } from "../api";
  import CaaEditor from "../editor/CaaEditor.svelte";
  import { compactTokens } from "../universal/element-caa";
  import type { StoredCaaText, StoredCaaToken } from "../universal/element-caa";
  import type { Alternative, CaaToken } from "../types";

  interface Props {
    title: string;
    text: string;
    stored: StoredCaaText | null;
    caa?: ComponentProps<typeof CaaEditor>;
    onCancel: () => void;
    onApply: (value: { text: string; tokens: StoredCaaToken[]; docJson: unknown }) => void;
    onToast?: (message: string, kind?: "info" | "warn" | "error") => void;
  }

  let { title, text, stored, caa, onCancel, onApply, onToast }: Props = $props();

  let editor = $state<CaaEditor | undefined>(undefined);
  let ready = $state(false);
  let paragraphTokens = new Map<string, CaaToken[]>();
  let selected = $state<{ pid: string; token: CaaToken } | null>(null);
  let searchQuery = $state("");
  let searchResults = $state<Alternative[]>([]);
  let searchBusy = $state(false);
  let searchError = $state("");

  onMount(() => {
    if (!caa) {
      void tick().then(() => (ready = true));
      return;
    }
    void tick().then(() => {
      ready = true;
      if (stored?.doc_json && editor) editor.setDocJson(stored.doc_json);
    });
  });

  function pick(pid: string, token: CaaToken) {
    selected = { pid, token };
    searchQuery = token.surface;
    searchResults = token.alternatives ?? [];
    searchError = "";
  }

  function sameToken(pid: string, index: number): boolean {
    return Boolean(selected && selected.pid === pid && selected.token.index === index);
  }

  async function search() {
    const query = searchQuery.trim();
    if (!query) {
      searchResults = selected?.token.alternatives ?? [];
      return;
    }
    if (!isTauri()) {
      searchError = "La ricerca dei pittogrammi richiede l'app desktop.";
      return;
    }
    searchBusy = true;
    searchError = "";
    try {
      const page = await api.searchSymbols(
        query,
        24,
        0,
        caa?.packId ?? null,
        caa?.commercialOnly ?? false
      );
      searchResults = page.items;
    } catch (error) {
      searchError = error instanceof Error ? error.message : String(error);
    } finally {
      searchBusy = false;
    }
  }

  function applyAlternative(alternative: Alternative) {
    if (!selected || !editor) return;
    const ok = editor.applyTokenSymbol(selected.pid, selected.token.index, {
      symbol_id: alternative.symbol_id,
      svg: alternative.svg,
      image: alternative.image ?? null,
      label: alternative.label,
      meaning: alternative.meaning,
      concept_id: alternative.concept_id,
      approximate: alternative.approximate,
    });
    if (!ok) onToast?.("Il pittogramma non è più applicabile: rianalizza il testo.", "warn");
    selected = null;
  }

  function toggleLock() {
    if (!selected || !editor) return;
    editor.applyTokenState(selected.pid, selected.token.index, {
      locked: !selected.token.locked,
    });
    selected = { ...selected, token: { ...selected.token, locked: !selected.token.locked } };
  }

  function toggleHidden() {
    if (!selected || !editor) return;
    editor.applyTokenState(selected.pid, selected.token.index, {
      hidden: !selected.token.hidden,
    });
    selected = { ...selected, token: { ...selected.token, hidden: !selected.token.hidden } };
  }

  /// Concatena i paragrafi in un unico testo e sposta gli indici dei token,
  /// così il rendering dell'elemento resta corretto anche con più righe.
  function flatten(): { text: string; tokens: CaaToken[] } {
    const paragraphs = editor?.getParagraphs() ?? [];
    if (paragraphs.length === 0) return { text: "", tokens: [] };
    const known = editor?.getTokens() ?? paragraphTokens;
    const tokens: CaaToken[] = [];
    let offset = 0;
    paragraphs.forEach((paragraph, index) => {
      if (index > 0) offset += 2;
      for (const token of known.get(paragraph.pid) ?? []) {
        tokens.push({ ...token, start: token.start + offset, end: token.end + offset });
      }
      offset += paragraph.text.length;
    });
    return {
      text: paragraphs.map((paragraph) => paragraph.text).join("\n\n"),
      tokens,
    };
  }

  function confirm() {
    if (!caa) {
      onApply({ text: (text ?? "").trim() ? text : "", tokens: [], docJson: null });
      return;
    }
    const result = flatten();
    onApply({
      text: result.text,
      tokens: compactTokens(result.tokens),
      docJson: editor?.getDocJson() ?? null,
    });
  }

  function handleKey(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.preventDefault();
      onCancel();
    }
  }
</script>

<svelte:window onkeydown={handleKey} />

<div class="ete" role="dialog" aria-modal="true" aria-labelledby="ete-title">
  <header class="ete__bar">
    <div>
      <h2 id="ete-title">{title}</h2>
      <p>
        Scrivi e modifica la frase: il cursore resta dentro le parole. Clicca un pittogramma per
        scegliere un'alternativa, bloccarlo o nasconderlo.
      </p>
    </div>
    <button class="btn btn--ghost" onclick={onCancel} aria-label="Chiudi senza salvare">×</button>
  </header>

  <div class="ete__body">
    <section class="ete__editor" aria-label="Testo dell'elemento">
      {#if caa}
        {#if ready}
          <CaaEditor
            bind:this={editor}
            {...caa}
            onAnalyzed={(map) => {
              paragraphTokens = map;
              caa?.onAnalyzed?.(map);
            }}
            onSelectToken={(pid, token) => pick(pid, token)}
          />
        {/if}
      {:else}
        <p class="ete__notice">
          La simbolizzazione richiede l'app desktop. Il testo resta modificabile, ma senza
          pittogrammi.
        </p>
      {/if}
    </section>

    <aside class="ete__aside" aria-label="Alternative del pittogramma">
      {#if selected}
        <div class="ete__selected">
          <strong>«{selected.token.surface}»</strong>
          <span class="ete__badge">{selected.token.symbol_label ?? selected.token.concept_label ?? "pittogramma"}</span>
        </div>
        <div class="ete__row">
          <button class="btn" class:btn--solid={selected.token.locked} onclick={toggleLock}>
            {selected.token.locked ? "Sblocca" : "Blocca"}
          </button>
          <button class="btn" class:btn--solid={selected.token.hidden} onclick={toggleHidden}>
            {selected.token.hidden ? "Mostra" : "Nascondi"}
          </button>
        </div>
        <form
          class="ete__search"
          onsubmit={(event) => {
            event.preventDefault();
            void search();
          }}
        >
          <input
            type="search"
            placeholder="Cerca un altro pittogramma…"
            aria-label="Cerca un altro pittogramma"
            bind:value={searchQuery}
          />
          <button class="btn" type="submit" disabled={searchBusy}>{searchBusy ? "…" : "Cerca"}</button>
        </form>
        {#if searchError}<p class="ete__error" role="alert">{searchError}</p>{/if}
        <ul class="ete__results">
          {#each searchResults as alternative (alternative.symbol_id)}
            <li>
              <button class="ete__result" onclick={() => applyAlternative(alternative)}>
                <span class="ete__thumb">
                  {#if alternative.image}
                    <img src={alternative.image} alt="" />
                  {:else if alternative.svg}
                    {@html alternative.svg}
                  {/if}
                </span>
                <span class="ete__label">{alternative.label}</span>
                <span class="ete__license">{alternative.license}</span>
              </button>
            </li>
          {:else}
            <li class="ete__empty">Nessuna alternativa: prova una ricerca.</li>
          {/each}
        </ul>
      {:else}
        <p class="ete__hint">
          Clicca il pittogramma di una parola per vedere le alternative. La sostituzione non cambia
          il testo.
        </p>
      {/if}
    </aside>
  </div>

  <footer class="ete__footer">
    <span class="ete__status">
      {#if paragraphTokens.size}
        {Array.from(paragraphTokens.values()).reduce((sum, list) => sum + list.length, 0)} parole analizzate
      {:else}
        In attesa dell'analisi…
      {/if}
    </span>
    <button class="btn" onclick={onCancel}>Annulla</button>
    <button class="btn btn--solid" onclick={confirm}>Applica all'elemento</button>
  </footer>
</div>

<style>
  .ete {
    position: fixed;
    inset: 0;
    z-index: 140;
    display: grid;
    grid-template-rows: auto minmax(0, 1fr) auto;
    background: #eef1f6;
    color: #0f172a;
  }
  .ete__bar {
    display: flex;
    justify-content: space-between;
    gap: 16px;
    align-items: flex-start;
    padding: 14px clamp(14px, 3vw, 28px);
    background: #fff;
    border-bottom: 1px solid #dbe3ee;
  }
  .ete__bar h2 { margin: 0; font-size: 18px; }
  .ete__bar p { margin: 4px 0 0; max-width: 780px; color: #475569; font-size: 12.5px; }
  .ete__body {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(260px, 320px);
    gap: 14px;
    padding: 14px clamp(14px, 3vw, 28px);
    min-height: 0;
  }
  .ete__editor {
    min-height: 0;
    overflow: auto;
    background: #fff;
    border: 1px solid #dbe3ee;
    border-radius: 12px;
    padding: 18px min(6vw, 46px);
  }
  .ete__aside {
    min-height: 0;
    overflow: auto;
    background: #fff;
    border: 1px solid #dbe3ee;
    border-radius: 12px;
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .ete__selected { display: flex; flex-wrap: wrap; gap: 8px; align-items: center; }
  .ete__badge {
    background: #e2e8f0;
    color: #334155;
    border-radius: 999px;
    padding: 2px 8px;
    font-size: 10.5px;
    font-weight: 700;
  }
  .ete__row { display: flex; gap: 8px; flex-wrap: wrap; }
  .ete__search { display: flex; gap: 6px; }
  .ete__search input { flex: 1; min-width: 0; }
  .ete__results { list-style: none; margin: 0; padding: 0; display: grid; gap: 6px; }
  .ete__result {
    width: 100%;
    display: grid;
    grid-template-columns: 44px minmax(0, 1fr);
    grid-template-rows: auto auto;
    gap: 2px 8px;
    align-items: center;
    text-align: left;
    border: 1px solid #e2e8f0;
    background: #fff;
    border-radius: 9px;
    padding: 6px;
    cursor: pointer;
  }
  .ete__result:hover { background: #f1f5f9; }
  .ete__thumb {
    grid-row: span 2;
    width: 44px;
    height: 44px;
    display: grid;
    place-items: center;
    overflow: hidden;
  }
  .ete__thumb :global(svg), .ete__thumb img { width: 100%; height: 100%; object-fit: contain; }
  .ete__label { font-size: 12.5px; font-weight: 600; }
  .ete__license { font-size: 10px; color: #64748b; }
  .ete__hint, .ete__empty, .ete__notice { color: #64748b; font-size: 12px; margin: 0; }
  .ete__error { color: #991b1b; font-size: 12px; margin: 0; }
  .ete__footer {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px clamp(14px, 3vw, 28px);
    background: #fff;
    border-top: 1px solid #dbe3ee;
  }
  .ete__status { margin-right: auto; color: #64748b; font-size: 12px; }
  @media (max-width: 760px) {
    .ete__body { grid-template-columns: minmax(0, 1fr); }
    .ete__aside { max-height: 40vh; }
  }
</style>
