<script lang="ts">
  import { onMount } from "svelte";
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import { listen } from "@tauri-apps/api/event";
  import { api } from "../api";
  import type { DeveloperDocument, DeveloperPackage, DeveloperStatus } from "../types";

  interface Props {
    onClose: () => void;
    onEdit: (path: string) => void;
    onChanged?: () => void;
    onToast?: (message: string, kind?: "info" | "warn" | "error") => void;
  }

  let { onClose, onEdit, onChanged, onToast }: Props = $props();

  let status = $state<DeveloperStatus | null>(null);
  let packages = $state<DeveloperPackage[]>([]);
  let loading = $state(true);
  let error = $state("");
  let notice = $state("");
  let busy = $state("");
  let query = $state("");
  let selectedPackageId = $state("");
  let selectedDocument = $state<DeveloperDocument | null>(null);
  let previewSvg = $state("");
  let previewLoading = $state(false);
  let previewError = $state("");
  let pendingDelete = $state<string | null>(null);
  let addOpen = $state(false);
  let addSource = $state("");
  let addFolder = $state<"boards" | "templates">("templates");
  let addCategoryId = $state("");
  let addSubcategoryId = $state("");
  let addCategoryName = $state("");
  let addSubcategoryName = $state("");
  let addBusy = $state(false);
  let addError = $state("");
  let reloadTimer: ReturnType<typeof setTimeout> | null = null;
  let unlistenChanges: (() => void) | null = null;

  const selectedPackage = $derived(
    packages.find((item) => item.id === selectedPackageId) ?? packages[0] ?? null
  );

  const documents = $derived.by(() => {
    if (!selectedPackage) return [];
    const needle = query.trim().toLowerCase();
    if (!needle) return selectedPackage.documents;
    return selectedPackage.documents.filter(
      (doc) =>
        doc.title.toLowerCase().includes(needle) ||
        doc.path.toLowerCase().includes(needle) ||
        doc.subcategory.toLowerCase().includes(needle) ||
        doc.kind.toLowerCase().includes(needle)
    );
  });

  const addSubcategories = $derived.by(() => {
    const pkg = packages.find((item) => item.id === addCategoryId);
    if (!pkg) return [];
    const map = new Map<string, { id: string; name: string; count: number }>();
    for (const doc of pkg.documents) {
      const key = doc.subcategory_id || "generale";
      const current = map.get(key);
      if (current) current.count += 1;
      else
        map.set(key, {
          id: key,
          name: doc.subcategory || "Generale",
          count: 1,
        });
    }
    return [...map.values()].sort((a, b) => a.name.localeCompare(b.name));
  });

  onMount(() => {
    void load();
    // Il bridge della GUI osserva resources/library e manda questo evento
    // quando un coding agent (CLI, MCP, editor esterno) modifica i pacchetti.
    void listen("developer-library-changed", () => {
      if (reloadTimer) clearTimeout(reloadTimer);
      reloadTimer = setTimeout(() => {
        void load();
      }, 250);
    })
      .then((unlisten) => {
        unlistenChanges = unlisten;
      })
      .catch(() => {
        /* anteprima browser o bridge non disponibile: resta il caricamento manuale */
      });
    return () => {
      if (reloadTimer) clearTimeout(reloadTimer);
      if (unlistenChanges) unlistenChanges();
    };
  });

  async function load() {
    loading = true;
    error = "";
    try {
      status = await api.developerLibraryStatus();
      if (!status.enabled) {
        error =
          "Modalità sviluppatore non attiva. Avvia CAA Writer con `npm run dev:library`.";
        packages = [];
        return;
      }
      packages = await api.developerLibraryPackages();
      if (!selectedPackageId && packages.length) selectedPackageId = packages[0].id;
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      loading = false;
    }
  }

  function selectPackage(id: string) {
    selectedPackageId = id;
    selectedDocument = null;
    previewSvg = "";
    previewError = "";
  }

  async function preview(doc: DeveloperDocument) {
    selectedDocument = doc;
    previewSvg = "";
    previewError = "";
    previewLoading = true;
    try {
      previewSvg = await api.developerLibraryPreview(doc.path);
    } catch (err) {
      previewError = err instanceof Error ? err.message : String(err);
    } finally {
      previewLoading = false;
    }
  }

  function closePreview() {
    selectedDocument = null;
    previewSvg = "";
    previewError = "";
  }

  async function replace(doc: DeveloperDocument) {
    const picked = await openDialog({
      title: `Sostituisci «${doc.title}»`,
      multiple: false,
      filters: [{ name: "Documento CAA Writer", extensions: ["caadoc"] }],
    });
    if (!picked || Array.isArray(picked)) return;
    busy = doc.path;
    error = "";
    notice = "";
    try {
      await api.developerLibraryReplace(doc.path, picked as string);
      notice = `«${doc.title}» sostituito con ${picked}.`;
      onToast?.(notice, "info");
      await load();
      onChanged?.();
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      busy = "";
    }
  }

  async function remove(doc: DeveloperDocument) {
    if (pendingDelete !== doc.path) {
      pendingDelete = doc.path;
      return;
    }
    pendingDelete = null;
    busy = doc.path;
    error = "";
    notice = "";
    try {
      await api.developerLibraryDelete(doc.path);
      notice = `«${doc.title}» eliminato dalla libreria.`;
      onToast?.(notice, "info");
      if (selectedDocument?.path === doc.path) closePreview();
      await load();
      onChanged?.();
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      busy = "";
    }
  }

  async function chooseAddSource() {
    const picked = await openDialog({
      title: "Scegli il documento .caadoc da aggiungere",
      multiple: false,
      filters: [{ name: "Documento CAA Writer", extensions: ["caadoc"] }],
    });
    if (!picked || Array.isArray(picked)) return;
    addSource = picked as string;
    addError = "";
  }

  function openAdd() {
    addOpen = true;
    addSource = "";
    addFolder = "templates";
    const first = packages[0];
    addCategoryId = first?.id ?? "";
    addCategoryName = first?.name ?? "";
    addSubcategoryId = "";
    addSubcategoryName = "";
    addError = "";
  }

  function onCategoryChange(value: string) {
    addCategoryId = value;
    const pkg = packages.find((item) => item.id === value);
    addCategoryName = pkg?.name ?? value;
    addSubcategoryId = "";
    addSubcategoryName = "";
  }

  function onSubcategoryChange(value: string) {
    addSubcategoryId = value;
    const sub = addSubcategories.find((item) => item.id === value);
    addSubcategoryName = sub?.name ?? value;
  }

  async function confirmAdd() {
    if (!addSource) {
      addError = "Scegli un file .caadoc.";
      return;
    }
    addBusy = true;
    addError = "";
    try {
      const result = await api.developerLibraryAdd(
        addSource,
        addFolder,
        addCategoryId,
        addSubcategoryId,
        addCategoryName,
        addSubcategoryName || "Generale"
      );
      notice = `Documento aggiunto: ${result.path}`;
      onToast?.(notice, "info");
      addOpen = false;
      await load();
      onChanged?.();
      if (result.entry?.category_id) selectedPackageId = result.entry.category_id;
    } catch (err) {
      addError = err instanceof Error ? err.message : String(err);
    } finally {
      addBusy = false;
    }
  }

  function formatPath(path: string): string {
    return path.replace(/^library\//, "");
  }
</script>

<div class="dev" role="dialog" aria-modal="true" aria-labelledby="dev-title">
  <header class="dev__top">
    <div class="dev__brand">
      <span class="dev__badge">DEV</span>
      <div>
        <h1 id="dev-title">Libreria — modalità sviluppatore</h1>
        <p>
          Aggiungi, modifica, sostituisci o elimina documenti dentro i pacchetti.
          Le modifiche sono scritte in <code>resources/library</code>.
        </p>
      </div>
    </div>
    <div class="dev__actions">
      <button class="btn btn--solid" onclick={openAdd}>Aggiungi documento</button>
      <button class="btn btn--ghost" onclick={onClose} aria-label="Chiudi">×</button>
    </div>
  </header>

  {#if error}
    <p class="notice notice--error" role="alert">{error}</p>
  {/if}
  {#if notice}
    <p class="notice notice--ok" role="status">{notice}</p>
  {/if}

  {#if loading}
    <p class="notice">Caricamento dei pacchetti…</p>
  {:else if !status?.enabled}
    <div class="empty">
      <h2>Modalità sviluppatore non attiva</h2>
      <p>
        Avvia l’app con <code>npm run dev:library</code> oppure con
        <code>tauri dev -- --dev-library</code>.
      </p>
      <p class="muted">La modalità funziona solo nella versione di sviluppo (debug).</p>
    </div>
  {:else}
    <div class="dev__body">
      <aside class="dev__side" aria-label="Pacchetti della libreria">
        <h2>Pacchetti</h2>
        {#each packages as pkg (pkg.id)}
          <button
            class="pkg"
            class:pkg--active={selectedPackage?.id === pkg.id}
            onclick={() => selectPackage(pkg.id)}
          >
            <span class="pkg__name">{pkg.name}</span>
            <span class="pkg__count">{pkg.count}</span>
          </button>
        {/each}
        <p class="dev__hint">
          {packages.length} pacchetti · {packages.reduce((sum, pkg) => sum + pkg.count, 0)} documenti
        </p>
      </aside>

      <main class="dev__main">
        <div class="dev__heading">
          <div>
            <h2>{selectedPackage?.name ?? "Pacchetto"}</h2>
            <p>{selectedPackage?.description ?? "Scegli un pacchetto."}</p>
          </div>
          <label class="dev__search">
            <span class="sr-only">Cerca documento</span>
            <input type="search" placeholder="Cerca per titolo, percorso, tipo…" bind:value={query} />
          </label>
        </div>

        {#if !selectedPackage}
          <p class="muted">Nessun pacchetto disponibile.</p>
        {:else if documents.length === 0}
          <p class="muted">Nessun documento corrisponde alla ricerca.</p>
        {:else}
          <div class="docs">
            {#each documents as doc (doc.path)}
              <article class="doc">
                <div class="doc__main">
                  <div class="doc__kicker">
                    <span class="tag">{doc.subcategory}</span>
                    <span class="tag tag--muted">{doc.kind}</span>
                    {#if doc.missing_symbols > 0}
                      <span class="tag tag--warn">{doc.missing_symbols} simboli mancanti</span>
                    {/if}
                  </div>
                  <h3>{doc.title}</h3>
                  <p class="doc__path">{formatPath(doc.path)}</p>
                  <p class="doc__meta">
                    {doc.pages} {doc.pages === 1 ? "pagina" : "pagine"} ·
                    {doc.elements} elementi
                  </p>
                </div>
                <div class="doc__actions">
                  <button class="btn" onclick={() => void preview(doc)}>Anteprima</button>
                  <button class="btn btn--solid" onclick={() => onEdit(doc.path)}>Modifica</button>
                  <button class="btn" disabled={busy === doc.path} onclick={() => void replace(doc)}>
                    {busy === doc.path ? "…" : "Sostituisci"}
                  </button>
                  <button
                    class="btn btn--danger"
                    disabled={busy === doc.path}
                    onclick={() => void remove(doc)}
                  >
                    {pendingDelete === doc.path ? "Conferma eliminazione" : "Elimina"}
                  </button>
                </div>
              </article>
            {/each}
          </div>
        {/if}
      </main>
    </div>
  {/if}

  {#if selectedDocument}
    <div class="preview" role="dialog" aria-modal="true" aria-labelledby="preview-title">
      <header class="preview__top">
        <div>
          <h2 id="preview-title">{selectedDocument.title}</h2>
          <p>{formatPath(selectedDocument.path)}</p>
        </div>
        <button class="btn btn--ghost" onclick={closePreview} aria-label="Chiudi anteprima">×</button>
      </header>
      <div class="preview__body">
        {#if previewLoading}
          <p class="notice">Generazione dell’anteprima…</p>
        {:else if previewError}
          <p class="notice notice--error" role="alert">{previewError}</p>
        {:else if previewSvg}
          <div class="preview__svg">{@html previewSvg}</div>
        {:else}
          <p class="muted">Anteprima non disponibile.</p>
        {/if}
      </div>
    </div>
  {/if}

  {#if addOpen}
    <div class="add" role="dialog" aria-modal="true" aria-labelledby="add-title">
      <header class="add__top">
        <div>
          <h2 id="add-title">Aggiungi un documento a un pacchetto</h2>
          <p>Il file viene copiato nella libreria e indicizzato nel catalogo.</p>
        </div>
        <button class="btn btn--ghost" onclick={() => (addOpen = false)} aria-label="Chiudi">×</button>
      </header>
      <div class="add__body">
        {#if addError}
          <p class="notice notice--error" role="alert">{addError}</p>
        {/if}
        <label class="field">
          <span>Documento .caadoc</span>
          <div class="field__row">
            <input readonly value={addSource} placeholder="Nessun file scelto" />
            <button class="btn" onclick={() => void chooseAddSource()}>Scegli file…</button>
          </div>
        </label>
        <label class="field">
          <span>Cartella</span>
          <select bind:value={addFolder}>
            <option value="templates">templates</option>
            <option value="boards">boards</option>
          </select>
        </label>
        <label class="field">
          <span>Pacchetto</span>
          <select value={addCategoryId} onchange={(event) => onCategoryChange(event.currentTarget.value)}>
            {#each packages as pkg (pkg.id)}
              <option value={pkg.id}>{pkg.name}</option>
            {/each}
          </select>
        </label>
        <label class="field">
          <span>Sottocategoria</span>
          <select value={addSubcategoryId} onchange={(event) => onSubcategoryChange(event.currentTarget.value)}>
            <option value="">Generale</option>
            {#each addSubcategories as sub (sub.id)}
              <option value={sub.id}>{sub.name}</option>
            {/each}
          </select>
        </label>
        <div class="add__actions">
          <button class="btn" onclick={() => (addOpen = false)}>Annulla</button>
          <button class="btn btn--solid" disabled={addBusy} onclick={() => void confirmAdd()}>
            {addBusy ? "Aggiunta…" : "Aggiungi alla libreria"}
          </button>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .dev {
    position: fixed;
    inset: 0;
    z-index: 80;
    display: flex;
    flex-direction: column;
    background: #f6f8fb;
    color: #0f172a;
    overflow: auto;
  }
  .dev__top {
    display: flex;
    flex-wrap: wrap;
    gap: 14px;
    align-items: center;
    justify-content: space-between;
    padding: 16px clamp(14px, 3vw, 32px);
    background: #fff;
    border-bottom: 1px solid #dbe3ee;
    position: sticky;
    top: 0;
    z-index: 2;
  }
  .dev__brand { display: flex; gap: 12px; align-items: flex-start; }
  .dev__brand h1 { margin: 0; font-size: 20px; }
  .dev__brand p { margin: 3px 0 0; color: #64748b; font-size: 12px; max-width: 760px; }
  .dev__badge {
    background: #0f766e;
    color: #fff;
    font-weight: 800;
    font-size: 11px;
    letter-spacing: 0.08em;
    border-radius: 6px;
    padding: 4px 7px;
    margin-top: 2px;
  }
  .dev__actions { display: flex; gap: 8px; align-items: center; }
  .dev__body { flex: 1; display: grid; grid-template-columns: 250px minmax(0, 1fr); min-height: 0; }
  .dev__side {
    padding: 16px 12px;
    border-right: 1px solid #dbe3ee;
    background: #fff;
    max-height: calc(100vh - 90px);
    overflow: auto;
  }
  .dev__side h2 { margin: 0 0 8px; font-size: 12px; text-transform: uppercase; letter-spacing: 0.08em; color: #64748b; }
  .pkg {
    width: 100%;
    display: flex;
    justify-content: space-between;
    gap: 8px;
    border: 0;
    background: transparent;
    padding: 9px 10px;
    border-radius: 8px;
    font: inherit;
    font-size: 13px;
    text-align: left;
    cursor: pointer;
    color: #1e293b;
  }
  .pkg:hover { background: #f1f5f9; }
  .pkg--active { background: #0f766e; color: #fff; font-weight: 700; }
  .pkg__count { opacity: 0.72; font-size: 11px; }
  .dev__hint { margin: 14px 8px 0; color: #94a3b8; font-size: 11px; }
  .dev__main { padding: 18px clamp(14px, 3vw, 32px) 30px; min-width: 0; }
  .dev__heading { display: flex; flex-wrap: wrap; gap: 12px; justify-content: space-between; align-items: flex-start; margin-bottom: 14px; }
  .dev__heading h2 { margin: 0; font-size: 24px; }
  .dev__heading p { margin: 3px 0 0; color: #64748b; font-size: 13px; max-width: 760px; }
  .dev__search input {
    width: min(360px, 70vw);
    padding: 8px 11px;
    border: 1px solid #cbd5e1;
    border-radius: 9px;
    font: inherit;
    background: #fff;
  }
  .docs { display: grid; gap: 10px; }
  .doc {
    display: flex;
    flex-wrap: wrap;
    gap: 14px;
    justify-content: space-between;
    align-items: center;
    background: #fff;
    border: 1px solid #dbe3ee;
    border-radius: 12px;
    padding: 14px 16px;
  }
  .doc__main { min-width: 220px; flex: 1 1 380px; }
  .doc__kicker { display: flex; flex-wrap: wrap; gap: 5px; margin-bottom: 5px; }
  .doc h3 { margin: 0; font-size: 16px; }
  .doc__path { margin: 3px 0 0; color: #64748b; font-size: 11px; font-family: ui-monospace, SFMono-Regular, Menlo, monospace; }
  .doc__meta { margin: 4px 0 0; color: #475569; font-size: 12px; }
  .doc__actions { display: flex; flex-wrap: wrap; gap: 7px; }
  .tag {
    display: inline-flex;
    align-items: center;
    border-radius: 999px;
    background: #ccfbf1;
    color: #115e59;
    padding: 2px 7px;
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .tag--muted { background: #e2e8f0; color: #475569; }
  .tag--warn { background: #fef3c7; color: #92400e; }
  .empty { padding: 40px 24px; max-width: 720px; }
  .empty h2 { margin: 0 0 8px; }
  .muted { color: #64748b; }
  .notice { margin: 10px clamp(14px, 3vw, 32px); }
  .sr-only { position: absolute; width: 1px; height: 1px; overflow: hidden; clip: rect(0 0 0 0); }
  .preview, .add {
    position: fixed;
    inset: 0;
    z-index: 90;
    background: rgba(15, 23, 42, 0.55);
    display: flex;
    flex-direction: column;
    padding: 20px;
  }
  .preview__top, .add__top {
    display: flex;
    gap: 12px;
    justify-content: space-between;
    align-items: flex-start;
    background: #fff;
    border-radius: 14px 14px 0 0;
    padding: 14px 18px;
  }
  .preview__top h2, .add__top h2 { margin: 0; font-size: 18px; }
  .preview__top p, .add__top p { margin: 3px 0 0; color: #64748b; font-size: 12px; }
  .preview__body {
    flex: 1;
    overflow: auto;
    background: #fff;
    border-radius: 0 0 14px 14px;
    padding: 18px;
    display: flex;
    justify-content: center;
    align-items: flex-start;
  }
  .preview__svg { width: min(900px, 100%); background: #fff; }
  .preview__svg :global(svg) { width: 100%; height: auto; display: block; }
  .add__body {
    background: #fff;
    border-radius: 0 0 14px 14px;
    padding: 18px;
    display: grid;
    gap: 12px;
    max-width: 720px;
  }
  .field { display: grid; gap: 5px; font-size: 12px; color: #475569; }
  .field input, .field select {
    padding: 8px 10px;
    border: 1px solid #cbd5e1;
    border-radius: 8px;
    font: inherit;
    background: #fff;
  }
  .field__row { display: flex; gap: 8px; }
  .field__row input { flex: 1; }
  .add__actions { display: flex; gap: 8px; justify-content: flex-end; }
  .btn--danger { color: #b91c1c; border-color: #fecaca; background: #fff; }
  .btn--danger:hover { background: #fef2f2; }
  @media (max-width: 860px) {
    .dev__body { grid-template-columns: 1fr; }
    .dev__side { max-height: none; border-right: 0; border-bottom: 1px solid #dbe3ee; }
  }
</style>
