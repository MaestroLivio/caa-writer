<script lang="ts">
  import { api, isTauri } from "../api";
  import { isExplicitContent } from "../content-filter";
  import ModelManager from "./ModelManager.svelte";
  import type { AssociationScope } from "../workspace/layout";
  import type { CollectionInfo, OnlineResult } from "../types";
  import type {
    AiChatMessage,
    AiChatReply,
    AiConfig,
    AiStatus,
    Alternative,
    AppInfo,
    CaaToken,
    DocumentSettings,
    ImageConfig,
    ImageStatus,
    LibraryStats,
    UniversalDocument,
    PackInfo,
    Preference,
    ResourcePlan,
    TemplateInfo,
    ToolOutcome,
    VoiceInfo,
    WordAffinity,
  } from "../types";

  type Tab = "simbolo" | "collezioni" | "impostazioni" | "studio" | "ia" | "documento";

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
    collections: CollectionInfo[];
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
    /// Ambito scelto per il prossimo pittogramma applicato.
    associationScope: AssociationScope;
    onAssociationScope: (scope: AssociationScope) => void;
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
    onGenerateImage: (prompt: string, mode: "apply" | "insert") => Promise<void>;
    onAiChat: (messages: AiChatMessage[]) => Promise<AiChatReply>;
    universalDoc: UniversalDocument | null;
    onUniversalTitle: (title: string) => void;
    onUniversalPage: (patch: Record<string, unknown>) => void;
    onOpenInEditor: (plan: ResourcePlan) => void;
    onOpenUniversalTemplate: (templateId: string) => void;
    onPrintResource: () => void;
    onToast: (message: string, kind?: "info" | "warn" | "error") => void;
    onClearHistory: () => Promise<void>;
    onRefreshCollections: () => Promise<void>;
    onImportImages: () => void;
    onImportFolder: () => void;
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
    collections,
    intent,
    closed = $bindable(false),
    overlay = false,
    onCollapse,
    onToggleDock,
    associationScope,
    onAssociationScope,
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
    onGenerateImage,
    onAiChat,
    universalDoc,
    onUniversalTitle,
    onUniversalPage,
    onOpenInEditor,
    onOpenUniversalTemplate,
    onPrintResource,
    onToast,
    onClearHistory,
    onRefreshCollections,
    onImportImages,
    onImportFolder,
  }: Props = $props();

  let tab = $state<Tab>("simbolo");
  let collectionFilter = $state<string | null>(null);
  let collectionFilterName = $state("");
  let renamingId = $state<string | null>(null);
  let renameValue = $state("");
  let deleteCandidate = $state<CollectionInfo | null>(null);
  let onlineResults = $state<OnlineResult[]>([]);
  let onlineThumbs = $state<Record<string, string>>({});
  let onlineBusy = $state(false);
  let onlineError = $state("");
  let onlineQuery = $state("");
  let onlineCached = $state(false);
  let autoOnlineKey = "";
  let onlineTimer: number | undefined;
  let onlineSeq = 0;
  const onlineCache = new Map<
    string,
    { results: OnlineResult[]; thumbs: Record<string, string>; query: string; cached: boolean }
  >();
  const thumbCache = new Map<string, string>();
  let hydratedMedia = $state<Record<string, string>>({});
  const hydrationRequested = new Set<string>();
  type MediaItem = {
    symbol_id: string;
    svg?: string | null;
    image?: string | null;
    pack_id?: string | null;
  };
  /// SVG o PNG (data URL) già presente, oppure scaricato on demand.
  const mediaFor = (item: MediaItem): { svg: string | null; image: string | null } => {
    const hydrated = hydratedMedia[item.symbol_id];
    return {
      svg: item.svg ?? (hydrated && !hydrated.startsWith("data:") ? hydrated : null),
      image: item.image ?? (hydrated?.startsWith("data:") ? hydrated : null),
    };
  };
  const applyMedia = <T extends MediaItem>(item: T): T & { svg: string | null; image: string | null } =>
    ({ ...item, ...mediaFor(item) });
  let searchActive = $state(false);
  let neuralBusy = $state(false);
  let showAdvanced = $state(false);
  let lastPanelSelectionKey = "";

  /// Etichetta breve della fonte, per la griglia compatta.
  function sourceLabel(item: {
    pack_id?: string | null;
    pack_name?: string | null;
    source_url?: string | null;
  }): string {
    const pack = (item.pack_id ?? "").toLocaleLowerCase("it");
    const name = item.pack_name ?? "";
    const sourceUrl = (item.source_url ?? "").toLocaleLowerCase("it");
    if (sourceUrl.includes("arasaac.org")) return "ARASAAC";
    if (sourceUrl.includes("globalsymbols")) return "Global Symbols";
    if (sourceUrl.includes("wikimedia")) return "Wikimedia";
    if (sourceUrl.includes("openverse")) return "Openverse";
    if (pack === "arasaac" || /arasaac/i.test(name)) return "ARASAAC";
    if (pack === "core" || /caa writer|core/i.test(name)) return "CAA Writer";
    if (pack === "personal" && /global symbols|inglese/i.test(name)) return "Global Symbols";
    if (pack === "personal") return "Personale";
    if (/wikimedia/i.test(name)) return "Wikimedia";
    if (/openverse/i.test(name)) return "Openverse";
    const short = name.split("—")[0].trim();
    return short || "Repertorio";
  }

  const normalizeText = (value: string) =>
    value
      .toLocaleLowerCase("it")
      .normalize("NFD")
      .replace(/[\u0300-\u036f]/g, "")
      .trim();

  /// Ordine automatico: prima l'etichetta che coincide con la parola, poi le
  /// alternative del motore nell'ordine già calcolato, quindi gli affini e
  /// l'inglese. I candidati approssimativi scendono in fondo.
  function relevanceScore(item: Alternative, word: string): number {
    const label = normalizeText(item.label);
    let score = 0;
    if (label === word) score += 120;
    else if (label.startsWith(`${word} `) || word.startsWith(`${label} `)) score += 70;
    else if (word.length >= 3 && (label.includes(word) || word.includes(label))) score += 25;
    const source = sourceLabel(item);
    // L'ordine delle fasi domina sulla somiglianza testuale: prima i simboli
    // locali/ARASAAC, poi i sinonimi italiani, infine Global Symbols. Dentro
    // ogni fase conta l'etichetta esatta e poi l'ordine del motore.
    if (source === "Global Symbols") score -= 1000;
    else if (item.neural) score -= 500;
    if (item.approximate) score -= 18;
    if (source === "ARASAAC") score += 12;
    else if (source === "CAA Writer") score += 6;
    return score;
  }

  function rankAlternatives(items: Alternative[], word: string): Alternative[] {
    // Deduplica per identificativo, non per etichetta: lo stesso lemma può
    // avere più pittogrammi diversi (più varianti ARASAAC) e collassare per
    // etichetta li nascondeva tutti tranne uno.
    const seen = new Set<string>();
    return items
      .filter((item) => {
        if (!item.symbol_id || seen.has(item.symbol_id)) return false;
        seen.add(item.symbol_id);
        return true;
      })
      .map((item, index) => ({ item, index, score: relevanceScore(item, word) }))
      .sort((a, b) => b.score - a.score || a.index - b.index)
      .map((entry) => entry.item);
  }

  const displayAlternatives = $derived.by(() => {
    if (!selected) return [];
    const word = normalizeText(selected.token.lemma ?? selected.token.surface);
    const items = mergedAlternatives.filter(
      (item) => item.symbol_id !== selected.token.symbol_id
    );
    return rankAlternatives(items, word).slice(0, 24);
  });

  const searchDisplay = $derived.by(() => {
    const word = normalizeText(searchQuery);
    return rankAlternatives(safeSearchResults, word).slice(0, 24);
  });

  async function applyFromPanel(alternative: Alternative) {
    const prepared = applyMedia(alternative);
    // Prima di applicare, assicura che il pittogramma scelto sia davvero
    // disponibile in locale: gli ARASAAC con file mancante vengono scaricati
    // e salvati nella cartella dati dell'utente.
    if (prepared.symbol_id && !prepared.svg && !prepared.image) {
      try {
        const hydrated = await api.hydrateSymbols([prepared.symbol_id]);
        const value = hydrated[prepared.symbol_id];
        if (value) {
          if (value.startsWith("data:")) prepared.image = value;
          else prepared.svg = value;
        }
      } catch {
        /* offline: l'idratazione riproverà al prossimo tentativo */
      }
    }
    searchActive = false;
    searchQuery = "";
    searchResults = [];
    await onApplyAlternative(prepared);
  }
  let searchQuery = $state("");
  /// La ricerca nel repertorio locale è in italiano. In modalità EN la
  /// richiesta viene tradotta dal modello neurale prima di interrogare FTS5.
  let searchLanguage = $state<"it" | "en">("it");
  let translatedQuery = $state("");
  let translateBusy = $state(false);
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
  let libQuery = $state("");
  let libCategory = $state("");
  let libKind = $state("");
  let libComplexity = $state("");
  let libMinCells = $state(0);
  let libUsage = $state("");
  const templateCategories = $derived(
    [...new Set(templates.map((template) => template.category))].sort()
  );
  const filteredTemplates = $derived(
    templates.filter((template) => {
      if (libCategory && template.category !== libCategory) return false;
      if (libKind && template.kind !== libKind) return false;
      if (libComplexity && (template.complexity ?? "") !== libComplexity) return false;
      if (libUsage === "printable" && template.printable === false) return false;
      if (libUsage === "interactive" && template.interactive !== true) return false;
      if (libMinCells > 0) {
        const cells = (template.grid?.columns ?? 0) * (template.grid?.rows ?? 0);
        if (cells < libMinCells) return false;
      }
      const query = libQuery.trim().toLowerCase();
      if (query) {
        const haystack = `${template.name} ${template.description} ${template.category}`.toLowerCase();
        if (!haystack.includes(query)) return false;
      }
      return true;
    })
  );
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
  // Il pannello dichiara lo stato reale: senza runtime o modello — o con
  // l'analisi neurale disattivata nelle impostazioni — i suggerimenti
  // neurali non vengono offerti come se fossero disponibili.
  const neuralReady = $derived(
    Boolean(
      settings.neural_context &&
        aiStatus?.enabled &&
        aiStatus.binary_present &&
        aiStatus.model_present
    )
  );

  // Nella scheda Cerca i gruppi restano legati alla parola effettivamente
  // cercata: digitando altro il blocco sparisce finché non si preme Cerca.
  const effectiveSearchText = $derived(
    searchLanguage === "en" && translatedQuery ? translatedQuery : searchQuery
  );
  const affinityMatchesSearch = $derived(
    Boolean(
      affinity &&
        effectiveSearchText.trim() &&
        affinity.word.toLocaleLowerCase() === effectiveSearchText.trim().toLocaleLowerCase()
    )
  );
  let showOtherResults = $state(false);
  const searchWord = $derived(effectiveSearchText.trim().toLocaleLowerCase());
  const safeAlternatives = $derived(
    (selected?.token.alternatives ?? []).filter(
      (item) =>
        settings.show_explicit_symbols ||
        !isExplicitContent(item.label, item.meaning, item.symbol_id)
    )
  );
  const safeSearchResults = $derived(
    searchResults.filter(
      (item) =>
        settings.show_explicit_symbols ||
        !isExplicitContent(item.label, item.meaning, item.symbol_id)
    )
  );
  const safeOnlineResults = $derived(
    onlineResults.filter(
      (item) => settings.show_explicit_symbols || !isExplicitContent(item.title, item.source)
    )
  );
  const hiddenExplicitCount = $derived(
    searchResults.length - safeSearchResults.length + (onlineResults.length - safeOnlineResults.length)
  );
  const relevantResults = $derived.by(() =>
    safeSearchResults.filter((item) => {
      const label = item.label.trim().toLocaleLowerCase();
      return label === searchWord || label.startsWith(`${searchWord} `);
    })
  );
  const otherResults = $derived(safeSearchResults.filter((item) => !relevantResults.includes(item)));

  // Alternative aggiunte dalla ricerca in inglese: il termine viene tradotto
  // dal modello locale (o dal glossario) e i pittogrammi ARASAAC trovati in
  // inglese vengono importati come copie offline e mostrati insieme agli altri.
  let englishItems = $state<Alternative[]>([]);
  let englishTerm = $state("");
  let englishBusy = $state(false);
  let englishError = $state("");
  // Sinonimi e concetti affini dal modello locale: richiesta a bassa
  // priorità, in background, una volta per parola.
  let neuralItems = $state<Alternative[]>([]);
  const neuralCache = new Map<string, Alternative[]>();
  const neuralFailures = new Set<string>();
  let neuralTimer: number | undefined;
  let neuralRequest = 0;
  const englishCache = new Map<string, { term: string; items: Alternative[] }>();
  const englishFailures = new Map<string, string>();
  let englishTimer: number | undefined;
  const englishIds = $derived(new Set(englishItems.map((item) => item.symbol_id)));
  const safeNeuralItems = $derived(
    neuralItems.filter(
      (item) =>
        settings.show_explicit_symbols ||
        !isExplicitContent(item.label, item.meaning, item.symbol_id)
    )
  );
  const safeEnglishItems = $derived(
    englishItems.filter(
      (item) =>
        settings.show_explicit_symbols ||
        !isExplicitContent(item.label, item.meaning, item.symbol_id)
    )
  );
  const mergedAlternatives = $derived.by(() => {
    const seen = new Set(safeAlternatives.map((item) => item.symbol_id));
    const withNeural = [...safeAlternatives];
    for (const item of safeNeuralItems) {
      if (!seen.has(item.symbol_id)) {
        seen.add(item.symbol_id);
        withNeural.push(item);
      }
    }
    for (const item of safeEnglishItems) {
      if (!seen.has(item.symbol_id)) {
        seen.add(item.symbol_id);
        withNeural.push(item);
      }
    }
    return withNeural;
  });

  $effect(() => {
    const ids = new Set<string>();
    const add = (
      item:
        | { symbol_id?: string | null; svg?: string | null; image?: string | null; pack_id?: string | null }
        | null
        | undefined
    ) => {
      if (
        item?.symbol_id &&
        !item.image &&
        (item.pack_id === "arasaac" || !item.svg) &&
        !hydrationRequested.has(item.symbol_id)
      ) {
        ids.add(item.symbol_id);
      }
    };
    for (const item of mergedAlternatives) add(item);
    for (const item of safeSearchResults) add(item);
    for (const group of affinity?.groups ?? []) {
      for (const candidate of group.candidates) add(candidate.symbol);
    }
    if (selected?.token) add(selected.token);
    if (ids.size === 0) return;
    const batch = [...ids].slice(0, 32);
    batch.forEach((id) => hydrationRequested.add(id));
    void api
      .hydrateSymbols(batch)
      .then((map) => {
        if (Object.keys(map).length > 0) hydratedMedia = { ...hydratedMedia, ...map };
        // I simboli non scaricati restano segnaposto; si riproverà più tardi.
        batch
          .filter((id) => !map[id])
          .forEach((id) => hydrationRequested.delete(id));
      })
      .catch(() => {
        batch.forEach((id) => hydrationRequested.delete(id));
      });
  });

  // Motore immagini neurale: stato reale, configurazione e download. Il
  // runtime e i modelli restano nei dati dell'app, mai nel repository.
  let imageStatus = $state<ImageStatus | null>(null);
  let imagePrompt = $state("");
  let imageBusy = $state(false);
  let imageActionBusy = $state(false);
  let imageActionNote = $state("");
  let imageKey = $state("");
  let imageKeyBusy = $state(false);

  let chatMessages = $state<AiChatMessage[]>([]);
  let chatInput = $state("");
  let chatBusy = $state(false);
  let chatError = $state("");
  let chatBackend = $state("");

  async function sendChat() {
    const text = chatInput.trim();
    if (!text || chatBusy) return;
    chatMessages = [...chatMessages, { role: "user", content: text }];
    chatInput = "";
    chatError = "";
    chatBusy = true;
    try {
      const reply = await onAiChat($state.snapshot(chatMessages) as AiChatMessage[]);
      chatMessages = [...chatMessages, { role: "assistant", content: reply.reply }];
      chatBackend = reply.backend;
    } catch (error) {
      chatError = error instanceof Error ? error.message : String(error);
    } finally {
      chatBusy = false;
    }
  }

  async function refreshImageStatus() {
    if (!isTauri()) return;
    try {
      imageStatus = await api.imageStatus();
    } catch (error) {
      imageActionNote = error instanceof Error ? error.message : String(error);
    }
  }

  $effect(() => {
    void refreshImageStatus();
  });

  $effect(() => {
    if (tab === "documento" && !universalDoc) tab = "simbolo";
  });

  async function updateImageConfig(patch: Partial<ImageConfig>) {
    if (!imageStatus) return;
    try {
      imageStatus = await api.imageSetConfig({ ...imageStatus.config, ...patch });
    } catch (error) {
      onToast(error instanceof Error ? error.message : String(error), "error");
    }
  }

  async function installImageRuntime() {
    imageActionBusy = true;
    imageActionNote = "Installazione del runtime: può richiedere molti minuti.";
    try {
      imageActionNote = await api.imageInstallRuntime();
    } catch (error) {
      imageActionNote = error instanceof Error ? error.message : String(error);
    } finally {
      imageActionBusy = false;
      await refreshImageStatus();
    }
  }

  async function downloadImageModel() {
    if (!imageStatus) return;
    imageActionBusy = true;
    imageActionNote = `Scarico «${imageStatus.config.model}»…`;
    try {
      imageActionNote = await api.imageDownloadModel(imageStatus.config.model);
    } catch (error) {
      imageActionNote = error instanceof Error ? error.message : String(error);
    } finally {
      imageActionBusy = false;
      await refreshImageStatus();
    }
  }

  async function saveImageKey() {
    if (!imageKey.trim()) {
      onToast("Incolla prima la chiave API.", "warn");
      return;
    }
    imageKeyBusy = true;
    try {
      imageStatus = await api.imageSetKey(imageKey.trim());
      imageKey = "";
      onToast("Chiave salvata nel portachiavi di sistema.", "info");
    } catch (error) {
      onToast(error instanceof Error ? error.message : String(error), "error");
    } finally {
      imageKeyBusy = false;
    }
  }

  async function clearImageKey() {
    try {
      imageStatus = await api.imageClearKey();
      onToast("Chiave API rimossa.", "info");
    } catch (error) {
      onToast(error instanceof Error ? error.message : String(error), "error");
    }
  }

  async function generateWithAI() {
    if (!selected) {
      onToast("Seleziona prima una parola nel documento.", "warn");
      return;
    }
    if (!imagePrompt.trim()) {
      onToast("Scrivi un prompt per l'immagine.", "warn");
      return;
    }
    imageBusy = true;
    try {
      await onGenerateImage(imagePrompt, "apply");
    } finally {
      imageBusy = false;
    }
  }

  async function generateAndInsert() {
    if (!imagePrompt.trim()) {
      onToast("Scrivi un prompt per l'immagine.", "warn");
      return;
    }
    imageBusy = true;
    try {
      await onGenerateImage(imagePrompt, "insert");
    } finally {
      imageBusy = false;
    }
  }

  async function loadEnglishAlternatives(word: string) {
    const clean = word.trim();
    if (!clean || !isTauri()) return;
    const key = clean.toLocaleLowerCase();
    const failed = englishFailures.get(key);
    if (failed) {
      englishTerm = "";
      englishItems = [];
      englishError = failed;
      return;
    }
    const cached = englishCache.get(key);
    if (cached) {
      englishTerm = cached.term;
      englishItems = cached.items;
      englishError = "";
      return;
    }
    englishBusy = true;
    englishError = "";
    try {
      const outcome = await api.englishAlternatives(clean, 4);
      englishTerm = outcome.term;
      englishItems = outcome.items;
      englishCache.set(key, { term: outcome.term, items: outcome.items });
      if (outcome.items.length === 0) {
        englishError = outcome.warnings[0] ?? `Nessun pittogramma inglese per «${outcome.term}».`;
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      englishTerm = "";
      englishItems = [];
      englishError = message;
      englishFailures.set(key, message);
    } finally {
      englishBusy = false;
    }
  }

  // I modelli e le metriche sono gestiti da `ModelManager`, montato nella
  // scheda Impostazioni: un solo posto, una sola fonte di stato.

  /// Carica i sinonimi suggeriti dal modello locale senza bloccare la
  /// scrittura. La richiesta viene sganciata dal ciclo di analisi e ha
  /// priorità bassa nella coda del motore; un errore non tocca le altre
  /// alternative.
  async function loadNeuralAlternatives(word: string) {
    const key = word.trim().toLocaleLowerCase("it");
    if (!key || !isTauri()) return;
    if (neuralCache.has(key)) {
      neuralItems = neuralCache.get(key) ?? [];
      return;
    }
    if (neuralFailures.has(key)) return;
    const request = ++neuralRequest;
    neuralBusy = true;
    try {
      const found = await api.aiAffinitySymbols(word);
      if (request !== neuralRequest) return;
      neuralCache.set(key, found);
      neuralItems = found;
    } catch {
      neuralFailures.add(key);
      if (request === neuralRequest) neuralItems = [];
    } finally {
      if (request === neuralRequest) neuralBusy = false;
    }
  }

  async function runRelatedSearch() {
    const raw = searchQuery.trim();
    if (!raw) {
      onToast("Scrivi una parola o una frase.", "warn");
      return;
    }
    if (!isTauri()) {
      onToast("La ricerca per significato richiede l'app desktop.", "warn");
      return;
    }
    relatedBusy = true;
    try {
      const query = searchLanguage === "en" ? await queryForSearch() : raw;
      if (query === null) return;
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
      tab = "simbolo";
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

  function onlineTerms(): string[] {
    if (!selected) return [];
    const token = selected.token;
    return [token.surface, token.lemma ?? "", token.concept_label ?? ""]
      .map((term) => term.trim())
      .filter(Boolean);
  }

  async function searchOnline(silent = false) {
    if (!selected) return;
    const terms = onlineTerms();
    if (terms.length === 0) return;
    const request = ++onlineSeq;
    onlineBusy = true;
    onlineError = "";
    try {
      const response = await api.searchOnlinePictograms(terms, false);
      if (request !== onlineSeq) return;
      const rank = (provider: string) =>
        provider === "arasaac" ? 0 : provider === "globalsymbols" ? 1 : provider === "wikimedia" ? 2 : provider === "openverse" ? 3 : 4;
      const normalize = (value: string) =>
        value.toLocaleLowerCase("it").normalize("NFD").replace(/[\u0300-\u036f]/g, "").trim();
      const word = normalize(selected?.token.surface ?? "");
      const relevance = (title: string) => {
        const value = normalize(title);
        if (value === word) return 0;
        if (word && (value.startsWith(word) || word.startsWith(value))) return 1;
        return 2;
      };
      const seen = new Set<string>();
      onlineResults = [...response.results]
        .sort((a, b) => rank(a.provider) - rank(b.provider) || relevance(a.title) - relevance(b.title))
        .filter((item) => {
          const key = `${item.provider}:${item.id}`;
          if (seen.has(key)) return false;
          seen.add(key);
          return true;
        });
      onlineQuery = response.query;
      onlineCached = response.cached;
      const thumbs: Record<string, string> = {};
      await Promise.allSettled(
        response.results.slice(0, 12).map(async (result) => {
          const url = result.thumbnail_url ?? result.image_url;
          if (thumbCache.has(url)) {
            thumbs[result.id] = thumbCache.get(url)!;
            return;
          }
          try {
            const thumb = await api.onlineThumbnail(url);
            thumbCache.set(url, thumb);
            thumbs[result.id] = thumb;
          } catch {
            /* anteprima non disponibile: resta il titolo */
          }
        })
      );
      if (request !== onlineSeq) return;
      onlineThumbs = thumbs;
      onlineCache.set(
        (selected?.token.surface ?? "").trim().toLocaleLowerCase("it"),
        {
          results: onlineResults,
          thumbs,
          query: onlineQuery,
          cached: onlineCached,
        }
      );
      if (response.results.length === 0 && !silent) {
        onToast(response.warnings[0] ?? "Nessun risultato online.", "warn");
      }
    } catch (error) {
      onlineError = error instanceof Error ? error.message : String(error);
    } finally {
      onlineBusy = false;
    }
  }

  async function useOnline(result: OnlineResult) {
    if (!selected) {
      onToast("Seleziona l'unità a cui applicare il pittogramma.", "warn");
      return;
    }
    onlineBusy = true;
    try {
      const lemma = selected.token.lemma ?? selected.token.surface;
      const alternative = await api.saveOnlinePictogram(result, lemma, "Online");
      onApplyAlternative(alternative);
      await onRefreshCollections();
      onToast("Pittogramma salvato nella collezione Online e applicato.", "info");
    } catch (error) {
      onToast(error instanceof Error ? error.message : String(error), "error");
    } finally {
      onlineBusy = false;
    }
  }

  function openCollection(collection: CollectionInfo) {
    collectionFilter = collection.id;
    collectionFilterName = collection.name;
    searchQuery = "";
    selectTab("simbolo");
    void runSearch();
  }

  function clearCollectionFilter() {
    collectionFilter = null;
    collectionFilterName = "";
    void runSearch();
  }

  function beginRename(collection: CollectionInfo) {
    renamingId = collection.id;
    renameValue = collection.name;
  }

  async function confirmRename() {
    if (!renamingId || !renameValue.trim()) return;
    try {
      await api.renamePictogramCollection(renamingId, renameValue.trim());
      await onRefreshCollections();
      if (collectionFilter === renamingId) collectionFilterName = renameValue.trim();
      onToast("Collezione rinominata.", "info");
      renamingId = null;
    } catch (error) {
      onToast(error instanceof Error ? error.message : String(error), "error");
    }
  }

  async function removeCollection(collection: CollectionInfo) {
    try {
      const removed = await api.deletePictogramCollection(collection.id);
      deleteCandidate = null;
      if (collectionFilter === collection.id) clearCollectionFilter();
      await onRefreshCollections();
      onToast(`${removed} pittogrammi eliminati con la collezione.`, "info");
    } catch (error) {
      onToast(error instanceof Error ? error.message : String(error), "error");
    }
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
    if (selectionReady && key && key !== lastSelectionKey) {
      tab = "simbolo";
      // La ricerca delle alternative parte sempre quando cambia l'unità:
      // l'utente non deve premere «Cerca» per vedere i candidati.
      searchQuery = selected!.token.surface;
      searchLanguage = "it";
      translatedQuery = "";
      void runSearch(true);
    }
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
    const word = tab === "simbolo" && selected ? selected.token.surface : "";
    // Cambiare repertorio o filtro commerciale deve ricalcolare i gruppi.
    void settings.pack_id;
    void settings.commercial_only;
    void settings.online_search;
    if (neuralTimer) window.clearTimeout(neuralTimer);
    // Invalida una richiesta ancora in volo per la parola precedente.
    neuralRequest += 1;
    if (!word) {
      englishItems = [];
      englishTerm = "";
      englishError = "";
      neuralItems = [];
      searchActive = false;
      searchQuery = "";
      searchResults = [];
      return;
    }
    const selectionKey = selected ? `${selected.pid}:${selected.token.index}` : "";
    if (selectionKey !== lastPanelSelectionKey) {
      lastPanelSelectionKey = selectionKey;
      // La ricerca manuale non deve restare appiccicata alla parola nuova.
      clearSearch();
    }
    const neuralKey = word.trim().toLocaleLowerCase("it");
    if (!neuralReady) {
      // Se il modello viene spento o non è ancora pronto, i risultati vuoti
      // non restano in cache: all'attivazione si riproverà.
      neuralCache.clear();
      neuralItems = [];
    } else {
      neuralItems = neuralCache.get(neuralKey) ?? [];
    }
    if (neuralReady && tab === "simbolo") {
      neuralTimer = window.setTimeout(() => {
        neuralTimer = undefined;
        void loadNeuralAlternatives(word);
      }, 1200);
    }
    // La ricerca inglese è un arricchimento: si attiva da sola solo quando la
    // ricerca online è consentita. Un errore di rete non blocca le alternative
    // e non viene ritentato a ogni tasto.
    if (settings.online_search !== "off") {
      if (englishTimer) window.clearTimeout(englishTimer);
      englishTimer = window.setTimeout(() => {
        englishTimer = undefined;
        void loadEnglishAlternatives(word);
      }, 450);
      // Ricerca online della parola selezionata: prima era codice morto
      // (mai chiamata, mai mostrata), quindi senza catalogo locale — come
      // su Windows — nessun pittogramma ARASAAC appariva. Parte quando la
      // ricerca locale non ha trovato un'etichetta esatta.
      if (onlineTimer) window.clearTimeout(onlineTimer);
      onlineTimer = window.setTimeout(() => {
        onlineTimer = undefined;
        const label = normalizeText(selected?.token.symbol_label ?? "");
        const exactLocally =
          label.length > 0 &&
          (label === neuralKey || label === normalizeText(selected?.token.lemma ?? ""));
        if (!exactLocally) void searchOnline(true);
      }, 700);
    }
    return () => {
      if (englishTimer) window.clearTimeout(englishTimer);
      if (onlineTimer) window.clearTimeout(onlineTimer);
      if (neuralTimer) window.clearTimeout(neuralTimer);
    };
  });

  /// Traduce la richiesta inglese in italiano con il modello locale. Se il
  /// modello non è disponibile restituisce null: non si inventa una traduzione
  /// e non si cercano parole diverse da quelle digitate.
  async function queryForSearch(): Promise<string | null> {
    const raw = searchQuery.trim();
    if (searchLanguage === "it" || !raw) {
      translatedQuery = "";
      return raw;
    }
    if (!isTauri()) {
      onToast("La traduzione richiede l'app desktop.", "warn");
      return null;
    }
    translateBusy = true;
    try {
      const translated = await api.translateSearchTerm(raw);
      translatedQuery = translated;
      return translated;
    } catch (error) {
      // La traduzione è un aiuto, non una condizione per cercare: se il
      // modello è lento o spento si prova comunque il termine originale.
      translatedQuery = raw;
      onToast(
        `Traduzione non disponibile (${error instanceof Error ? error.message : String(error)}). Cerco il termine originale.`,
        "warn"
      );
      return raw;
    } finally {
      translateBusy = false;
    }
  }

  async function runSearch(silent = false) {
    const raw = searchQuery.trim();
    if (!raw && !collectionFilter) {
      searchResults = [];
      affinity = null;
      affinityCacheKey = "";
      translatedQuery = "";
      return;
    }
    if (!isTauri()) {
      onToast("La ricerca simboli richiede l'app desktop.", "warn");
      return;
    }
    searchBusy = true;
    const query = await queryForSearch();
    if (query === null) {
      searchResults = [];
      searchBusy = false;
      return;
    }
    try {
      const page = await api.searchSymbols(
        query,
        60,
        0,
        settings.pack_id,
        settings.commercial_only,
        collectionFilter
      );
      let items = [...page.items];
      // La ricerca manuale è anch'essa online: i risultati ARASAAC/Global
      // Symbols vengono importati come copie locali e compaiono nello stesso
      // elenco, senza un pulsante separato. Wikimedia è disattivato a monte.
      if (!collectionFilter) {
        try {
          const response = await api.searchOnlinePictograms([query], false);
          const online = response.results
            .filter((result) => result.kind === "pittogramma")
            .slice(0, 6);
          for (const result of online) {
            try {
              const alternative = await api.saveOnlinePictogram(result, query, "Ricerca");
              if (!items.some((item) => item.symbol_id === alternative.symbol_id)) {
                items.push(alternative);
              }
            } catch {
              /* un provider non raggiungibile non blocca gli altri risultati */
            }
          }
        } catch {
          /* offline: restano i risultati locali */
        }
      }
      const seen = new Set<string>();
      searchResults = items.filter((item) => {
        const key = item.symbol_id;
        if (seen.has(key)) return false;
        seen.add(key);
        return true;
      });
      searchActive = true;
      showOtherResults = false;
      if (!silent && searchResults.length === 0) onToast("Nessun simbolo trovato per questa ricerca.", "warn");
    } catch (error) {
      onToast(error instanceof Error ? error.message : String(error), "error");
    } finally {
      searchBusy = false;
    }
  }

  function clearSearch() {
    searchActive = false;
    searchQuery = "";
    searchResults = [];
    searchBusy = false;
  }

  /// Qualsiasi parola, anche senza concetto o senza pittogramma, può ricevere
  /// un'immagine: si cerca la parola intera nel repertorio e si applica il
  /// risultato all'unità selezionata.
  async function searchForSelectedWord() {
    if (!selected) return;
    searchQuery = selected.token.surface;
    searchLanguage = "it";
    translatedQuery = "";
    onSwitchTab("simbolo");
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

  async function openTemplate(template: (typeof templates)[number]) {
    templateChoice = template.id;
    studioBusy = true;
    try {
      plan = await onGenerateResource(studioPrompt.trim() || template.name, template.id);
    } finally {
      studioBusy = false;
    }
  }

  async function handleImportSvg() {
    if (!svgFile) {
      onToast("Scegli prima un'immagine o un SVG.", "warn");
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
            {#each group.candidates.filter((candidate) => settings.show_explicit_symbols || !isExplicitContent(candidate.symbol.label, candidate.symbol.meaning)) as candidate (candidate.symbol.symbol_id)}
              <button
                class="symbol-tile"
                title={`${candidate.symbol.meaning || candidate.symbol.label} — ${candidate.reason}`}
                onclick={() => onApplyAlternative(applyMedia(candidate.symbol))}
              >
                <span class="symbol-tile__media">
                  {@render symbolMedia(candidate.symbol)}
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

{#snippet symbolMedia(item: MediaItem)}
  {@const media = mediaFor(item)}
  {#if media.image}
    <img src={media.image} alt="" draggable="false" />
  {:else if media.svg}
    {@html media.svg}
  {:else}
    <span class="caa-chip__empty">?</span>
  {/if}
{/snippet}

{#snippet alternativeTile(alternative: Alternative)}
  <button
    class="symbol-tile symbol-tile--compact"
    aria-current={selected?.token.symbol_id === alternative.symbol_id}
    title={`${alternative.meaning || alternative.label} — ${sourceLabel(alternative)} (${alternative.license})`}
    onclick={() => void applyFromPanel(alternative)}
  >
    <span class="symbol-tile__media">
      {@render symbolMedia(alternative)}
    </span>
    <span class="symbol-tile__label">{alternative.label}</span>
    <span class="symbol-tile__source">{sourceLabel(alternative)}</span>
    {#if alternative.approximate}
      <span class="symbol-tile__mark" title="Pittogramma approssimativo: verifica prima di scegliere">≈</span>
    {/if}
    {#if alternative.neural}
      <span class="symbol-tile__mark" title="Sinonimo o concetto affine suggerito dal modello locale">affine</span>
    {/if}
  </button>
{/snippet}

{#snippet onlineCard(result: OnlineResult)}
  <div class="online-card">
    {#if onlineThumbs[result.id]}
      <img class="online-thumb" src={onlineThumbs[result.id]} alt="" />
    {:else}
      <span class="online-thumb online-thumb--empty">?</span>
    {/if}
    <strong class="online-title">{result.title}</strong>
    <span class="online-meta">
      <span class="import-badge">{result.kind === "pittogramma" ? "pittogramma" : "fotografia"}</span>
      {result.source} · {result.license}
      {#if result.author} · {result.author}{/if}
    </span>
    <div class="online-actions">
      <button
        class="btn btn--solid"
        onclick={() => void useOnline(result)}
        disabled={onlineBusy || !selected}
      >Salva e applica</button>
      {#if result.page_url}
        <a href={result.page_url} target="_blank" rel="noreferrer">Fonte</a>
      {/if}
    </div>
  </div>
{/snippet}

{#snippet scopeChoice()}
  <div class="scope-choice">
    <label for="association-scope">Applica il pittogramma a:</label>
    <select
      id="association-scope"
      value={associationScope}
      onchange={(event) => onAssociationScope((event.currentTarget as HTMLSelectElement).value as AssociationScope)}
    >
      <option value="occurrence">Solo questa occorrenza</option>
      <option value="document">Tutte le occorrenze equivalenti nel documento</option>
      <option value="profile">Solo per questo profilo Utilizzatore</option>
      <option value="global">Per tutti i documenti futuri</option>
      <option value="vocabulary">Nel vocabolario condiviso del Creator</option>
    </select>
    <p class="hint">
      {associationScope === "occurrence"
        ? "La scelta vale solo per la parola selezionata."
        : associationScope === "document"
          ? "La scelta vale per le altre occorrenze dello stesso concetto nel documento; le associazioni bloccate non vengono toccate."
          : associationScope === "profile"
            ? "La scelta vale solo per il profilo Utilizzatore attivo, anche nei prossimi documenti; gli altri profili non vengono toccati."
            : associationScope === "vocabulary"
              ? "La scelta entra nel vocabolario condiviso del Creator ed è l'ultima risorsa dopo le preferenze del profilo e quelle globali."
              : "La scelta vale per tutti i documenti futuri, ma resta subordinata alla disambiguazione quando il significato cambia."}
    </p>
  </div>
{/snippet}

<aside class:closed class="sidepanel" class:sidepanel--closed={closed} aria-label="Pannello strumenti">
  <div class="sidepanel__tabs" role="tablist" aria-label="Sezioni del pannello">
    <button
      class="tab"
      role="tab"
      aria-selected={tab === "simbolo"}
      onclick={() => selectTab("simbolo")}>Simbolo</button
    >
    <button
      class="tab"
      role="tab"
      aria-selected={tab === "collezioni"}
      onclick={() => selectTab("collezioni")}>Collezioni</button
    >
    <button class="tab" role="tab" aria-selected={tab === "studio"} onclick={() => selectTab("studio")}
      >Studio</button
    >
    <button class="tab" role="tab" aria-selected={tab === "ia"} onclick={() => selectTab("ia")}
      >IA</button
    >
    {#if universalDoc}
      <button
        class="tab"
        role="tab"
        aria-selected={tab === "documento"}
        onclick={() => selectTab("documento")}>Documento</button
      >
    {/if}
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
      {#if tab === "simbolo"}
        <div class="symbol-tab">
          {#if selected}
            <div class="symbol-head">
              <div class="symbol-head__word">
                <span class="symbol-head__label">{selected.token.surface}</span>
                <span class="symbol-head__source">
                  {selected.token.symbol_id ? sourceLabel(selected.token) : "nessun pittogramma"}
                </span>
              </div>
              <div class="symbol-head__tools">
                {#if englishBusy || neuralBusy}
                  <span class="symbol-working" title="Cerco altre alternative in background">…</span>
                {/if}
                <button
                  class="icon-btn"
                  title="Ascolta la parola"
                  aria-label="Ascolta la parola"
                  onclick={() => void onSpeak(selected.token.surface)}
                >▶</button>
                <button
                  class="icon-btn"
                  class:active={selected.token.locked}
                  title={selected.token.locked ? "Sblocca il pittogramma" : "Blocca il pittogramma"}
                  aria-label={selected.token.locked ? "Sblocca il pittogramma" : "Blocca il pittogramma"}
                  onclick={() => onSetUnitState({ locked: !selected?.token.locked })}
                >{selected.token.locked ? "🔒" : "🔓"}</button>
                <button
                  class="icon-btn"
                  class:active={selected.token.hidden}
                  title={selected.token.hidden ? "Mostra il pittogramma" : "Nascondi il pittogramma"}
                  aria-label={selected.token.hidden ? "Mostra il pittogramma" : "Nascondi il pittogramma"}
                  onclick={() => onSetUnitState({ hidden: !selected?.token.hidden })}
                >{selected.token.hidden ? "◌" : "◉"}</button>
                <button
                  class="icon-btn"
                  class:active={showAdvanced}
                  title="Altre azioni"
                  aria-label="Altre azioni"
                  onclick={() => (showAdvanced = !showAdvanced)}
                >⋯</button>
              </div>
            </div>

            {#if showAdvanced}
              <div class="symbol-advanced">
                <div class="symbol-advanced__row">
                  <button class="btn btn--ghost btn--tiny" onclick={onResetUnit}>Ripristina scelta</button>
                  {#if safeAlternatives.length > 0}
                    <button
                      class="btn btn--ghost btn--tiny"
                      onclick={() => onSaveGlobalPreference(safeAlternatives[0])}
                      disabled={!selected.token.symbol_id}
                    >Usa sempre</button>
                  {/if}
                </div>
                <label class="symbol-advanced__scope">
                  <span>Ambito</span>
                  <select
                    value={associationScope}
                    onchange={(event) =>
                      onAssociationScope((event.currentTarget as HTMLSelectElement).value as AssociationScope)}
                  >
                    <option value="occurrence">Questa occorrenza</option>
                    <option value="document">Tutto il documento</option>
                    <option value="profile">Questo profilo</option>
                    <option value="global">Tutti i documenti</option>
                    <option value="vocabulary">Vocabolario condiviso</option>
                  </select>
                </label>
                <label class="symbol-advanced__import">
                  <span>Importa immagine</span>
                  <input
                    type="file"
                    accept="image/png,image/jpeg,image/webp,image/gif,image/bmp,image/tiff,image/avif,image/x-icon,.png,.jpg,.jpeg,.webp,.gif,.bmp,.tif,.tiff,.avif,.ico,.svg,image/svg+xml"
                    onchange={(event) => (svgFile = (event.currentTarget as HTMLInputElement).files?.[0] ?? null)}
                  />
                </label>
                {#if svgFile}
                  <button class="btn btn--ghost btn--tiny" onclick={handleImportSvg}>
                    Importa «{svgFile.name}»
                  </button>
                {/if}
              </div>
            {/if}

            {#if searchActive}
              <div class="symbol-results-head">
                <span>Risultati per «{searchQuery}»</span>
                <button class="btn btn--ghost btn--tiny" onclick={clearSearch}>Alternative</button>
              </div>
              {#if searchBusy}
                <p class="symbol-empty">Cerco…</p>
              {:else if searchDisplay.length === 0}
                <p class="symbol-empty">Nessun risultato. Prova un'altra parola.</p>
              {:else}
                <div class="symbol-grid symbol-grid--dense">
                  {#each searchDisplay as alternative (alternative.symbol_id)}
                    {@render alternativeTile(alternative)}
                  {/each}
                </div>
              {/if}
            {:else if displayAlternatives.length > 0}
              <div class="symbol-grid symbol-grid--dense">
                {#each displayAlternatives as alternative (alternative.symbol_id)}
                  {@render alternativeTile(alternative)}
                {/each}
              </div>
            {:else}
              <p class="symbol-empty">Nessun pittogramma automatico: cerca una parola qui sotto.</p>
            {/if}
          {:else}
            <p class="symbol-empty">Seleziona una parola nel testo per vedere i pittogrammi.</p>
          {/if}

          <form
            class="symbol-search"
            onsubmit={(event) => {
              event.preventDefault();
              void runSearch();
            }}
          >
            <input
              type="search"
              bind:value={searchQuery}
              placeholder="Cerca un'altra parola…"
              aria-label="Cerca un'altra parola da attribuire alla parola selezionata"
              disabled={!selected}
              oninput={() => {
                if (!searchQuery.trim()) clearSearch();
              }}
            />
            {#if searchActive}
              <button class="icon-btn" type="button" title="Torna alle alternative" aria-label="Torna alle alternative" onclick={clearSearch}>×</button>
            {/if}
            <button class="btn btn--solid btn--tiny" type="submit" disabled={!selected || searchBusy}>
              {searchBusy ? "…" : "Cerca"}
            </button>
          </form>

        </div>
      {/if}
      {#if tab === "ia"}
        <section class="panel-section">
          <h3>Chat con l'IA</h3>
          <p class="hint">
            Usa il modello locale se attivo, altrimenti il provider online configurato. I messaggi
            restano in questa sessione e non vengono salvati nel documento.
          </p>
          {#if chatMessages.length === 0}
            <p class="hint">Scrivi un messaggio per iniziare.</p>
          {:else}
            <div class="chat-log">
              {#each chatMessages as message}
                <div class:chat-bubble--user={message.role === "user"} class="chat-bubble">
                  {message.content}
                </div>
              {/each}
              {#if chatBusy}
                <div class="chat-bubble chat-bubble--pending">…</div>
              {/if}
            </div>
          {/if}
          <textarea
            class="image-prompt"
            bind:value={chatInput}
            rows="2"
            placeholder="Scrivi un messaggio…"
            aria-label="Messaggio per l'IA"
            onkeydown={(event) => {
              if (event.key === "Enter" && !event.shiftKey) {
                event.preventDefault();
                void sendChat();
              }
            }}
          ></textarea>
          <div class="image-actions">
            <button class="btn btn--solid" onclick={() => void sendChat()} disabled={chatBusy || !chatInput.trim()}>
              {chatBusy ? "Rispondo…" : "Invia"}
            </button>
            <button class="btn btn--ghost" onclick={() => (chatMessages = [])} disabled={chatBusy}>
              Nuova chat
            </button>
          </div>
          {#if chatBackend}<p class="hint">Ultima risposta: {chatBackend}</p>{/if}
          {#if chatError}<p class="import-error">{chatError}</p>{/if}
        </section>

        <section class="panel-section">
          <h3>Genera un'immagine</h3>
          <p class="hint">
            Il prompt viene inviato solo al motore configurato. Puoi applicare l'immagine alla
            parola selezionata come pittogramma oppure inserirla nel documento universale aperto.
          </p>
          <textarea
            class="image-prompt"
            bind:value={imagePrompt}
            rows="2"
            placeholder="es. pittogramma semplice di un gatto seduto, contorno nero, sfondo bianco"
            aria-label="Prompt per l'immagine"
          ></textarea>
          <div class="image-actions">
            <button
              class="btn btn--solid"
              onclick={() => void generateWithAI()}
              disabled={imageBusy || !imageStatus?.available || !selected}
              title={!selected
                ? "Seleziona una parola nel documento"
                : imageStatus?.available
                  ? "Genera e applica alla parola selezionata"
                  : "Motore immagini non disponibile"}
            >{imageBusy ? "Genero…" : "Genera e applica alla parola"}</button>
            <button
              class="btn"
              onclick={() => void generateAndInsert()}
              disabled={imageBusy || !imageStatus?.available}
              title="Inserisce l'immagine nel documento universale aperto"
            >Inserisci nel documento</button>
          </div>
          {#if imageStatus}<p class="hint">{imageStatus.note}</p>{/if}
          {#if imageActionNote}<p class="hint">{imageActionNote}</p>{/if}
        </section>

        <section class="panel-section">
          <h3>Modelli di diffusione</h3>
          <p class="hint">
            Motore per generare pittogrammi e immagini dal prompt. Su Apple Silicon usa la GPU
            (MPS) e MLX quando il runtime MLX Stable Diffusion è presente. Il runtime Python e i
            modelli non sono inclusi nell'installer: si scaricano qui, restano nei dati dell'app.
          </p>
          {#if imageStatus}
            <div class="settings-row">
              <label for="image-backend">Motore</label>
              <select
                id="image-backend"
                value={imageStatus.config.backend}
                onchange={(event) =>
                  void updateImageConfig({ backend: (event.currentTarget as HTMLSelectElement).value })}
              >
                <option value="auto">Automatico (MLX se disponibile, altrimenti GPU)</option>
                <option value="mlx">MLX (Apple Silicon)</option>
                <option value="gpu">GPU MPS</option>
                <option value="online">Provider online</option>
                <option value="off">Disattivato</option>
              </select>
            </div>
            {#if imageStatus.config.backend === "online"}
              <div class="settings-row">
                <label for="image-endpoint">Endpoint</label>
                <input
                  id="image-endpoint"
                  value={imageStatus.config.endpoint}
                  placeholder="https://api.openai.com/v1"
                  onchange={(event) =>
                    void updateImageConfig({ endpoint: (event.currentTarget as HTMLInputElement).value })}
                />
              </div>
              <div class="settings-row">
                <label for="image-online-model">Modello online</label>
                <input
                  id="image-online-model"
                  value={imageStatus.config.online_model}
                  placeholder="gpt-image-1"
                  onchange={(event) =>
                    void updateImageConfig({ online_model: (event.currentTarget as HTMLInputElement).value })}
                />
              </div>
              <div class="settings-row">
                <label for="image-key">Chiave API</label>
                <input
                  id="image-key"
                  type="password"
                  bind:value={imageKey}
                  placeholder={imageStatus.has_key ? "chiave salvata nel portachiavi" : "incolla la chiave"}
                />
                <button class="btn" onclick={() => void saveImageKey()} disabled={imageKeyBusy}>
                  {imageKeyBusy ? "Salvo…" : "Salva"}
                </button>
                {#if imageStatus.has_key}
                  <button class="btn btn--ghost" onclick={() => void clearImageKey()}>Rimuovi</button>
                {/if}
              </div>
            {:else if imageStatus.config.backend !== "off"}
              <div class="settings-row">
                <label for="image-model">Modello locale</label>
                <input
                  id="image-model"
                  value={imageStatus.config.model}
                  placeholder="stabilityai/sd-turbo"
                  onchange={(event) =>
                    void updateImageConfig({ model: (event.currentTarget as HTMLInputElement).value })}
                />
              </div>
              <div class="settings-row">
                <label for="image-size">Dimensione</label>
                <input
                  id="image-size"
                  type="number"
                  min="256"
                  max="1024"
                  step="64"
                  value={imageStatus.config.size}
                  onchange={(event) =>
                    void updateImageConfig({ size: Number((event.currentTarget as HTMLInputElement).value) || 512 })}
                />
                <label for="image-steps">Passi</label>
                <input
                  id="image-steps"
                  type="number"
                  min="4"
                  max="60"
                  value={imageStatus.config.steps}
                  onchange={(event) =>
                    void updateImageConfig({ steps: Number((event.currentTarget as HTMLInputElement).value) || 16 })}
                />
              </div>
              <div class="image-actions">
                <button class="btn" onclick={() => void installImageRuntime()} disabled={imageActionBusy}>
                  {imageActionBusy ? "Operazione in corso…" : "Installa runtime immagini"}
                </button>
                <button
                  class="btn btn--ghost"
                  onclick={() => void downloadImageModel()}
                  disabled={imageActionBusy || !imageStatus.config.model.trim()}
                >Scarica modello</button>
              </div>
            {/if}
            <p class="hint">{imageStatus.note}</p>
            {#if imageStatus.runtime.torch || imageStatus.runtime.mlx}
              <p class="hint">
                Runtime:
                {imageStatus.runtime.mlx ? "MLX" : "torch"}
                {#if imageStatus.runtime.mps} · GPU MPS{/if}
                {#if !imageStatus.runtime.diffusers} · diffusers assente{/if}
              </p>
            {/if}
            {#if imageActionNote}
              <p class="hint">{imageActionNote}</p>
            {/if}
          {:else}
            <p class="hint">Stato del motore immagini non disponibile.</p>
          {/if}
        </section>

      {/if}

      {#if tab === "documento" && universalDoc}
        <section class="panel-section">
          <h3>Documento</h3>
          <div class="settings-row">
            <label for="doc-title">Titolo</label>
            <input
              id="doc-title"
              value={universalDoc.title}
              onchange={(event) => onUniversalTitle(event.currentTarget.value)}
            />
          </div>
          <div class="settings-row">
            <label for="doc-format">Formato foglio</label>
            <select
              id="doc-format"
              value={universalDoc.page.width_mm && universalDoc.page.height_mm
                ? "custom"
                : `${universalDoc.page.size}|${universalDoc.page.orientation}`}
              onchange={(event) => {
                const value = event.currentTarget.value;
                if (value === "custom") return;
                const [size, orientation] = value.split("|");
                onUniversalPage({ size, orientation, width_mm: null, height_mm: null });
              }}
            >
              <option value="A4|portrait">A4 verticale (210×297)</option>
              <option value="A4|landscape">A4 orizzontale (297×210)</option>
              <option value="A3|portrait">A3 verticale (297×420)</option>
              <option value="A3|landscape">A3 orizzontale (420×297)</option>
              <option value="A5|portrait">A5 verticale (148×210)</option>
              <option value="LETTER|portrait">Letter (216×279)</option>
              <option value="16:9|landscape">16:9 (339×191)</option>
              <option value="4:3|landscape">4:3 (254×191)</option>
              <option value="custom">Personalizzato…</option>
            </select>
          </div>
          <div class="settings-row">
            <label for="doc-width">Larghezza (mm)</label>
            <input
              id="doc-width"
              type="number"
              min="50"
              max="2000"
              value={Math.round(universalDoc.page.width_mm ?? 210)}
              onchange={(event) =>
                onUniversalPage({
                  size: "custom",
                  orientation: "portrait",
                  width_mm: Number(event.currentTarget.value) || 210,
                  height_mm: universalDoc.page.height_mm ?? 297,
                })}
            />
          </div>
          <div class="settings-row">
            <label for="doc-height">Altezza (mm)</label>
            <input
              id="doc-height"
              type="number"
              min="50"
              max="2000"
              value={Math.round(universalDoc.page.height_mm ?? 297)}
              onchange={(event) =>
                onUniversalPage({
                  size: "custom",
                  orientation: "portrait",
                  width_mm: universalDoc.page.width_mm ?? 210,
                  height_mm: Number(event.currentTarget.value) || 297,
                })}
            />
          </div>
          <div class="settings-row">
            <label for="doc-margin">Margini (mm)</label>
            <input
              id="doc-margin"
              type="number"
              min="0"
              max="80"
              step="0.5"
              value={universalDoc.page.margin_mm ?? 15}
              onchange={(event) => onUniversalPage({ margin_mm: Number(event.currentTarget.value) || 0 })}
            />
          </div>
          <p class="hint">
            {universalDoc.pages.length} pagine · {universalDoc.pages.reduce(
              (total, page) => total + page.elements.length,
              0
            )} elementi. Le stesse impostazioni sono nel pannello sinistro delle pagine.
          </p>
        </section>
      {/if}

      {#if tab === "collezioni"}
        <section class="panel-section">
          <h3>Collezioni personali</h3>
          <p class="hint">
            I pittogrammi importati restano nell'archivio dell'app, sono ricercabili per lemma e
            sinonimi e vengono usati dalla simbolizzazione automatica.
          </p>
          <div class="collection-actions">
            <button class="btn btn--solid" onclick={onImportImages}>Importa immagini</button>
            <button class="btn" onclick={onImportFolder}>Importa cartella</button>
          </div>

          {#if deleteCandidate}
            <div class="collection-warning" role="alert">
              <p>
                Eliminare «{deleteCandidate.name}» e i suoi {deleteCandidate.symbols} pittogrammi?
                L'operazione non può essere annullata.
              </p>
              <div class="collection-actions">
                <button class="btn btn--solid" onclick={() => void removeCollection(deleteCandidate!)}>
                  Elimina definitivamente
                </button>
                <button class="btn btn--ghost" onclick={() => (deleteCandidate = null)}>Annulla</button>
              </div>
            </div>
          {/if}

          {#if collections.length === 0}
            <p class="hint">Nessuna collezione. Importa una cartella per iniziare.</p>
          {/if}

          {#each collections as collection (collection.id)}
            <div class="collection-row">
              {#if renamingId === collection.id}
                <input bind:value={renameValue} aria-label="Nuovo nome della collezione" />
                <button class="btn btn--solid" onclick={() => void confirmRename()}>Salva</button>
                <button class="btn btn--ghost" onclick={() => (renamingId = null)}>Annulla</button>
              {:else}
                <button
                  class="collection-open"
                  onclick={() => openCollection(collection)}
                  title="Cerca i pittogrammi di questa collezione"
                >{collection.name}</button>
                <span class="collection-count">{collection.symbols}</span>
                <button class="btn btn--ghost" onclick={() => beginRename(collection)}>Rinomina</button>
                <button class="btn btn--ghost" onclick={() => (deleteCandidate = collection)}>Elimina</button>
              {/if}
            </div>
          {/each}
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
            <label for="symbol-layout-select">Disposizione pittogrammi</label>
            <select
              id="symbol-layout-select"
              value={settings.symbol_layout}
              onchange={(event) =>
                onUpdateSettings({ symbol_layout: (event.currentTarget as HTMLSelectElement).value as never })}
            >
              <option value="riga">Striscia sopra il paragrafo</option>
              <option value="parola">Sopra ogni parola</option>
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
          <div class="settings-row">
            <label for="explicit-symbols-toggle">Mostra contenuti espliciti (adulti)</label>
            <input
              id="explicit-symbols-toggle"
              type="checkbox"
              checked={settings.show_explicit_symbols}
              onchange={(event) =>
                onUpdateSettings({ show_explicit_symbols: (event.currentTarget as HTMLInputElement).checked })}
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
            <label for="smartboard-toggle">Controlli smartboard e tastiera</label>
            <input
              id="smartboard-toggle"
              type="checkbox"
              checked={settings.smartboard_controls}
              onchange={(event) =>
                onUpdateSettings({ smartboard_controls: (event.currentTarget as HTMLInputElement).checked })}
            />
          </div>
          <div class="settings-row">
            <label for="online-search-select">Ricerca online</label>
            <select
              id="online-search-select"
              value={settings.online_search}
              onchange={(event) =>
                onUpdateSettings({ online_search: (event.currentTarget as HTMLSelectElement).value as never })}
            >
              <option value="off">Disattivata</option>
              <option value="suggest">Suggerisci automaticamente nel pannello</option>
              <option value="auto">Cerca automaticamente l'unità senza simbolo</option>
            </select>
          </div>
          <div class="settings-row">
            <label for="online-auto-select">Inserisci ARASAAC mentre scrivi</label>
            <input
              id="online-auto-select"
              type="checkbox"
              disabled={settings.online_search === "off"}
              checked={settings.online_auto_insert}
              onchange={(event) =>
                onUpdateSettings({ online_auto_insert: (event.currentTarget as HTMLInputElement).checked })}
            />
          </div>
          <div class="settings-row">
            <label for="animations-select">Animazioni</label>
            <select
              id="animations-select"
              value={settings.animations}
              onchange={(event) =>
                onUpdateSettings({ animations: (event.currentTarget as HTMLSelectElement).value as never })}
            >
              <option value="full">Complete</option>
              <option value="reduced">Ridotte</option>
              <option value="off">Disattivate</option>
            </select>
          </div>
          <div class="settings-row">
            <label for="typing-sound">Suoni di interfaccia</label>
            <input
              id="typing-sound"
              type="checkbox"
              checked={settings.typing_sound}
              onchange={(event) => onUpdateSettings({ typing_sound: (event.currentTarget as HTMLInputElement).checked })}
            />
          </div>
          <div class="settings-row">
            <label for="speak-words">Pronuncia la parola (spazio/Invio)</label>
            <input
              id="speak-words"
              type="checkbox"
              checked={settings.speak_words}
              onchange={(event) => onUpdateSettings({ speak_words: (event.currentTarget as HTMLInputElement).checked })}
            />
          </div>
          <div class="settings-row">
            <label for="speak-sentences">Leggi la frase completata</label>
            <input
              id="speak-sentences"
              type="checkbox"
              checked={settings.speak_sentences}
              onchange={(event) => onUpdateSettings({ speak_sentences: (event.currentTarget as HTMLInputElement).checked })}
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
          <h3>Immagini neurali</h3>
          <p class="hint">
            I modelli di diffusione, il runtime e il provider online si gestiscono nella scheda
            <strong>IA</strong>.
          </p>
          <button class="btn" onclick={() => selectTab("ia")}>Apri la scheda IA</button>
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
          <h3>Biblioteca modelli <span class="hint">({filteredTemplates.length}/{templates.length})</span></h3>
          {#if templates.length === 0}
            <p class="hint">Nessun modello caricato. Verifica la cartella resources/templates.</p>
          {:else}
            <div class="template-filters">
              <input type="search" bind:value={libQuery} placeholder="Cerca un modello…" aria-label="Cerca un modello" />
              <select bind:value={libCategory} aria-label="Categoria">
                <option value="">Tutte le categorie</option>
                {#each templateCategories as category (category)}
                  <option value={category}>{category}</option>
                {/each}
              </select>
              <select bind:value={libKind} aria-label="Tipologia">
                <option value="">Tutte le tipologie</option>
                <option value="grid">Tabella</option>
                <option value="schedule">Agenda</option>
                <option value="sequence">Sequenza</option>
                <option value="story">Storia</option>
                <option value="tokens">Gettoni</option>
                <option value="worksheet">Scheda</option>
              </select>
              <select bind:value={libComplexity} aria-label="Complessità">
                <option value="">Qualsiasi complessità</option>
                <option value="semplice">Semplice</option>
                <option value="intermedia">Intermedia</option>
                <option value="articolata">Articolata</option>
              </select>
              <select bind:value={libMinCells} aria-label="Numero minimo di celle">
                <option value={0}>Qualsiasi dimensione</option>
                <option value={4}>Almeno 4 celle</option>
                <option value={6}>Almeno 6 celle</option>
                <option value={9}>Almeno 9 celle</option>
                <option value={12}>Almeno 12 celle</option>
                <option value={16}>Almeno 16 celle</option>
                <option value={20}>Almeno 20 celle</option>
                <option value={24}>Almeno 24 celle</option>
                <option value={36}>Almeno 36 celle</option>
              </select>
              <select bind:value={libUsage} aria-label="Uso">
                <option value="">Stampa e interattivo</option>
                <option value="printable">Solo stampabile</option>
                <option value="interactive">Interattivo</option>
              </select>
            </div>
            <div class="template-list">
              {#each filteredTemplates as template (template.id)}
                <div class="template-card">
                  <div class="template-card__head">
                    <strong>{template.name}</strong>
                    <span class="hint">{template.category}</span>
                  </div>
                  <p class="hint">{template.description}</p>
                  <div class="template-card__meta">
                    <span>{template.items.length} elementi</span>
                    {#if template.grid}<span>{template.grid.columns}×{template.grid.rows}</span>{/if}
                    {#if template.complexity}<span>{template.complexity}</span>{/if}
                    {#if template.interactive}<span>interattivo</span>{/if}
                    {#if template.printable !== false}<span>stampabile</span>{/if}
                  </div>
                  <div style="display:flex;flex-wrap:wrap;gap:6px">
                    <button
                      class="btn btn--solid"
                      onclick={() => onOpenUniversalTemplate(template.id)}
                    >Apri documento</button>
                    <button
                      class="btn btn--ghost"
                      disabled={studioBusy}
                      onclick={() => void openTemplate(template)}
                    >Genera anteprima</button>
                  </div>
                </div>
              {/each}
            </div>
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
                  {#if item.image}
                    <img src={item.image} alt="" />
                  {:else if item.svg}
                    {@html item.svg}
                  {:else}
                    <span class="caa-chip__empty" style="width:44px;height:44px">?</span>
                  {/if}
                  <input
                    value={item.label}
                    aria-label={`Etichetta ${item.label}`}
                    oninput={(event) => (item.label = (event.currentTarget as HTMLInputElement).value)}
                  />
                  {#if item.approximate}<span class="badge badge--warn">≈</span>{/if}
                </div>
              {/each}
            </div>
            {#each plan.notes as note}
              <p class="hint">{note}</p>
            {/each}
            <div style="display:flex;flex-wrap:wrap;gap:6px;margin-top:8px">
              <button class="btn btn--solid" onclick={() => onOpenInEditor(plan!)}>Apri nell'editor</button>
              <button class="btn btn--ghost" onclick={onPrintResource}>Stampa / PDF</button>
            </div>
            <p class="hint">
              Aprendo il modello nell'editor puoi modificare testi, formattazione e pittogrammi
              liberamente; il modello originale non viene sovrascritto.
            </p>
          </section>
        {/if}
      {/if}
    </div>
  {/if}
</aside>
