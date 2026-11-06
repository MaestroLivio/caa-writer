<script lang="ts">
  import { api } from "../api";
  import type {
    ImportLicense,
    ImportPreview,
    ImportReport,
    ImportItemInput,
  } from "../types";

  interface Props {
    paths: string[];
    onClose: () => void;
    onImported: (report: ImportReport, collection: string) => void;
  }

  let { paths, onClose, onImported }: Props = $props();

  type Mode = "filename" | "manual" | "ai";
  type Row = {
    path: string;
    name: string;
    relative: string;
    lemma: string;
    category: string;
    synonyms: string;
    supported: boolean;
    duplicate: boolean;
    include: boolean;
    preview: string | null;
    reason: string | null;
    format: string;
  };

  let preview = $state<ImportPreview | null>(null);
  let rows = $state<Row[]>([]);
  let loading = $state(true);
  let error = $state("");
  let collectionName = $state("");
  let category = $state("");
  let mode = $state<Mode>("filename");
  let licensePreset = $state("personal");
  let author = $state("");
  let sourceUrl = $state("");
  let importing = $state(false);
  let cancelRequested = $state(false);
  let progressDone = $state(0);
  let progressTotal = $state(0);
  let result = $state<ImportReport | null>(null);
  let showUnsupported = $state(false);

  const licenses: Record<string, { label: string; name: string; url: string | null; commercial: boolean }> = {
    personal: {
      label: "Uso personale (non commerciale)",
      name: "Uso personale (non commerciale)",
      url: null,
      commercial: false,
    },
    "cc-by-nc-sa": {
      label: "CC BY-NC-SA 4.0",
      name: "CC BY-NC-SA 4.0",
      url: "https://creativecommons.org/licenses/by-nc-sa/4.0/",
      commercial: false,
    },
    "cc-by": {
      label: "CC BY 4.0",
      name: "CC BY 4.0",
      url: "https://creativecommons.org/licenses/by/4.0/",
      commercial: true,
    },
    unknown: {
      label: "Licenza non specificata",
      name: "Licenza non specificata",
      url: null,
      commercial: false,
    },
  };

  function deriveCategory(relative: string): string {
    const parts = relative.split("/");
    return parts.length > 1 ? parts[0] : "";
  }

  function setMode(next: Mode) {
    mode = next;
    if (next === "filename" || next === "ai") {
      const source = preview;
      if (!source) return;
      rows = rows.map((row) => {
        const entry = source.entries.find((item) => item.path === row.path);
        return { ...row, lemma: entry?.suggested_lemma ?? row.lemma };
      });
    }
  }

  $effect(() => {
    let cancelled = false;
    void (async () => {
      loading = true;
      error = "";
      try {
        const scanned = await api.scanPictograms(paths);
        if (cancelled) return;
        preview = scanned;
        collectionName = scanned.suggested_collection;
        rows = scanned.entries.map((entry) => ({
          path: entry.path,
          name: entry.original_name,
          relative: entry.relative_path,
          lemma: entry.suggested_lemma,
          category: deriveCategory(entry.relative_path),
          synonyms: "",
          supported: entry.supported,
          duplicate: entry.duplicate,
          include: entry.supported && !entry.duplicate,
          preview: entry.preview,
          reason: entry.reason,
          format: entry.format,
        }));
        if (scanned.warnings.length > 0) {
          error = scanned.warnings.join(" · ");
        }
      } catch (err) {
        if (!cancelled) error = err instanceof Error ? err.message : String(err);
      } finally {
        if (!cancelled) loading = false;
      }
    })();
    return () => {
      cancelled = true;
    };
  });

  const visibleRows = $derived(
    showUnsupported ? rows : rows.filter((row) => row.supported)
  );
  const selectedRows = $derived(rows.filter((row) => row.include && row.supported));
  const missingLemmas = $derived(selectedRows.filter((row) => !row.lemma.trim()).length);

  function selectAll(value: boolean) {
    rows = rows.map((row) =>
      row.supported ? { ...row, include: value } : row
    );
  }

  function applyCategory() {
    const value = category.trim();
    if (!value) return;
    rows = rows.map((row) => ({ ...row, category: value }));
  }

  function buildEntries(): ImportItemInput[] {
    return selectedRows
      .filter((row) => row.lemma.trim().length > 0)
      .map((row) => ({
        path: row.path,
        lemma: row.lemma.trim(),
        collection: collectionName.trim() || null,
        category: row.category.trim() || null,
        synonyms: row.synonyms
          .split(",")
          .map((item) => item.trim())
          .filter(Boolean),
        relative_path: row.relative,
        original_name: row.name,
      }));
  }

  function importLicense(): ImportLicense {
    const preset = licenses[licensePreset] ?? licenses.personal;
    return {
      name: preset.name,
      url: preset.url,
      author: author.trim() || null,
      source_url: sourceUrl.trim() || null,
      commercial_ok: preset.commercial,
    };
  }

  function mergeReport(current: ImportReport | null, next: ImportReport): ImportReport {
    if (!current) return next;
    return {
      imported: current.imported + next.imported,
      skipped: current.skipped + next.skipped,
      failed: current.failed + next.failed,
      items: [...current.items, ...next.items],
      warnings: [...current.warnings, ...next.warnings],
    };
  }

  async function startImport() {
    if (importing || selectedRows.length === 0) return;
    const entries = buildEntries();
    if (entries.length === 0) {
      error = "Nessun elemento con un lemma valido.";
      return;
    }
    const collection = collectionName.trim() || preview?.suggested_collection || "Pittogrammi importati";
    importing = true;
    cancelRequested = false;
    result = null;
    error = "";
    progressDone = 0;
    progressTotal = entries.length;
    let aggregated: ImportReport | null = null;
    const chunkSize = 20;
    try {
      for (let index = 0; index < entries.length; index += chunkSize) {
        if (cancelRequested) break;
        const chunk = entries.slice(index, index + chunkSize);
        const report = await api.importPictograms(chunk, collection, importLicense());
        aggregated = mergeReport(aggregated, report);
        progressDone = Math.min(index + chunk.length, entries.length);
      }
      result = aggregated ?? {
        imported: 0,
        skipped: 0,
        failed: 0,
        items: [],
        warnings: [],
      };
      onImported(result, collection);
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      importing = false;
    }
  }

  const progressPercent = $derived(
    progressTotal > 0 ? Math.round((progressDone / progressTotal) * 100) : 0
  );
