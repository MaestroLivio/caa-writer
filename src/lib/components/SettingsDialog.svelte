<script lang="ts">
  import { onMount } from "svelte";
  import { api, isTauri } from "../api";
  import ModelManager from "./ModelManager.svelte";
  import type {
    AiConfig,
    AiStatus,
    AppInfo,
    DocumentSettings,
    LibraryStats,
    PackInfo,
    Preference,
    ToolOutcome,
  } from "../types";

  interface Props {
    settings: DocumentSettings;
    appInfo: AppInfo | null;
    stats: LibraryStats | null;
    packs: PackInfo[];
    aiStatus: AiStatus | null;
    preferences: Preference[];
    onUpdateSettings: (patch: Partial<DocumentSettings>) => void;
    onSetAiConfig: (config: AiConfig) => Promise<void>;
    onStatusChange: () => Promise<void>;
    onClearHistory: () => Promise<void>;
    onToast: (message: string, kind?: "info" | "warn" | "error") => void;
    onClose: () => void;
  }

  let {
    settings,
    appInfo,
    stats,
    packs,
    aiStatus,
    preferences,
    onUpdateSettings,
    onSetAiConfig,
    onStatusChange,
    onClearHistory,
    onToast,
    onClose,
  }: Props = $props();

  let dialog = $state<HTMLDivElement | null>(null);
  let aiDraft = $state<AiConfig>({
    enabled: false,
    binary_path: "",
    model_path: "",
    threads: 2,
    context_size: 1024,
    timeout_ms: 8000,
    backend: "auto",
    extra_args: [],
  });
  let toolChoice = $state("analyse_text_clarity");
  let toolInput = $state("");
  let toolOutcome = $state<ToolOutcome | null>(null);
  let toolBusy = $state(false);

  const toolDefs = [
    { id: "analyse_text_clarity", label: "Chiarezza del testo", key: "text", hint: "Incolla un testo da analizzare" },
    { id: "simplify_sentence", label: "Semplifica una frase", key: "text", hint: "Scrivi la frase da semplificare" },
    { id: "generate_task_sequence", label: "Sequenza di passaggi", key: "task", hint: "Attività da scomporre, per esempio preparare lo zaino" },
    { id: "explain_application_feature", label: "Come si fa…", key: "question", hint: "Per esempio: come creo una tabella?" },
    { id: "prepare_svg_description", label: "Descrizione per un nuovo simbolo", key: "label", hint: "Etichetta del simbolo mancante" },
  ];
  const activeTool = $derived(toolDefs.find((entry) => entry.id === toolChoice) ?? toolDefs[0]);

  onMount(() => {
    dialog?.focus();
    if (aiStatus) {
      aiDraft = {
        enabled: aiStatus.enabled,
        binary_path: aiStatus.binary_path,
        model_path: aiStatus.model_path,
        threads: aiStatus.threads,
        context_size: aiStatus.context_size,
        timeout_ms: aiStatus.timeout_ms,
        backend: aiStatus.backend,
        extra_args: [],
      };
    }
  });

  function onKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.preventDefault();
      onClose();
    }
  }

  async function saveAi() {
    try {
      await onSetAiConfig(aiDraft);
      onToast("Configurazione IA salvata.", "info");
    } catch (error) {
      onToast(error instanceof Error ? error.message : String(error), "error");
    }
  }

  async function runTool() {
    const value = toolInput.trim();
    if (!value) {
      onToast("Scrivi il testo da elaborare.", "warn");
      return;
    }
    if (!isTauri()) {
      onToast("Gli strumenti IA richiedono l'app desktop.", "warn");
      return;
    }
    toolBusy = true;
    toolOutcome = null;
    try {
      toolOutcome = await api.aiRunTool(toolChoice, { [activeTool.key]: value });
    } catch (error) {
      onToast(error instanceof Error ? error.message : String(error), "error");
    } finally {
      toolBusy = false;
    }
  }
</script>

<div
  class="dialog-backdrop"
  role="presentation"
  onclick={(event) => {
    if (event.target === event.currentTarget) onClose();
  }}
