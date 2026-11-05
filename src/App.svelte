<script lang="ts">
  import { onMount } from "svelte";
  import { get } from "svelte/store";
  import { language } from "./lib/i18n";
  import { open as openDialog, save as saveDialog } from "@tauri-apps/plugin-dialog";
  import CaaEditor from "./lib/editor/CaaEditor.svelte";
  import SidePanel from "./lib/components/SidePanel.svelte";
  import SettingsDialog from "./lib/components/SettingsDialog.svelte";
  import LayoutPreview from "./lib/components/LayoutPreview.svelte";
  import Communicator from "./lib/components/Communicator.svelte";
  import HomeScreen from "./lib/components/HomeScreen.svelte";
  import FreeBoard from "./lib/components/FreeBoard.svelte";
  import { recordRecent } from "./lib/recent";
  import { api, isTauri } from "./lib/api";
  import { rasterizeSymbols } from "./lib/raster";
  import {
    defaultSettings,
    type AiConfig,
    type AiStatus,
    type Alternative,
    type AppInfo,
    type CaaMode,
    type CaaToken,
    type CreationMode,
    type DocumentSettings,
    type ExportDoc,
    type LayoutDocument,
    type LayoutSpecification,
    type LibraryStats,
    type PackInfo,
    type Preference,
    type ResourcePlan,
    type TemplateInfo,
    type UnitModifiers,
    type VoiceInfo,
    noModifiers,
  } from "./lib/types";

  type Toast = { id: number; text: string; kind: "info" | "warn" | "error" };

  let settings = $state<DocumentSettings>({ ...defaultSettings });
  let docTitle = $state("Documento senza titolo");
  let docPath = $state<string | null>(null);
  let docKey = $state(`doc-${Date.now().toString(36)}`);
  let editor = $state<CaaEditor | undefined>(undefined);

  let selected = $state<{ pid: string; token: CaaToken } | null>(null);
  let tokensByParagraph = $state<Map<string, CaaToken[]>>(new Map());

  let appInfo = $state<AppInfo | null>(null);
  let stats = $state<LibraryStats | null>(null);
  let packs = $state<PackInfo[]>([]);
  let voices = $state<VoiceInfo[]>([]);
  let aiStatus = $state<AiStatus | null>(null);
  let preferences = $state<Preference[]>([]);
  let templates = $state<TemplateInfo[]>([]);

  let toasts = $state<Toast[]>([]);
  let toastSeq = 0;
  let dirty = $state(false);
  let autoSavedAt = $state<string | null>(null);
  let recovery = $state<{ plain_text: string; content: unknown; title: string } | null>(null);
  let recovered = $state(false);
  let resourcePlan = $state<ResourcePlan | null>(null);
  let resourceLayout = $state<LayoutDocument | null>(null);
  let showLayoutPreview = $state(false);
  let communicatorOpen = $state(false);
  let layoutPrompt = $state("");
  let layoutBusy = $state(false);
  let layoutHistory = $state<LayoutSpecification[]>([]);
  let layoutRedo = $state<LayoutSpecification[]>([]);
  let layoutApplied = $state<string[]>([]);
  let printingResource = $state(false);
  let aiConfig = $state<AiConfig | null>(null);

  type SideTab = "alternative" | "cerca" | "impostazioni" | "studio";
  type SideIntent = { tab: SideTab; prompt?: string; templateId?: string | null; nonce: number };

  let view = $state<"home" | "editor" | "board">("home");
  // Le impostazioni sono una finestra modale autonoma: non dipendono più
  // dall'apertura del pannello laterale, che restava chiuso o vuoto.
  let settingsOpen = $state(false);
  // Larghezza del pannello laterale scelta dall'utente (280-680 px), ricordata
  // tra le sessioni: il foglio si adatta allo spazio rimasto.
  let panelWidth = $state(
    typeof localStorage !== "undefined"
      ? Math.min(680, Math.max(280, Number(localStorage.getItem("ui.panelWidth")) || 372))
      : 372
  );
  let sideIntent = $state<SideIntent | null>(null);
  let intentSeq = 0;

  // La modalità CAA automatica è attiva per impostazione predefinita: con
  // l'interruttore spento le associazioni restano memorizzate ma non si vedono.
  const effectiveMode = $derived<CaaMode>(settings.auto_caa ? settings.mode : "testo");
  const neuralConfigured = $derived(
    Boolean(aiStatus?.enabled && aiStatus.binary_present && aiStatus.model_present)
  );
  const neuralReady = $derived(Boolean(settings.neural_context && neuralConfigured));

  const STUDIO_PRESETS: Record<
    "grid" | "schedule" | "aba" | "teacch" | "smart",
    { title: string; prompt: string; templateId: string | null }
  > = {
    grid: {
      title: "Tabella di comunicazione",
      prompt: "Crea una tabella di comunicazione con 12 simboli per le attività della giornata: ",
      templateId: "communication_grid",
    },
    schedule: {
      title: "Agenda visiva",
      prompt:
        "Crea un'agenda visiva settimanale per la scuola primaria con: entrata a scuola, intervallo, pranzo, gioco",
      templateId: "visual_schedule",
    },
    aba: {
      title: "Materiale ABA",
      prompt: "Crea una token board con 5 gettoni per: ",
      templateId: "token_board",
    },
    teacch: {
      title: "Materiale TEACCH",
      prompt: "Crea una sequenza prima/dopo con: ",
      templateId: "first_then",
    },
    smart: {
      title: "Generazione intelligente",
      prompt:
        "Crea una tabella di comunicazione con 20 simboli per un bambino che deve scegliere le attività durante l'intervallo.",
      templateId: null,
    },
  };

  const native = isTauri();
  const unitKey = (pid: string, index: number) => `${pid}:${index}`;

  function toast(text: string, kind: Toast["kind"] = "info") {
    const id = ++toastSeq;
    toasts = [...toasts, { id, text, kind }];
    setTimeout(() => {
      toasts = toasts.filter((item) => item.id !== id);
    }, kind === "error" ? 9000 : 5000);
  }

  function patchSettings(patch: Partial<DocumentSettings>) {
    settings = { ...settings, ...patch };
    void persistSettings();
  }

  async function persistSettings() {
    if (!native) return;
    try {
      await api.setSetting("ui.settings", JSON.stringify(settings));
    } catch {
      /* la persistenza delle preferenze non deve bloccare la scrittura */
    }
  }

  $effect(() => {
    const root = document.documentElement;
    root.dataset.theme = settings.theme;
    root.dataset.touch = String(settings.touch_mode);
    root.dataset.mono = String(settings.mono);
    root.style.setProperty("--text-size", `${settings.font_size}px`);
    root.style.setProperty("--symbol-size", settings.touch_mode ? "76px" : "56px");
  });

  onMount(() => {
    if (!native) return;
    void (async () => {
      try {
        appInfo = await api.appInfo();
        stats = await api.libraryStats();
        packs = await api.listPacks();
        voices = await api.listVoices();
        aiStatus = await api.aiStatus();
        preferences = await api.listPreferences();
        templates = await api.listTemplates();
        aiConfig = {
          enabled: aiStatus.enabled,
          binary_path: aiStatus.binary_path,
          model_path: aiStatus.model_path,
          threads: aiStatus.threads,
          context_size: aiStatus.context_size,
          timeout_ms: aiStatus.timeout_ms,
          backend: aiStatus.backend,
          extra_args: [],
        };

        const stored = await api.getSetting("ui.settings");
        if (stored) {
          try {
            settings = { ...defaultSettings, ...(JSON.parse(stored) as Partial<DocumentSettings>) };
          } catch {
            settings = { ...defaultSettings };
          }
        }
        await checkRecovery();
      } catch (error) {
        toast(error instanceof Error ? error.message : String(error), "error");
      }
    })();

    const onKey = (event: KeyboardEvent) => {
      const meta = event.metaKey || event.ctrlKey;
      if (!meta) return;
      const key = event.key.toLowerCase();
      if (key === "s") {
        event.preventDefault();
        void saveDocument(false);
      } else if (key === "o") {
        event.preventDefault();
        void openDocument();
      } else if (key === "n") {
        event.preventDefault();
        newDocument();
      } else if (key === "p") {
        event.preventDefault();
        printDocument();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  // ---------- Documento ----------

  function newDocument(silent = false) {
    docTitle = "Documento senza titolo";
    docPath = null;
    docKey = `doc-${Date.now().toString(36)}`;
    selected = null;
    tokensByParagraph = new Map();
    editor?.insertPlainText("");
    dirty = false;
    view = "editor";
    if (!silent) toast("Nuovo documento creato.", "info");
  }

  // ---------- Schermata iniziale ----------

  function create(mode: CreationMode) {
    if (mode === "free") {
      view = "board";
      return;
    }
    if (mode === "word") {
      newDocument(true);
      return;
    }
    const preset = STUDIO_PRESETS[mode];
    newDocument(true);
    docTitle = preset.title;
    sideIntent = {
      tab: "studio",
      prompt: preset.prompt,
      templateId: preset.templateId,
      nonce: ++intentSeq,
    };
    view = "editor";
  }

  function openLibrary() {
    view = "editor";
    sideIntent = { tab: "cerca", nonce: ++intentSeq };
  }

  function openSettingsPanel() {
    settingsOpen = true;
  }

  function clampPanel(width: number): number {
    return Math.min(680, Math.max(280, width));
  }

  function rememberPanelWidth() {
    try {
      localStorage.setItem("ui.panelWidth", String(panelWidth));
    } catch {
      // Archiviazione non disponibile: la larghezza vale solo per questa sessione.
    }
  }

  function resizePanelBy(delta: number) {
    panelWidth = clampPanel(panelWidth + delta);
    rememberPanelWidth();
  }

  /// Trascinamento del bordo del pannello: nessuna operazione sul documento e
  /// nessuna perdita del cursore, perché non si tocca mai la selezione.
  function startPanelResize(event: PointerEvent) {
    event.preventDefault();
    const startX = event.clientX;
    const startWidth = panelWidth;
    const move = (moveEvent: PointerEvent) => {
      panelWidth = clampPanel(startWidth - (moveEvent.clientX - startX));
    };
    const stop = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", stop);
      rememberPanelWidth();
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", stop);
  }

  function openTemplate(templateId: string) {
    const template = templates.find((item) => item.id === templateId);
    newDocument(true);
    docTitle = template?.name ?? "Nuovo materiale";
    sideIntent = {
      tab: "studio",
      prompt: template?.description ?? "",
      templateId,
      nonce: ++intentSeq,
    };
    view = "editor";
  }

  function buildPayload() {
    const assets: { symbol_id: string; svg: string | null; png_base64: string | null }[] = [];
    const seen = new Set<string>();
    for (const tokens of tokensByParagraph.values()) {
      for (const token of tokens) {
        if (token.symbol_id && token.svg && !seen.has(token.symbol_id)) {
          seen.add(token.symbol_id);
          assets.push({ symbol_id: token.symbol_id, svg: token.svg, png_base64: null });
        }
      }
    }
    return {
      title: docTitle,
      content: editor?.getDocJson() ?? null,
      plain_text: editor?.getPlainText() ?? "",
      settings,
      assets,
      // Blocco, visibilità e modificatori viaggiano con il documento: alla
      // riapertura ogni unità ritrova il proprio stato.
      preferences: [...tokensByParagraph.entries()].flatMap(([pid, tokens]) =>
        tokens
          .filter((token) => token.locked || token.hidden || (token.modifiers?.manual ?? false))
          .map((token) => ({
            unit_key: unitKey(pid, token.index),
            symbol_id: token.symbol_id,
            concept_id: token.concept_id,
            locked: token.locked,
            hidden: token.hidden,
            modifiers: token.modifiers ?? { ...noModifiers },
          }))
      ),
      layout: resourceLayout ? resourceLayout.spec : null,
    };
  }

  async function saveDocument(askPath: boolean) {
    if (!native) {
      toast("Il salvataggio richiede l'app desktop.", "warn");
      return;
    }
    if (!editor) return;
    let path = docPath;
    if (!path || askPath) {
      const picked = await saveDialog({
        title: "Salva documento CAA",
        defaultPath: `${docTitle || "documento"}.caadoc`,
        filters: [{ name: "Documento CAA Writer", extensions: ["caadoc"] }],
      });
      if (!picked) return;
      path = picked as string;
    }
    try {
      const result = await api.saveDocument(path, buildPayload());
      docPath = path;
      docTitle = path.split(/[\\/]/).pop()?.replace(/\.caadoc$/i, "") ?? docTitle;
      dirty = false;
      recordRecent({
        path,
        title: docTitle,
        excerpt: editor?.getPlainText() ?? "",
      });
      if (result.warnings.length) result.warnings.forEach((warning) => toast(warning, "warn"));
      toast(`Documento salvato in ${path}`, "info");
    } catch (error) {
      toast(error instanceof Error ? error.message : String(error), "error");
    }
  }

  async function openDocument() {
    if (!native) {
      toast("L'apertura dei documenti richiede l'app desktop.", "warn");
      return;
    }
    const picked = await openDialog({
      title: "Apri documento CAA",
      multiple: false,
      filters: [{ name: "Documento CAA Writer", extensions: ["caadoc"] }],
    });
    if (!picked) return;
    await loadFrom(String(picked));
  }

  async function loadFrom(path: string) {
    try {
      const loaded = await api.loadDocument(path);
      docTitle = loaded.title || path.split(/[\\/]/).pop() || "Documento";
      docPath = path;
      docKey = `doc-${path}`;
      if (loaded.settings && typeof loaded.settings === "object") {
        settings = { ...defaultSettings, ...(loaded.settings as Partial<DocumentSettings>) };
      }
      editor?.setDocJson(loaded.content, loaded.plain_text);
      dirty = false;
      view = "editor";
      selected = null;
      resourceLayout = null;
      layoutHistory = [];
      layoutRedo = [];
      layoutApplied = [];
      layoutPrompt = "";
      if (loaded.layout) {
        try {
          resourceLayout = await api.buildLayout(loaded.layout);
        } catch (error) {
          toast(
            `Layout salvato non ricostruito: ${error instanceof Error ? error.message : String(error)}`,
            "warn"
          );
        }
      }
      recordRecent({ path, title: docTitle, excerpt: loaded.plain_text });
      loaded.warnings.forEach((warning) => toast(warning, "warn"));
      toast(`Aperto: ${docTitle}`, "info");
    } catch (error) {
      toast(error instanceof Error ? error.message : String(error), "error");
    }
  }

  async function importDocument() {
    if (!native) {
      toast("L'importazione richiede l'app desktop.", "warn");
      return;
    }
    const picked = await openDialog({
      title: "Importa testo",
      multiple: false,
      filters: [
        { name: "Documenti supportati", extensions: ["txt", "docx", "html", "htm", "md"] },
      ],
    });
    if (!picked) return;
    try {
      const text = await api.importDocumentText(String(picked));
      docTitle = String(picked).split(/[\\/]/).pop() ?? "Importato";
      docPath = null;
      docKey = `doc-${Date.now().toString(36)}`;
      editor?.insertPlainText(text);
      toast("Testo importato: la formattazione originale non è conservata integralmente.", "warn");
    } catch (error) {
      toast(error instanceof Error ? error.message : String(error), "error");
    }
  }

  function onAnalyzed(map: Map<string, CaaToken[]>) {
    tokensByParagraph = map;
    dirty = true;
  }

  function collectTokens(): CaaToken[] {
    const out: CaaToken[] = [];
    for (const tokens of tokensByParagraph.values()) out.push(...tokens);
    return out;
  }

  function buildExportDoc(): ExportDoc {
    const paragraphs = (editor?.getParagraphs() ?? []).map((paragraph) => {
      const tokens = tokensByParagraph.get(paragraph.pid) ?? [];
      return {
        plain_text: paragraph.text,
        style: paragraph.style,
        units: tokens
          // I pittogrammi nascosti non entrano nelle esportazioni visive.
          .filter((token) => !token.is_punct && !token.hidden)
          .map((token) => ({
            surface: token.surface,
            // Didascalia visibile = testo scritto dall'utente; l'etichetta del
            // repertorio resta nei metadati della licenza.
            caption: token.surface,
            symbol_id: token.symbol_id,
            svg: token.svg,
            png_base64: null,
            approximate: token.approximate,
          })),
      };
    });
    return {
      title: docTitle,
      paragraphs,
      include_symbols: settings.mode !== "testo",
      include_captions: settings.show_captions,
      mode: settings.mode,
      attributions: [],
      metadata: {
        app: "CAA Writer",
        mode: settings.mode,
        intensity: settings.intensity,
        generated_at: new Date().toISOString(),
      },
    };
  }

  async function exportHtml() {
    if (!native) {
      toast("L'esportazione richiede l'app desktop.", "warn");
      return;
    }
    const path = await saveDialog({
      title: "Esporta in HTML",
      defaultPath: `${docTitle || "documento"}.html`,
      filters: [{ name: "Pagina HTML", extensions: ["html"] }],
    });
    if (!path) return;
    try {
      await api.exportHtml(String(path), buildExportDoc());
      toast("Esportazione HTML completata.", "info");
    } catch (error) {
      toast(error instanceof Error ? error.message : String(error), "error");
    }
  }

  async function exportDocx() {
    if (!native) {
      toast("L'esportazione richiede l'app desktop.", "warn");
      return;
    }
    const path = await saveDialog({
      title: "Esporta in DOCX",
      defaultPath: `${docTitle || "documento"}.docx`,
      filters: [{ name: "Documento Word", extensions: ["docx"] }],
    });
    if (!path) return;
    try {
      const doc = buildExportDoc();
      const svgs = new Map<string, string>();
      for (const token of collectTokens()) {
        if (token.symbol_id && token.svg) svgs.set(token.symbol_id, token.svg);
      }
      const { pngs, failed } = await rasterizeSymbols(svgs);
      doc.paragraphs.forEach((paragraph) =>
        paragraph.units.forEach((unit) => {
          if (unit.symbol_id) unit.png_base64 = pngs.get(unit.symbol_id) ?? null;
        })
      );
      const result = await api.exportDocx(String(path), doc);
      if (failed.length) {
        toast(
          `${failed.length} pittogrammi non rasterizzabili in PNG: nel DOCX compaiono solo come testo.`,
          "warn"
        );
      }
      result.warnings.forEach((warning) => toast(warning, "warn"));
      toast("Esportazione DOCX completata.", "info");
    } catch (error) {
      toast(error instanceof Error ? error.message : String(error), "error");
    }
  }

  async function exportAttributions() {
    if (!native) return;
    const path = await saveDialog({
      title: "Esporta registro delle licenze",
      defaultPath: `${docTitle || "documento"}-licenze.html`,
      filters: [{ name: "Pagina HTML", extensions: ["html"] }],
    });
    if (!path) return;
    try {
      await api.exportAttributions(String(path));
      toast("Registro delle attribuzioni esportato.", "info");
    } catch (error) {
      toast(error instanceof Error ? error.message : String(error), "error");
    }
  }

  function printDocument() {
    printingResource = false;
    document.body.dataset.print = "document";
    window.print();
  }

  function printResource() {
    if (!resourcePlan && !resourceLayout) return;
    showLayoutPreview = false;
    printingResource = true;
    document.body.dataset.print = "resource";
    window.print();
  }

  onMount(() => {
    const afterPrint = () => {
      delete document.body.dataset.print;
      printingResource = false;
    };
    window.addEventListener("afterprint", afterPrint);
    return () => window.removeEventListener("afterprint", afterPrint);
  });

  async function speak(text: string) {
    if (!text.trim()) {
      toast("Niente da leggere.", "warn");
      return;
    }
    if (!native) {
      toast("La sintesi vocale richiede l'app desktop.", "warn");
      return;
    }
    try {
      await api.speak(text, null, 1.0, get(language));
    } catch (error) {
      toast(error instanceof Error ? error.message : String(error), "error");
    }
  }

  function speakParagraph() {
    const pid = selected?.pid;
    if (pid) {
      const tokens = tokensByParagraph.get(pid) ?? [];
      const text = tokens.map((token) => token.surface).join(" ").trim();
      void speak(text);
      return;
    }
    void speak(editor?.getPlainText() ?? "");
  }

  // ---------- Scelte manuali ----------

  async function applyAlternative(alternative: Alternative) {
    if (!selected) {
      toast("Seleziona prima un pittogramma nel documento, poi scegli il simbolo.", "warn");
      return;
    }
    if (!native) {
      toast("La sostituzione richiede l'app desktop.", "warn");
      return;
    }
    const { pid, token } = selected;
    // La selezione può essere invecchiata dopo una rianalisi o una modifica
    // del testo: applicare il simbolo all'indice sbagliato sarebbe peggio che
    // chiedere di selezionare di nuovo.
    const current = editor?.tokenAt(pid, token.index);
    if (!current || current.surface !== token.surface) {
      selected = null;
      toast("La parola selezionata è cambiata: selezionala di nuovo.", "warn");
      return;
    }
    try {
      await api.setDocumentOverride(
        docKey,
        unitKey(pid, token.index),
        alternative.symbol_id,
        alternative.concept_id
      );
      // Aggiornamento immediato dello stato dell'editor: il foglio cambia nello
      // stesso istante della scelta, senza dipendere da una rianalisi.
      editor?.applyTokenSymbol(pid, token.index, alternative);
      // Poi il paragrafo viene riletto dal motore: la scelta è salvata come
      // override bloccato, quindi la rianalisi la rispetta e aggiorna il resto.
      editor?.refreshParagraph(pid);
      toast(`Pittogramma impostato per «${token.surface}».`, "info");
    } catch (error) {
      toast(error instanceof Error ? error.message : String(error), "error");
    }
  }

  // ---------- Stato dell'unità: blocco, visibilità, modificatori ----------

  let unitMenu = $state<{ x: number; y: number } | null>(null);

  const modifierOptions = [
    { kind: "plural" as const, label: "Plurale" },
    { kind: "negation" as const, label: "Negazione" },
    { kind: "past" as const, label: "Passato" },
    { kind: "future" as const, label: "Futuro" },
  ];

  async function setUnitState(patch: {
    locked?: boolean;
    hidden?: boolean;
    modifiers?: UnitModifiers;
  }) {
    if (!selected) {
      toast("Seleziona prima un pittogramma nel documento.", "warn");
      return;
    }
    if (!native) {
      toast("La modifica del pittogramma richiede l'app desktop.", "warn");
      return;
    }
    const { pid, token } = selected;
    const locked = patch.locked ?? token.locked;
    const hidden = patch.hidden ?? token.hidden;
    const modifiers = patch.modifiers ?? token.modifiers;
    try {
      await api.setUnitState(docKey, unitKey(pid, token.index), locked, hidden, modifiers);
      editor?.applyTokenState(pid, token.index, { locked, hidden, modifiers });
      unitMenu = null;
    } catch (error) {
      toast(error instanceof Error ? error.message : String(error), "error");
    }
  }

  function toggleModifier(kind: "plural" | "negation" | "past" | "future") {
    if (!selected) return;
    const current = selected.token.modifiers ?? noModifiers;
    // Qualsiasi modifica manuale fissa la configurazione: il ricalcolo
    // automatico non la sovrascrive più.
    void setUnitState({
      modifiers: { ...current, [kind]: !current[kind], manual: true },
    });
  }

  function resetModifiers() {
    void setUnitState({ modifiers: { ...noModifiers } });
  }

  async function savePreference(alternative: Alternative, context: string | null) {
    if (!selected) {
      toast("Seleziona prima un'unità nel documento.", "warn");
      return;
    }
    if (!native) return;
    const lemma = selected.token.lemma ?? selected.token.surface.toLowerCase();
    try {
      await api.setPreference({
        scope: context ? "context" : "global",
        lemma,
        context_key: context,
        symbol_id: alternative.symbol_id,
        concept_id: alternative.concept_id,
        locked: false,
        created_at: "",
      });
      preferences = await api.listPreferences();
      editor?.refreshAll();
      toast(
        context
          ? `Preferenza salvata per «${lemma}» nel contesto ${context}.`
          : `Preferenza globale salvata per «${lemma}».`,
        "info"
      );
    } catch (error) {
      toast(error instanceof Error ? error.message : String(error), "error");
    }
  }

  async function resetUnit() {
    if (!selected || !native) return;
    try {
      await api.clearDocumentOverride(docKey, unitKey(selected.pid, selected.token.index));
      editor?.refreshParagraph(selected.pid);
      toast("Ripristinata la scelta automatica per questa unità.", "info");
    } catch (error) {
      toast(error instanceof Error ? error.message : String(error), "error");
    }
  }

  async function importSvg(file: File) {
    if (!selected) {
      toast("Seleziona l'unità a cui associare il pittogramma.", "warn");
      return;
    }
    const svg = await file.text();
    try {
      const conceptId = selected.token.concept_id;
      const symbolId = await api.importSvgSymbol(
        file.name.replace(/\.svg$/i, ""),
        svg,
        selected.token.lemma ?? selected.token.surface.toLowerCase(),
        selected.token.meaning,
        conceptId
      );
      await api.setDocumentOverride(docKey, unitKey(selected.pid, selected.token.index), symbolId, conceptId);
      preferences = await api.listPreferences();
      editor?.refreshParagraph(selected.pid);
      toast("Pittogramma importato e applicato.", "info");
    } catch (error) {
      toast(error instanceof Error ? error.message : String(error), "error");
    }
  }

  async function setAiConfig(config: AiConfig) {
    if (!native) return;
    try {
      aiStatus = await api.setAiConfig(config);
      aiConfig = config;
      toast("Configurazione IA aggiornata.", "info");
    } catch (error) {
      toast(error instanceof Error ? error.message : String(error), "error");
    }
  }

  /// Ricarica lo stato del motore neurale dopo un cambio di modello o un
  /// download: lo stato mostrato deve essere quello reale, non una copia.
  async function refreshAiStatus() {
    if (!native) return;
    try {
      aiStatus = await api.aiStatus();
    } catch (error) {
      toast(error instanceof Error ? error.message : String(error), "error");
    }
  }

  // Mappa il piano (contenuti + griglia) in una specifica di layout. Le misure
  // restano al motore Rust: qui si scelgono solo struttura e pagine.
  function layoutSpecFromPlan(plan: ResourcePlan): LayoutSpecification {
    const columns = Math.max(1, Math.min(12, Math.round(plan.grid.columns) || 3));
    const rows = Math.max(1, Math.min(40, Math.round(plan.grid.rows) || 4));
    const capacity = columns * rows;
    return {
      schema_version: 1,
      document_type: plan.kind || "communication_grid",
      title: plan.title,
      page: {
        size: "A4",
        orientation: columns > 4 ? "landscape" : "portrait",
        margin_mm: 12,
      },
      layout: { kind: "grid", columns, rows, gap_mm: 3 },
      elements: plan.items.map((item, index) => ({
        id: `unit-${index + 1}`,
        kind: item.svg ? "symbol" : "text",
        label: item.label,
        concept_id: item.concept_id,
        symbol_id: item.symbol_id,
        svg: item.svg,
        page: Math.floor(index / capacity),
        row: Math.floor((index % capacity) / columns),
        column: index % columns,
        row_span: 1,
        column_span: 1,
        hidden: false,
        locked: false,
      })),
      accessibility: {
        large_symbols: true,
        high_contrast: settings.theme === "contrasto",
      },
    };
  }

  async function buildResourceLayout(plan: ResourcePlan) {
    if (!native) return;
    try {
      const built = await api.buildLayout(layoutSpecFromPlan(plan));
      resourceLayout = built;
      layoutHistory = [];
      layoutRedo = [];
      layoutApplied = [];
      layoutPrompt = "";
      built.notes.forEach((note) => toast(note, "warn"));
      showLayoutPreview = true;
    } catch (error) {
      resourceLayout = null;
      toast(
        `Impaginazione non riuscita: ${error instanceof Error ? error.message : String(error)}`,
        "error"
      );
    }
  }

  async function applyLayoutEdit() {
    if (!native || !resourceLayout || layoutBusy) return;
    const prompt = layoutPrompt.trim();
    if (!prompt) return;
    layoutBusy = true;
    try {
      const previous = resourceLayout.spec;
      const result = await api.editLayout(previous, prompt);
      layoutHistory = [...layoutHistory, previous].slice(-20);
      layoutRedo = [];
      resourceLayout = { spec: result.spec, pages: result.pages, notes: result.notes };
      layoutApplied = result.operations;
      layoutPrompt = "";
      result.notes.forEach((note) => toast(note, "warn"));
      if (result.operations.length > 0) {
        toast(`Layout aggiornato: ${result.operations.join(" · ")}`, "info");
      }
    } catch (error) {
      toast(
        `Modifica non applicata: ${error instanceof Error ? error.message : String(error)}`,
        "error"
      );
    } finally {
      layoutBusy = false;
    }
  }

  async function undoLayoutEdit() {
    if (!native || !resourceLayout || layoutHistory.length === 0) return;
    const previous = layoutHistory[layoutHistory.length - 1];
    try {
      const rebuilt = await api.buildLayout(previous);
      layoutRedo = [...layoutRedo, resourceLayout.spec].slice(-20);
      layoutHistory = layoutHistory.slice(0, -1);
      resourceLayout = rebuilt;
      layoutApplied = [];
    } catch (error) {
      toast(
        `Ripristino non riuscito: ${error instanceof Error ? error.message : String(error)}`,
        "error"
      );
    }
  }

  async function redoLayoutEdit() {
    if (!native || !resourceLayout || layoutRedo.length === 0) return;
    const next = layoutRedo[layoutRedo.length - 1];
    try {
      const rebuilt = await api.buildLayout(next);
      layoutHistory = [...layoutHistory, resourceLayout.spec].slice(-20);
      layoutRedo = layoutRedo.slice(0, -1);
      resourceLayout = rebuilt;
      layoutApplied = [];
    } catch (error) {
      toast(
        `Ripetizione non riuscita: ${error instanceof Error ? error.message : String(error)}`,
        "error"
      );
    }
  }

  async function generateResource(prompt: string, templateId: string | null): Promise<ResourcePlan | null> {
    if (!native) {
      toast("Il Visual Resource Studio richiede l'app desktop.", "warn");
      return null;
    }
    try {
      const plan = await api.planResource(prompt, templateId);
      resourcePlan = plan;
      plan.notes.forEach((note) => toast(note, "warn"));
      await buildResourceLayout(plan);
      return plan;
    } catch (error) {
      toast(error instanceof Error ? error.message : String(error), "error");
      return null;
    }
  }

  async function clearHistory() {
    if (!native) return;
    try {
      await api.clearHistory();
      toast("Cronologia delle ricerche cancellata.", "info");
    } catch (error) {
      toast(error instanceof Error ? error.message : String(error), "error");
    }
  }

  async function checkRecovery() {
    if (!appInfo) return;
    const path = `${appInfo.data_dir}/recovery.caadoc`;
    try {
      const loaded = await api.loadDocument(path);
      if (loaded.plain_text.trim().length > 0) {
        recovery = { plain_text: loaded.plain_text, content: loaded.content, title: loaded.title };
      }
    } catch {
      /* nessun file di recupero: condizione normale al primo avvio */
    }
  }

  function restoreRecovery() {
    if (!recovery) return;
    editor?.setDocJson(recovery.content, recovery.plain_text);
    docTitle = recovery.title || "Documento recuperato";
    docPath = null;
    docKey = `doc-recovery-${Date.now().toString(36)}`;
    recovered = true;
    recovery = null;
    toast("Documento recuperato. Salvalo per conservarlo.", "info");
  }

  // Autosalvataggio di recupero: solo se ci sono modifiche non salvate.
  $effect(() => {
    if (!native || !appInfo || !dirty || recovered) return;
    const timer = setTimeout(() => {
      void (async () => {
        try {
          await api.saveDocument(`${appInfo!.data_dir}/recovery.caadoc`, buildPayload());
          autoSavedAt = new Date().toLocaleTimeString();
        } catch {
          /* l'autosalvataggio non deve interrompere la scrittura */
        }
      })();
    }, 20000);
    return () => clearTimeout(timer);
  });
</script>

<svelte:head>
  <title>{view === "editor" ? `${docTitle} · CAA Writer` : "CAA Writer"}</title>
</svelte:head>{#if view === "home"}
  <HomeScreen
    {stats}
    {templates}
    {appInfo}
    onCreate={create}
    onOpenDocument={() => void openDocument()}
    onOpenRecent={(path) => void loadFrom(path)}
    onOpenLibrary={openLibrary}
    onOpenSettings={openSettingsPanel}
    onOpenTemplate={openTemplate}
  />
{:else if view === "board"}
  <FreeBoard
    packId={settings.pack_id}
    commercialOnly={settings.commercial_only}
    onToast={toast}
    onExit={() => (view = "home")}
  />
{:else}
<div class="app">
  <header class="topbar no-print">
    <div class="topbar__brand">
      <button class="btn btn--ghost" onclick={() => (view = "home")} title="Torna alla schermata iniziale" aria-label="Torna alla schermata iniziale">⌂</button>
      <span class="topbar__title">CAA Writer</span>
      <span class="topbar__doc" title={docPath ?? docTitle}>{docTitle}{dirty ? " •" : ""}</span>
    </div>

    <div class="topbar__group" role="group" aria-label="Documento">
      <button class="btn" onclick={() => newDocument()} title="Nuovo documento (Ctrl+N)">Nuovo</button>
      <button class="btn" onclick={() => void openDocument()} title="Apri (Ctrl+O)">Apri</button>
      <button class="btn btn--solid" onclick={() => void saveDocument(false)} title="Salva (Ctrl+S)">Salva</button>
      <button class="btn" onclick={() => void saveDocument(true)} title="Salva con nome">Salva con nome</button>
      <button class="btn" onclick={() => void importDocument()}>Importa</button>
    </div>

    <div class="topbar__group" role="group" aria-label="Formattazione">
      <button class="btn" onclick={() => editor?.undoAction()} aria-label="Annulla">↶</button>
      <button class="btn" onclick={() => editor?.redoAction()} aria-label="Ripristina">↷</button>
      <button class="btn" onclick={() => editor?.toggleBold()} aria-label="Grassetto"><strong>B</strong></button>
      <button class="btn" onclick={() => editor?.toggleItalic()} aria-label="Corsivo"><em>I</em></button>
      <button class="btn" onclick={() => editor?.toggleUnderline()} aria-label="Sottolineato"><u>U</u></button>
      <button class="btn" onclick={() => editor?.applyHeading(1)} aria-label="Titolo">H1</button>
      <button class="btn" onclick={() => editor?.applyHeading(null)} aria-label="Paragrafo normale">¶</button>
      <button class="btn" onclick={() => editor?.applyAlignment("left")} aria-label="Allinea a sinistra">⯇</button>
      <button class="btn" onclick={() => editor?.applyAlignment("center")} aria-label="Centra">≡</button>
      <button class="btn" onclick={() => editor?.applyAlignment(null)} aria-label="Allineamento predefinito">⟲</button>
      <button class="btn" onclick={() => editor?.toggleList("bullet")} aria-label="Elenco puntato">•</button>
      <button class="btn" onclick={() => editor?.toggleList("ordered")} aria-label="Elenco numerato">1.</button>
      <button class="btn" onclick={() => editor?.insertTable(2, 2)} aria-label="Inserisci tabella">▦</button>
      <input
        type="color"
        aria-label="Colore del testo"
        title="Colore del testo"
        style="width:34px;height:30px;padding:2px"
        onchange={(event) => editor?.applyTextStyle((event.currentTarget as HTMLInputElement).value, null)}
      />
      <select
        aria-label="Carattere del documento"
        title="Carattere del documento (salvato con il documento)"
        value={settings.font_family}
        onchange={(event) => patchSettings({ font_family: (event.currentTarget as HTMLSelectElement).value })}
      >
        <option value="">Carattere di sistema</option>
        <option value="Arial, Helvetica, sans-serif">Arial</option>
        <option value="Verdana, Geneva, sans-serif">Verdana</option>
        <option value="Calibri, Carlito, sans-serif">Calibri</option>
        <option value="OpenDyslexic, Atkinson Hyperlegible, sans-serif">OpenDyslexic</option>
      </select>
      <button
        class="btn"
        class:btn--solid={settings.uppercase}
        aria-pressed={settings.uppercase}
        title="Mostra il testo in maiuscolo: cambia solo la visualizzazione, non il significato né i pittogrammi"
        onclick={() => patchSettings({ uppercase: !settings.uppercase })}>AA</button
      >
      <select
        aria-label="Dimensione carattere"
        onchange={(event) => {
          const value = (event.currentTarget as HTMLSelectElement).value;
          editor?.applyTextStyle(null, value || null);
        }}
      >
        <option value="">Dimensione</option>
        <option value="12px">12</option>
        <option value="14px">14</option>
        <option value="16px">16</option>
        <option value="20px">20</option>
        <option value="28px">28</option>
      </select>
    </div>

    <div class="topbar__group" role="group" aria-label="Rappresentazione CAA">
      <button
        class="btn"
        class:btn--solid={settings.auto_caa}
        aria-pressed={settings.auto_caa}
        title={settings.auto_caa
          ? "CAA automatica attiva: i pittogrammi si aggiornano mentre scrivi"
          : "CAA automatica disattivata: le associazioni restano memorizzate e ricompaiono attivandola"}
        onclick={() => patchSettings({ auto_caa: !settings.auto_caa })}>
        CAA automatica: {settings.auto_caa ? "Attiva" : "Disattiva"}
      </button>
      <select
        aria-label="Disposizione dei pittogrammi"
        value={settings.mode}
        disabled={!settings.auto_caa}
        onchange={(event) => patchSettings({ mode: (event.currentTarget as HTMLSelectElement).value as never })}
      >
        <option value="testo">A · Testo</option>
        <option value="sopra">B · Simboli sopra</option>
        <option value="sotto">C · Didascalia sotto</option>
        <option value="solo_simboli">D · Solo simboli</option>
        <option value="semplificata">E · Semplificata</option>
      </select>
      <select
        aria-label="Livello di simbolizzazione"
        value={settings.intensity}
        onchange={(event) => patchSettings({ intensity: (event.currentTarget as HTMLSelectElement).value as never })}
      >
        <option value="minima">Essenziale</option>
        <option value="media">Bilanciata</option>
        <option value="completa">Estesa</option>
      </select>
      <button
        class="btn"
        class:btn--solid={neuralConfigured && settings.neural_context}
        aria-pressed={settings.neural_context}
        title={neuralConfigured
          ? `Analisi contestuale neurale attiva${aiStatus?.active_model ? ` (${aiStatus.active_model})` : ""}: propone simboli affini durante la scrittura e può correggere le scelte incerte`
          : "Modello linguistico locale non configurato: resta attivo il motore lessicale. Puoi scaricarne uno in Impostazioni → Gestione modelli"}
        onclick={() => patchSettings({ neural_context: !settings.neural_context })}>
        Analisi neurale: {settings.neural_context ? (neuralConfigured ? "Attiva" : "non configurata") : "Disattiva"}
      </button>
      <button
        class="btn"
        title="Apre il pannello laterale sulla scheda Impostazioni"
        onclick={() => openSettingsPanel()}>Impostazioni</button
      >
    </div>

    <div class="topbar__group" role="group" aria-label="Esportazione">
      <button class="btn" onclick={printDocument} title="Stampa o salva come PDF (Ctrl+P)">PDF</button>
      <button class="btn" onclick={() => void exportDocx()}>DOCX</button>
      <button class="btn" onclick={() => void exportHtml()}>HTML</button>
      <button class="btn" onclick={() => void exportAttributions()} title="Registro delle licenze dei simboli usati">Licenze</button>
    </div>

    <button class="btn" onclick={speakParagraph} title="Leggi il paragrafo o il documento">🔊 Ascolta</button>
    <button
      class="btn"
      onclick={() => (communicatorOpen = true)}
      disabled={!resourceLayout}
      title={resourceLayout
        ? "Apri la tabella come comunicatore interattivo"
        : "Genera o apri una tabella per usare il comunicatore"}
    >
      Comunicatore
    </button>
    <div class="topbar__spacer"></div>
    <span class="hint" aria-live="polite">{autoSavedAt ? `Bozza salvata alle ${autoSavedAt}` : ""}</span>
  </header>

  {#if !native}
    <div class="banner banner--demo" role="status">
      <strong>Anteprima browser.</strong>
      Il motore nativo (Rust, SQLite, pittogrammi) non è disponibile qui: l'editor e l'interfaccia sono
      reali, ma i pittogrammi non vengono mostrati. Per usare il software completo avvia
      <code>npm run tauri dev</code>.
    </div>
  {/if}

  {#if recovery}
    <div class="banner banner--recovery" role="alert">
      <strong>Documento non salvato recuperato.</strong>
      C'è una bozza automatica con {recovery.plain_text.length} caratteri.
      <button class="btn btn--solid" onclick={restoreRecovery}>Recupera</button>
      <button class="btn" onclick={() => (recovery = null)}>Ignora</button>
    </div>
  {/if}

  <div class="workspace">
    <main class="stage" aria-label="Area di scrittura">
      <div
        class="paper"
        class:paper--a4-orizzontale={settings.page === "a4-orizzontale"}
        class:paper--a3={settings.page === "a3"}
        data-uppercase={settings.uppercase ? "true" : "false"}
        style={settings.font_family ? `font-family:${settings.font_family}` : ""}
      >
        <CaaEditor
          bind:this={editor}
          mode={effectiveMode}
          autoEnabled={settings.auto_caa}
          neural={neuralReady}
          symbolizeArticles={settings.symbolize_articles}
          symbolizePrepositions={settings.symbolize_prepositions}
          symbolizeConjunctions={settings.symbolize_conjunctions}
          onTokenMenu={(_pid, _token, at) => (unitMenu = at)}
          intensity={settings.intensity}
          documentKey={docKey}
          commercialOnly={settings.commercial_only}
          packId={settings.pack_id}
          selectedUnit={selected ? { pid: selected.pid, index: selected.token.index } : null}
          showCaptions={settings.show_captions}
          borderStyle={settings.border_style}
          onSelectToken={(pid, token) => (selected = { pid, token })}
          onError={(message) => toast(message, "error")}
          onAnalyzed={onAnalyzed}
        />
      </div>

      <div class="resource-print" class:active={printingResource}>
        {#if resourceLayout}
          <LayoutPreview doc={resourceLayout} />
        {:else if resourcePlan}
          <h1>{resourcePlan.title}</h1>
          <div class="print-grid" style={`grid-template-columns: repeat(${resourcePlan.grid.columns}, 1fr);`}>
            {#each resourcePlan.items as item (item.label)}
              <div class="print-cell">
                {#if item.svg}
                  {@html item.svg}
                {:else}
                  <span class="caa-chip__empty">?</span>
                {/if}
                <div class="print-cell__label">{item.label}</div>
              </div>
            {/each}
          </div>
          <p class="hint">Generato con CAA Writer. Pittogrammi: vedere il registro delle licenze.</p>
        {/if}
      </div>
    </main>

    {#if showLayoutPreview && resourceLayout}
      <div
        role="dialog"
        aria-modal="true"
        aria-label="Anteprima di stampa della risorsa"
        style="position:fixed;inset:0;z-index:60;background:rgba(18,18,20,0.72);overflow:auto;padding:18px"
      >
        <div
          style="display:flex;flex-wrap:wrap;align-items:center;gap:10px;background:var(--surface,#fff);padding:8px 12px;border-radius:10px;margin-bottom:12px"
        >
          <strong>{resourceLayout.spec.title || "Anteprima"}</strong>
          <span class="hint">
            {resourceLayout.pages.length} pagina/e · {resourceLayout.spec.page.size}
            {resourceLayout.spec.page.orientation === "landscape" ? "orizzontale" : "verticale"}
          </span>
          <span style="flex:1 1 auto"></span>
          <button class="btn" onclick={() => (showLayoutPreview = false)}>Chiudi</button>
          <button class="btn" onclick={() => (communicatorOpen = true)}>Usa come comunicatore</button>
          <button class="btn btn--solid" onclick={() => printResource()}>Stampa</button>
        </div>
        <div
          style="display:flex;flex-wrap:wrap;align-items:center;gap:8px;background:var(--surface,#fff);padding:8px 12px;border-radius:10px;margin-bottom:12px"
        >
          <input
            bind:value={layoutPrompt}
            placeholder="Es. metti tutto in orizzontale, aumenta i pittogrammi, dividi in due pagine"
            aria-label="Modifica il layout con una frase"
            style="flex:1 1 260px;min-width:200px"
            onkeydown={(event) => {
              if (event.key === "Enter") {
                event.preventDefault();
                void applyLayoutEdit();
              }
            }}
          />
          <button
            class="btn btn--solid"
            onclick={() => void applyLayoutEdit()}
            disabled={layoutBusy || !layoutPrompt.trim()}
          >
            {layoutBusy ? "Applico…" : "Applica"}
          </button>
          <button class="btn" onclick={() => void undoLayoutEdit()} disabled={layoutHistory.length === 0}>
            Annulla modifica
          </button>
          <button class="btn" onclick={() => void redoLayoutEdit()} disabled={layoutRedo.length === 0}>
            Ripeti
          </button>
        </div>
        {#if layoutApplied.length > 0}
          <p
            class="hint"
            style="background:var(--surface,#fff);padding:6px 10px;border-radius:8px;margin-bottom:12px"
          >
            Applicato: {layoutApplied.join(" · ")}
          </p>
        {/if}
        <LayoutPreview doc={resourceLayout} compact />
      </div>
    {/if}

    {#if communicatorOpen && resourceLayout}
      <Communicator
        doc={resourceLayout}
        onSpeak={(text) => void speak(text)}
        onClose={() => (communicatorOpen = false)}
      />
    {/if}

    <div class="panel-wrap" style={`width:${panelWidth}px`}>
      <button
        type="button"
        class="panel-resizer"
        aria-label="Ridimensiona il pannello laterale con le frecce"
        title="Trascina per ridimensionare il pannello (frecce con la tastiera)"
        onpointerdown={startPanelResize}
        onkeydown={(event) => {
          if (event.key === "ArrowLeft") {
            event.preventDefault();
            resizePanelBy(16);
          } else if (event.key === "ArrowRight") {
            event.preventDefault();
            resizePanelBy(-16);
          }
        }}
      ></button>
      <SidePanel
        {selected}
      {settings}
      {appInfo}
      {stats}
      {packs}
      {voices}
      {aiStatus}
      {preferences}
      {templates}
      intent={sideIntent}
      onSwitchTab={() => {}}
      onApplyAlternative={applyAlternative}
      onSaveGlobalPreference={(alternative) => savePreference(alternative, null)}
      onSaveContextPreference={(alternative, context) => savePreference(alternative, context)}
      onResetUnit={resetUnit}
      onUpdateSettings={patchSettings}
      onImportSvg={importSvg}
      onSpeak={speak}
      onSetAiConfig={setAiConfig}
      onStatusChange={refreshAiStatus}
      onGenerateResource={generateResource}
      onPrintResource={printResource}
      onToast={toast}
      onClearHistory={clearHistory}
      onSetUnitState={(patch) => void setUnitState(patch)}
    />
    </div>
  </div>

</div>
{/if}

{#if unitMenu && selected}
  <div class="unit-menu" style={`left:${unitMenu.x}px; top:${unitMenu.y}px`} role="menu" aria-label="Azioni sul pittogramma">
    <p class="unit-menu__title">«{selected.token.surface}»</p>
    <button class="unit-menu__item" role="menuitem" onclick={() => void setUnitState({ locked: !selected!.token.locked })}>
      {selected.token.locked ? "Sblocca pittogramma" : "Blocca pittogramma"}
    </button>
    <button class="unit-menu__item" role="menuitem" onclick={() => void setUnitState({ hidden: !selected!.token.hidden })}>
      {selected.token.hidden ? "Mostra pittogramma" : "Nascondi pittogramma"}
    </button>
    <p class="unit-menu__group">Modificatori</p>
    {#each modifierOptions as option (option.kind)}
      <button
        class="unit-menu__item"
        role="menuitemcheckbox"
        aria-checked={selected.token.modifiers?.[option.kind] ?? false}
        onclick={() => toggleModifier(option.kind)}>
        {selected.token.modifiers?.[option.kind] ? "✓ " : ""}{option.label}
      </button>
    {/each}
    <button class="unit-menu__item" role="menuitem" onclick={resetModifiers}>Ripristina automatici</button>
    <button class="unit-menu__item unit-menu__item--close" role="menuitem" onclick={() => (unitMenu = null)}>Chiudi</button>
  </div>
{/if}

{#if settingsOpen}
  <SettingsDialog
    {settings}
    {appInfo}
    {stats}
    {packs}
    {aiStatus}
    {preferences}
    onUpdateSettings={patchSettings}
    onSetAiConfig={setAiConfig}
    onStatusChange={refreshAiStatus}
    onClearHistory={clearHistory}
    onToast={toast}
    onClose={() => (settingsOpen = false)}
  />
{/if}

<div class="toast-area" aria-live="polite" aria-atomic="false">
  {#each toasts as item (item.id)}
    <div class="toast" class:toast--warn={item.kind === "warn"} class:toast--error={item.kind === "error"}>
      <button class="toast__close" aria-label="Chiudi avviso" onclick={() => (toasts = toasts.filter((t) => t.id !== item.id))}>×</button>
      {item.text}
    </div>
  {/each}
</div>
