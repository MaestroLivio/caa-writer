<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "../api";
  import PackageManager from "./PackageManager.svelte";
  import type { CatalogItem, CatalogOverview, CatalogPage } from "../types";

  interface Props {
    onOpen: (id: string, sourceType: string, readOnly: boolean) => void;
    onClose: () => void;
  }

  let { onOpen, onClose }: Props = $props();

  const FAVORITES_KEY = "caa.library.favorites.v2";
  const RECENTS_KEY = "caa.library.recents.v2";

  type Stored = { id: string; title: string; source_type: string; at: number };

  function readStored(key: string): Stored[] {
    try {
      const raw = localStorage.getItem(key);
      if (!raw) return [];
      const parsed = JSON.parse(raw);
      return Array.isArray(parsed)
        ? parsed.filter((item) => item && typeof item.id === "string")
        : [];
    } catch {
      return [];
    }
  }

  function writeStored(key: string, value: Stored[]) {
    try {
      localStorage.setItem(key, JSON.stringify(value.slice(0, 40)));
    } catch {
      /* lo spazio locale può essere pieno: la libreria resta usabile */
    }
  }

  let favorites = $state<Stored[]>([]);
  let recents = $state<Stored[]>([]);
  let overview = $state<CatalogOverview | null>(null);
  let page = $state<CatalogPage | null>(null);
  let personal = $state<CatalogItem[]>([]);
  let category = $state("tutti");
  let subcategory = $state("");
  let query = $state("");
  let audience = $state("tutte");
  let interactiveOnly = $state(false);
  let symbolsOnly = $state(false);
  let sort = $state("rilevanza");
  let compact = $state(false);
  let loading = $state(true);
  let error = $state("");
  let offset = $state(0);
  let timer: ReturnType<typeof setTimeout> | null = null;
  let packagesOpen = $state(false);

  const selectedCategory = $derived(
    overview?.categories.find((item) => item.id === category) ?? null
  );
  const favoriteIds = $derived(new Set(favorites.map((item) => item.id)));
  const personalCount = $derived(
    new Set([...favorites.map((item) => item.id), ...recents.map((item) => item.id)]).size
  );

  onMount(() => {
    favorites = readStored(FAVORITES_KEY);
    recents = readStored(RECENTS_KEY);
    void (async () => {
      try {
        overview = await api.libraryCatalog();
      } catch (err) {
        error = err instanceof Error ? err.message : String(err);
      }
      await runSearch(true);
    })();
  });

  /// Dopo aver attivato o sospeso un pacchetto i conteggi e i risultati
  /// devono riflettere subito la scelta, senza riavviare l'app.
  async function reloadAfterPackages() {
    try {
      overview = await api.libraryCatalog();
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    }
    if (category === "personali") await loadPersonal();
    else await runSearch(true);
  }

  async function runSearch(reset = true) {
    if (category === "personali") {
      await loadPersonal();
      return;
    }
    loading = true;
    error = "";
    const nextOffset = reset ? 0 : offset;
    try {
      page = await api.librarySearch({
        query: query.trim() || undefined,
        category: category === "tutti" ? undefined : category,
        subcategory: subcategory || undefined,
        audience: audience === "tutte" ? undefined : audience,
        interactive: interactiveOnly ? true : undefined,
        needs_symbols: symbolsOnly ? true : undefined,
        sort,
        offset: nextOffset,
        limit: 24,
      });
      offset = nextOffset;
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
      page = null;
    } finally {
      loading = false;
    }
  }

  async function loadPersonal() {
    loading = true;
    error = "";
    const ids = [
      ...new Set([...favorites.map((item) => item.id), ...recents.map((item) => item.id)]),
    ];
    try {
      personal = ids.length ? await api.libraryByIds(ids) : [];
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
      personal = [];
    } finally {
      loading = false;
    }
  }

  function selectCategory(id: string) {
    category = id;
    subcategory = "";
    offset = 0;
    if (id === "personali") {
      void loadPersonal();
    } else {
      void runSearch(true);
    }
  }

  function selectSubcategory(id: string) {
    subcategory = subcategory === id ? "" : id;
    offset = 0;
    void runSearch(true);
  }

  function onQueryInput() {
    if (timer) clearTimeout(timer);
    timer = setTimeout(() => void runSearch(true), 260);
  }

  function applyFilter() {
    offset = 0;
    void runSearch(true);
  }

  function changePage(delta: number) {
    if (!page) return;
    const next = Math.max(0, Math.min(page.total - 1, page.offset + delta * page.limit));
    if (next === page.offset) return;
    offset = next;
    void runSearch(false);
  }

  function remember(item: { id: string; title: string; source_type: string }, key: "fav" | "recent") {
    const entry: Stored = {
      id: item.id,
      title: item.title,
      source_type: item.source_type,
      at: Date.now(),
    };
    if (key === "fav") {
      const exists = favorites.some((value) => value.id === item.id);
      favorites = exists
        ? favorites.filter((value) => value.id !== item.id)
        : [entry, ...favorites];
      writeStored(FAVORITES_KEY, favorites);
    } else {
      recents = [entry, ...recents.filter((value) => value.id !== item.id)];
      writeStored(RECENTS_KEY, recents);
    }
  }

  function open(item: CatalogItem, readOnly: boolean) {
    remember(item, "recent");
    onOpen(item.id, item.source_type, readOnly);
  }

  function categoryName(id: string): string {
    return overview?.categories.find((item) => item.id === id)?.name ?? id;
  }