>
  <div
    class="dialog"
    role="dialog"
    aria-modal="true"
    aria-labelledby="settings-title"
    tabindex="-1"
    bind:this={dialog}
    onkeydown={onKeydown}
  >
    <header class="dialog__head">
      <h2 id="settings-title">Impostazioni</h2>
      <button class="btn btn--ghost" onclick={onClose} aria-label="Chiudi impostazioni">Chiudi ✕</button>
    </header>

    <div class="dialog__body">
      <section class="block">
        <h3>Rappresentazione CAA</h3>
        <div class="row">
          <label for="dlg-mode">Modalità</label>
          <select
            id="dlg-mode"
            value={settings.mode}
            onchange={(event) => onUpdateSettings({ mode: (event.currentTarget as HTMLSelectElement).value as never })}
          >
            <option value="testo">A · Solo testo</option>
            <option value="sopra">B · Simboli sopra le parole</option>
            <option value="sotto">C · Simboli con didascalia sotto</option>
            <option value="solo_simboli">D · Solo simboli</option>
            <option value="semplificata">E · CAA semplificata</option>
          </select>
        </div>
        <div class="row">
          <label for="dlg-intensity">Intensità</label>
          <select
            id="dlg-intensity"
            value={settings.intensity}
            onchange={(event) => onUpdateSettings({ intensity: (event.currentTarget as HTMLSelectElement).value as never })}
          >
            <option value="minima">Minima (parole chiave)</option>
            <option value="media">Intermedia</option>
            <option value="completa">Completa</option>
          </select>
        </div>
        <div class="row">
          <label for="dlg-auto">CAA automatica</label>
          <input
            id="dlg-auto"
            type="checkbox"
            checked={settings.auto_caa}
            onchange={(event) => onUpdateSettings({ auto_caa: (event.currentTarget as HTMLInputElement).checked })}
          />
        </div>
        <div class="row">
          <label for="dlg-neural">Analisi neurale</label>
          <input
            id="dlg-neural"
            type="checkbox"
            checked={settings.neural_context}
            onchange={(event) => onUpdateSettings({ neural_context: (event.currentTarget as HTMLInputElement).checked })}
          />
        </div>
        <div class="row">
          <label for="dlg-sym-articles">Simbolizza articoli (il, lo, la, un…)</label>
          <input
            id="dlg-sym-articles"
            type="checkbox"
            checked={settings.symbolize_articles}
            onchange={(event) => onUpdateSettings({ symbolize_articles: (event.currentTarget as HTMLInputElement).checked })}
          />
        </div>
        <div class="row">
          <label for="dlg-sym-preps">Simbolizza preposizioni (di, a, con…)</label>
          <input
            id="dlg-sym-preps"
            type="checkbox"
            checked={settings.symbolize_prepositions}
            onchange={(event) => onUpdateSettings({ symbolize_prepositions: (event.currentTarget as HTMLInputElement).checked })}
          />
        </div>
        <div class="row">
          <label for="dlg-sym-conj">Simbolizza congiunzioni (e, o, ma…)</label>
          <input
            id="dlg-sym-conj"
            type="checkbox"
            checked={settings.symbolize_conjunctions}
            onchange={(event) => onUpdateSettings({ symbolize_conjunctions: (event.currentTarget as HTMLInputElement).checked })}
          />
        </div>
        <div class="row">
          <label for="dlg-font">Carattere del documento</label>
          <select
            id="dlg-font"
            value={settings.font_family}
            onchange={(event) => onUpdateSettings({ font_family: (event.currentTarget as HTMLSelectElement).value })}
          >
            <option value="">Carattere di sistema</option>
            <option value="Arial, Helvetica, sans-serif">Arial</option>
            <option value="Verdana, Geneva, sans-serif">Verdana</option>
            <option value="Calibri, Carlito, sans-serif">Calibri</option>
            <option value="OpenDyslexic, Atkinson Hyperlegible, sans-serif">OpenDyslexic</option>
          </select>
        </div>
        <div class="row">
          <label for="dlg-uppercase">Mostra in maiuscolo (TUTTO MAIUSCOLE)</label>
          <input
            id="dlg-uppercase"
            type="checkbox"
            checked={settings.uppercase}
            onchange={(event) => onUpdateSettings({ uppercase: (event.currentTarget as HTMLInputElement).checked })}
          />
        </div>
        <div class="row">
          <label for="dlg-captions">Didascalie visibili</label>
          <input
            id="dlg-captions"
            type="checkbox"
            checked={settings.show_captions}
            onchange={(event) => onUpdateSettings({ show_captions: (event.currentTarget as HTMLInputElement).checked })}
          />
        </div>
        <div class="row">
          <label for="dlg-border">Bordo del pittogramma</label>
          <select
            id="dlg-border"
            value={settings.border_style}
            onchange={(event) =>
              onUpdateSettings({ border_style: (event.currentTarget as HTMLSelectElement).value as never })}
          >
            <option value="nessuno">Nessun bordo</option>
            <option value="immagine">Bordo solo immagine</option>
            <option value="immagine_testo">Bordo immagine e testo</option>
            <option value="separato">Bordi separati</option>
            <option value="divisore">Bordo con divisore</option>
          </select>
        </div>
        <div class="row">
          <label for="dlg-mono">Stampa monocromatica</label>
          <input
            id="dlg-mono"
            type="checkbox"
            checked={settings.mono}
            onchange={(event) => onUpdateSettings({ mono: (event.currentTarget as HTMLInputElement).checked })}
          />
        </div>
      </section>

      <section class="block">
        <h3>Accessibilità</h3>
        <div class="row">
          <label for="dlg-theme">Tema</label>
          <select
            id="dlg-theme"
            value={settings.theme}
            onchange={(event) => onUpdateSettings({ theme: (event.currentTarget as HTMLSelectElement).value as never })}
          >
            <option value="chiaro">Chiaro</option>
            <option value="scuro">Scuro</option>
            <option value="contrasto">Alto contrasto</option>
          </select>
        </div>
        <div class="row">
          <label for="dlg-touch">Modalità tocco (smartboard)</label>
          <input
            id="dlg-touch"
            type="checkbox"
            checked={settings.touch_mode}
            onchange={(event) => onUpdateSettings({ touch_mode: (event.currentTarget as HTMLInputElement).checked })}
          />
        </div>
      </section>

      <section class="block">
        <h3>Repertorio pittografico</h3>
        <div class="row">
          <label for="dlg-pack">Pacchetto</label>
          <select
            id="dlg-pack"
            value={settings.pack_id ?? ""}
            onchange={(event) => onUpdateSettings({ pack_id: (event.currentTarget as HTMLSelectElement).value || null })}
          >
            <option value="">Tutti i repertori</option>
            {#each packs as pack (pack.id)}
              <option value={pack.id}>{pack.name}</option>
            {/each}
          </select>
        </div>
        <div class="row">
          <label for="dlg-commercial">Solo simboli per uso commerciale</label>
          <input
            id="dlg-commercial"
            type="checkbox"
            checked={settings.commercial_only}
            onchange={(event) => onUpdateSettings({ commercial_only: (event.currentTarget as HTMLInputElement).checked })}
          />
        </div>
        {#if stats}
          <dl class="facts">
            <dt>Simboli</dt><dd>{stats.symbols}</dd>
            <dt>Concetti</dt><dd>{stats.concepts}</dd>
            <dt>Lemmi</dt><dd>{stats.lemmas}</dd>
            <dt>Espressioni</dt><dd>{stats.expressions}</dd>
          </dl>
        {/if}
        <p class="hint">
          Pittogrammi ARASAAC: autore Sergio Palao, titolare dei diritti ARASAAC — Governo di Aragona,
          licenza CC BY-NC-SA 4.0 (https://arasaac.org).
        </p>
        <button class="btn btn--ghost" onclick={() => void onClearHistory()}>
          Cancella cronologia ricerche
        </button>
      </section>

      <section class="block">
        <h3>Intelligenza artificiale locale</h3>
        {#if aiStatus}
          <p class="hint">{aiStatus.note}</p>
          {#if aiStatus.last_error}
            <p class="warn">Ultimo errore: {aiStatus.last_error}</p>
          {/if}
        {/if}
        <div class="row">
          <label for="dlg-ai-enabled">Attiva modello locale</label>
          <input
            id="dlg-ai-enabled"
            type="checkbox"
            checked={aiDraft.enabled}
            onchange={(event) => (aiDraft.enabled = (event.currentTarget as HTMLInputElement).checked)}
          />
        </div>
        <label class="hint" for="dlg-ai-binary">Eseguibile llama.cpp</label>
        <input id="dlg-ai-binary" type="text" bind:value={aiDraft.binary_path} style="width:100%" />
        <label class="hint" for="dlg-ai-model">Modello GGUF</label>
        <input id="dlg-ai-model" type="text" bind:value={aiDraft.model_path} style="width:100%" />
        <div class="row">
          <label for="dlg-ai-threads">Thread</label>
          <input id="dlg-ai-threads" type="number" min="1" max="8" bind:value={aiDraft.threads} style="width:70px" />
          <label for="dlg-ai-ctx">Contesto</label>
          <input id="dlg-ai-ctx" type="number" min="512" max="8192" step="256" bind:value={aiDraft.context_size} style="width:90px" />
          <label for="dlg-ai-timeout">Timeout (ms)</label>
          <input id="dlg-ai-timeout" type="number" min="1000" max="60000" step="500" bind:value={aiDraft.timeout_ms} style="width:90px" />
        </div>
        <button class="btn btn--solid" onclick={() => void saveAi()}>Salva configurazione</button>
      </section>

      <ModelManager {aiStatus} {onToast} {onStatusChange} />

      <section class="block">
        <h3>Strumenti IA</h3>
        <select bind:value={toolChoice} aria-label="Strumento IA">
          {#each toolDefs as definition (definition.id)}
            <option value={definition.id}>{definition.label}</option>
          {/each}
        </select>
        <textarea bind:value={toolInput} placeholder={activeTool.hint} aria-label="Testo per lo strumento IA"></textarea>
        <button class="btn btn--solid" disabled={toolBusy} onclick={() => void runTool()}>
          {toolBusy ? "Elaborazione…" : "Esegui"}
        </button>
        {#if toolOutcome}
          <p class="hint">Origine: {toolOutcome.source}</p>
          <pre>{JSON.stringify(toolOutcome.result, null, 2)}</pre>
          {#each toolOutcome.notes as note}
            <p class="hint">{note}</p>
          {/each}
        {/if}
      </section>

      <section class="block">
        <h3>Preferenze salvate</h3>
        {#if preferences.length === 0}
          <p class="hint">Nessuna preferenza. Le scelte fatte restano locali.</p>
        {:else}
          <ul class="prefs">
            {#each preferences as preference (preference.scope + preference.lemma + (preference.context_key ?? ""))}
              <li>
                «{preference.lemma}» → {preference.symbol_id}
                <span class="hint">({preference.scope}{preference.context_key ? ` · ${preference.context_key}` : ""})</span>
              </li>
            {/each}
          </ul>
        {/if}
      </section>

      {#if appInfo}
        <section class="block">
          <h3>Informazioni</h3>
          <dl class="facts">
            <dt>Versione</dt><dd>{appInfo.version}</dd>
            <dt>Schema DB</dt><dd>v{appInfo.schema_version}</dd>
            <dt>Lessico</dt><dd>{appInfo.lexicon_lemmas} lemmi / {appInfo.lexicon_concepts} concetti</dd>
            <dt>Piattaforma</dt><dd>{appInfo.platform}</dd>
          </dl>
          <p class="hint">Dati locali: {appInfo.data_dir}</p>
          {#each appInfo.warnings as warning}
            <p class="warn">{warning}</p>
          {/each}
        </section>
      {/if}
    </div>
  </div>
</div>

<style>
  .dialog-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(15, 23, 42, 0.55);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
    z-index: 80;
  }
  .dialog {
    background: var(--surface, #fff);
    color: inherit;
    width: min(760px, 100%);
    max-height: 86vh;
    border-radius: 14px;
    box-shadow: 0 24px 60px rgba(15, 23, 42, 0.35);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .dialog__head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 14px 18px;
    border-bottom: 1px solid rgba(100, 116, 139, 0.25);
  }
  .dialog__head h2 {
    margin: 0;
    font-size: 1.05rem;
  }
  .dialog__body {
    overflow: auto;
    padding: 8px 18px 20px;
  }
  .block {
    padding: 12px 0;
    border-bottom: 1px solid rgba(100, 116, 139, 0.18);
  }
  .block:last-child {
    border-bottom: none;
  }
  .block h3 {
    margin: 0 0 8px;
    font-size: 0.9rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 6px 0;
    flex-wrap: wrap;
  }
  .row label {
    font-size: 0.85rem;
  }
  .hint {
    font-size: 0.78rem;
    opacity: 0.75;
    margin: 4px 0;
  }
  .warn {
    font-size: 0.8rem;
    color: #b45309;
    margin: 4px 0;
  }
  .facts {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 2px 12px;
    margin: 8px 0;
    font-size: 0.82rem;
  }
  .facts dt {
    font-weight: 600;
  }
  .facts dd {
    margin: 0;
  }
  .prefs {
    margin: 8px 0;
    padding-left: 18px;
    font-size: 0.82rem;
  }
  textarea {
    width: 100%;
    min-height: 64px;
    margin-top: 6px;
  }
  pre {
    white-space: pre-wrap;
    font-size: 0.76rem;
    background: rgba(100, 116, 139, 0.12);
    padding: 8px;
    border-radius: 8px;
  }
</style>
