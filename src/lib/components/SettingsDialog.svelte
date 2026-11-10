<script lang="ts">
  import { onMount } from "svelte";
  import { api, isTauri } from "../api";
  import ModelManager from "./ModelManager.svelte";
  import type {
    AiConfig,
    AiProviderConfig,
    AiProviderStatus,
    AiProviderTest,
    AiStatus,
    OnlineProviderInfo,
    OnlineCustomProvider,
    AppInfo,
    DocumentSettings,
    LibraryStats,
    PackInfo,
    Preference,
    ToolOutcome,
    EditorMetrics,
    RuntimeDiagnostics,
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
    editorMetrics: EditorMetrics;
    diagnostics: RuntimeDiagnostics | null;
    onRefreshDiagnostics: () => Promise<void>;
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
    editorMetrics,
    diagnostics,
    onRefreshDiagnostics,
  }: Props = $props();

  let dialog = $state<HTMLDivElement | null>(null);
  let aiDraft = $state<AiConfig>({
    enabled: false,
    binary_path: "",
    model_path: "",
    threads: 1,
    context_size: 512,
    timeout_ms: 4000,
    backend: "auto",
    extra_args: [],
  });
  let toolChoice = $state("analyse_text_clarity");
  let toolInput = $state("");
  let toolOutcome = $state<ToolOutcome | null>(null);
  let toolBusy = $state(false);
  let diagnosticsBusy = $state(false);
  let provider = $state<AiProviderConfig>({
    kind: "local",
    endpoint: "",
    model: "",
    timeout_ms: 20000,
    max_tokens: 512,
    allow_online: false,
    privacy_ack: false,
  });
  let providerStatus = $state<AiProviderStatus | null>(null);
  let providerKey = $state("");
  let providerTest = $state<AiProviderTest | null>(null);
  let providerBusy = $state(false);
  let onlineProviders = $state<OnlineProviderInfo[]>([]);
  let customProvider = $state<OnlineCustomProvider>({
    id: "",
    name: "",
    search_url: "",
    enabled: true,
    note: "",
  });
  let customProviderBusy = $state(false);

  const profiles = [
    {
      id: "risparmio",
      label: "Risparmio risorse",
      hint: "1 thread, contesto 512, analisi meno frequente. Per computer con 4 GB di RAM.",
      threads: 1,
      context: 512,
      timeout: 4000,
      analysis: 420,
      refine: 1200,
    },
    {
      id: "bilanciato",
      label: "Bilanciato",
      hint: "2 thread, contesto 1024: compromesso predefinito tra qualità e reattività.",
      threads: 2,
      context: 1024,
      timeout: 8000,
      analysis: 240,
      refine: 600,
    },
    {
      id: "elevate",
      label: "Prestazioni elevate",
      hint: "Più thread e contesto maggiore; la digitazione resta prioritaria.",
      threads: 2,
      context: 2048,
      timeout: 15000,
      analysis: 140,
      refine: 320,
    },
  ];

  function applyProfile(id: "risparmio" | "bilanciato" | "elevate" | "personalizzato") {
    if (id === "personalizzato") {
      onUpdateSettings({ perf_profile: id });
      return;
    }
    const profile = profiles.find((entry) => entry.id === id);
    if (!profile) return;
    const threads =
      id === "elevate"
        ? Math.min(8, Math.max(2, aiStatus?.hardware.cpu_cores ?? 4))
        : profile.threads;
    aiDraft = {
      ...aiDraft,
      threads,
      context_size: profile.context,
      timeout_ms: profile.timeout,
    };
    onUpdateSettings({
      perf_profile: id,
      analysis_debounce_ms: profile.analysis,
      refine_debounce_ms: profile.refine,
    });
    onToast(
      `Profilo «${profile.label}» applicato. Ricorda di salvare la configurazione IA.`,
      "info"
    );
  }

  async function refreshDiagnostics() {
    diagnosticsBusy = true;
    try {
      await onRefreshDiagnostics();
    } catch (error) {
      onToast(error instanceof Error ? error.message : String(error), "error");
    } finally {
      diagnosticsBusy = false;
    }
  }

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
    if (isTauri()) {
      void api.listOnlineProviders().then((items) => (onlineProviders = items)).catch(() => {});
      void api.aiProviderStatus().then((status) => {
        providerStatus = status;
        provider = { ...status.config };
      }).catch(() => {});
    }
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

  async function saveCustomProvider() {
    if (!isTauri()) return;
    customProviderBusy = true;
    try {
      onlineProviders = await api.saveOnlineProvider(customProvider);
      customProvider = { id: "", name: "", search_url: "", enabled: true, note: "" };
      onToast("Provider online aggiunto.", "info");
    } catch (error) {
      onToast(error instanceof Error ? error.message : String(error), "error");
    } finally {
      customProviderBusy = false;
    }
  }

  async function removeCustomProvider(id: string) {
    customProviderBusy = true;
    try {
      onlineProviders = await api.deleteOnlineProvider(id);
      onToast("Provider rimosso.", "info");
    } catch (error) {
      onToast(error instanceof Error ? error.message : String(error), "error");
    } finally {
      customProviderBusy = false;
    }
  }

  async function saveProvider() {
    providerBusy = true;
    try {
      providerStatus = await api.aiProviderSave(provider, providerKey || null);
      provider = { ...providerStatus.config };
      providerKey = "";
      onToast(
        providerStatus.usable
          ? "Provider IA salvato. La chiave è nel portachiavi di sistema."
          : "Provider salvato, ma non ancora utilizzabile: controlla i campi.",
        providerStatus.usable ? "info" : "warn"
      );
    } catch (error) {
      onToast(error instanceof Error ? error.message : String(error), "error");
    } finally {
      providerBusy = false;
    }
  }

  async function testProvider() {
    providerBusy = true;
    providerTest = null;
    try {
      providerTest = await api.aiProviderTest();
    } catch (error) {
      onToast(error instanceof Error ? error.message : String(error), "error");
    } finally {
      providerBusy = false;
    }
  }

  async function clearProviderKey() {
    providerBusy = true;
    try {
      providerStatus = await api.aiProviderClearKey();
      providerKey = "";
      onToast("Chiave rimossa dal portachiavi.", "info");
    } catch (error) {
      onToast(error instanceof Error ? error.message : String(error), "error");
    } finally {
      providerBusy = false;
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
          <label for="dlg-explicit">Mostra contenuti espliciti (adulti)</label>
          <input
            id="dlg-explicit"
            type="checkbox"
            checked={settings.show_explicit_symbols}
            onchange={(event) => onUpdateSettings({ show_explicit_symbols: (event.currentTarget as HTMLInputElement).checked })}
          />
        </div>
        <p class="hint">
          Disattivato per impostazione predefinita. Esclude da ricerca, alternative, documenti ed
          esportazioni i termini sessuali o anatomici espliciti. Le scelte manuali non vengono
          cancellate; i contenuti espliciti vengono solo esclusi dalla vista e dalla stampa.
        </p>
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
            <option value="caa">Stile CAA (icone CAA, simboli grandi)</option>
            <option value="pastello">Pastello</option>
          </select>
        </div>
        <div class="row">
          <label>Interfaccia rapida</label>
          <div style="display:flex;gap:6px;flex-wrap:wrap">
            <button
              class="btn btn--solid"
              type="button"
              onclick={() =>
                onUpdateSettings({
                  theme: "caa",
                  symbol_layout: "parola",
                  border_style: "immagine_testo",
                  symbol_size: 64,
                  mode: "sopra",
                })}
            >Stile CAA</button>
            <button
              class="btn"
              type="button"
              onclick={() =>
                onUpdateSettings({
                  theme: "pastello",
                  symbol_layout: "parola",
                  border_style: "immagine_testo",
                  symbol_size: 60,
                  mode: "sopra",
                })}
            >Stile pastello</button>
          </div>
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
        <div class="row">
          <label for="dlg-smartboard">Controlli smartboard e tastiera a schermo</label>
          <input
            id="dlg-smartboard"
            type="checkbox"
            checked={settings.smartboard_controls}
            onchange={(event) => onUpdateSettings({ smartboard_controls: (event.currentTarget as HTMLInputElement).checked })}
          />
        </div>
        <div class="row">
          <label for="dlg-online">Ricerca online</label>
          <select
            id="dlg-online"
            value={settings.online_search}
            onchange={(event) => onUpdateSettings({ online_search: (event.currentTarget as HTMLSelectElement).value as never })}
          >
            <option value="off">Disattivata</option>
            <option value="suggest">Suggerisci automaticamente nel pannello</option>
            <option value="auto">Cerca automaticamente l'unità senza simbolo</option>
          </select>
        </div>
        <div class="row">
          <label for="dlg-online-auto">Inserisci i pittogrammi ARASAAC mentre scrivi</label>
          <input
            id="dlg-online-auto"
            type="checkbox"
            disabled={settings.online_search === "off"}
            checked={settings.online_auto_insert}
            onchange={(event) => onUpdateSettings({ online_auto_insert: (event.currentTarget as HTMLInputElement).checked })}
          />
        </div>
        <p class="hint">
          Le parole minuscole senza simbolo vengono cercate su ARASAAC e salvate come copia
          offline; la frase completa non lascia il dispositivo. Nomi propri, numeri, email e URL
          sono esclusi. Disattiva questa opzione per usare solo il pannello.
        </p>
        <div class="row">
          <label for="dlg-animations">Animazioni</label>
          <select
            id="dlg-animations"
            value={settings.animations}
            onchange={(event) => onUpdateSettings({ animations: (event.currentTarget as HTMLSelectElement).value as never })}
          >
            <option value="full">Complete</option>
            <option value="reduced">Ridotte</option>
            <option value="off">Disattivate</option>
          </select>
        </div>
        <div class="row">
          <label for="dlg-typing-sound">Suoni di interfaccia</label>
          <input
            id="dlg-typing-sound"
            type="checkbox"
            checked={settings.typing_sound}
            onchange={(event) => onUpdateSettings({ typing_sound: (event.currentTarget as HTMLInputElement).checked })}
          />
        </div>
        <div class="row">
          <label for="dlg-speech-enabled">Sintesi vocale attiva</label>
          <input
            id="dlg-speech-enabled"
            type="checkbox"
            checked={settings.speech_enabled}
            onchange={(event) => onUpdateSettings({ speech_enabled: (event.currentTarget as HTMLInputElement).checked })}
          />
        </div>
        <p class="hint">
          Disattivata per default: nessuna voce viene avviata finché non la abiliti.
          La lettura usa la voce neurale locale se disponibile, altrimenti la voce di sistema.
        </p>
        <div class="row">
          <label for="dlg-speak-words">Pronuncia la parola premendo spazio o Invio</label>
          <input
            id="dlg-speak-words"
            type="checkbox"
            disabled={!settings.speech_enabled}
            checked={settings.speak_words}
            onchange={(event) => onUpdateSettings({ speak_words: (event.currentTarget as HTMLInputElement).checked })}
          />
        </div>
        <div class="row">
          <label for="dlg-speak-sentences">Leggi la frase completata</label>
          <input
            id="dlg-speak-sentences"
            type="checkbox"
            disabled={!settings.speech_enabled}
            checked={settings.speak_sentences}
            onchange={(event) => onUpdateSettings({ speak_sentences: (event.currentTarget as HTMLInputElement).checked })}
          />
        </div>
      </section>

      <section class="block">
        <h3>Librerie online</h3>
        <p class="hint">
          I repertori non sono più duplicati interamente in locale: ARASAAC, Global Symbols, Wikimedia e Openverse
          vengono interrogati via API pubblica e i risultati scelti vengono salvati come copia
          offline. Puoi aggiungere un provider pubblico che restituisca JSON con
          <code>results</code>, <code>image_url</code>, <code>title</code>, <code>license</code>.
        </p>
        <ul class="prefs">
          {#each onlineProviders as item (item.id)}
            <li>
              <strong>{item.name}</strong>
              <span class="hint"> · {item.enabled ? "attivo" : "disattivato"} · {item.id}</span>
              {#if !["arasaac", "wikimedia"].includes(item.id)}
                <button class="btn btn--ghost" disabled={customProviderBusy} onclick={() => void removeCustomProvider(item.id)}>Rimuovi</button>
              {/if}
            </li>
          {/each}
        </ul>
        <div class="row">
          <label for="dlg-online-id">ID</label>
          <input id="dlg-online-id" bind:value={customProvider.id} placeholder="mio-provider" style="width:120px" />
          <label for="dlg-online-name">Nome</label>
          <input id="dlg-online-name" bind:value={customProvider.name} placeholder="Repertorio personale" style="width:160px" />
        </div>
        <input bind:value={customProvider.search_url} placeholder={"https://api.example.org/search?q={query}"} aria-label="URL di ricerca del provider" style="width:100%" />
        <div class="row">
          <label><input type="checkbox" bind:checked={customProvider.enabled} /> Attivo</label>
          <button class="btn btn--solid" disabled={customProviderBusy} onclick={() => void saveCustomProvider()}>Aggiungi provider</button>
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
        <h3>Prestazioni</h3>
        <p class="hint">
          La digitazione ha sempre la priorità: l'analisi lessicale e quella neurale
          vengono pianificate dopo una pausa e possono essere annullate quando il testo cambia.
        </p>
        <div class="row" style="flex-wrap:wrap;gap:6px">
          {#each profiles as profile (profile.id)}
            <button
              class="btn"
              class:btn--solid={settings.perf_profile === profile.id}
              title={profile.hint}
              onclick={() => applyProfile(profile.id as "risparmio" | "bilanciato" | "elevate")}
            >{profile.label}</button>
          {/each}
          <button
            class="btn"
            class:btn--solid={settings.perf_profile === "personalizzato"}
            onclick={() => applyProfile("personalizzato")}
          >Personalizzato</button>
        </div>
        <p class="hint">
          {profiles.find((profile) => profile.id === settings.perf_profile)?.hint ??
            "Valori scelti manualmente: regola attesa dell'analisi e thread del modello."}
        </p>
        <div class="row">
          <label for="dlg-analysis-debounce">Attesa analisi (ms)</label>
          <input
            id="dlg-analysis-debounce"
            type="number"
            min="80"
            max="2000"
            step="20"
            value={settings.analysis_debounce_ms}
            onchange={(event) => onUpdateSettings({ perf_profile: "personalizzato", analysis_debounce_ms: Number((event.currentTarget as HTMLInputElement).value) || 240 })}
            style="width:90px"
          />
          <label for="dlg-refine-debounce">Attesa rifinitura IA (ms)</label>
          <input
            id="dlg-refine-debounce"
            type="number"
            min="200"
            max="5000"
            step="50"
            value={settings.refine_debounce_ms}
            onchange={(event) => onUpdateSettings({ perf_profile: "personalizzato", refine_debounce_ms: Number((event.currentTarget as HTMLInputElement).value) || 600 })}
            style="width:90px"
          />
        </div>
      </section>

      <section class="block">
        <h3>Provider IA</h3>
        <p class="hint">
          Il provider locale resta il default e funziona offline. Un provider online viene
          contattato solo dopo consenso esplicito. La chiave API è conservata nel portachiavi
          del sistema operativo, mai nei documenti o nei log.
        </p>
        <div class="row">
          <label for="dlg-provider-kind">Provider</label>
          <select id="dlg-provider-kind" bind:value={provider.kind}>
            <option value="local">Modello locale</option>
            <option value="openai-compatible">Online compatibile OpenAI</option>
          </select>
        </div>
        {#if provider.kind === "openai-compatible"}
          <label class="hint" for="dlg-provider-endpoint">Endpoint</label>
          <input id="dlg-provider-endpoint" type="text" bind:value={provider.endpoint} placeholder="https://api.example.com/v1" style="width:100%" />
          <label class="hint" for="dlg-provider-model">Modello</label>
          <input id="dlg-provider-model" type="text" bind:value={provider.model} placeholder="nome del modello" style="width:100%" />
          <label class="hint" for="dlg-provider-key">Chiave API (salvata nel portachiavi, non nel documento)</label>
          <input id="dlg-provider-key" type="password" bind:value={providerKey} autocomplete="off" style="width:100%" />
          <div class="row">
            <label><input type="checkbox" bind:checked={provider.allow_online} /> Consento l'invio online per azioni esplicite</label>
          </div>
          <div class="row">
            <label><input type="checkbox" bind:checked={provider.privacy_ack} /> Ho letto l'informativa: non inviare dati personali degli alunni</label>
          </div>
        {/if}
        <div class="row" style="flex-wrap:wrap;gap:6px">
          <button class="btn btn--solid" disabled={providerBusy} onclick={() => void saveProvider()}>
            {providerBusy ? "Salvo…" : "Salva provider"}
          </button>
          <button class="btn" disabled={providerBusy} onclick={() => void testProvider()}>Test connessione</button>
          {#if providerStatus?.has_key}
            <button class="btn" disabled={providerBusy} onclick={() => void clearProviderKey()}>Rimuovi chiave</button>
          {/if}
        </div>
        {#if providerStatus}
          <p class="hint">{providerStatus.note} · chiave: {providerStatus.has_key ? "presente" : "assente"} · {providerStatus.key_storage}</p>
        {/if}
        {#if providerTest}
          <p class:ok={providerTest.ok} class:warn={!providerTest.ok}>
            Test: {providerTest.ok ? "riuscito" : "fallito"} · HTTP {providerTest.status} · {providerTest.elapsed_ms} ms
            {#if providerTest.reply}· «{providerTest.reply}»{/if}
          </p>
        {/if}
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
        <h3>Diagnostica misurata</h3>
        <p class="hint">
          Valori letti dal sistema e dal WebView, non stimati. Apri questa sezione quando
          vuoi un controllo: la lettura non avviene durante la digitazione.
        </p>
        <div class="row">
          <button class="btn btn--solid" disabled={diagnosticsBusy} onclick={() => void refreshDiagnostics()}>
            {diagnosticsBusy ? "Misuro…" : "Aggiorna misure"}
          </button>
          <span class="hint">
            {diagnostics ? `Ultima lettura · ${diagnostics.platform}` : "Nessuna lettura ancora"}
          </span>
        </div>
        <dl class="facts">
          <dt>Latenza input p95</dt><dd>{editorMetrics.input_p95_ms} ms · peggiore {editorMetrics.input_worst_ms} ms</dd>
          <dt>Aggiornamento editor p95</dt><dd>{editorMetrics.update_p95_ms} ms</dd>
          <dt>Analisi eseguite</dt><dd>{editorMetrics.analysis_runs} · ultima {editorMetrics.last_analysis_ms} ms</dd>
          <dt>Blocchi &gt; 120 ms</dt><dd>{editorMetrics.long_tasks} · massimo {editorMetrics.longest_task_ms} ms</dd>
        </dl>
        {#if diagnostics}
          <dl class="facts">
            <dt>Memoria app</dt><dd>{diagnostics.app_process.resident_mb} MiB · CPU {diagnostics.app_process.cpu_percent}%</dd>
            <dt>Memoria modello</dt>
            <dd>
              {#if diagnostics.model_process}
                {diagnostics.model_process.resident_mb} MiB · PID {diagnostics.model_process.pid}
                {#if diagnostics.model_process.note}<span class="hint"> · {diagnostics.model_process.note}</span>{/if}
              {:else}
                modello non in esecuzione
              {/if}
            </dd>
            <dt>Modello attivo</dt><dd>{diagnostics.active_model}</dd>
            <dt>Coda neurale</dt><dd>{diagnostics.ai.pending}/{diagnostics.ai.queue_limit} · annullate {diagnostics.ai.cancelled}</dd>
            <dt>Inferenze</dt><dd>{diagnostics.ai.completed} completate · ultima {diagnostics.ai.last_ms} ms</dd>
            <dt>Simbolizzazione</dt><dd>{diagnostics.symbols.paragraphs} paragrafi · media {diagnostics.symbols.paragraphs > 0 ? Math.round(diagnostics.symbols.total_ms / diagnostics.symbols.paragraphs) : 0} ms · peggiore {diagnostics.symbols.worst_ms} ms</dd>
            <dt>Risultati obsoleti scartati</dt><dd>{diagnostics.symbols.stale_results}</dd>
            <dt>Voce attiva</dt><dd>{diagnostics.speech_active ? "sì" : "no"}</dd>
          </dl>
          <p class="hint">{diagnostics.app_process.note}</p>
        {:else}
          <p class="hint">
            Le misure di CPU e memoria richiedono l'app desktop: nella sola anteprima browser
            restano dichiarate come non disponibili.
          </p>
        {/if}
      </section>

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
