<script lang="ts">
  // Generatore di fogli di pregrafismo.
  //
  // Il foglio è un SVG vero, prodotto dal backend: lettere riempite con un
  // motivo di punti, guide di scrittura, contorno continuo o tratteggiato.
  // Si può inserire nel documento come immagine modificabile; la stampa e le
  // esportazioni usano lo stesso renderer del documento.
  import { onMount } from "svelte";
  import { language, strings } from "../i18n";
  import { api, isTauri } from "../api";
  import { documentFontGroups } from "../workspace/fonts";
  import type { TraceConfig } from "../types";

  interface Props {
    pageWidthMm?: number;
    pageHeightMm?: number;
    onClose: () => void;
    onInsert: (svg: string, widthMm: number, heightMm: number, label: string) => void;
  }

  let { pageWidthMm = 297, pageHeightMm = 210, onClose, onInsert }: Props = $props();

  let config = $state<TraceConfig>({
    text: "Aa",
    title: "Pregrafismo",
    font_family: "Andika, Arial, sans-serif",
    width_mm: pageWidthMm,
    height_mm: pageHeightMm,
    font_size_mm: 40,
    dot_radius_mm: 0.55,
    dot_spacing_mm: 2.2,
    outline_mm: 0.35,
    dashed_outline: false,
    color: "#1f2937",
    guides: true,
    uppercase: false,
    margin_mm: 15,
  });
  let svg = $state("");
  let busy = $state(false);
  let error = $state("");

  const traceFonts = documentFontGroups
    .filter((group) => ["pregrafismo", "scuola", "maiuscolo", "corsivo"].includes(group.id))
    .flatMap((group) => group.fonts.filter((font) => font.stack));

  function closeHandler(event: KeyboardEvent) {
    if (event.key === "Escape") {
      onClose();
    }
  }

  onMount(() => {
    void generate();
    window.addEventListener("keydown", closeHandler);
    return () => {
      window.removeEventListener("keydown", closeHandler);
    };
  });

  async function generate() {
    if (!isTauri()) {
      error = "Il foglio di pregrafismo si genera nell'app desktop.";
      return;
    }
    busy = true;
    error = "";
    try {
      svg = await api.generateTraceSheet(config);
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
      svg = "";
    } finally {
      busy = false;
    }
  }

  function insert() {
    if (!svg) return;
    onInsert(svg, config.width_mm, config.height_mm, `Pregrafismo · ${config.text.slice(0, 24)}`);
    onClose();
  }

  const ts = $derived.by(() => strings[$language].traceSheet);
</script>

