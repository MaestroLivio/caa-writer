<script lang="ts">
  import { api, isTauri } from "../api";
  import type { Alternative, FreeBoardCell, FreeBoardData } from "../types";

  interface Props {
    packId: string | null;
    commercialOnly: boolean;
    onToast: (message: string, kind?: "info" | "warn" | "error") => void;
    onExit: () => void;
    initial?: FreeBoardData | null;
    onSave?: (data: FreeBoardData) => Promise<void>;
  }

  let { packId, commercialOnly, onToast, onExit, initial = null, onSave }: Props = $props();

  type Cell = FreeBoardCell;

  let title = $state(initial?.title ?? "Progetto libero");
  let columns = $state(initial?.columns ?? 4);
  let cells = $state<Cell[]>(initial ? [...initial.cells] : []);
  let saving = $state(false);
  let query = $state("");
  let results = $state<Alternative[]>([]);
  let busy = $state(false);
  let printed = false;

  let seq = 0;
  const uid = () => `c${Date.now().toString(36)}${(seq += 1)}`;

  async function search() {
    const text = query.trim();
    if (!text) return;
    if (!isTauri()) {
      onToast("La ricerca dei pittogrammi richiede l'app desktop.", "warn");
      return;
    }
    busy = true;
    try {
      const page = await api.searchSymbols(text, 24, 0, packId, commercialOnly);
      results = page.items;
      if (page.items.length === 0) onToast("Nessun pittogramma trovato per questa ricerca.", "warn");
    } catch (error) {
      onToast(error instanceof Error ? error.message : String(error), "error");
    } finally {
      busy = false;
    }
  }

  function add(alternative: Alternative) {
    cells = [
      ...cells,
      {
        id: uid(),
        label: alternative.label,
        svg: alternative.svg,
        image: alternative.image ?? null,
        symbolId: alternative.symbol_id,
        conceptId: alternative.concept_id,
        approximate: alternative.approximate,
      },
    ];
  }

  function addBlank() {
    cells = [
      ...cells,
      { id: uid(), label: "", svg: null, image: null, symbolId: null, conceptId: null, approximate: false },
    ];
  }

  function remove(id: string) {
    cells = cells.filter((cell) => cell.id !== id);
  }

  function move(id: string, delta: number) {
    const index = cells.findIndex((cell) => cell.id === id);
    const target = index + delta;
    if (index < 0 || target < 0 || target >= cells.length) return;
    const next = [...cells];
    [next[index], next[target]] = [next[target], next[index]];
    cells = next;
  }

  async function save() {
    if (!onSave) {
      onToast("Il salvataggio richiede l'app desktop.", "warn");
      return;
    }
    saving = true;
    try {
      await onSave({ title, columns, cells });
    } finally {
      saving = false;
    }
  }

  async function print() {
    if (cells.length === 0) {
      onToast("Aggiungi almeno un elemento prima di stampare.", "warn");
      return;
    }
    document.body.dataset.print = "board";
    printed = true;
    await new Promise((resolve) => requestAnimationFrame(() => resolve(null)));
    try {
      if (isTauri()) {
        await api.printWindow();
      } else {
        window.print();
      }
    } catch (error) {
      onToast(error instanceof Error ? error.message : String(error), "error");
    }
  }

  $effect(() => {
    if (!printed) return;
    const after = () => {
      delete document.body.dataset.print;
      printed = false;
    };
    window.addEventListener("afterprint", after);
    return () => window.removeEventListener("afterprint", after);
  });
</script>