</script>

<svelte:window
  onkeydown={(event) => {
    if (event.key === "Escape" && !importing) onClose();
  }}
/>

<div class="dialog-backdrop">
  <div
    class="report-dialog import-dialog"
    role="dialog"
    aria-modal="true"
    aria-label="Importa pittogrammi"
  >
    <header class="report-dialog__head">
      <h2>Importa pittogrammi</h2>
      <button class="btn btn--ghost" onclick={onClose} disabled={importing} aria-label="Chiudi">×</button>
    </header>

    {#if loading}
      <p>Analisi dei file in corso…</p>
    {:else if result}
      <section class="import-summary">
        <h3>Importazione conclusa</h3>
        <p>
          <strong>{result.imported}</strong> importati,
          <strong>{result.skipped}</strong> già presenti,
          <strong>{result.failed}</strong> non riusciti.
        </p>
        {#if result.failed > 0}
          <ul class="import-errors">
            {#each result.items.filter((item) => item.status === "failed") as item (item.path)}
              <li>{item.path.split("/").pop()} — {item.reason}</li>
            {/each}
          </ul>
        {/if}
        <p class="hint">
          I pittogrammi sono subito disponibili nella ricerca, nelle tabelle e nella
          simbolizzazione automatica.
        </p>
        <div class="import-actions">
          <button class="btn btn--solid" onclick={onClose}>Chiudi</button>
        </div>
      </section>
    {:else}
      <p class="hint">
        {preview?.total ?? 0} file trovati ·
        {preview?.supported ?? 0} supportati ·
        {preview?.duplicates ?? 0} duplicati
        {#if preview?.truncated} · scansione limitata{/if}
      </p>

      {#if error}
        <p class="import-error">{error}</p>
      {/if}

      <div class="import-settings">
        <label>
          Nome della collezione
          <input bind:value={collectionName} placeholder={preview?.suggested_collection ?? "Pittogrammi"} />
        </label>
        <label>
          Come assegnare i lemmi
          <select value={mode} onchange={(event) => setMode((event.currentTarget as HTMLSelectElement).value as Mode)}>
            <option value="filename">Usa i nomi dei file</option>
            <option value="manual">Chiedimi il lemma per ogni pittogramma</option>
            <option value="ai">Suggerisci con l'IA</option>
          </select>
        </label>
        <label>
          Categoria comune
          <input bind:value={category} placeholder="Es. Animali" />
        </label>
        <button class="btn btn--ghost" onclick={applyCategory}>Applica a tutti</button>
      </div>

      {#if mode === "ai"}
        <p class="hint">
          Il modello incluso è testuale: i suggerimenti usano nomi dei file,
          sottocartelle e metadati, non riconoscono visivamente il disegno.
          Ogni proposta resta modificabile.
        </p>
      {/if}

      <div class="import-license">
        <label>
          Licenza
          <select bind:value={licensePreset}>
            {#each Object.entries(licenses) as [key, value] (key)}
              <option value={key}>{value.label}</option>
            {/each}
          </select>
        </label>
        <label>
          Autore / titolare
          <input bind:value={author} placeholder="Nome dell'autore o dell'ente" />
        </label>
        <label>
          URL della fonte
          <input bind:value={sourceUrl} placeholder="https://…" />
        </label>
      </div>

      <div class="import-toolbar">
        <button class="btn btn--ghost" onclick={() => selectAll(true)}>Seleziona tutti</button>
        <button class="btn btn--ghost" onclick={() => selectAll(false)}>Deseleziona</button>
        <label class="import-checkbox">
          <input type="checkbox" bind:checked={showUnsupported} />
          Mostra file non supportati
        </label>
        <span class="hint">
          {selectedRows.length} selezionati
          {#if missingLemmas > 0} · {missingLemmas} senza lemma{/if}
        </span>
      </div>

      <div class="import-table-wrap">
        <table class="import-table">
          <thead>
            <tr>
              <th>Includi</th>
              <th>Anteprima</th>
              <th>File originale</th>
              <th>Lemma</th>
              <th>Categoria</th>
              <th>Sinonimi</th>
              <th>Stato</th>
            </tr>
          </thead>
          <tbody>
            {#each visibleRows as row (row.path)}
              <tr class:import-row--missing={row.include && !row.lemma.trim()}>
                <td>
                  <input
                    type="checkbox"
                    bind:checked={row.include}
                    disabled={!row.supported || importing}
                  />
                </td>
                <td>
                  {#if row.preview}
                    <img class="import-thumb" src={row.preview} alt="" />
                  {:else}
                    <span class="import-thumb import-thumb--empty">?</span>
                  {/if}
                </td>
                <td>
                  <div class="import-file">{row.name}</div>
                  <div class="import-path">{row.relative}</div>
                </td>
                <td>
                  <input
                    class="import-lemma"
                    bind:value={row.lemma}
                    disabled={!row.supported || importing}
                    aria-label={`Lemma per ${row.name}`}
                  />
                </td>
                <td><input class="import-lemma" bind:value={row.category} disabled={importing} /></td>
                <td><input class="import-lemma" bind:value={row.synonyms} placeholder="separati da virgola" disabled={importing} /></td>
                <td>
                  {#if !row.supported}
                    <span class="import-badge import-badge--error">Non supportato</span>
                    {#if row.reason}<div class="import-path">{row.reason}</div>{/if}
                  {:else if row.duplicate}
                    <span class="import-badge">Già presente</span>
                  {:else}
                    <span class="import-badge import-badge--ok">{row.format}</span>
                  {/if}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>

      {#if rows.length === 0}
        <p class="hint">Nessuna immagine trovata nei percorsi selezionati.</p>
      {/if}

      {#if importing}
        <div class="import-progress">
          <progress max={progressTotal} value={progressDone}></progress>
          <span>{progressDone}/{progressTotal} · {progressPercent}%</span>
          <button class="btn btn--ghost" onclick={() => (cancelRequested = true)}>Interrompi</button>
        </div>
      {/if}

      <div class="import-actions">
        <button class="btn btn--ghost" onclick={onClose} disabled={importing}>Annulla</button>
        <button
          class="btn btn--solid"
          onclick={() => void startImport()}
          disabled={importing || selectedRows.length === 0 || missingLemmas > 0}
        >
          {importing ? "Importazione…" : `Importa ${selectedRows.length} pittogrammi`}
        </button>
      </div>
    {/if}
  </div>
</div>
