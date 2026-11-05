<script lang="ts">
  import { api, isTauri } from "../api";
  import ModelManager from "./ModelManager.svelte";
  import type {
    AiConfig,
    AiStatus,
    Alternative,
    AppInfo,
    CaaToken,
    DocumentSettings,
    LibraryStats,
    PackInfo,
    Preference,
    ResourcePlan,
    TemplateInfo,
    ToolOutcome,
    VoiceInfo,
    WordAffinity,
  } from "../types";

  type Tab = "alternative" | "cerca" | "impostazioni" | "studio";

  interface Props {
    selected: { pid: string; token: CaaToken } | null;
    settings: DocumentSettings;
    appInfo: AppInfo | null;
    stats: LibraryStats | null;
    packs: PackInfo[];
    voices: VoiceInfo[];
    aiStatus: AiStatus | null;
    preferences: Preference[];
    templates: TemplateInfo[];
    /// Richiesta di apertura di una scheda specifica, inviata dalla schermata iniziale
    /// o dal Ribbon. `search` apre la ricerca sulla parola, `ai` usa gli affini IA.
    intent: {
      tab: Tab;
      prompt?: string;
      templateId?: string | null;
      search?: string;
      ai?: boolean;
      nonce: number;
    } | null;
    /// Il pannello può essere aperto/chiuso anche dal Ribbon.
    closed?: boolean;
    /// Vero quando il pannello è sovrapposto al documento.
    overlay?: boolean;
    onCollapse?: () => void;
    onToggleDock?: () => void;
    onSwitchTab: (tab: Tab) => void;
    onApplyAlternative: (alternative: Alternative) => Promise<void>;
    onSaveGlobalPreference: (alternative: Alternative) => Promise<void>;
    onSaveContextPreference: (alternative: Alternative, context: string) => Promise<void>;
    onResetUnit: () => Promise<void>;
    onSetUnitState: (patch: {
      locked?: boolean;
      hidden?: boolean;
      modifiers?: { plural: boolean; negation: boolean; past: boolean; future: boolean; manual: boolean };
    }) => void;
    onUpdateSettings: (patch: Partial<DocumentSettings>) => void;
    onImportSvg: (file: File) => Promise<void>;
    onSpeak: (text: string) => Promise<void>;
    onSetAiConfig: (config: AiConfig) => Promise<void>;
    onStatusChange: () => Promise<void>;
    onGenerateResource: (prompt: string, templateId: string | null) => Promise<ResourcePlan | null>;
    onPrintResource: () => void;
    onToast: (message: string, kind?: "info" | "warn" | "error") => void;
    onClearHistory: () => Promise<void>;
  }

  let {
    selected,
    settings,
    appInfo,
    stats,
    packs,
    voices,
    aiStatus,
    preferences,
    templates,
    intent,
    closed = $bindable(false),
    overlay = false,
    onCollapse,
    onToggleDock,
    onSwitchTab,
    onApplyAlternative,
    onSaveGlobalPreference,
    onSaveContextPreference,
    onResetUnit,
    onSetUnitState,
    onUpdateSettings,
    onImportSvg,
    onSpeak,
    onSetAiConfig,
    onStatusChange,
    onGenerateResource,
    onPrintResource,
    onToast,
    onClearHistory,
  }: Props = $props();

  let tab = $state<Tab>("alternative");
  let searchQuery = $state("");
  let searchResults = $state<Alternative[]>([]);
  let searchBusy = $state(false);
  let contextChoice = $state("scuola");
  const modifierKinds = [
    { kind: "plural", label: "Plurale" },
    { kind: "negation", label: "Negazione" },
    { kind: "past", label: "Passato" },
    { kind: "future", label: "Futuro" },
  ] as const;
  let studioPrompt = $state("");
  let plan = $state<ResourcePlan | null>(null);
  let studioBusy = $state(false);
  let templateChoice = $state<string | null>(null);
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
  let svgFile = $state<File | null>(null);
  let appliedIntent = 0;
  let relatedBusy = $state(false);

  // Affinità della parola: gruppi esatto/lemma/varianti/sinonimi/generale/
  // affine. La richiesta è deterministica e non applica nulla al documento.
  let affinity = $state<WordAffinity | null>(null);
  let affinityBusy = $state(false);
  let affinityError = $state("");
  // Chiave non reattiva: evita richieste ripetute quando il token viene
  // ridisegnato con la stessa parola.
  let affinityCacheKey = "";
  let affinityRequest = 0;

  let toolChoice = $state("analyse_text_clarity");
  let toolInput = $state("");
  let toolOutcome = $state<ToolOutcome | null>(null);
  let toolBusy = $state(false);

  // Strumenti realmente implementati lato Rust: etichetta, campo di input e
  // segnaposto. Non ne vengono elencati altri.
  const toolDefs = [
    { id: "analyse_text_clarity", label: "Chiarezza del testo", key: "text", hint: "Incolla un testo da analizzare" },
    { id: "simplify_sentence", label: "Semplifica una frase", key: "text", hint: "Scrivi la frase da semplificare" },
    { id: "generate_task_sequence", label: "Sequenza di passaggi", key: "task", hint: "Attività da scomporre, per esempio preparare lo zaino" },
    { id: "explain_application_feature", label: "Come si fa…", key: "question", hint: "Per esempio: come creo una tabella?" },
    { id: "prepare_svg_description", label: "Descrizione per un nuovo simbolo", key: "label", hint: "Etichetta del simbolo mancante" },
  ];
  const activeTool = $derived(toolDefs.find((entry) => entry.id === toolChoice) ?? toolDefs[0]);
  // Il pannello dichiara lo stato reale: senza runtime o modello i suggerimenti
  // neurali non vengono offerti come se fossero disponibili.
  const neuralReady = $derived(
    Boolean(aiStatus?.enabled && aiStatus.binary_present && aiStatus.model_present)
  );

  // Nella scheda Cerca i gruppi restano legati alla parola effettivamente
  // cercata: digitando altro il blocco sparisce finché non si preme Cerca.
  const affinityMatchesSearch = $derived(
    Boolean(
      affinity &&
        searchQuery.trim() &&
        affinity.word.toLocaleLowerCase() === searchQuery.trim().toLocaleLowerCase()
    )
  );

  // I modelli e le metriche sono gestiti da `ModelManager`, montato nella
  // scheda Impostazioni: un solo posto, una sola fonte di stato.

  async function runRelatedSearch() {
    const query = searchQuery.trim();
    if (!query) {
      onToast("Scrivi una parola o una frase.", "warn");
      return;
    }
    if (!isTauri()) {
      onToast("La ricerca per significato richiede l'app desktop.", "warn");
      return;
    }
    relatedBusy = true;
    try {
      const found = await api.aiRelatedSymbols(query);
      if (found.length === 0) {
        onToast("Il modello non ha trovato simboli affini con un'etichetta corrispondente.", "warn");
        return;
      }
      searchResults = found;
      onToast(`${found.length} simboli affini proposti dal modello: verifica prima di scegliere.`, "info");
    } catch (error) {
      onToast(error instanceof Error ? error.message : String(error), "error");
    } finally {
      relatedBusy = false;
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

  // La schermata iniziale può aprire una scheda con testo e modello già scelti.
  $effect(() => {
    if (!intent || intent.nonce === appliedIntent) return;
    appliedIntent = intent.nonce;
    tab = intent.tab;
    closed = false;
    if (typeof intent.prompt === "string") studioPrompt = intent.prompt;
    if (intent.templateId !== undefined) templateChoice = intent.templateId;
    if (typeof intent.search === "string" && intent.search.trim()) {
      searchQuery = intent.search;
      tab = "cerca";
      void runSearch();
      if (intent.ai) void runRelatedSearch();
    }
  });

  function selectTab(next: Tab) {
    tab = next;
    // Con il pannello chiuso i pulsanti restavano visibili ma il corpo era
    // nascosto: la scheda richiesta non mostrava nulla. Cliccare una scheda
    // riapre sempre il pannello.
    closed = false;
    onSwitchTab(next);
  }

  // Passa alla scheda «Alternative» solo quando la selezione cambia davvero:
  // eseguire il cambio a ogni montaggio sovrascriveva la scheda richiesta
  // dalla schermata iniziale (per esempio «Impostazioni»).
  // Il primo giro dell'effetto registra la selezione esistente senza cambiare
  // scheda: altrimenti al montaggio sovrascriverebbe la scheda richiesta dalla
  // schermata iniziale (per esempio «Impostazioni»).
  let selectionReady = false;
  let lastSelectionKey = "";
  $effect(() => {
    const key = selected ? `${selected.pid}:${selected.token.index}` : "";
    if (selectionReady && key && key !== lastSelectionKey) tab = "alternative";
    selectionReady = true;
    lastSelectionKey = key;
  });

  // Copia la configurazione quando cambia lo stato del motore. Legge solo
  // `aiStatus` e scrive `aiDraft`: leggere anche `aiDraft` creerebbe un ciclo
  // infinito (effetto che riattiva se stesso) quando i percorsi sono vuoti.
  $effect(() => {
    if (!aiStatus) return;
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
  });

  /// Carica i gruppi di affinità per una parola. Non modifica il documento:
  /// i candidati diventano scelte solo con `onApplyAlternative`.
  async function loadAffinity(word: string, force = false) {
    const query = word.trim();
    if (!query) {
      affinity = null;
      affinityError = "";
      affinityCacheKey = "";
      return;
    }
    if (!isTauri()) return;
    const key = `${query}\u0001${settings.pack_id ?? ""}\u0001${settings.commercial_only}`;
    if (!force && key === affinityCacheKey) return;
    const request = ++affinityRequest;
    affinityBusy = true;
    affinityError = "";
    try {
      const found = await api.wordAffinities(query, settings.pack_id, settings.commercial_only);
      if (request !== affinityRequest) return;
      affinity = found;
      affinityCacheKey = key;
    } catch (error) {
      if (request !== affinityRequest) return;
      affinity = null;
      affinityCacheKey = "";
      affinityError = error instanceof Error ? error.message : String(error);
    } finally {
      if (request === affinityRequest) affinityBusy = false;
    }
  }

  // Nella scheda Alternative l'analisi parte da sola quando cambia l'unità
  // selezionata: è così che una parola senza pittogramma riceve comunque dei
  // candidati, senza che l'utente debba cercarli a mano.
  $effect(() => {
    const word = tab === "alternative" && selected ? selected.token.surface : "";
    // Cambiare repertorio o filtro commerciale deve ricalcolare i gruppi.
    void settings.pack_id;
    void settings.commercial_only;
    if (!word) return;
    void loadAffinity(word);
  });

  async function runSearch() {
    const query = searchQuery.trim();
    if (!query) {
      searchResults = [];
      affinity = null;
      affinityCacheKey = "";
      return;
    }
    if (!isTauri()) {
      onToast("La ricerca simboli richiede l'app desktop.", "warn");
      return;
    }
    searchBusy = true;
    // La ricerca testuale e l'analisi per affinità sono indipendenti: la
    // seconda non deve rallentare la comparsa dei risultati FTS.
    void loadAffinity(query, true);
    try {
      const page = await api.searchSymbols(query, 60, 0, settings.pack_id, settings.commercial_only);
      searchResults = page.items;
      if (page.items.length === 0) onToast("Nessun simbolo trovato per questa ricerca.", "warn");
    } catch (error) {
      onToast(error instanceof Error ? error.message : String(error), "error");
    } finally {
      searchBusy = false;
    }
  }

  /// Qualsiasi parola, anche senza concetto o senza pittogramma, può ricevere
  /// un'immagine: si cerca la parola intera nel repertorio e si applica il
  /// risultato all'unità selezionata.
  async function searchForSelectedWord() {
    if (!selected) return;
    searchQuery = selected.token.surface;
    onSwitchTab("cerca");
    await runSearch();
  }

  async function generate() {
    const prompt = studioPrompt.trim();
    if (!prompt) {
      onToast("Descrivi la risorsa che vuoi creare.", "warn");
      return;
    }
    studioBusy = true;
    try {
      plan = await onGenerateResource(prompt, templateChoice);
    } finally {
      studioBusy = false;
    }
  }

  function alternativeLabel(alternative: Alternative): string {
    const parts = [alternative.label];
    if (alternative.pack_name) parts.push(alternative.pack_name);
    return parts.join(" — ");
  }

  async function handleImportSvg() {
    if (!svgFile) {
      onToast("Scegli prima un file SVG.", "warn");
      return;
    }
    await onImportSvg(svgFile);
    svgFile = null;
  }
</script>

{#snippet affinityBlock()}
  {#if affinityBusy}
    <p class="hint">Cerco le affinità della parola…</p>
  {:else if affinityError}
    <p class="warn">Affinità non disponibili: {affinityError}</p>
  {:else if affinity && affinity.groups.length > 0}
    <div class="affinity-block">
      <h4 class="affinity-block__title">Affinità per «{affinity.word}»</h4>
      <p class="hint">{affinity.explanation}</p>
      {#each affinity.groups as group (group.level)}
        <div class="affinity-group">
          <h5 class="affinity-group__title">
            {group.title}
            {#if group.approximate}<span class="badge badge--warn">approssimato</span>{/if}
          </h5>
          <div class="symbol-grid">
            {#each group.candidates as candidate (candidate.symbol.symbol_id)}
              <button
                class="symbol-tile"
                title={`${candidate.symbol.meaning || candidate.symbol.label} — ${candidate.reason}`}
                onclick={() => onApplyAlternative(candidate.symbol)}
              >
                <span class="symbol-tile__media">
                  {#if candidate.symbol.svg}
                    {@html candidate.symbol.svg}
                  {:else}
                    <span class="caa-chip__empty">?</span>
                  {/if}
                </span>
                <span class="symbol-tile__label">{candidate.symbol.label}</span>
                <span class="badge badge--score" title="Indice interno di affinità, non una probabilità">
                  {candidate.score}/100
                </span>
                {#if candidate.approximate}
                  <span class="badge badge--warn" title="Candidato approssimativo: verifica prima di scegliere">≈</span>
                {/if}
                <span class="affinity-reason">{candidate.reason}</span>
              </button>
            {/each}
          </div>
        </div>
      {/each}
    </div>
  {:else if affinity}
    <p class="hint">
      Nessun pittogramma per «{affinity.word}»: la parola resta senza simbolo.
    </p>
  {/if}
{/snippet}

<aside class:closed class="sidepanel" class:sidepanel--closed={closed} aria-label="Pannello strumenti">
  <div class="sidepanel__tabs" role="tablist" aria-label="Sezioni del pannello">
    <button
      class="tab"
      role="tab"
      aria-selected={tab === "alternative"}
      onclick={() => selectTab("alternative")}>Alternative</button
    >
    <button class="tab" role="tab" aria-selected={tab === "cerca"} onclick={() => selectTab("cerca")}
      >Cerca</button
    >
    <button
      class="tab"
      role="tab"
      aria-selected={tab === "impostazioni"}
      onclick={() => selectTab("impostazioni")}>Impostazioni</button
    >
    <button class="tab" role="tab" aria-selected={tab === "studio"} onclick={() => selectTab("studio")}
      >Studio</button
    >
    <button
      class="btn btn--ghost"
      aria-label={overlay ? "Ancora il pannello" : "Apri il pannello sovrapposto"}
      title={overlay ? "Ancora il pannello al documento" : "Apri il pannello sovrapposto"}
      onclick={() => onToggleDock?.()}
    >{overlay ? "⇤" : "⇥"}</button>
    <button
      class="btn"
      aria-label="Riduci il pannello a icona"
      title="Riduci il pannello a icona"
      onclick={() => onCollapse?.()}>»</button>
  </div>

  {#if !closed}
    <div class="sidepanel__body">
      {#if tab === "alternative"}
        {#if selected}
          <section class="panel-section">
            <h3>Unità selezionata</h3>
            <p style="margin:0 0 6px;font-weight:600;font-size:1.05rem">
              «{selected.token.surface}»
            </p>
            <dl class="detail-list">
              <dt>Lemma</dt><dd>{selected.token.lemma ?? "—"}</dd>
              <dt>Significato</dt>
              <dd>{selected.token.meaning ?? "nessun significato associato"}</dd>
              <dt>Concetto</dt><dd>{selected.token.concept_label ?? selected.token.concept_id ?? "—"}</dd>
              <dt>Etichetta del repertorio</dt><dd>{selected.token.symbol_label ?? "—"}</dd>
              <dt>Strategia</dt>
              <dd>
                {selected.token.strategy}
                {#if selected.token.approximate}<span class="badge badge--warn">approssimato</span>{/if}
                {#if selected.token.locked}<span class="badge badge--ok">bloccato</span>{/if}
              </dd>
              <dt>Affidabilità</dt>
              <dd>
                indice interno {Math.round(selected.token.confidence * 100)}%
                <span class="hint">(non è una probabilità calibrata)</span>
              </dd>
            </dl>
          </section>

          <section class="panel-section">
            <h3>Alternative disponibili</h3>
            {#if selected.token.alternatives.length === 0}
              <p class="hint">
                Nessuna alternativa automatica. Qui sotto ci sono i gruppi di affinità della parola;
                se non basta, cerca direttamente nel repertorio o importa un SVG.
              </p>
              {@render affinityBlock()}
            {:else}
              <div class="symbol-grid">
                {#each selected.token.alternatives as alternative (alternative.symbol_id)}
                  <button
                    class="symbol-tile"
                    aria-current={selected.token.symbol_id === alternative.symbol_id}
                    title={`${alternative.meaning || alternative.label} — ${alternative.pack_name} (${alternative.license})`}
                    onclick={() => onApplyAlternative(alternative)}
                  >
                    <span class="symbol-tile__media">
                      {#if alternative.svg}
                        {@html alternative.svg}
                      {:else}
                        <span class="caa-chip__empty">?</span>
                      {/if}
                    </span>
                    <span class="symbol-tile__label">{alternative.label}</span>
                    <span class="symbol-tile__pack">{alternative.pack_name}</span>
                    {#if alternative.neural}
                      <span class="badge badge--ai" title="Suggerito dal modello neurale: simbolo affine, da verificare">IA affine</span>
                    {/if}
                  </button>
                {/each}
              </div>
            {/if}
          </section>

          <section class="panel-section">
            <h3>Applica la scelta</h3>
            <div style="display:flex;flex-direction:column;gap:8px">
              <button class="btn btn--solid" onclick={() => void searchForSelectedWord()}>
                Cerca un pittogramma per «{selected.token.surface}»
              </button>
              <button class="btn btn--ghost" onclick={onResetUnit}>Ripristina scelta automatica</button>
              {#if selected.token.alternatives.length > 0}
                <button
                  class="btn btn--ghost"
                  onclick={() => onSaveGlobalPreference(selected.token.alternatives[0])}
                  disabled={!selected.token.symbol_id}
                >
                  Usa sempre il pittogramma selezionato per «{selected.token.lemma ?? selected.token.surface}»
                </button>
              {/if}
              <div style="display:flex;gap:6px;align-items:center">
                <select bind:value={contextChoice} aria-label="Contesto comunicativo">
                  <option value="scuola">scuola</option>
                  <option value="casa">casa</option>
                  <option value="cibo">cibo</option>
                  <option value="fuori">fuori</option>
                  <option value="salute">salute</option>
                  <option value="sport">sport</option>
                </select>
                <button
                  class="btn btn--ghost"
                  disabled={!selected.token.symbol_id}
                  onclick={() => {
                    const alt = selected.token.alternatives.find(
                      (item) => item.symbol_id === selected.token?.symbol_id
                    );
                    if (alt) onSaveContextPreference(alt, contextChoice);
                  }}
                >
                  Usa in questo contesto
                </button>
              </div>
              <button class="btn btn--ghost" onclick={() => onSpeak(selected.token.surface)}>
                Ascolta la parola
              </button>
            </div>
          </section>

          <section class="panel-section">
            <h3>Blocco e visibilità</h3>
            <div style="display:flex;flex-direction:column;gap:8px">
              <button
                class="btn btn--ghost"
                aria-pressed={selected.token.locked}
                onclick={() => onSetUnitState({ locked: !selected.token.locked })}>
                {selected.token.locked ? "Sblocca pittogramma" : "Blocca pittogramma"}
              </button>
              <button
                class="btn btn--ghost"
                aria-pressed={selected.token.hidden}
                onclick={() => onSetUnitState({ hidden: !selected.token.hidden })}>
                {selected.token.hidden ? "Mostra pittogramma" : "Nascondi pittogramma"}
              </button>
              <p class="hint">
                Un pittogramma bloccato non viene mai sostituito dall'IA né dalla rivalutazione della
                frase. Nascosto, il testo resta nel foglio ma il simbolo non viene stampato.
              </p>
            </div>
          </section>

          <section class="panel-section">
            <h3>Modificatori</h3>
            <p class="hint">
              Segni sovrapposti al pittogramma, senza modificare l'immagine originale. Restano nella
              stampa e nell'esportazione.
            </p>
            <div style="display:flex;flex-direction:column;gap:4px">
              {#each modifierKinds as option (option.kind)}
                <label style="display:flex;align-items:center;gap:8px">
                  <input
                    type="checkbox"
                    checked={selected.token.modifiers?.[option.kind] ?? false}
                    onchange={() => {
                      const current = selected.token.modifiers;
                      if (!current) return;
                      onSetUnitState({
                        modifiers: { ...current, [option.kind]: !current[option.kind], manual: true },
                      });
                    }}
                  />
                  {option.label}
                </label>
              {/each}
              <button
                class="btn btn--ghost"
                onclick={() =>
                  onSetUnitState({
                    modifiers: { plural: false, negation: false, past: false, future: false, manual: false },
                  })}>
                Ripristina modificatori automatici
              </button>
            </div>
          </section>

          <section class="panel-section">
            <h3>Importa un pittogramma SVG</h3>
            <p class="hint">
              Il simbolo viene validato (niente script, niente riferimenti esterni) e diventa
              disponibile offline per la parola «{selected.token.lemma ?? selected.token.surface}».
            </p>
            <input
              type="file"
              accept=".svg,image/svg+xml"
              onchange={(event) => (svgFile = (event.currentTarget as HTMLInputElement).files?.[0] ?? null)}
            />
            <button class="btn btn--ghost" onclick={handleImportSvg} disabled={!svgFile} style="margin-top:8px">
              Importa e usa
            </button>
          </section>
        {:else}
          <p class="hint">
            Seleziona un pittogramma nel documento per vedere le alternative, cambiarlo o bloccare la
            scelta. Puoi anche cercare un simbolo dalla scheda <strong>Cerca</strong>.
          </p>
        {/if}
      {/if}

      {#if tab === "cerca"}
        <section class="panel-section">
          <h3>Ricerca simboli</h3>
          <form
            onsubmit={(event) => {
              event.preventDefault();
              void runSearch();
            }}
            style="display:flex;gap:6px"
          >
            <input
              type="search"
              bind:value={searchQuery}
              placeholder="es. cane, mangiare, scuola…"
              aria-label="Cerca un pittogramma"
              style="flex:1"
            />
            <button class="btn btn--solid" type="submit" disabled={searchBusy}>Cerca</button>
            <button
              class="btn btn--ghost"
              type="button"
              disabled={relatedBusy || !neuralReady}
              title={neuralReady
                ? "Chiede al modello neurale parole affini e cerca i pittogrammi corrispondenti"
                : "Richiede un modello neurale attivo (Impostazioni → Gestione modelli)"}
              onclick={() => void runRelatedSearch()}
            >{relatedBusy ? "Cerco…" : "Affini con l'IA"}</button>
          </form>
          <p class="hint">
            La ricerca usa l'indice locale (FTS5). Con un'unità selezionata, scegliere un risultato
            sostituisce il suo pittogramma. «Affini con l'IA» cerca per significato: i risultati sono
            suggerimenti approssimativi da verificare. I gruppi di affinità distinguono parola esatta,
            forma base, varianti, sinonimi, concetto generale e affini.
          </p>
          {#if affinityMatchesSearch}
            {@render affinityBlock()}
          {/if}
          {#if searchResults.length > 0}
            <div class="symbol-grid" style="margin-top:10px">
              {#each searchResults as alternative (alternative.symbol_id)}
                <button
                  class="symbol-tile"
                  title={`${alternative.meaning || alternative.label} — ${alternative.pack_name} (${alternative.license})`}
                  onclick={() => onApplyAlternative(alternative)}
                >
                  <span class="symbol-tile__media">
                    {#if alternative.svg}
                      {@html alternative.svg}
                    {:else}
                      <span class="caa-chip__empty">?</span>
                    {/if}
                  </span>
                  <span class="symbol-tile__label">{alternative.label}</span>
                  <span class="symbol-tile__pack">{alternative.pack_name}</span>
                  {#if alternative.neural}
                    <span class="badge badge--ai" title="Suggerito dal modello neurale: simbolo affine, da verificare">IA affine</span>
                  {/if}
                </button>
              {/each}
            </div>
          {/if}
        </section>
      {/if}

      {#if tab === "impostazioni"}
        <section class="panel-section">
          <h3>Rappresentazione CAA</h3>
          <div class="settings-row">
            <label for="mode-select">Modalità</label>
            <select
              id="mode-select"
              value={settings.mode}
              onchange={(event) =>
                onUpdateSettings({ mode: (event.currentTarget as HTMLSelectElement).value as never })}
            >
              <option value="testo">A · Solo testo</option>
              <option value="sopra">B · Simboli sopra le parole</option>
              <option value="sotto">C · Simboli con didascalia sotto</option>
              <option value="solo_simboli">D · Solo simboli</option>
              <option value="semplificata">E · CAA semplificata</option>
            </select>
          </div>
          <div class="settings-row">
            <label for="intensity-select">Intensità</label>
            <select
              id="intensity-select"
              value={settings.intensity}
              onchange={(event) =>
                onUpdateSettings({ intensity: (event.currentTarget as HTMLSelectElement).value as never })}
            >
              <option value="minima">Minima (parole chiave)</option>
              <option value="media">Intermedia</option>
              <option value="completa">Completa</option>
            </select>
          </div>
          <div class="settings-row">
            <label for="caption-toggle">Didascalie visibili</label>
            <input
              id="caption-toggle"
              type="checkbox"
              checked={settings.show_captions}
              onchange={(event) => onUpdateSettings({ show_captions: (event.currentTarget as HTMLInputElement).checked })}
            />
          </div>
          <div class="settings-row">
            <label for="border-select">Bordo del pittogramma</label>
            <select
              id="border-select"
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
          <div class="settings-row">
            <label for="mono-toggle">Stampa monocromatica</label>
            <input
              id="mono-toggle"
              type="checkbox"
              checked={settings.mono}
              onchange={(event) => onUpdateSettings({ mono: (event.currentTarget as HTMLInputElement).checked })}
            />
          </div>
        </section>

        <section class="panel-section">
          <h3>Accessibilità</h3>
          <div class="settings-row">
            <label for="theme-select">Tema</label>
            <select
              id="theme-select"
              value={settings.theme}
              onchange={(event) =>
                onUpdateSettings({ theme: (event.currentTarget as HTMLSelectElement).value as never })}
            >
              <option value="chiaro">Chiaro</option>
              <option value="scuro">Scuro</option>
              <option value="contrasto">Alto contrasto</option>
            </select>
          </div>
          <div class="settings-row">
            <label for="touch-toggle">Modalità tocco (smartboard)</label>
            <input
              id="touch-toggle"
              type="checkbox"
              checked={settings.touch_mode}
              onchange={(event) => onUpdateSettings({ touch_mode: (event.currentTarget as HTMLInputElement).checked })}
            />
          </div>
          <div class="settings-row">
            <label for="font-range">Dimensione testo</label>
            <input
              id="font-range"
              type="range"
              min="12"
              max="28"
              value={settings.font_size}
              oninput={(event) => onUpdateSettings({ font_size: Number((event.currentTarget as HTMLInputElement).value) })}
            />
          </div>
          <div class="settings-row">
            <label for="page-select">Formato pagina</label>
            <select
              id="page-select"
              value={settings.page}
              onchange={(event) =>
                onUpdateSettings({ page: (event.currentTarget as HTMLSelectElement).value as never })}
            >
              <option value="a4">A4 verticale</option>
              <option value="a4-orizzontale">A4 orizzontale</option>
              <option value="a3">A3</option>
            </select>
          </div>
          <div class="settings-row">
            <label for="voice-select">Voce di sintesi</label>
            <select id="voice-select" aria-label="Voce di sintesi">
              <option value="">Voce predefinita</option>
              {#each voices as voice (voice.id)}
                <option value={voice.id}>{voice.name} ({voice.language})</option>
              {/each}
            </select>
          </div>
        </section>

        <section class="panel-section">
          <h3>Repertori e licenze</h3>
          <div class="settings-row">
            <label for="pack-select">Repertorio</label>
            <select
              id="pack-select"
              value={settings.pack_id ?? ""}
              onchange={(event) =>
                onUpdateSettings({ pack_id: (event.currentTarget as HTMLSelectElement).value || null })}
            >
              <option value="">Tutti i repertori</option>
              {#each packs as pack (pack.id)}
                <option value={pack.id}>{pack.name} ({pack.symbol_count})</option>
              {/each}
            </select>
          </div>
          <div class="settings-row">
            <label for="commercial-toggle">Solo simboli compatibili con uso commerciale</label>
            <input
              id="commercial-toggle"
              type="checkbox"
              checked={settings.commercial_only}
              onchange={(event) =>
                onUpdateSettings({ commercial_only: (event.currentTarget as HTMLInputElement).checked })}
            />
          </div>
          {#if stats}
            <dl class="detail-list" style="margin-top:8px">
              <dt>Simboli</dt><dd>{stats.symbols}</dd>
              <dt>Concetti</dt><dd>{stats.concepts}</dd>
              <dt>Lemmi</dt><dd>{stats.lemmas}</dd>
              <dt>Espressioni</dt><dd>{stats.expressions}</dd>
            </dl>
          {/if}
          <p class="hint" style="margin-top:8px">
            Pittogrammi ARASAAC inclusi (intero catalogo italiano): autore Sergio Palao, titolare dei
            diritti ARASAAC — Governo di Aragona, licenza CC BY-NC-SA 4.0 (https://arasaac.org).
            Modifica applicata: PNG originale incorporato in un contenitore SVG.
          </p>
          <button class="btn btn--ghost" style="margin-top:8px" onclick={() => onClearHistory()}>
            Cancella cronologia ricerche
          </button>
        </section>

        <section class="panel-section">
          <h3>Intelligenza artificiale locale (facoltativa)</h3>
          {#if aiStatus}
            <p class="hint">{aiStatus.note}</p>
            {#if aiStatus.last_error}
              <p class="warn">Ultimo errore: {aiStatus.last_error}</p>
            {/if}
          {/if}
          <div class="settings-row">
            <label for="ai-enabled">Attiva modello locale</label>
            <input
              id="ai-enabled"
              type="checkbox"
              checked={aiDraft.enabled}
              onchange={(event) => (aiDraft.enabled = (event.currentTarget as HTMLInputElement).checked)}
            />
          </div>
          <label class="hint" for="ai-binary">Eseguibile llama.cpp (llama-cli)</label>
          <input id="ai-binary" type="text" bind:value={aiDraft.binary_path} placeholder="/percorso/llama-cli" style="width:100%" />
          <label class="hint" for="ai-model">Modello GGUF</label>
          <input id="ai-model" type="text" bind:value={aiDraft.model_path} placeholder="/percorso/qwen3-0.6b-q4_k_m.gguf" style="width:100%" />
          <div style="display:flex;gap:6px;margin-top:8px">
            <label class="hint" for="ai-threads">Thread</label>
            <input id="ai-threads" type="number" min="1" max="8" bind:value={aiDraft.threads} style="width:70px" />
            <label class="hint" for="ai-ctx">Contesto</label>
            <input id="ai-ctx" type="number" min="512" max="8192" step="256" bind:value={aiDraft.context_size} style="width:90px" />
            <label class="hint" for="ai-timeout">Timeout (ms)</label>
            <input id="ai-timeout" type="number" min="1000" max="60000" step="500" bind:value={aiDraft.timeout_ms} style="width:90px" />
          </div>
          <button class="btn btn--solid" style="margin-top:8px" onclick={() => onSetAiConfig(aiDraft)}>
            Salva configurazione
          </button>
        </section>

        <ModelManager {aiStatus} {onToast} {onStatusChange} />

        <section class="panel-section">
          <h3>Strumenti IA</h3>
          <p class="hint">
            Strumenti tipizzati con schema, timeout e fallback. Se il modello non è attivo
            rispondono le regole deterministiche e il risultato lo dichiara.
          </p>
          <select bind:value={toolChoice} aria-label="Strumento IA">
            {#each toolDefs as definition (definition.id)}
              <option value={definition.id}>{definition.label}</option>
            {/each}
          </select>
          <textarea
            bind:value={toolInput}
            placeholder={activeTool.hint}
            aria-label="Testo per lo strumento IA"
            style="margin-top:6px"
          ></textarea>
          <button class="btn btn--solid" style="margin-top:8px" disabled={toolBusy} onclick={runTool}>
            {toolBusy ? "Elaborazione…" : "Esegui"}
          </button>
          {#if toolOutcome}
            <p class="hint" style="margin-top:6px">Origine: {toolOutcome.source}</p>
            <pre style="white-space:pre-wrap;font-size:0.78rem">{JSON.stringify(toolOutcome.result, null, 2)}</pre>
            {#each toolOutcome.notes as note}
              <p class="hint">{note}</p>
            {/each}
          {/if}
        </section>

        <section class="panel-section">
          <h3>Preferenze salvate</h3>
          {#if preferences.length === 0}
            <p class="hint">Nessuna preferenza. Le scelte fatte qui restano locali.</p>
          {:else}
            <ul style="margin:0;padding-left:18px;font-size:0.82rem">
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
          <section class="panel-section">
            <h3>Informazioni</h3>
            <dl class="detail-list">
              <dt>Versione</dt><dd>{appInfo.version}</dd>
              <dt>Schema DB</dt><dd>v{appInfo.schema_version}</dd>
              <dt>Lessico</dt><dd>{appInfo.lexicon_lemmas} lemmi / {appInfo.lexicon_concepts} concetti</dd>
              <dt>Piattaforma</dt><dd>{appInfo.platform}</dd>
            </dl>
            <p class="hint" style="margin-top:6px">Dati locali: {appInfo.data_dir}</p>
            {#each appInfo.warnings as warning}
              <p class="warn">{warning}</p>
            {/each}
          </section>
        {/if}
      {/if}

      {#if tab === "studio"}
        <section class="panel-section">
          <h3>Visual Resource Studio</h3>
          <p class="hint">
            Descrivi la risorsa: il sistema sceglie un modello, estrae i contenuti e propone i
            pittogrammi. Tutto resta modificabile.
          </p>
          <textarea
            bind:value={studioPrompt}
            placeholder="Crea un'agenda visiva settimanale per la scuola primaria con: entrata a scuola, intervallo, pranzo, gioco"
            aria-label="Richiesta per generare la risorsa"
          ></textarea>
          <div style="display:flex;gap:6px;margin-top:8px">
            <select bind:value={templateChoice} aria-label="Modello">
              <option value={null}>Modello automatico</option>
              {#each templates as template (template.id)}
                <option value={template.id}>{template.name}</option>
              {/each}
            </select>
            <button class="btn btn--solid" onclick={generate} disabled={studioBusy}>Genera</button>
          </div>
        </section>

        <section class="panel-section">
          <h3>Modelli disponibili</h3>
          {#if templates.length === 0}
            <p class="hint">Nessun modello caricato. Verifica la cartella resources/templates.</p>
          {:else}
            <ul style="margin:0;padding-left:18px;font-size:0.82rem">
              {#each templates as template (template.id)}
                <li style="margin-bottom:4px">
                  <strong>{template.name}</strong>
                  <span class="hint"> · {template.category}</span>
                  <div class="hint">{template.description}</div>
                </li>
              {/each}
            </ul>
          {/if}
        </section>

        {#if plan}
          <section class="panel-section">
            <h3>Anteprima · {plan.title}</h3>
            <p class="hint">
              {plan.items.length} elementi · modello {plan.template_id} · interpretazione {plan.source}
            </p>
            <div
              class="studio-grid"
              style={`grid-template-columns: repeat(${plan.grid.columns}, minmax(0, 1fr));`}
            >
              {#each plan.items as item (item.label)}
                <div class="studio-cell" title={item.concept_id ?? "nessun concetto"}>
                  {#if item.svg}
                    {@html item.svg}
                  {:else}
                    <span class="caa-chip__empty" style="width:44px;height:44px">?</span>
                  {/if}
                  <input value={item.label} aria-label={`Etichetta ${item.label}`} readonly />
                  {#if item.approximate}<span class="badge badge--warn">≈</span>{/if}
                </div>
              {/each}
            </div>
            {#each plan.notes as note}
              <p class="hint">{note}</p>
            {/each}
            <div style="display:flex;gap:6px;margin-top:8px">
              <button class="btn btn--ghost" onclick={onPrintResource}>Stampa / PDF</button>
            </div>
          </section>
        {/if}
      {/if}
    </div>
  {/if}
</aside>