<div class="board">
  <header class="board__top board__ui">
    <button class="btn" onclick={onExit}>← Home</button>
    <label class="board__title">
      <span class="hint">Titolo del foglio</span>
      <input bind:value={title} aria-label="Titolo del foglio" />
    </label>
    <label class="board__cols">
      <span class="hint">Colonne</span>
      <input type="range" min="1" max="8" bind:value={columns} aria-label="Numero di colonne" />
      <span>{columns}</span>
    </label>
    <div class="board__spacer"></div>
    <button class="btn" onclick={addBlank}>+ Casella vuota</button>
    <button class="btn" disabled={saving} onclick={() => void save()}>
      {saving ? "Salvo…" : "Salva .caadoc"}
    </button>
    <button class="btn btn--solid" onclick={print}>Stampa / PDF</button>
  </header>

  <div class="board__search board__ui">
    <form
      onsubmit={(event) => {
        event.preventDefault();
        void search();
      }}
    >
      <input
        type="search"
        bind:value={query}
        placeholder="Cerca un pittogramma da aggiungere…"
        aria-label="Cerca un pittogramma da aggiungere"
      />
      <button class="btn btn--solid" type="submit" disabled={busy}>Cerca</button>
    </form>
    {#if results.length > 0}
      <div class="board__results">
        {#each results as alternative (alternative.symbol_id)}
          <button
            class="pick"
            title={`${alternative.meaning || alternative.label} — ${alternative.pack_name} (${alternative.license})`}
            onclick={() => add(alternative)}
          >
            {#if alternative.image}
              <span class="pick__media"><img src={alternative.image} alt="" /></span>
            {:else if alternative.svg}
              <span class="pick__media">{@html alternative.svg}</span>
            {:else}
              <span class="caa-chip__empty">?</span>
            {/if}
            <span class="pick__label">{alternative.label}</span>
          </button>
        {/each}
      </div>
    {/if}
    <p class="hint">
      Clic su un risultato per aggiungerlo al foglio. Le frecce di ogni casella cambiano l'ordine.
    </p>
  </div>

  <main class="sheet">
    <h1 class="sheet__title">{title}</h1>
    {#if cells.length === 0}
      <p class="sheet__empty">Foglio vuoto: cerca un pittogramma qui sopra oppure aggiungi una casella vuota.</p>
    {:else}
      <div class="sheet__grid" style={`grid-template-columns: repeat(${columns}, minmax(0, 1fr));`}>
        {#each cells as cell (cell.id)}
          <div class="cell">
            <div class="cell__media">
              {#if cell.image}
                <img src={cell.image} alt="" />
              {:else if cell.svg}
                {@html cell.svg}
              {:else}
                <span class="caa-chip__empty">?</span>
              {/if}
            </div>
            <input class="cell__label" bind:value={cell.label} aria-label="Didascalia della casella" />
            {#if cell.approximate}
              <span class="cell__badge" title="Pittogramma approssimativo">≈</span>
            {/if}
            <div class="cell__tools board__ui">
              <button class="mini" aria-label="Sposta indietro" onclick={() => move(cell.id, -1)}>←</button>
              <button class="mini" aria-label="Sposta avanti" onclick={() => move(cell.id, 1)}>→</button>
              <button class="mini mini--danger" aria-label="Rimuovi casella" onclick={() => remove(cell.id)}>×</button>
            </div>
          </div>
        {/each}
      </div>
    {/if}
    <footer class="sheet__foot">Creato con CAA Writer · Pittogrammi: vedere il registro delle licenze.</footer>
  </main>

  <p class="hint board__ui board__note">
    Il foglio libero si salva in <code>.caadoc</code> con titolo, colonne, celle e pittogrammi
    incorporati, e si riapre dallo stesso file. La stampa/PDF resta disponibile.
  </p>
</div>

<style>
  .board {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 18px clamp(14px, 4vw, 40px) 28px;
    background: var(--bg, #f7f8fa);
  }

  .board__top {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    align-items: flex-end;
  }

  .board__title {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 240px;
  }

  .board__title input {
    font-size: 1rem;
    font-weight: 600;
  }

  .board__cols {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .board__cols input[type="range"] {
    width: 110px;
  }

  .board__spacer {
    flex: 1;
  }

  .board__search form {
    display: flex;
    gap: 8px;
    max-width: 620px;
  }

  .board__search form input {
    flex: 1;
  }

  .board__results {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-top: 10px;
  }

  .pick {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    width: 86px;
    padding: 8px 6px;
    border: 1px solid var(--line, #d8dde6);
    border-radius: 10px;
    background: var(--panel, #fff);
    cursor: pointer;
    color: inherit;
  }

  .pick:hover,
  .pick:focus-visible {
    border-color: #2f6fed;
  }

  .pick__media :global(svg),
  .pick__media :global(img) {
    width: 44px;
    height: 44px;
  }

  .pick__label {
    font-size: 0.74rem;
    text-align: center;
    line-height: 1.2;
  }

  .sheet {
    flex: 1;
    background: #fff;
    color: #111;
    border: 1px solid var(--line, #d8dde6);
    border-radius: 10px;
    padding: 22px;
    display: flex;
    flex-direction: column;
    gap: 14px;
    max-width: 1180px;
    width: 100%;
    margin: 0 auto;
  }

  .sheet__title {
    margin: 0;
    font-size: 1.3rem;
  }

  .sheet__empty {
    margin: 0;
    color: #5b6472;
    font-size: 0.9rem;
  }

  .sheet__grid {
    display: grid;
    gap: 10px;
  }

  .cell {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    border: 2px solid #38414f;
    border-radius: 8px;
    padding: 10px 8px 6px;
    min-height: 130px;
  }

  .cell__media {
    display: grid;
    place-items: center;
    width: 100%;
    min-height: 72px;
  }

  .cell__media :global(svg),
  .cell__media :global(img) {
    width: 72px;
    height: 72px;
  }

  .cell__label {
    width: 100%;
    border: 0;
    border-top: 1px dashed #cfd6e2;
    text-align: center;
    font-size: 0.86rem;
    padding: 4px 2px 0;
    background: transparent;
  }

  .cell__label:focus {
    outline: 2px solid #2f6fed;
    outline-offset: 1px;
  }

  .cell__badge {
    position: absolute;
    top: 4px;
    right: 6px;
    color: #8a5a00;
    font-weight: 700;
  }

  .cell__tools {
    display: flex;
    gap: 4px;
  }

  .mini {
    width: 26px;
    height: 24px;
    border: 1px solid #cfd6e2;
    border-radius: 6px;
    background: #f8fafc;
    cursor: pointer;
    font-size: 0.78rem;
    color: #1b2330;
  }

  .mini--danger:hover {
    background: #fdecec;
    border-color: #d64545;
  }

  .sheet__foot {
    margin-top: auto;
    font-size: 0.72rem;
    color: #5b6472;
  }

  .board__note {
    max-width: 1180px;
    margin: 0 auto;
  }

  @media print {
    .board__ui {
      display: none !important;
    }

    .board {
      padding: 0;
      background: #fff;
    }

    .sheet {
      border: 0;
      padding: 0;
      max-width: none;
    }
  }
</style>