</script>

<div class="library">
  <header class="library__top">
    <div class="library__brand">
      <button class="btn btn--ghost" onclick={onClose} aria-label="Torna alla schermata iniziale">‹ Home</button>
      <div>
        <h1>Libreria dei materiali</h1>
        <p>Modelli originali, organizzati per categoria, obiettivo e tipo di attività.</p>
      </div>
    </div>
    <div class="library__search">
      <label class="sr-only" for="library-query">Cerca nella libreria</label>
      <input
        id="library-query"
        type="search"
        placeholder="Cerca per nome, obiettivo, competenza o sinonimo…"
        bind:value={query}
        oninput={onQueryInput}
      />
      {#if query}
        <button class="btn btn--ghost" onclick={() => { query = ""; applyFilter(); }} aria-label="Cancella ricerca">×</button>
      {/if}
    </div>
    <button
      class="btn"
      aria-pressed={compact}
      onclick={() => (compact = !compact)}
      title="Passa dalla griglia visiva all'elenco compatto"
    >{compact ? "Griglia" : "Elenco"}</button>
    <button class="btn btn--solid" onclick={() => (packagesOpen = true)}>Pacchetti</button>
  </header>

  <div class="library__body">
    <aside class="library__side" aria-label="Categorie">
      <button
        class="side-item"
        class:side-item--active={category === "tutti"}
        onclick={() => selectCategory("tutti")}
      >
        <span>Tutti i materiali</span>
        <span class="side-count">{overview?.total ?? "…"}</span>
      </button>
      {#if overview}
        {#each overview.categories as item (item.id)}
          {#if item.id !== "personali"}
            <button
              class="side-item"
              class:side-item--active={category === item.id}
              onclick={() => selectCategory(item.id)}
            >
              <span>{item.name}</span>
              <span class="side-count">{item.count}</span>
            </button>
            {#if category === item.id && item.subcategories.length}
              <div class="subcategories">
                {#each item.subcategories as sub (sub.id)}
                  {#if sub.count > 0}
                    <button
                      class="sub-item"
                      class:sub-item--active={subcategory === sub.id}
                      title={sub.description}
                      onclick={() => selectSubcategory(sub.id)}
                    >{sub.name} <span>{sub.count}</span></button>
                  {/if}
                {/each}
              </div>
            {/if}
          {/if}
        {/each}
        <button
          class="side-item"
          class:side-item--active={category === "personali"}
          onclick={() => selectCategory("personali")}
        >
          <span>Documenti personali</span>
          <span class="side-count">{personalCount}</span>
        </button>
      {/if}
      <div class="library__filters">
        <h2>Filtri</h2>
        <label>Pubblico
          <select bind:value={audience} onchange={applyFilter}>
            <option value="tutte">Tutti</option>
            <option value="infanzia">Infanzia</option>
            <option value="primaria">Primaria</option>
            <option value="secondaria">Secondaria</option>
            <option value="adulti">Adulti</option>
          </select>
        </label>
        <label class="check"><input type="checkbox" bind:checked={interactiveOnly} onchange={applyFilter} /> Solo interattivi</label>
        <label class="check"><input type="checkbox" bind:checked={symbolsOnly} onchange={applyFilter} /> Usa pittogrammi</label>
        <label>Ordina
          <select bind:value={sort} onchange={applyFilter}>
            <option value="rilevanza">Rilevanza</option>
            <option value="nome">Nome</option>
            <option value="pagine">Numero di pagine</option>
            <option value="complessita">Complessità</option>
          </select>
        </label>
      </div>
    </aside>

    <main class="library__main">
      <nav class="breadcrumb" aria-label="Percorso">
        <button onclick={() => selectCategory("tutti")}>Libreria</button>
        {#if category !== "tutti"}
          <span aria-hidden="true">›</span>
          <button onclick={() => selectCategory(category)}>{category === "personali" ? "Documenti personali" : categoryName(category)}</button>
        {/if}
        {#if subcategory}
          <span aria-hidden="true">›</span>
          <span>{selectedCategory?.subcategories.find((sub) => sub.id === subcategory)?.name ?? subcategory}</span>
        {/if}
      </nav>

      <div class="library__heading">
        <div>
          <h2>
            {#if category === "personali"}
              Documenti personali
            {:else if selectedCategory}
              {selectedCategory.name}
            {:else}
              Tutti i materiali
            {/if}
          </h2>
          <p>
            {#if category === "personali"}
              Preferiti e materiali aperti di recente su questo dispositivo.
            {:else if selectedCategory}
              {selectedCategory.description}
            {:else}
              Scegli una categoria o cerca per obiettivo e competenza.
            {/if}
          </p>
        </div>
        <span class="result-count">
          {#if category === "personali"}{personalCount}{:else if page}{page.total}{:else}…{/if} materiali
        </span>
      </div>

      {#if error}
        <p class="notice notice--error" role="alert">{error}</p>
      {:else if loading}
        <p class="notice">Caricamento della libreria…</p>
      {:else if category === "personali"}
        {#if personal.length === 0}
          <div class="empty">
            <h3>Nessun documento personale</h3>
            <p>Apri un modello e premi la stella per ritrovarlo qui. I documenti aperti di recente compaiono automaticamente.</p>
          </div>
        {:else}
          <div class="cards" class:cards--compact={compact}>
            {#each personal as item (item.id)}
              {@render materialCard(item)}
            {/each}
          </div>
        {/if}
      {:else if overview && overview.categories.filter((item) => item.id !== "personali").length === 0}
        <div class="empty">
          <h3>Libreria materiali non installata</h3>
          <p>
            Questa installazione è snella: i modelli e le tabelle della libreria non sono inclusi
            nel pacchetto. L'app funziona normalmente; apri Pacchetti per scaricare i repertori o
            consultare la pagina della release.
          </p>
          <button class="btn btn--solid" onclick={() => (packagesOpen = true)}>Apri Pacchetti</button>
        </div>
      {:else if page && page.items.length === 0}
        <div class="empty">
          <h3>Nessun risultato</h3>
          <p>Prova con un sinonimo, togli un filtro o scegli un'altra categoria.</p>
          <button class="btn" onclick={() => { query = ""; audience = "tutte"; interactiveOnly = false; symbolsOnly = false; subcategory = ""; applyFilter(); }}>Azzera i filtri</button>
        </div>
      {:else if page}
        <div class="cards" class:cards--compact={compact}>
          {#each page.items as item (item.id)}
            {@render materialCard(item)}
          {/each}
        </div>
        {#if page.total > page.limit}
          <div class="pager">
            <button class="btn" disabled={page.offset === 0} onclick={() => changePage(-1)}>‹ Precedenti</button>
            <span>{page.offset + 1}–{Math.min(page.offset + page.limit, page.total)} di {page.total}</span>
            <button class="btn" disabled={page.offset + page.limit >= page.total} onclick={() => changePage(1)}>Successivi ›</button>
          </div>
        {/if}
      {/if}
    </main>
  </div>
</div>

{#if packagesOpen}
  <PackageManager
    onClose={() => (packagesOpen = false)}
    onChanged={() => void reloadAfterPackages()}
  />
{/if}

{#snippet materialCard(item: CatalogItem)}
  <article class="card" class:card--compact={compact}>
    <div class="preview">
      {#if item.preview_svg}
        {@html item.preview_svg}
      {:else}
        <span class="preview__missing">Anteprima non disponibile</span>
      {/if}
    </div>
    <div class="card__body">
      <div class="card__kicker">
        <span>{item.classification.subcategory}</span>
        {#if item.featured}<span class="badge badge--accent">Consigliato</span>{/if}
        {#if item.classification.interactive}<span class="badge">Interattivo</span>{/if}
      </div>
      <h3>{item.title}</h3>
      <p class="card__desc">{item.description}</p>
      <p class="card__objective">{item.classification.objective}</p>
      <div class="card__meta">
        <span>{item.pages} {item.pages === 1 ? "pagina" : "pagine"}</span>
        <span>{item.classification.audience}</span>
        <span>{item.classification.complexity}</span>
        {#if item.classification.needs_symbols}<span>pittogrammi</span>{/if}
        {#if item.missing_symbols}<span class="badge badge--warn">{item.missing_symbols} simboli mancanti</span>{/if}
      </div>
      <div class="card__actions">
        <button class="btn btn--solid" onclick={() => open(item, false)}>Apri</button>
        <button class="btn" onclick={() => open(item, true)}>Anteprima</button>
        <button
          class="star"
          aria-pressed={favoriteIds.has(item.id)}
          aria-label={favoriteIds.has(item.id) ? `Rimuovi ${item.title} dai preferiti` : `Aggiungi ${item.title} ai preferiti`}
          title="Preferito"
          onclick={() => remember(item, "fav")}
        >{favoriteIds.has(item.id) ? "★" : "☆"}</button>
      </div>
    </div>
  </article>
{/snippet}

<style>
  .library {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
    background: var(--bg, #f6f8fb);
    color: #0f172a;
  }
  .library__top {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    align-items: center;
    justify-content: space-between;
    padding: 14px clamp(14px, 3vw, 32px);
    background: #fff;
    border-bottom: 1px solid #dbe3ee;
    position: sticky;
    top: 0;
    z-index: 5;
  }
  .library__brand { display: flex; gap: 12px; align-items: center; }
  .library__brand h1 { margin: 0; font-size: 19px; }
  .library__brand p { margin: 2px 0 0; color: #64748b; font-size: 12px; }
  .library__search { flex: 1 1 320px; display: flex; gap: 6px; max-width: 620px; }
  .library__search input {
    flex: 1;
    min-width: 160px;
    padding: 9px 12px;
    border: 1px solid #cbd5e1;
    border-radius: 10px;
    font: inherit;
    background: #f8fafc;
  }
  .library__body { flex: 1; display: grid; grid-template-columns: 260px minmax(0, 1fr); gap: 0; }
  .library__side {
    padding: 16px 12px 24px;
    border-right: 1px solid #dbe3ee;
    background: #fff;
    max-height: calc(100vh - 74px);
    overflow: auto;
  }
  .side-item, .sub-item {
    width: 100%;
    display: flex;
    justify-content: space-between;
    gap: 8px;
    border: 0;
    background: transparent;
    padding: 8px 9px;
    border-radius: 8px;
    font: inherit;
    font-size: 13px;
    text-align: left;
    cursor: pointer;
    color: #1e293b;
  }
  .side-item:hover, .sub-item:hover { background: #f1f5f9; }
  .side-item--active { background: #0f766e; color: #fff; font-weight: 700; }
  .side-count { opacity: 0.72; font-size: 11px; }
  .subcategories { padding: 2px 0 8px 10px; border-left: 2px solid #e2e8f0; margin-left: 8px; }
  .sub-item { font-size: 12px; padding: 5px 7px; color: #475569; }
  .sub-item--active { background: #ccfbf1; color: #115e59; font-weight: 700; }
  .library__filters { margin-top: 18px; padding-top: 14px; border-top: 1px solid #e2e8f0; display: grid; gap: 8px; }
  .library__filters h2 { margin: 0 0 2px; font-size: 12px; text-transform: uppercase; letter-spacing: 0.08em; color: #64748b; }
  .library__filters label { display: grid; gap: 4px; font-size: 12px; color: #475569; }
  .library__filters select { padding: 6px 8px; border: 1px solid #cbd5e1; border-radius: 8px; font: inherit; background: #fff; }
  .library__filters .check { display: flex; gap: 7px; align-items: center; }
  .library__main { padding: 18px clamp(14px, 3vw, 32px) 30px; min-width: 0; }
  .breadcrumb { display: flex; gap: 6px; align-items: center; font-size: 12px; color: #64748b; flex-wrap: wrap; }
  .breadcrumb button { border: 0; background: none; color: #0f766e; font: inherit; cursor: pointer; padding: 0; }
  .library__heading { display: flex; justify-content: space-between; gap: 16px; align-items: flex-start; margin: 8px 0 14px; }
  .library__heading h2 { margin: 0; font-size: 24px; }
  .library__heading p { margin: 4px 0 0; color: #64748b; font-size: 13px; max-width: 760px; }
  .result-count { white-space: nowrap; font-size: 12px; color: #64748b; padding-top: 6px; }
  .cards { display: grid; grid-template-columns: repeat(auto-fill, minmax(270px, 1fr)); gap: 16px; }
  .cards--compact { grid-template-columns: 1fr; gap: 8px; }
  .card {
    background: #fff;
    border: 1px solid #dbe3ee;
    border-radius: 14px;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    transition: box-shadow 140ms ease, transform 140ms ease;
  }
  .card:hover { box-shadow: 0 8px 24px #0f172a1a; transform: translateY(-1px); }
  .card--compact { flex-direction: row; }
  .card--compact .preview { width: 180px; min-width: 180px; height: auto; }
  .preview {
    aspect-ratio: 4 / 3;
    background: #eef2f7;
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
    border-bottom: 1px solid #e2e8f0;
  }
  .preview :global(svg) { width: 100%; height: 100%; display: block; }
  .preview__missing { color: #64748b; font-size: 12px; }
  .card__body { padding: 12px 14px 14px; display: flex; flex-direction: column; gap: 6px; flex: 1; }
  .card__kicker { display: flex; flex-wrap: wrap; gap: 6px; align-items: center; font-size: 11px; color: #0f766e; font-weight: 700; text-transform: uppercase; letter-spacing: 0.05em; }
  .badge { display: inline-flex; align-items: center; border-radius: 999px; background: #e2e8f0; color: #334155; padding: 2px 7px; font-size: 10px; font-weight: 700; letter-spacing: 0; text-transform: none; }
  .badge--accent { background: #ccfbf1; color: #115e59; }
  .badge--warn { background: #fef3c7; color: #92400e; }
  .card h3 { margin: 0; font-size: 16px; line-height: 1.25; }
  .card__desc { margin: 0; font-size: 12.5px; color: #475569; line-height: 1.4; }
  .card__objective { margin: 0; font-size: 11.5px; color: #0f766e; }
  .card__meta { display: flex; flex-wrap: wrap; gap: 6px; font-size: 11px; color: #64748b; margin-top: auto; }
  .card__meta > span:not(.badge)::after { content: "·"; margin-left: 6px; color: #cbd5e1; }
  .card__meta > span:last-child::after { content: ""; }
  .card__actions { display: flex; gap: 6px; align-items: center; margin-top: 4px; }
  .star { margin-left: auto; border: 0; background: none; font-size: 22px; cursor: pointer; color: #f59e0b; padding: 0 4px; }
  .pager { display: flex; justify-content: center; align-items: center; gap: 12px; margin-top: 22px; font-size: 12px; color: #64748b; }
  .notice { padding: 14px; border-radius: 10px; background: #fff; border: 1px solid #dbe3ee; color: #475569; }
  .notice--error { border-color: #fecaca; background: #fef2f2; color: #991b1b; }
  .empty { background: #fff; border: 1px dashed #cbd5e1; border-radius: 14px; padding: 34px; text-align: center; color: #475569; }
  .empty h3 { margin: 0 0 6px; color: #0f172a; }
  .empty p { margin: 0 0 14px; }
  .btn { border: 1px solid #cbd5e1; background: #fff; color: #1e293b; border-radius: 8px; padding: 7px 11px; font: inherit; font-size: 12px; cursor: pointer; }
  .btn:hover { background: #f1f5f9; }
  .btn--solid { background: #0f766e; border-color: #0f766e; color: #fff; font-weight: 700; }
  .btn--solid:hover { background: #115e59; }
  .btn--ghost { border-color: transparent; background: transparent; }
  .btn:disabled { opacity: 0.45; cursor: not-allowed; }
  .sr-only { position: absolute; width: 1px; height: 1px; overflow: hidden; clip: rect(0 0 0 0); white-space: nowrap; }
  @media (max-width: 900px) {
    .library__body { grid-template-columns: 1fr; }
    .library__side { max-height: none; border-right: 0; border-bottom: 1px solid #dbe3ee; }
    .card--compact { flex-direction: column; }
    .card--compact .preview { width: 100%; }
  }
  @media (prefers-reduced-motion: reduce) {
    .card { transition: none; }
  }
</style>