<div class="trace" role="dialog" aria-modal="true" aria-labelledby="trace-title">
  <div class="trace__window">
    <header class="trace__bar">
      <div>
        <h2 id="trace-title">{ts.title}</h2>
        <p>{ts.description}</p>
      </div>
      <button class="btn btn--ghost" onclick={onClose} aria-label={ts.close}>×</button>
    </header>

    <div class="trace__body">
      <section class="trace__form" aria-label="Impostazioni del foglio">
        <label class="trace__field">
          <span>{ts.textToTrace}</span>
          <textarea rows="3" bind:value={config.text}></textarea>
        </label>
        <label class="trace__field">
          <span>{ts.sheetTitle}</span>
          <input bind:value={config.title} />
        </label>
        <label class="trace__field">
          <span>{ts.font}</span>
          <select bind:value={config.font_family}>
            {#each traceFonts as font (font.stack)}
              <option value={font.stack}>{font.label}</option>
            {/each}
          </select>
        </label>
        <div class="trace__grid">
          <label class="trace__field">
            <span>{ts.fontSize}</span>
            <input type="number" min="8" max="120" step="1" bind:value={config.font_size_mm} />
          </label>
          <label class="trace__field">
            <span>{ts.dotRadius}</span>
            <input type="number" min="0.15" max="3" step="0.05" bind:value={config.dot_radius_mm} />
          </label>
          <label class="trace__field">
            <span>{ts.dotSpacing}</span>
            <input type="number" min="0.8" max="10" step="0.1" bind:value={config.dot_spacing_mm} />
          </label>
          <label class="trace__field">
            <span>{ts.outline}</span>
            <input type="number" min="0" max="3" step="0.05" bind:value={config.outline_mm} />
          </label>
          <label class="trace__field">
            <span>{ts.color}</span>
            <input type="color" bind:value={config.color} />
          </label>
          <label class="trace__field">
            <span>{ts.margin}</span>
            <input type="number" min="5" max="60" step="1" bind:value={config.margin_mm} />
          </label>
        </div>
        <div class="trace__checks">
          <label><input type="checkbox" bind:checked={config.guides} /> {ts.guides}</label>
          <label><input type="checkbox" bind:checked={config.dashed_outline} /> {ts.dashedOutline}</label>
          <label><input type="checkbox" bind:checked={config.uppercase} /> {ts.uppercase}</label>
        </div>
        <div class="trace__grid">
          <label class="trace__field">
            <span>{ts.pageWidth}</span>
            <input type="number" min="60" max="1200" step="1" bind:value={config.width_mm} />
          </label>
          <label class="trace__field">
            <span>{ts.pageHeight}</span>
            <input type="number" min="60" max="1200" step="1" bind:value={config.height_mm} />
          </label>
        </div>
        <p class="trace__hint">
          {ts.hint}
        </p>
        {#if error}<p class="trace__error" role="alert">{ts.errorPrefix}{error}</p>{/if}
      </section>

      <section class="trace__preview" aria-label={ts.previewLabel}>
        {#if svg}
          {@html svg}
        {:else if busy}
          <p>{ts.generating}</p>
        {:else}
          <p>{ts.noPreview}</p>
        {/if}
      </section>
    </div>

    <footer class="trace__footer">
      {#if svg}<span class="trace__hint">
        {ts.sheetDimensions(String(config.width_mm), String(config.height_mm), String(config.text.trim().length))}
      </span>{/if}
      <button class="btn" onclick={onClose}>{ts.cancel}</button>
      <button class="btn" disabled={busy} onclick={() => void generate()}>{ts.updatePreview}</button>
      <button class="btn btn--solid" disabled={!svg || busy} onclick={insert}>
        {ts.insertIntoDocument}
      </button>
    </footer>
  </div>
</div>

<style>
  .trace {
    position: fixed;
    inset: 0;
    z-index: 160;
    display: grid;
    place-items: center;
    background: rgba(15, 23, 42, 0.45);
    padding: 16px;
  }
  .trace__window {
    width: min(1180px, 100%);
    height: min(860px, 100%);
    display: grid;
    grid-template-rows: auto minmax(0, 1fr) auto;
    background: #fff;
    border-radius: 14px;
    overflow: hidden;
    box-shadow: 0 24px 60px rgba(15, 23, 42, 0.3);
  }
  .trace__bar {
    display: flex;
    justify-content: space-between;
    gap: 14px;
    padding: 14px 18px;
    border-bottom: 1px solid #dbe3ee;
  }
  .trace__bar h2 { margin: 0; font-size: 1.1rem; }
  .trace__bar p { margin: 4px 0 0; color: #475569; font-size: 0.82rem; }
  .trace__body {
    display: grid;
    grid-template-columns: minmax(260px, 320px) minmax(0, 1fr);
    gap: 14px;
    padding: 14px 18px;
    min-height: 0;
  }
  .trace__form {
    display: flex;
    flex-direction: column;
    gap: 10px;
    overflow: auto;
    padding-right: 4px;
  }
  .trace__field { display: grid; gap: 4px; font-size: 0.8rem; font-weight: 600; }
  .trace__field input, .trace__field textarea {
    width: 100%;
    border: 1px solid #cbd5e1;
    border-radius: 8px;
    padding: 7px 9px;
    font: inherit;
    font-weight: 400;
  }
  .trace__grid { display: grid; grid-template-columns: 1fr 1fr; gap: 8px; }
  .trace__checks { display: grid; gap: 6px; font-size: 0.82rem; }
  .trace__checks label { display: flex; align-items: center; gap: 6px; }
  .trace__preview {
    min-height: 0;
    overflow: auto;
    background: #eef1f6;
    border: 1px solid #dbe3ee;
    border-radius: 10px;
    padding: 12px;
    display: grid;
    place-items: center;
  }
  .trace__preview :global(svg) { max-width: 100%; height: auto; background: #fff; box-shadow: 0 6px 18px rgba(15, 23, 42, 0.12); }
  .trace__footer {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 18px;
    border-top: 1px solid #dbe3ee;
  }
  .trace__hint { margin-right: auto; color: #64748b; font-size: 0.78rem; }
  .trace__error { color: #991b1b; background: #fef2f2; border: 1px solid #fecaca; border-radius: 8px; padding: 8px; font-size: 0.8rem; }
  .btn {
    border: 1px solid #cbd5e1;
    background: #fff;
    color: #1e293b;
    border-radius: 8px;
    padding: 8px 12px;
    font: inherit;
    font-size: 0.82rem;
    cursor: pointer;
  }
  .btn:hover { background: #f1f5f9; }
  .btn--solid { background: #0f766e; border-color: #0f766e; color: #fff; font-weight: 700; }
  .btn--ghost { border-color: transparent; background: transparent; font-size: 1.2rem; }
  .btn:disabled { opacity: 0.5; cursor: not-allowed; }
  @media (max-width: 860px) {
    .trace__body { grid-template-columns: minmax(0, 1fr); }
    .trace__preview { max-height: 40vh; }
  }
</style>
