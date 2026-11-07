<script lang="ts">
  import { onDestroy, onMount, tick } from "svelte";
  import { get } from "svelte/store";
  import { language } from "./lib/i18n";
  import { open as openDialog, save as saveDialog } from "@tauri-apps/plugin-dialog";
  import CaaEditor from "./lib/editor/CaaEditor.svelte";
  import SidePanel from "./lib/components/SidePanel.svelte";
  import SidebarRail from "./lib/components/SidebarRail.svelte";
  import SettingsDialog from "./lib/components/SettingsDialog.svelte";
  import Ribbon from "./lib/components/Ribbon.svelte";
  import FindReplace from "./lib/components/FindReplace.svelte";
  import ReportDialog from "./lib/components/ReportDialog.svelte";
  import PresetDialog from "./lib/components/PresetDialog.svelte";
  import LayoutPreview from "./lib/components/LayoutPreview.svelte";
  import Communicator from "./lib/components/Communicator.svelte";
  import ImportDialog from "./lib/components/ImportDialog.svelte";
  import DocumentAssistant from "./lib/components/DocumentAssistant.svelte";
  import CliConsole from "./lib/components/CliConsole.svelte";
  import UserHome from "./lib/components/UserHome.svelte";
  import UserEditor from "./lib/components/UserEditor.svelte";
  import UniversalEditor from "./lib/components/UniversalEditor.svelte";
  import BoardPlayer from "./lib/components/BoardPlayer.svelte";
  import ProfileSetup from "./lib/components/ProfileSetup.svelte";
  import ProfileManager from "./lib/components/ProfileManager.svelte";
  import HomeScreen from "./lib/components/HomeScreen.svelte";
  import FreeBoard from "./lib/components/FreeBoard.svelte";
  import { recordRecent } from "./lib/recent";
  import { freeBoardFromLayout, freeBoardSpec } from "./lib/freeboard";
  import { api, isTauri } from "./lib/api";
  import { rasterizeSymbols } from "./lib/raster";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { listen } from "@tauri-apps/api/event";
  import { equivalentTargets } from "./lib/association/scope";
  import type { FormattingState } from "./lib/editor/formatting";
  import { emptyFormatting } from "./lib/editor/formatting";
  import type { RibbonCommand, RibbonGroup, RibbonMode, RibbonTab } from "./lib/ribbon/types";
  import {
    applyPreset as applyStoredWorkspacePreset,
    clampSidebarWidth,
    customPresets,
    defaultQuickAccess,
    presetNames,
    resetWorkspace,
    saveCurrentPreset,
    updateWorkspace,
    workspace,
    type AssociationScope,
    type WorkspacePanel,
  } from "./lib/workspace/layout";
  import type { CollectionInfo, DocumentOperation, ImportReport, Profile, ProfileBoard } from "./lib/types";
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
    type FreeBoardData,
    type LayoutDocument,
    type LayoutSpecification,
    type LibraryStats,
    type PackInfo,
    type Preference,
    type ResourcePlan,
    type RuntimeDiagnostics,
    type TemplateInfo,
    type UnitModifiers,
    type UniversalDocument,
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
  let freeBoardData = $state<FreeBoardData | null>(null);
  let universalDoc = $state<UniversalDocument | null>(null);
  let universalPath = $state<string | null>(null);
  let universalFromLibrary = $state(false);
  let universalReadOnly = $state(false);
  let resourceLayout = $state<LayoutDocument | null>(null);
  let showLayoutPreview = $state(false);
  let communicatorOpen = $state(false);
  let layoutPrompt = $state("");
  let layoutBusy = $state(false);
  let layoutHistory = $state<LayoutSpecification[]>([]);
  let layoutRedo = $state<LayoutSpecification[]>([]);
  let layoutApplied = $state<string[]>([]);
  let printingResource = $state(false);
  let showEditorRuler = $state(false);
  let aiConfig = $state<AiConfig | null>(null);
  let analyzing = $state(false);
  let speechActive = $state(false);
  let userEditorOpen = $state(false);
  let speechPoll: ReturnType<typeof setInterval> | null = null;
  let diagnostics = $state<RuntimeDiagnostics | null>(null);
  let editorMetrics = $state({
    input_samples: 0,
    input_p95_ms: 0,
    input_worst_ms: 0,
    update_p95_ms: 0,
    analysis_runs: 0,
    long_tasks: 0,
    longest_task_ms: 0,
    last_analysis_ms: 0,
  });

  type SideTab = "alternative" | "cerca" | "collezioni" | "impostazioni" | "studio";
  type SideIntent = {
    tab: SideTab;
    prompt?: string;
    templateId?: string | null;
    search?: string;
    ai?: boolean;
    nonce: number;
  };

  let view = $state<"home" | "editor" | "board" | "universal">("home");
  // Le impostazioni sono una finestra modale autonoma: non dipendono più
  // dall'apertura del pannello laterale, che restava chiuso o vuoto.
  let settingsOpen = $state(false);
  let sideIntent = $state<SideIntent | null>(null);
  let intentSeq = 0;

  let formatState = $state<FormattingState>({ ...emptyFormatting });
  let ribbonTab = $state("home");
  let findOpen = $state(false);
  let report = $state<{ title: string; lines: string[] } | null>(null);
  let focusBarVisible = $state(false);
  let presetDialogOpen = $state(false);
  let clickAudio: AudioContext | null = null;

  function playClick() {
    try {
      clickAudio ??= new AudioContext();
      const oscillator = clickAudio.createOscillator();
      const gain = clickAudio.createGain();
      oscillator.frequency.value = 880;
      gain.gain.value = 0.025;
      oscillator.connect(gain);
      gain.connect(clickAudio.destination);
      oscillator.start();
      oscillator.stop(clickAudio.currentTime + 0.018);
    } catch {
      /* audio non disponibile: nessun errore bloccante */
    }
  }

  let importPaths = $state<string[]>([]);
  let pictogramCollections = $state<CollectionInfo[]>([]);
  let assistantOpen = $state(false);
  let cliOpen = $state(false);
  let profiles = $state<Profile[]>([]);
  let activeProfile = $state<Profile | null>(null);
  let userBoards = $state<ProfileBoard[]>([]);
  let creatorUnlocked = $state(true);
  let creatorPin = $state("");
  let setupOpen = $state(false);
  let profileManagerOpen = $state(false);
  let boardPlayerId = $state<string | null>(null);
  const userMode = $derived(Boolean(activeProfile) && !creatorUnlocked);
  let lastDocumentEdit = $state<{
    settings: DocumentSettings;
    table: boolean;
  } | null>(null);

  function applyDocumentPlan(operations: DocumentOperation[]): {
    applied: number;
    warnings: string[];
  } {
    const before = { ...settings };
    const next = { ...settings };
    let applied = 0;
    let usedTable = false;
    const warnings: string[] = [];

    for (const operation of operations) {
      switch (operation.type) {
        case "set_uppercase":
          next.uppercase = operation.value;
          applied += 1;
          break;
        case "scale_font_size":
          next.font_size = Math.min(72, Math.max(8, Math.round(next.font_size * operation.factor)));
          applied += 1;
          break;
        case "set_font_size":
          next.font_size = Math.min(72, Math.max(8, Math.round(operation.value)));
          applied += 1;
          break;
        case "scale_symbol_size":
          next.symbol_size = Math.min(
            200,
            Math.max(24, Math.round(next.symbol_size * operation.factor))
          );
          applied += 1;
          break;
        case "set_symbol_gap":
          next.symbol_gap = Math.min(64, Math.max(0, operation.value));
          applied += 1;
          break;
        case "set_chip_border_color":
          next.chip_border_color = operation.value;
          applied += 1;
          break;
        case "set_alignment":
          editor?.applyAlignment(operation.value);
          applied += 1;
          break;
        case "insert_table":
          editor?.insertTable(operation.rows, operation.columns);
          usedTable = true;
          applied += 1;
          break;
        case "add_table_row":
        case "add_table_column": {
          const done = editor?.tableAction(
            operation.type === "add_table_row" ? "addRowAfter" : "addColumnAfter"
          );
          if (done) {
            usedTable = true;
            applied += 1;
          } else {
            warnings.push("Aggiungi riga/colonna richiede il cursore dentro una tabella.");
          }
          break;
        }
        case "generate_board":
          sideIntent = {
            tab: "studio",
            prompt: operation.prompt,
            nonce: ++intentSeq,
          };
          openPanelTab("studio");
          warnings.push(
            "La tabella comunicativa è stata aperta nello Studio come proposta modificabile; non è stata inserita automaticamente nel documento."
          );
          applied += 1;
          break;
      }
    }

    patchSettings(next);
    lastDocumentEdit = { settings: before, table: usedTable };
    return { applied, warnings };
  }

  function undoDocumentPlan(): boolean {
    if (!lastDocumentEdit) return false;
    patchSettings(lastDocumentEdit.settings);
    if (lastDocumentEdit.table) editor?.undoEdit();
    lastDocumentEdit = null;
    toast("Ultima modifica annullata.", "info");
    return true;
  }

  async function refreshPictogramCollections() {
    if (!native) return;
    try {
      pictogramCollections = await api.listPictogramCollections();
    } catch {
      /* la libreria personale non deve bloccare l'avvio */
    }
  }

  async function loadBoardsForProfile(profile: Profile) {
    try {
      userBoards = await api.profileBoards(profile.id);
    } catch {
      userBoards = [];
    }
  }

  async function initializeProfiles() {
    if (!native) return;
    try {
      profiles = await api.profilesList();
      if (profiles.length === 0) {
        activeProfile = null;
        userBoards = [];
        setupOpen = true;
        creatorUnlocked = true;
        return;
      }
      const active = await api.profileActive();
      activeProfile = active ?? profiles[0];
      if (activeProfile) await loadBoardsForProfile(activeProfile);
      creatorUnlocked = false;
    } catch (error) {
      toast(error instanceof Error ? error.message : String(error), "error");
    }
  }

  async function saveFirstProfile(profile: Profile, pin: string | null) {
    await api.profileSave(profile, null);
    if (pin) await api.creatorSetPin(pin, null);
    await api.profileSetActive(profile.id);
    try {
      const library = await api.universalLibrary();
      for (const [index, entry] of library.entries.slice(0, 3).entries()) {
        await api.profileAssignBoard(profile.id, entry.id, true, index, pin);
      }
    } catch {
      /* i materiali si possono assegnare in seguito dalla Gestione utenti */
    }
    await initializeProfiles();
    setupOpen = false;
    creatorUnlocked = true;
    toast(`Profilo «${profile.name}» creato.`, "info");
  }

  async function openCreator() {
    if (native) {
      try {
        const needsPin = await api.creatorPinStatus();
        if (needsPin) {
          const pin = window.prompt("PIN dell'Area Creator");
          if (!pin) return;
          if (!(await api.creatorVerifyPin(pin))) {
            toast("PIN non valido.", "warn");
            return;
          }
          creatorPin = pin;
        }
      } catch (error) {
        toast(error instanceof Error ? error.message : String(error), "error");
        return;
      }
    }
    creatorUnlocked = true;
    boardPlayerId = null;
    if (activeProfile) await loadBoardsForProfile(activeProfile);
  }

  async function exitCreator() {
    creatorUnlocked = false;
    creatorPin = "";
    boardPlayerId = null;
    userEditorOpen = false;
    universalDoc = null;
    universalPath = null;
    // Si torna sempre alla schermata dell'Utilizzatore: senza questo, un
    // documento aperto in Area Creator restava visibile dopo il cambio modo.
    view = "home";
    if (activeProfile) await loadBoardsForProfile(activeProfile);
  }

  async function refreshProfileData() {
    profiles = await api.profilesList();
    const active = await api.profileActive();
    const next = active ?? profiles[0] ?? null;
    activeProfile = next;
    if (next) await loadBoardsForProfile(next);
    else userBoards = [];
  }

  async function importPictogramImages() {
    if (!native) {
      toast("L'importazione richiede l'app desktop.", "warn");
      return;
    }
    const picked = await openDialog({
      title: "Importa immagini",
      multiple: true,
      filters: [
        {
          name: "Immagini supportate",
          extensions: ["png", "jpg", "jpeg", "svg", "webp", "gif", "bmp", "tif", "tiff", "avif", "heic", "heif", "ico"],
        },
      ],
    });
    if (!picked) return;
    importPaths = Array.isArray(picked) ? picked.map(String) : [String(picked)];
  }

  async function importPictogramFolder() {
    if (!native) {
      toast("L'importazione richiede l'app desktop.", "warn");
      return;
    }
    const picked = await openDialog({
      title: "Importa una cartella di pittogrammi",
      directory: true,
      multiple: false,
    });
    if (!picked) return;
    importPaths = [String(picked)];
  }

  async function onPictogramsImported(report: ImportReport, collection: string) {
    await refreshPictogramCollections();
    stats = await api.libraryStats().catch(() => stats);
    editor?.refreshAll();
    if (report.imported > 0) {
      toast(
        `${report.imported} pittogrammi importati in «${collection}».`,
        "info"
      );
    } else if (report.skipped > 0) {
      toast("Nessun nuovo pittogramma: erano già presenti.", "info");
    }
  }

  type AssociationItem = {
    pid: string;
    index: number;
    unitKey: string;
    previousSymbol: string | null;
    previousConcept: string | null;
    previousLocked: boolean;
    newSymbol: string;
    newConcept: string | null;
  };
  type AssociationAction = {
    items: AssociationItem[];
    remembered: { scope: string; lemma: string; contextKey: string | null } | null;
  };
  let associationUndo = $state<AssociationAction[]>([]);
  let associationRedo = $state<AssociationAction[]>([]);
  let associationPending = $state(false);

  const zoom = $derived($workspace.zoom);
  const fullscreen = $derived($workspace.fullscreen);
  const ribbonMode = $derived($workspace.ribbonMode);
  const panelClosed = $derived($workspace.sidebarMode === "collapsed");
  let windowWidth = $state(typeof window === "undefined" ? 1440 : window.innerWidth);
  // Su finestre medie il pannello ancorato diventa temporaneamente sovrapposto,
  // senza modificare la preferenza salvata.
  const effectiveSidebarMode = $derived(
    windowWidth < 720 && $workspace.sidebarMode !== "collapsed"
      ? "collapsed"
      : windowWidth < 1100 && $workspace.sidebarMode === "docked"
        ? "overlay"
        : $workspace.sidebarMode
  );
  // Se la scheda contestuale non è più pertinente, si torna a Home.
  $effect(() => {
    const ids = ribbonTabs().map((tab) => tab.id);
    if (!ids.includes(ribbonTab)) ribbonTab = "home";
  });

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

  onMount(() => {
    if (!native) return;
    let unlisten: (() => void) | null = null;
    void (async () => {
      try {
        const pending = await api.takePendingOpen();
        if (pending) await loadFrom(pending, true);
        unlisten = await listen<string>("open-document", (event) => {
          if (event.payload) void loadFrom(event.payload, true);
        });
      } catch {
        /* il player resta utilizzabile anche se l'evento non è disponibile */
      }
    })();
    return () => {
      if (unlisten) unlisten();
    };
  });

  onDestroy(() => {
    if (speechPoll) clearInterval(speechPoll);
  });
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
    if (patch.speech_enabled !== undefined && native) {
      void api.setSetting("speech.enabled", String(patch.speech_enabled));
    }
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
    root.dataset.animations = settings.animations;
    root.style.setProperty("--text-size", `${settings.font_size}px`);
    root.style.setProperty(
      "--symbol-size",
      settings.touch_mode ? `${Math.max(64, settings.symbol_size + 16)}px` : `${settings.symbol_size}px`
    );
    root.style.setProperty("--symbol-gap", `${settings.symbol_gap}px`);
    root.style.setProperty("--chip-border-color", settings.chip_border_color || "var(--border)");
    root.style.setProperty("--chip-border-width", `${settings.chip_border_width}px`);
    root.style.setProperty("--chip-border-style", settings.chip_border_style);
    root.style.setProperty("--chip-radius", `${settings.chip_radius}px`);
    root.style.setProperty("--chip-padding", `${settings.chip_padding}px`);
    root.style.setProperty("--chip-bg", settings.chip_background || "var(--surface)");
    root.style.setProperty("--page-bg", settings.page_color);
    root.style.setProperty(
      "--strip-justify",
      settings.pictogram_align === "center"
        ? "center"
        : settings.pictogram_align === "right"
          ? "flex-end"
          : "flex-start"
    );
    root.style.setProperty(
      "--page-margin",
      settings.page_margin === "stretto"
        ? "24px 30px 32px"
        : settings.page_margin === "ampio"
          ? "56px 72px 72px"
          : "40px 48px 56px"
    );
  });

  onMount(() => {
    if (!native) return;
    if ($workspace.fullscreen) {
      void getCurrentWindow()
        .setFullscreen(true)
        .catch(() => updateWorkspace({ fullscreen: false }));
    }
    void (async () => {
      try {
        appInfo = await api.appInfo();
        stats = await api.libraryStats();
        packs = await api.listPacks();
        voices = await api.listVoices();
        aiStatus = await api.aiStatus();
        preferences = await api.listPreferences();
        templates = await api.listTemplates();
        await refreshPictogramCollections();
        await initializeProfiles();
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
        await api.setSetting("speech.enabled", String(settings.speech_enabled));
        await checkRecovery();
      } catch (error) {
        toast(error instanceof Error ? error.message : String(error), "error");
      }
    })();

    const onKey = (event: KeyboardEvent) => {
      if (userMode) return;
      if (event.key === "Escape") {
        if (findOpen) {
          findOpen = false;
          return;
        }
        if ($workspace.focusMode) {
          updateWorkspace({ focusMode: false });
          return;
        }
      }
      const meta = event.metaKey || event.ctrlKey;
      if (!meta) return;
      const key = event.key.toLowerCase();
      if (associationPending && key === "z" && associationUndo.length > 0 && !event.shiftKey) {
        event.preventDefault();
        event.stopPropagation();
        void undoAssociation();
        return;
      }
      if (associationPending && key === "z" && associationRedo.length > 0 && event.shiftKey) {
        event.preventDefault();
        event.stopPropagation();
        void redoAssociation();
        return;
      }
      if (event.key === "F1") {
        event.preventDefault();
        if (event.shiftKey) {
          withStageScroll(() =>
            updateWorkspace({ topbarMinimized: !$workspace.topbarMinimized })
          );
        } else {
          updateWorkspace({
            ribbonMode: $workspace.ribbonMode === "hidden" ? "full" : "hidden",
          });
        }
        return;
      }
      if (event.shiftKey && key === "f") {
        event.preventDefault();
        updateWorkspace({ focusMode: !$workspace.focusMode });
        return;
      }
      if (key === "s") {
        event.preventDefault();
        void saveDocument(event.shiftKey);
      } else if (key === "f" || key === "h") {
        event.preventDefault();
        findOpen = true;
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
    const onTypingKey = (event: KeyboardEvent) => {
      if (userMode) return;
      const target = event.target as HTMLElement | null;
      if (!target?.closest(".ProseMirror")) return;
      if (event.metaKey || event.ctrlKey || event.altKey) return;
      if (!native) return;
      const sentenceTrigger = ["Enter", ".", "!", "?", ":", ";"].includes(event.key);
      const wordTrigger = ["Enter", " "].includes(event.key);

      if (settings.speak_sentences && sentenceTrigger) {
        if (event.key === "Enter") {
          // Invio può dividere il paragrafo: si legge l'ultima frase del
          // blocco corrente prima che la transazione lo spezzi, senza
          // ripetere le frasi precedenti già lette.
          const sentence = editor?.currentSentenceText() ?? "";
          speakSentence(sentence, "enter");
        } else {
          // Il segno di punteggiatura non è ancora nel documento durante
          // keydown: si legge dopo l'inserimento, altrimenti la frase
          // risulterebbe incompleta e verrebbe scartata.
          window.setTimeout(() => {
            speakSentence(editor?.sentenceBeforeCursor() ?? "", "punct");
          }, 0);
        }
        return;
      }
      if (settings.speak_words && wordTrigger) {
        if (event.key === "Enter") {
          const text = editor?.currentBlockText() ?? "";
          const match = text.match(/[\p{L}\p{N}][\p{L}\p{N}'’\-]*\s*$/u);
          if (match) void speak(match[0].trim(), true);
        } else {
          const word = editor?.wordBeforeCursor();
          if (word) void speak(word, true);
        }
      }
    };
    const onClick = (event: MouseEvent) => {
      if (!settings.typing_sound) return;
      const target = event.target as HTMLElement | null;
      if (target?.closest("button, summary, .ribbon__tab, .tab")) playClick();
    };
    const onResize = () => (windowWidth = window.innerWidth);
    window.addEventListener("keydown", onKey, true);
    window.addEventListener("keydown", onTypingKey, true);
    window.addEventListener("click", onClick, true);
    window.addEventListener("resize", onResize);
    return () => {
      window.removeEventListener("keydown", onKey, true);
      window.removeEventListener("keydown", onTypingKey, true);
      window.removeEventListener("click", onClick, true);
      window.removeEventListener("resize", onResize);
    };
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
      freeBoardData = null;
      docPath = null;
      docTitle = "Progetto libero";
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
    void refreshDiagnostics();
  }

  async function refreshDiagnostics() {
    if (!native) {
      diagnostics = null;
      return;
    }
    diagnostics = await api.runtimeDiagnostics();
  }

  function resizePanelBy(delta: number) {
    updateWorkspace({
      sidebarWidth: clampSidebarWidth($workspace.sidebarWidth + delta),
      sidebarMode: $workspace.sidebarMode === "collapsed" ? "docked" : $workspace.sidebarMode,
    });
  }

  /// Trascinamento del bordo del pannello: nessuna operazione sul documento e
  /// nessuna perdita del cursore, perché non si tocca mai la selezione.
  function startPanelResize(event: PointerEvent) {
    event.preventDefault();
    const startX = event.clientX;
    const startWidth = $workspace.sidebarWidth;
    const move = (moveEvent: PointerEvent) => {
      updateWorkspace({
        sidebarWidth: clampSidebarWidth(startWidth - (moveEvent.clientX - startX)),
        sidebarMode: "docked",
      });
    };
    const stop = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", stop);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", stop);
  }

  async function openUniversalDocument(
    id: string,
    sourceType: string | null,
    readOnly = false
  ): Promise<boolean> {
    if (!native) {
      toast("La biblioteca universale richiede l'app desktop.", "warn");
      return false;
    }
    try {
      const opened = await api.universalOpen(id, sourceType);
      universalDoc = opened.document;
      universalPath = opened.path;
      universalFromLibrary = true;
      universalReadOnly = readOnly;
      docTitle = opened.document.title;
      view = "universal";
      return true;
    } catch (error) {
      toast(error instanceof Error ? error.message : String(error), "error");
      return false;
    }
  }

  async function openBoardUniversal(boardId: string) {
    // Ora un profilo può avere assegnato un template, una scheda o un sistema:
    // si cerca nell'intera biblioteca, non solo nei 30 sistemi comunicativi.
    const opened = await openUniversalDocument(boardId, null, true);
    if (!opened) boardPlayerId = boardId;
  }

  async function saveUniversalTo(document: UniversalDocument, path: string) {
    const assets = new Map<string, { symbol_id: string; svg: string | null; png_base64: null }>();
    for (const page of document.pages) {
      for (const element of page.elements) {
        if (element.symbol_id && element.svg && !assets.has(element.symbol_id)) {
          assets.set(element.symbol_id, {
            symbol_id: element.symbol_id,
            svg: element.svg,
            png_base64: null,
          });
        }
        for (const cell of element.grid?.cells ?? []) {
          if (cell.symbol_id && cell.svg && !assets.has(cell.symbol_id)) {
            assets.set(cell.symbol_id, {
              symbol_id: cell.symbol_id,
              svg: cell.svg,
              png_base64: null,
            });
          }
        }
      }
    }
    const plain = document.pages
      .flatMap((page) => [
        page.title,
        ...page.elements.flatMap((element) => [
          element.text,
          ...(element.grid?.cells.map((cell) => cell.text) ?? []),
        ]),
      ])
      .filter(Boolean)
      .join("\n");
    const result = await api.saveDocument(path, {
      title: document.title || "Documento",
      content: { type: "doc", content: [] },
      plain_text: plain,
      settings,
      assets: [...assets.values()],
      preferences: [],
      layout: null,
      universal: document,
    });
    universalPath = path;
    universalFromLibrary = false;
    docTitle = document.title;
    recordRecent({ path, title: docTitle, excerpt: plain.slice(0, 160) });
    if (result.warnings.length) result.warnings.forEach((warning) => toast(warning, "warn"));
    toast(`Documento salvato in ${path}`, "info");
  }

  async function saveUniversal(document: UniversalDocument) {
    if (!native) {
      toast("Il salvataggio richiede l'app desktop.", "warn");
      return;
    }
    let path = universalPath;
    if (!path || universalFromLibrary || universalReadOnly) {
      const picked = await saveDialog({
        title: "Salva documento universale",
        defaultPath: `${document.title || "documento"}.caadoc`,
        filters: [{ name: "Documento CAA Writer", extensions: ["caadoc"] }],
      });
      if (!picked) return;
      path = picked as string;
    }
    try {
      await saveUniversalTo(document, path);
    } catch (error) {
      toast(error instanceof Error ? error.message : String(error), "error");
    }
  }

  async function saveUniversalPlayer(document: UniversalDocument) {
    if (!native) {
      toast("Il salvataggio richiede l'app desktop.", "warn");
      return;
    }
    const picked = await saveDialog({
      title: "Salva copia per il player",
      defaultPath: `${document.title || "documento"}.caaplay`,
      filters: [{ name: "CAA Writer Player", extensions: ["caaplay"] }],
    });
    if (!picked) return;
    try {
      await saveUniversalTo(document, picked as string);
      universalReadOnly = true;
    } catch (error) {
      toast(error instanceof Error ? error.message : String(error), "error");
    }
  }

  async function exportUniversal(
    document: UniversalDocument,
    format: "html" | "svg" | "pdf" | "png" | "docx" | "pptx"
  ) {
    if (!native) {
      toast("L'esportazione richiede l'app desktop.", "warn");
      return;
    }
    const picked = await saveDialog({
      title: `Esporta ${format.toUpperCase()}`,
      defaultPath: `${document.title || "documento"}.${format}`,
      filters: [{ name: format.toUpperCase(), extensions: [format] }],
    });
    if (!picked) return;
    try {
      const result = await api.documentExport(document, format, picked as string);
      toast(`Esportato in ${result.path}`, "info");
    } catch (error) {
      toast(error instanceof Error ? error.message : String(error), "error");
    }
  }

  async function openUniversalFile(asPlayer: boolean) {
    if (!native) {
      toast("L'apertura dei documenti richiede l'app desktop.", "warn");
      return;
    }
    const picked = await openDialog({
      title: asPlayer ? "Apri nel player" : "Apri documento",
      multiple: false,
      filters: [{ name: "Documento CAA Writer", extensions: ["caadoc", "caaplay"] }],
    });
    if (!picked) return;
    await loadFrom(String(picked), asPlayer);
  }

  async function importUniversalFile() {
    if (!native) {
      toast("L'importazione richiede l'app desktop.", "warn");
      return;
    }
    const picked = await openDialog({
      title: "Importa una copia modificabile",
      multiple: false,
      filters: [{ name: "Documento CAA Writer", extensions: ["caadoc", "caaplay"] }],
    });
    if (!picked) return;
    try {
      const imported = await api.universalImport(String(picked));
      universalDoc = imported.document;
      universalPath = imported.path;
      universalFromLibrary = false;
      universalReadOnly = false;
      docTitle = imported.document.title;
      view = "universal";
      toast("Documento importato come copia modificabile.", "info");
    } catch (error) {
      toast(error instanceof Error ? error.message : String(error), "error");
    }
  }

  function editUniversalCopy() {
    universalReadOnly = false;
    universalFromLibrary = true;
  }

  function openTemplate(templateId: string) {
    const template = templates.find((item) => item.id === templateId);
    void (async () => {
      const opened = await openUniversalDocument(templateId, "template", false);
      if (opened) return;
      // Fallback dichiarato: se la biblioteca universale non è installata,
      // resta disponibile il vecchio percorso dello Studio.
      newDocument(true);
      docTitle = template?.name ?? "Nuovo materiale";
      sideIntent = {
        tab: "studio",
        prompt: template?.description ?? "",
        templateId,
        nonce: ++intentSeq,
      };
      view = "editor";
    })();
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

  async function saveFreeBoard(data: FreeBoardData) {
    if (!native) {
      toast("Il salvataggio richiede l'app desktop.", "warn");
      return;
    }
    let path = docPath;
    if (!path) {
      const picked = await saveDialog({
        title: "Salva foglio libero",
        defaultPath: `${data.title || "progetto-libero"}.caadoc`,
        filters: [{ name: "Documento CAA Writer", extensions: ["caadoc"] }],
      });
      if (!picked) return;
      path = picked as string;
    }
    const spec = freeBoardSpec(data);
    const assetMap = new Map<string, { symbol_id: string; svg: string | null; png_base64: null }>();
    for (const cell of data.cells) {
      if (cell.symbolId && cell.svg && !assetMap.has(cell.symbolId)) {
        assetMap.set(cell.symbolId, {
          symbol_id: cell.symbolId,
          svg: cell.svg,
          png_base64: null,
        });
      }
    }
    const assets = [...assetMap.values()];
    try {
      const result = await api.saveDocument(path, {
        title: data.title || "Progetto libero",
        content: { type: "doc", content: [] },
        plain_text: data.cells.map((cell) => cell.label).filter(Boolean).join("\n"),
        settings,
        assets,
        preferences: [],
        layout: spec,
      });
      docPath = path;
      docTitle = data.title || "Progetto libero";
      freeBoardData = data;
      recordRecent({ path, title: docTitle, excerpt: `${data.cells.length} caselle` });
      if (result.warnings.length) result.warnings.forEach((warning) => toast(warning, "warn"));
      toast(`Foglio libero salvato in ${path}`, "info");
    } catch (error) {
      toast(error instanceof Error ? error.message : String(error), "error");
    }
  }

  async function applyCommunicatorSpec(spec: LayoutSpecification) {
    try {
      resourceLayout = await api.buildLayout(spec);
      toast("Comunicatore aggiornato: celle, azioni e pagine sono state applicate.", "info");
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      toast(`Comunicatore non applicato: ${message}`, "error");
      throw error;
    }
  }

  async function saveLayoutDocument(spec: LayoutSpecification) {
    if (!native) {
      toast("Il salvataggio richiede l'app desktop.", "warn");
      return;
    }
    // Il documento salvato dal comunicatore si riapre nel comunicatore, non
    // come semplice materiale da stampa.
    const savedSpec: LayoutSpecification = { ...spec, document_type: "communicator" };
    const picked = await saveDialog({
      title: "Salva comunicatore",
      defaultPath: `${spec.title || "comunicatore"}.caadoc`,
      filters: [{ name: "Documento CAA Writer", extensions: ["caadoc"] }],
    });
    if (!picked) return;
    const path = picked as string;
    const assetMap = new Map<string, { symbol_id: string; svg: string | null; png_base64: null }>();
    for (const element of spec.elements ?? []) {
      if (element.symbol_id && element.svg && !assetMap.has(element.symbol_id)) {
        assetMap.set(element.symbol_id, {
          symbol_id: element.symbol_id,
          svg: element.svg,
          png_base64: null,
        });
      }
    }
    const assets = [...assetMap.values()];
    try {
      const result = await api.saveDocument(path, {
        title: spec.title || "Comunicatore",
        content: { type: "doc", content: [] },
        plain_text: (spec.elements ?? [])
          .map((element) => element.label)
          .filter(Boolean)
          .join("\n"),
        settings,
        assets,
        preferences: [],
        layout: savedSpec,
      });
      docPath = path;
      docTitle = spec.title || "Comunicatore";
      recordRecent({
        path,
        title: docTitle,
        excerpt: `${(spec.elements ?? []).length} celle comunicative`,
      });
      if (result.warnings.length) result.warnings.forEach((warning) => toast(warning, "warn"));
      toast(`Comunicatore salvato in ${path}`, "info");
    } catch (error) {
      toast(error instanceof Error ? error.message : String(error), "error");
    }
  }

  async function loadFrom(path: string, asPlayer = false) {
    try {
      const loaded = await api.loadDocument(path);
      docTitle = loaded.title || path.split(/[\\/]/).pop() || "Documento";
      docPath = path;
      docKey = `doc-${path}`;
      if (loaded.settings && typeof loaded.settings === "object") {
        settings = { ...defaultSettings, ...(loaded.settings as Partial<DocumentSettings>) };
      }
      if (loaded.universal) {
        universalDoc = loaded.universal as UniversalDocument;
        universalPath = path;
        universalFromLibrary = false;
        universalReadOnly = asPlayer;
        selected = null;
        view = "universal";
        recordRecent({
          path,
          title: docTitle,
          excerpt: `${universalDoc.pages.length} pagine`,
        });
        loaded.warnings.forEach((warning) => toast(warning, "warn"));
        toast(`Aperto: ${docTitle}`, "info");
        return;
      }
      const savedLayout = loaded.layout as
        | { document_type?: string; layout?: { columns?: number }; elements?: Array<Record<string, unknown>> }
        | null;
      if (savedLayout?.document_type === "free_board") {
        freeBoardData = freeBoardFromLayout(loaded.title, savedLayout);
        selected = null;
        view = "board";
        recordRecent({ path, title: docTitle, excerpt: `${freeBoardData.cells.length} caselle` });
        loaded.warnings.forEach((warning) => toast(warning, "warn"));
        toast(`Aperto: ${docTitle}`, "info");
        return;
      }
      // L'editor può non essere ancora montato (apertura dalla schermata
      // iniziale): prima si passa alla vista documento, poi si inserisce il
      // contenuto, altrimenti il documento apparirebbe vuoto.
      selected = null;
      view = "editor";
      await tick();
      editor?.setDocJson(loaded.content, loaded.plain_text);
      dirty = false;
      resourceLayout = null;
      layoutHistory = [];
      layoutRedo = [];
      layoutApplied = [];
      layoutPrompt = "";
      if (loaded.layout) {
        try {
          resourceLayout = await api.buildLayout(loaded.layout);
          if (
            typeof loaded.layout === "object" &&
            loaded.layout !== null &&
            (loaded.layout as { document_type?: string }).document_type === "communicator"
          ) {
            communicatorOpen = true;
          }
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
    // Dopo un'unione, una separazione o un annullamento l'indice può puntare a
    // un'unità cambiata: il pannello deve mostrare lo stato reale, non una
    // copia vecchia del token.
    if (selected) {
      const token = map
        .get(selected.pid)
        ?.find((item) => item.index === selected!.token.index);
      selected = token ? { pid: selected.pid, token } : null;
    }
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
      include_captions:
        settings.mode === "sotto" ||
        ["immagine_testo", "separato", "divisore"].includes(settings.border_style),
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

  async function runPrint() {
    // Il WebView non stampa in modo affidabile con window.print(): si usa
    // l'API nativa della finestra dopo aver lasciato aggiornare il DOM.
    await new Promise((resolve) => requestAnimationFrame(() => resolve(null)));
    await new Promise((resolve) => requestAnimationFrame(() => resolve(null)));
    try {
      if (native) {
        await api.printWindow();
      } else {
        window.print();
      }
    } catch (error) {
      toast(error instanceof Error ? error.message : String(error), "error");
    }
  }

  function printDocument() {
    printingResource = false;
    document.body.dataset.print = "document";
    void runPrint();
  }

  function printResource() {
    if (!resourcePlan && !resourceLayout) return;
    showLayoutPreview = false;
    printingResource = true;
    document.body.dataset.print = "resource";
    void runPrint();
  }

  onMount(() => {
    const afterPrint = () => {
      delete document.body.dataset.print;
      printingResource = false;
    };
    window.addEventListener("afterprint", afterPrint);
    return () => window.removeEventListener("afterprint", afterPrint);
  });

  // Punto e Invio possono chiudere la stessa frase: si evita di leggerla
  // due volte di seguito, ma una frase diversa viene sempre letta.
  let lastSpokenSentence = "";
  let lastSpokenAt = 0;
  let lastSentenceTrigger: "punct" | "enter" | null = null;

  function speakSentence(text: string, trigger: "punct" | "enter") {
    const sentence = text.trim();
    if (!sentence) return;
    const now = Date.now();
    // Solo la coppia «punto seguito subito da Invio» è una ripetizione:
    // due frasi uguali digitate di seguito vengono lette entrambe.
    if (
      trigger === "enter" &&
      lastSentenceTrigger === "punct" &&
      sentence === lastSpokenSentence &&
      now - lastSpokenAt < 3000
    ) {
      lastSentenceTrigger = "enter";
      return;
    }
    lastSpokenSentence = sentence;
    lastSpokenAt = now;
    lastSentenceTrigger = trigger;
    void speak(sentence, true);
  }

  /// `silent` serve alle chiamate automatiche (lettura di parola o frase
  /// durante la scrittura): non devono mostrare avvisi a ogni tasto. I
  /// pulsanti espliciti restano invece sempre spiegati all'utente.
  async function speak(text: string, silent = false) {
    if (!text.trim()) {
      if (!silent) toast("Niente da leggere.", "warn");
      return;
    }
    if (!settings.speech_enabled) {
      if (!silent) {
        toast(
          "Sintesi vocale disattivata. Attivala in Impostazioni → Accessibilità.",
          "warn"
        );
      }
      return;
    }
    if (!native) {
      if (!silent) toast("La sintesi vocale richiede l'app desktop.", "warn");
      return;
    }
    // Il comando è asincrono: la UI mostra subito lo stato e resta scrivibile.
    speechActive = true;
    try {
      await api.speak(text, null, 1.0, get(language));
      trackSpeech();
    } catch (error) {
      speechActive = false;
      if (!silent) {
        toast(error instanceof Error ? error.message : String(error), "error");
      }
    }
  }

  /// La durata della frase non è nota in anticipo: si interroga il backend
  /// reale invece di indovinarla. Il polling è leggero (700 ms) e si ferma
  /// appena la voce termina.
  function trackSpeech() {
    if (speechPoll) clearInterval(speechPoll);
    speechPoll = setInterval(async () => {
      try {
        const active = await api.speechActive();
        speechActive = active;
        if (!active && speechPoll) {
          clearInterval(speechPoll);
          speechPoll = null;
        }
      } catch {
        speechActive = false;
        if (speechPoll) clearInterval(speechPoll);
        speechPoll = null;
      }
    }, 700);
  }

  async function stopSpeech() {
    if (speechPoll) {
      clearInterval(speechPoll);
      speechPoll = null;
    }
    speechActive = false;
    try {
      await api.stopSpeaking();
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
      toast("Seleziona prima una parola nel documento, poi scegli il simbolo.", "warn");
      return;
    }
    if (!native) {
      toast("La sostituzione richiede l'app desktop.", "warn");
      return;
    }
    const { pid, token } = selected;
    // Se la rianalisi ha già sostituito l'oggetto token, la superficie resta
    // il riferimento stabile. Se manca del tutto, l'override per indice viene
    // comunque salvato e la rianalisi lo applica appena possibile.
    const current = editor?.tokenAt(pid, token.index);
    if (current && current.surface !== token.surface) {
      selected = null;
      toast("La parola selezionata è cambiata: selezionala di nuovo.", "warn");
      return;
    }

    const scope: AssociationScope = get(workspace).associationScope;
    const conceptId = alternative.concept_id ?? token.concept_id ?? null;
    const effective: Alternative = {
      ...alternative,
      concept_id: conceptId,
    };

    const targets =
      scope === "document"
        ? equivalentTargets(token, conceptId, pid, editor?.getTokens() ?? tokensByParagraph)
        : [{ pid, token }];
    const items: AssociationItem[] = targets.map((target) => ({
      pid: target.pid,
      index: target.token.index,
      unitKey: unitKey(target.pid, target.token.index),
      previousSymbol: target.token.symbol_id ?? null,
      previousConcept: target.token.concept_id ?? null,
      previousLocked: target.token.locked,
      newSymbol: alternative.symbol_id,
      newConcept: conceptId,
    }));
    const remembered =
      scope === "global" || scope === "profile" || scope === "vocabulary"
        ? {
            scope,
            lemma: token.lemma ?? token.surface.toLocaleLowerCase(),
            contextKey:
              scope === "profile"
                ? activeProfile?.id
                  ? `profile:${activeProfile.id}${conceptId ? `:concept:${conceptId}` : ""}`
                  : null
                : conceptId
                  ? `concept:${conceptId}`
                  : null,
          }
        : null;

    try {
      await Promise.all(
        targets.map((target) =>
          api.setDocumentOverride(
            docKey,
            unitKey(target.pid, target.token.index),
            alternative.symbol_id,
            conceptId
          )
        )
      );
      editor?.applyTokenSymbols(
        targets.map((target) => ({
          pid: target.pid,
          index: target.token.index,
          alternative: effective,
        }))
      );
      for (const affected of new Set(targets.map((target) => target.pid))) {
        editor?.refreshParagraph(affected);
      }

      if (remembered) {
        await api.setPreference({
          scope: remembered.scope,
          lemma: remembered.lemma,
          context_key: remembered.contextKey,
          symbol_id: alternative.symbol_id,
          concept_id: conceptId,
          locked: false,
          created_at: "",
        });
        preferences = await api.listPreferences();
      }

      associationUndo = [...associationUndo, { items, remembered }].slice(-20);
      associationRedo = [];
      associationPending = true;
      if (scope === "document" && targets.length > 1) {
        toast(`Pittogramma applicato a ${targets.length} occorrenze.`, "info");
      } else if (scope === "profile") {
        toast(
          `Associazione salvata per il profilo attivo: «${token.surface}» userà questo pittogramma.`,
          "info"
        );
      } else if (scope === "vocabulary") {
        toast(
          `«${token.surface}» aggiunto al vocabolario condiviso del Creator.`,
          "info"
        );
      } else if (scope === "global") {
        toast(
          `Associazione salvata per «${token.surface}»: verrà considerata nei prossimi documenti.`,
          "info"
        );
      } else {
        toast("Pittogramma applicato a questa parola.", "info");
      }
      // Il pannello sovrapposto è temporaneo: dopo la scelta torna al documento.
      if (get(workspace).sidebarMode === "overlay") closeSidebar();
    } catch (error) {
      toast(error instanceof Error ? error.message : String(error), "error");
    }
  }

  async function restoreAssociation(action: AssociationAction) {
    await Promise.all(
      action.items.map((item) =>
        item.previousLocked && item.previousSymbol
          ? api.setDocumentOverride(
              docKey,
              item.unitKey,
              item.previousSymbol,
              item.previousConcept
            )
          : api.clearDocumentOverride(docKey, item.unitKey)
      )
    );
    if (action.remembered) {
      await api.deletePreference(
        action.remembered.scope,
        action.remembered.lemma,
        action.remembered.contextKey ?? ""
      );
    }
    editor?.refreshAll();
  }

  function syncSelectedSnapshot() {
    if (!selected) return;
    const { pid, token } = selected;
    const current = editor?.tokenAt(pid, token.index);
    selected = current ? { pid, token: current } : null;
  }

  function syncSelectedSoon() {
    window.setTimeout(syncSelectedSnapshot, 260);
  }

  async function undoAssociation() {
    const action = associationUndo[associationUndo.length - 1];
    if (!action) return;
    try {
      await restoreAssociation(action);
      associationUndo = associationUndo.slice(0, -1);
      associationRedo = [...associationRedo, action].slice(-20);
      associationPending = true;
      syncSelectedSoon();
      toast("Associazione annullata.", "info");
    } catch (error) {
      toast(error instanceof Error ? error.message : String(error), "error");
    }
  }

  async function redoAssociation() {
    const action = associationRedo[associationRedo.length - 1];
    if (!action) return;
    try {
      await Promise.all(
        action.items.map((item) =>
          api.setDocumentOverride(docKey, item.unitKey, item.newSymbol, item.newConcept)
        )
      );
      if (action.remembered) {
        const first = action.items[0];
        await api.setPreference({
          scope: action.remembered.scope,
          lemma: action.remembered.lemma,
          context_key: action.remembered.contextKey,
          symbol_id: first.newSymbol,
          concept_id: first.newConcept,
          locked: false,
          created_at: "",
        });
      }
      associationRedo = associationRedo.slice(0, -1);
      associationUndo = [...associationUndo, action].slice(-20);
      associationPending = true;
      editor?.refreshAll();
      syncSelectedSoon();
      toast("Associazione riapplicata.", "info");
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

  async function openPlanInEditor(plan: ResourcePlan) {
    try {
      const document = await api.universalFromPlan(plan);
      universalDoc = document;
      universalPath = null;
      universalFromLibrary = false;
      universalReadOnly = false;
      docTitle = plan.title;
      view = "universal";
      toast(
        `«${plan.title}» è un documento nativo: puoi modificarne celle, testi, pittogrammi e azioni.`,
        "info"
      );
    } catch (error) {
      toast(error instanceof Error ? error.message : String(error), "error");
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

  // ---------------------------------------------------------------------
  // Ribbon: comandi realmente operativi, raggruppati per scheda.
  // ---------------------------------------------------------------------

  function cmd(
    id: string,
    label: string,
    icon: string,
    execute: () => void,
    extra: Partial<RibbonCommand> = {}
  ): RibbonCommand {
    return { id, label, icon, execute, ...extra };
  }

  function toggle(
    id: string,
    label: string,
    icon: string,
    active: boolean,
    execute: () => void,
    extra: Partial<RibbonCommand> = {}
  ): RibbonCommand {
    return { ...cmd(id, label, icon, execute, extra), active };
  }

  function select(
    id: string,
    label: string,
    value: string,
    options: { value: string; label: string }[],
    onChange: (value: string) => void,
    extra: Partial<RibbonCommand> = {}
  ): RibbonCommand {
    return { id, label, kind: "select", value, options, onChange, ...extra };
  }

  function color(
    id: string,
    label: string,
    value: string,
    onChange: (value: string) => void,
    extra: Partial<RibbonCommand> = {}
  ): RibbonCommand {
    return { id, label, kind: "color", value, onChange, ...extra };
  }

  function group(
    id: string,
    label: string,
    commands: RibbonCommand[],
    visible?: number
  ): RibbonGroup {
    return { id, label, commands, visible };
  }

  function selectedWord(): string {
    return (
      selected?.token.surface ??
      editor?.getSelectedText().trim() ??
      ""
    );
  }

  function requireToken(): boolean {
    if (selected) return true;
    toast("Seleziona prima una parola o un pittogramma nel documento.", "warn");
    return false;
  }

  async function copySelection(cut = false) {
    const text = editor?.getSelectedText() ?? "";
    if (!text.trim()) {
      toast("Seleziona il testo da copiare.", "warn");
      return;
    }
    try {
      await navigator.clipboard.writeText(text);
      if (cut) editor?.deleteSelection();
      toast(cut ? "Testo tagliato." : "Testo copiato.", "info");
    } catch {
      toast("Gli appunti non sono disponibili in questa finestra.", "warn");
    }
  }

  async function pasteClipboard() {
    try {
      const text = await navigator.clipboard.readText();
      editor?.insertTextAtSelection(text);
    } catch {
      toast("La lettura degli appunti non è disponibile: usa Ctrl+V nell'editor.", "warn");
    }
  }

  function openFind() {
    findOpen = true;
  }

  function moveRibbonTab(tab: string) {
    ribbonTab = tab;
  }

  /// I cambi di layout non devono far saltare lo scorrimento del documento.
  function withStageScroll(action: () => void) {
    const stage = document.querySelector(".stage") as HTMLElement | null;
    const top = stage?.scrollTop ?? 0;
    const left = stage?.scrollLeft ?? 0;
    action();
    if (stage) {
      requestAnimationFrame(() => {
        stage.scrollTop = top;
        stage.scrollLeft = left;
      });
    }
  }

  function setRibbonMode(mode: RibbonMode) {
    withStageScroll(() => updateWorkspace({ ribbonMode: mode, ribbonPinned: false }));
  }

  function toggleQuickAccess(id: string) {
    const current = $workspace.quickAccess;
    updateWorkspace({
      quickAccess: current.includes(id)
        ? current.filter((item) => item !== id)
        : [...current, id],
    });
  }

  function toggleQuickLabels() {
    updateWorkspace({ quickLabels: !$workspace.quickLabels });
  }

  function openSidebar(tab: WorkspacePanel = $workspace.activeSidebarPanel) {
    // La vecchia scheda «Impostazioni» della barra laterale era vuota: ora
    // apre la finestra modale unica, sia dai preferiti sia dai preset.
    if (tab === "impostazioni") {
      openSettingsPanel();
      return;
    }
    withStageScroll(() =>
      updateWorkspace({
        activeSidebarPanel: tab,
        sidebarMode: $workspace.sidebarMode === "collapsed" ? "overlay" : $workspace.sidebarMode,
      })
    );
    sideIntent = { tab, nonce: ++intentSeq };
  }

  function closeSidebar() {
    withStageScroll(() => updateWorkspace({ sidebarMode: "collapsed" }));
    editor?.focus();
  }

  function toggleSidebarDock() {
    if (windowWidth < 1100) {
      withStageScroll(() => updateWorkspace({ sidebarMode: "docked" }));
      toast("Finestra stretta: il pannello resta sovrapposto finché non allarghi la finestra.", "info");
      return;
    }
    withStageScroll(() =>
      updateWorkspace({ sidebarMode: $workspace.sidebarMode === "docked" ? "overlay" : "docked" })
    );
  }

  function toggleFocusMode() {
    withStageScroll(() => updateWorkspace({ focusMode: !$workspace.focusMode }));
  }

  function applyWorkspacePreset(name: string) {
    if (!name) return;
    withStageScroll(() => applyStoredWorkspacePreset(name));
    sideIntent = { tab: get(workspace).activeSidebarPanel, nonce: ++intentSeq };
    toast(`Area di lavoro «${name}» applicata.`, "info");
  }

  function quickCatalog(): RibbonCommand[] {
    return [
      cmd("save", "Salva", "save", () => void saveDocument(false), { shortcut: "Ctrl+S" }),
      cmd("undo", "Annulla", "undo", () => editor?.undoAction(), {
        shortcut: "Ctrl+Z",
        enabled: formatState.canUndo,
      }),
      cmd("redo", "Ripristina", "redo", () => editor?.redoAction(), {
        shortcut: "Ctrl+Shift+Z",
        enabled: formatState.canRedo,
      }),
      cmd("search", "Cerca", "search", openFind, { shortcut: "Ctrl+F" }),
      cmd("listen", "Ascolta", "headphones", speakParagraph),
      cmd("export", "Esporta PDF", "printer", printDocument),
      cmd("find-replace", "Trova e sostituisci", "replace", openFind),
      cmd("word-count", "Conteggio parole", "calculator", showWordCount),
      cmd("panel", "Pannello laterale", "panel-right", () => (panelClosed ? openSidebar() : closeSidebar())),
      cmd("touch", "Modalità touch", "touchpad", () => patchSettings({ touch_mode: !settings.touch_mode }), {
        active: settings.touch_mode,
      }),
      cmd("zoom-in", "Zoom avanti", "zoom-in", () => setZoom(zoom + 0.1)),
      cmd("zoom-out", "Zoom indietro", "zoom-out", () => setZoom(zoom - 0.1)),
      cmd("fullscreen", "Schermo intero", "fullscreen", () => void toggleFullscreen()),
      cmd("auto-caa", "CAA automatica", "sparkles", () => patchSettings({ auto_caa: !settings.auto_caa }), {
        active: settings.auto_caa,
      }),
      cmd("refresh", "Rivaluta documento", "refresh", () => editor?.refreshAll()),
      cmd("focus", "Concentrazione", "eye", toggleFocusMode, {
        active: $workspace.focusMode,
      }),
    ];
  }

  function quickOptions() {
    return [
      { id: "save", label: "Salva" },
      { id: "undo", label: "Annulla" },
      { id: "redo", label: "Ripristina" },
      { id: "search", label: "Cerca" },
      { id: "listen", label: "Ascolta" },
      { id: "export", label: "Esporta PDF" },
      { id: "find-replace", label: "Trova e sostituisci" },
      { id: "word-count", label: "Conteggio parole" },
      { id: "panel", label: "Pannello laterale" },
      { id: "touch", label: "Modalità touch" },
      { id: "zoom-in", label: "Zoom avanti" },
      { id: "zoom-out", label: "Zoom indietro" },
      { id: "fullscreen", label: "Schermo intero" },
      { id: "auto-caa", label: "CAA automatica" },
      { id: "refresh", label: "Rivaluta documento" },
      { id: "focus", label: "Concentrazione" },
    ];
  }

  function quickCommands(): RibbonCommand[] {
    const catalog = new Map(quickCatalog().map((command) => [command.id, command]));
    return $workspace.quickAccess
      .map((id) => catalog.get(id))
      .filter((command): command is RibbonCommand => Boolean(command));
  }

  function setZoom(value: number) {
    updateWorkspace({ zoom: Math.min(1.8, Math.max(0.5, Math.round(value * 100) / 100)) });
  }

  function fitWidth() {
    const stage = document.querySelector(".stage") as HTMLElement | null;
    const paper = document.querySelector(".paper") as HTMLElement | null;
    if (!stage || !paper) return;
    const available = stage.clientWidth - 48;
    if (available > 0) setZoom(available / paper.offsetWidth);
  }

  async function toggleFullscreen() {
    if (!native) return;
    try {
      const current = getCurrentWindow();
      const next = !(await current.isFullscreen());
      await current.setFullscreen(next);
      updateWorkspace({ fullscreen: next });
    } catch (error) {
      toast(error instanceof Error ? error.message : String(error), "error");
    }
  }

  function openSearchForSelected(ai = false) {
    const word = selectedWord();
    if (!word) {
      toast("Seleziona una parola nel testo, poi cerca un pittogramma.", "warn");
      return;
    }
    updateWorkspace({
      activeSidebarPanel: "cerca",
      sidebarMode: $workspace.sidebarMode === "collapsed" ? "overlay" : $workspace.sidebarMode,
    });
    sideIntent = { tab: "cerca", search: word, ai, nonce: ++intentSeq };
  }

  function openPanelTab(tab: SideTab) {
    if (tab === "impostazioni") {
      openSettingsPanel();
      return;
    }
    updateWorkspace({
      activeSidebarPanel: tab,
      sidebarMode: $workspace.sidebarMode === "collapsed" ? "overlay" : $workspace.sidebarMode,
    });
    sideIntent = { tab, nonce: ++intentSeq };
  }

  function showWordCount() {
    const text = editor?.getPlainText() ?? "";
    const words = text.match(/[\p{L}\p{N}][\p{L}\p{N}'’-]*/gu)?.length ?? 0;
    report = {
      title: "Conteggio parole",
      lines: [
        `Parole: ${words}`,
        `Caratteri (spazi inclusi): ${text.length}`,
        `Caratteri (senza spazi): ${text.replace(/\s/g, "").length}`,
        `Paragrafi: ${editor?.getParagraphs().length ?? 0}`,
      ],
    };
  }

  function unitStats() {
    let units = 0;
    let symbolized = 0;
    let missing = 0;
    let approximate = 0;
    let manual = 0;
    let locked = 0;
    let withModifiers = 0;
    for (const tokens of tokensByParagraph.values()) {
      for (const token of tokens) {
        if (token.is_punct) continue;
        units += 1;
        if (token.symbol_id) symbolized += 1;
        if (token.key && !token.symbol_id) missing += 1;
        if (token.approximate) approximate += 1;
        if (token.strategy === "manual") manual += 1;
        if (token.locked) locked += 1;
        if (token.modifiers && Object.values(token.modifiers).some(Boolean)) withModifiers += 1;
      }
    }
    return { units, symbolized, missing, approximate, manual, locked, withModifiers };
  }

  function showReviewReport(kind: "semantic" | "coverage" | "modifiers") {
    const stats = unitStats();
    if (kind === "semantic") {
      report = {
        title: "Analisi semantica",
        lines: [
          `Unità nel documento: ${stats.units}`,
          `Unità con pittogramma: ${stats.symbolized}`,
          `Unità senza pittogramma: ${stats.missing}`,
          `Pittogrammi approssimativi: ${stats.approximate}`,
          `Associazioni manuali: ${stats.manual}`,
          `Unità bloccate: ${stats.locked}`,
        ],
      };
    } else if (kind === "coverage") {
      report = {
        title: "Copertura CAA",
        lines: [
          `Parole senza simbolo: ${stats.missing}`,
          `Associazioni manuali: ${stats.manual}`,
          `Pittogrammi bloccati: ${stats.locked}`,
          `Pittogrammi approssimativi: ${stats.approximate}`,
        ],
      };
    } else {
      report = {
        title: "Verifica modificatori",
        lines: [
          `Unità con modificatori attivi: ${stats.withModifiers}`,
          `Unità bloccate: ${stats.locked}`,
          "I modificatori manuali non vengono sovrascritti dalla rivalutazione.",
        ],
      };
    }
  }

  function applyPreset(preset: string) {
    if (preset === "essenziale") {
      patchSettings({
        mode: "sopra",
        intensity: "minima",
        border_style: "nessuno",
        symbol_size: 44,
        symbol_gap: 4,
        show_captions: false,
      });
    } else if (preset === "classico") {
      patchSettings({
        mode: "sotto",
        intensity: "media",
        border_style: "immagine_testo",
        symbol_size: 56,
        symbol_gap: 6,
        show_captions: true,
      });
    } else if (preset === "contrasto") {
      patchSettings({
        theme: "contrasto",
        border_style: "immagine",
        chip_border_color: "#000000",
        chip_border_width: 2,
        chip_background: "#ffffff",
      });
    } else if (preset === "grandi") {
      patchSettings({ mode: "sopra", symbol_size: 80, symbol_gap: 10, show_captions: false });
    } else if (preset === "schede") {
      patchSettings({
        mode: "solo_simboli",
        symbol_size: 72,
        symbol_gap: 8,
        border_style: "divisore",
        show_captions: true,
      });
    } else if (preset === "illustrato") {
      patchSettings({
        mode: "sopra",
        symbol_size: 40,
        symbol_gap: 6,
        border_style: "nessuno",
        show_captions: false,
      });
    }
  }

  function rememberSelected() {
    const symbolId = selected?.token.symbol_id;
    if (!symbolId) {
      toast("La parola selezionata non ha ancora un pittogramma.", "warn");
      return;
    }
    const token = selected!.token;
    void savePreference(
      {
        symbol_id: symbolId,
        concept_id: token.concept_id,
        label: token.symbol_label ?? token.surface,
        meaning: token.meaning ?? "",
        pack_id: token.pack_id ?? "",
        pack_name: "",
        format: "svg",
        svg: null,
        approximate: token.approximate,
        license: "",
        source_url: null,
        author: null,
      },
      null
    );
  }

  async function runAiReport(tool: string, title: string, input?: string) {
    if (!native) {
      toast("Gli strumenti IA richiedono l'app desktop.", "warn");
      return;
    }
    try {
      const value = input ?? editor?.getPlainText() ?? "";
      if (!value.trim()) {
        toast("Scrivi o seleziona del testo da elaborare.", "warn");
        return;
      }
      const outcome = await api.aiRunTool(tool, { text: value });
      const lines = [`Origine: ${outcome.source}`];
      const result =
        typeof outcome.result === "string"
          ? outcome.result
          : JSON.stringify(outcome.result, null, 2);
      lines.push(...result.split("\n").slice(0, 80));
      lines.push(...outcome.notes);
      report = { title, lines };
    } catch (error) {
      toast(error instanceof Error ? error.message : String(error), "error");
    }
  }

  async function createResource(kind: "grid" | "schedule" | "social" | "sequence" | "teaching") {
    const prompts = {
      grid: "Crea una tabella di comunicazione con 20 parole utili per la scuola primaria.",
      schedule: "Crea un'agenda visiva della giornata scolastica con le attività principali.",
      social: "Crea una storia sociale in quattro passaggi per prepararsi ad andare a scuola.",
      sequence: "Crea una sequenza visiva per preparare lo zaino prima di uscire.",
      teaching: "Crea un materiale didattico con una breve attività e i pittogrammi necessari.",
    };
    const template =
      kind === "grid" ? "communication_grid" : kind === "schedule" ? "visual_schedule" : null;
    const plan = await generateResource(prompts[kind], template);
    if (plan) toast("Materiale creato: controlla l'anteprima prima di stampare.", "info");
  }

  function fileCommands(): RibbonCommand[] {
    return [
      cmd("new", "Nuovo", "file-plus", () => newDocument(), { shortcut: "Ctrl+N" }),
      cmd("open", "Apri", "folder-open", () => void openDocument(), { shortcut: "Ctrl+O" }),
      cmd("save", "Salva", "save", () => void saveDocument(false), { shortcut: "Ctrl+S" }),
      cmd("save-as", "Salva con nome", "save-all", () => void saveDocument(true)),
      cmd("import", "Importa", "import", () => void importDocument()),
      cmd("exit-creator", "Esci dal Creator", "arrow-right", () => void exitCreator(), {
        enabled: Boolean(activeProfile && creatorUnlocked),
      }),
    ];
  }

  function ribbonTabs(): RibbonTab[] {
    const disabled = (reason: string) => ({ enabled: false, tooltip: reason });
    const unavailable = (reason: string) => ({ tooltip: reason });
    const hasToken = Boolean(selected);
    const token = selected?.token;

    const tabs: RibbonTab[] = [
      {
        id: "home",
        label: "Home",
        groups: [
          group("clipboard", "Appunti", [
            cmd("cut", "Taglia", "scissors", () => void copySelection(true), { shortcut: "Ctrl+X" }),
            cmd("copy", "Copia", "copy", () => void copySelection(false), { shortcut: "Ctrl+C" }),
            cmd("paste", "Incolla", "clipboard-paste", () => void pasteClipboard(), { shortcut: "Ctrl+V" }),
            cmd("paste-plain", "Incolla solo testo", "clipboard-type", () => void pasteClipboard()),
            cmd("copy-format", "Copia formattazione", "paintbrush", () => editor?.copyFormatting()),
            cmd("paste-format", "Incolla formattazione", "paintbrush", () => editor?.applyCopiedFormatting()),
          ], 3),
          group("font", "Carattere", [
            select(
              "font-family",
              "Carattere",
              settings.font_family,
              [
                { value: "", label: "Sistema" },
                { value: "Arial, Helvetica, sans-serif", label: "Arial" },
                { value: "Verdana, Geneva, sans-serif", label: "Verdana" },
                { value: "Calibri, Carlito, sans-serif", label: "Calibri" },
                {
                  value: "OpenDyslexic, Atkinson Hyperlegible, sans-serif",
                  label: "OpenDyslexic",
                },
              ],
              (value) => patchSettings({ font_family: value }),
              { width: "130px" }
            ),
            select(
              "font-size",
              "Dimensione",
              formatState.fontSize ?? "",
              [
                { value: "", label: "Predefinita" },
                { value: "12px", label: "12" },
                { value: "14px", label: "14" },
                { value: "16px", label: "16" },
                { value: "20px", label: "20" },
                { value: "28px", label: "28" },
              ],
              (value) => editor?.applyTextStyle(null, value || null),
              { width: "92px" }
            ),
            cmd("font-grow", "Aumenta dimensione", "plus", () => editor?.adjustFontSize(1)),
            cmd("font-shrink", "Riduci dimensione", "minus", () => editor?.adjustFontSize(-1)),
            toggle("bold", "Grassetto", "bold", formatState.bold, () => editor?.toggleBold(), {
              shortcut: "Ctrl+B",
            }),
            toggle("italic", "Corsivo", "italic", formatState.italic, () => editor?.toggleItalic(), {
              shortcut: "Ctrl+I",
            }),
            toggle("underline", "Sottolineato", "underline", formatState.underline, () => editor?.toggleUnderline(), {
              shortcut: "Ctrl+U",
            }),
            toggle("strike", "Barrato", "strikethrough", formatState.strike, () => editor?.toggleStrike()),
            color("text-color", "Colore testo", formatState.color ?? "#1f2933", (value) =>
              editor?.applyTextStyle(value, null)
            ),
            toggle("highlight", "Evidenziatore", "highlighter", formatState.highlight, () =>
              editor?.toggleHighlight()
            ),
            toggle("uppercase", "TUTTO MAIUSCOLE", "case-upper", settings.uppercase, () =>
              patchSettings({ uppercase: !settings.uppercase })
            ),
            cmd("clear-format", "Cancella formattazione", "eraser", () => editor?.clearFormatting()),
          ], 6),
          group("paragraph", "Paragrafo", [
            toggle("align-left", "Allinea a sinistra", "align-left", formatState.align === "left", () =>
              editor?.applyAlignment("left")
            ),
            toggle("align-center", "Centra", "align-center", formatState.align === "center", () =>
              editor?.applyAlignment("center")
            ),
            toggle("align-right", "Allinea a destra", "align-right", formatState.align === "right", () =>
              editor?.applyAlignment("right")
            ),
            toggle("align-justify", "Giustifica", "align-justify", formatState.align === "justify", () =>
              editor?.applyAlignment("justify")
            ),
            toggle("bullet-list", "Elenco puntato", "list", formatState.inList, () => editor?.toggleList("bullet")),
            toggle("ordered-list", "Elenco numerato", "list-ordered", formatState.inList, () =>
              editor?.toggleList("ordered")
            ),
            cmd("indent-more", "Aumenta rientro", "indent-increase", () =>
              editor?.applyParagraphAttr({ indent: Math.min(6, formatState.indent + 1) })
            ),
            cmd("indent-less", "Riduci rientro", "indent-decrease", () =>
              editor?.applyParagraphAttr({ indent: Math.max(0, formatState.indent - 1) })
            ),
            select(
              "line-height",
              "Interlinea",
              formatState.lineHeight ?? "",
              [
                { value: "", label: "Predefinita" },
                { value: "1.2", label: "1,2" },
                { value: "1.5", label: "1,5" },
                { value: "2", label: "2,0" },
              ],
              (value) => editor?.applyParagraphAttr({ line_height: value || null }),
              { width: "100px" }
            ),
            select(
              "space-after",
              "Spaziatura",
              "",
              [
                { value: "", label: "Predefinita" },
                { value: "0", label: "Nessuna" },
                { value: "6px", label: "6 pt" },
                { value: "12px", label: "12 pt" },
              ],
              (value) => editor?.applyParagraphAttr({ space_after: value || null }),
              { width: "100px" }
            ),
            toggle("paragraph-border", "Bordi paragrafo", "square", formatState.paragraphBorder, () =>
              editor?.applyParagraphAttr({ border: !formatState.paragraphBorder })
            ),
          ], 4),
          group("styles", "Stili", [
            select(
              "style-select",
              "Stile",
              formatState.heading ? `h${formatState.heading}` : "normal",
              [
                { value: "normal", label: "Normale" },
                { value: "h1", label: "Titolo 1" },
                { value: "h2", label: "Titolo 2" },
                { value: "h3", label: "Titolo 3" },
                { value: "caa", label: "Testo CAA" },
                { value: "readable", label: "Alta leggibilità" },
              ],
              (value) => {
                if (value === "normal") editor?.applyHeading(null);
                else if (value === "caa") patchSettings({ mode: "sopra", intensity: "media", auto_caa: true });
                else if (value === "readable")
                  patchSettings({
                    font_family: "OpenDyslexic, Atkinson Hyperlegible, sans-serif",
                    font_size: 18,
                  });
                else editor?.applyHeading(Number(value.slice(1)));
              },
              { width: "150px" }
            ),
            cmd("style-normal", "Normale", "pilcrow", () => editor?.applyHeading(null)),
            cmd("style-title", "Titolo", "heading1", () => editor?.applyHeading(1)),
            cmd("style-subtitle", "Sottotitolo", "heading2", () => editor?.applyHeading(2)),
            cmd("style-h1", "Titolo 1", "heading1", () => editor?.applyHeading(1)),
            cmd("style-h2", "Titolo 2", "heading2", () => editor?.applyHeading(2)),
            cmd("style-caa", "Testo CAA", "book-open", () =>
              patchSettings({ mode: "sopra", intensity: "media", auto_caa: true })
            ),
            cmd("style-readable", "Testo alta leggibilità", "accessibility", () =>
              patchSettings({
                font_family: "OpenDyslexic, Atkinson Hyperlegible, sans-serif",
                font_size: 18,
              })
            ),
          ], 1),
          group("edit", "Modifica", [
            cmd("find", "Trova", "search", openFind, { shortcut: "Ctrl+F" }),
            cmd("replace", "Sostituisci", "replace", openFind, { shortcut: "Ctrl+H" }),
            cmd("select-all", "Seleziona tutto", "check-check", () => editor?.selectAllText(), {
              shortcut: "Ctrl+A",
            }),
          ], 3),
        ],
      },
      {
        id: "insert",
        label: "Inserisci",
        groups: [
          group("pages", "Pagine e contenuti", [
            cmd("page-break", "Interruzione di pagina", "file-text", () => editor?.insertPageBreak()),
            cmd("insert-date", "Data", "calendar", () =>
              editor?.insertTextAtSelection(new Intl.DateTimeFormat("it-IT").format(new Date()))
            ),
            cmd("insert-time", "Ora", "clock", () =>
              editor?.insertTextAtSelection(
                new Intl.DateTimeFormat("it-IT", { hour: "2-digit", minute: "2-digit" }).format(new Date())
              )
            ),
          ]),
          group("elements", "Elementi", [
            cmd("insert-table", "Tabella", "table", () => editor?.insertTable(3, 3)),
            cmd("insert-symbol", "Pittogramma", "image", () => openSearchForSelected()),
          ]),
          group("caa-insert", "CAA", [
            cmd("symbol-word", "Pittogramma sulla parola", "image-plus", () => openSearchForSelected()),
            cmd("symbol-selection", "Pittogramma sulla selezione", "scan-search", () => openSearchForSelected(), {
              ...disabled("La selezione multipla non è ancora supportata: seleziona una parola."),
            }),
            cmd("visual-sequence", "Sequenza pittografica", "list-tree", () => void createResource("sequence")),
            cmd("communication-table", "Tabella comunicativa", "table-properties", () =>
              void createResource("grid")
            ),
            cmd("visual-strip", "Striscia visiva", "layout-template", () => void createResource("schedule")),
          ]),
          group("resources", "Risorse", [
            cmd("library-arasaac", "Libreria ARASAAC", "library", () => openPanelTab("cerca")),
            cmd("saved-templates", "Modelli salvati", "archive", () => (view = "home")),
          ]),
        ],
      },
      {
        id: "layout",
        label: "Layout",
        groups: [
          group("page", "Pagina", [
            select(
              "page-format",
              "Formato",
              settings.page,
              [
                { value: "a4", label: "A4" },
                { value: "a4-orizzontale", label: "A4 orizzontale" },
                { value: "a3", label: "A3" },
                { value: "a5", label: "A5" },
              ],
              (value) => patchSettings({ page: value as never }),
              { width: "130px" }
            ),
            select(
              "page-margin",
              "Margini",
              settings.page_margin,
              [
                { value: "stretto", label: "Stretti" },
                { value: "normale", label: "Normali" },
                { value: "ampio", label: "Ampi" },
              ],
              (value) => patchSettings({ page_margin: value as never }),
              { width: "110px" }
            ),
            color("page-color", "Colore pagina", settings.page_color, (value) =>
              patchSettings({ page_color: value })
            ),
            cmd("ruler", "Righello", "layout", () => (showEditorRuler = !showEditorRuler), {
              active: showEditorRuler,
            }),
          ]),
          group("pictograms-layout", "Pittogrammi", [
            select(
              "symbol-size",
              "Dimensione",
              String(settings.symbol_size),
              ["40", "48", "56", "64", "72", "80"].map((value) => ({ value, label: value })),
              (value) => patchSettings({ symbol_size: Number(value) }),
              { width: "80px" }
            ),
            select(
              "symbol-gap",
              "Spaziatura",
              String(settings.symbol_gap),
              ["2", "4", "6", "8", "10", "12"].map((value) => ({ value, label: value })),
              (value) => patchSettings({ symbol_gap: Number(value) }),
              { width: "80px" }
            ),
            select(
              "pictogram-align",
              "Allineamento",
              settings.pictogram_align,
              [
                { value: "left", label: "Sinistra" },
                { value: "center", label: "Centro" },
                { value: "right", label: "Destra" },
              ],
              (value) => patchSettings({ pictogram_align: value as never }),
              { width: "105px" }
            ),
          ]),
          group("caa-borders", "Bordi CAA", [
            select(
              "border-style",
              "Bordo",
              settings.border_style,
              [
                { value: "nessuno", label: "Nessuno" },
                { value: "immagine", label: "Solo immagine" },
                { value: "immagine_testo", label: "Immagine e testo" },
                { value: "separato", label: "Separati" },
                { value: "divisore", label: "Con divisore" },
              ],
              (value) => patchSettings({ border_style: value as never }),
              { width: "140px" }
            ),
            color("border-color", "Colore", settings.chip_border_color || "#c9d3e2", (value) =>
              patchSettings({ chip_border_color: value })
            ),
            select(
              "border-width",
              "Spessore",
              String(settings.chip_border_width),
              ["0", "1", "2", "3"].map((value) => ({ value, label: value })),
              (value) => patchSettings({ chip_border_width: Number(value) }),
              { width: "76px" }
            ),
            select(
              "border-kind",
              "Stile",
              settings.chip_border_style,
              [
                { value: "solid", label: "Continuo" },
                { value: "dashed", label: "Tratteggio" },
                { value: "dotted", label: "Punti" },
              ],
              (value) => patchSettings({ chip_border_style: value as never }),
              { width: "110px" }
            ),
            select(
              "border-radius",
              "Arrotondamento",
              String(settings.chip_radius),
              ["0", "4", "8", "12", "16"].map((value) => ({ value, label: value })),
              (value) => patchSettings({ chip_radius: Number(value) }),
              { width: "90px" }
            ),
            select(
              "border-padding",
              "Padding",
              String(settings.chip_padding),
              ["0", "2", "4", "6", "8"].map((value) => ({ value, label: value })),
              (value) => patchSettings({ chip_padding: Number(value) }),
              { width: "82px" }
            ),
            color("chip-background", "Sfondo", settings.chip_background || "#ffffff", (value) =>
              patchSettings({ chip_background: value })
            ),
          ]),
          group("presets", "Layout predefiniti", [
            cmd("preset-essential", "Essenziale", "layout-template", () => applyPreset("essenziale")),
            cmd("preset-classic", "Classico CAA", "layout-template", () => applyPreset("classico")),
            cmd(
              "preset-contrast",
              settings.theme === "contrasto" ? "Disattiva alto contrasto" : "Alto contrasto",
              "contrast",
              () => {
                if (settings.theme === "contrasto") {
                  patchSettings({
                    theme: "chiaro",
                    chip_border_color: "",
                    chip_border_width: 1,
                    chip_background: "",
                  });
                } else {
                  applyPreset("contrasto");
                }
              },
              { active: settings.theme === "contrasto" }
            ),
            cmd("preset-large", "Grandi pittogrammi", "maximize", () => applyPreset("grandi")),
            cmd("preset-cards", "Schede comunicative", "layout-grid", () => applyPreset("schede")),
            cmd("preset-illustrated", "Testo illustrato", "book-open", () => applyPreset("illustrato")),
          ]),
        ],
      },
      {
        id: "pictograms",
        label: "Pittogrammi",
        groups: [
          group("import", "Importa", [
            cmd("import-images", "Importa immagini", "image-plus", () => void importPictogramImages()),
            cmd("import-folder", "Importa cartella", "folder-open", () => void importPictogramFolder()),
            cmd("open-collections", "Collezioni", "library", () => openSidebar("collezioni")),
          ]),
          group("symbolization", "Simbolizzazione", [
            toggle("auto-caa", "CAA automatica", "sparkles", settings.auto_caa, () =>
              patchSettings({ auto_caa: !settings.auto_caa })
            ),
            toggle("intensity-min", "Essenziale", "minus", settings.intensity === "minima", () =>
              patchSettings({ intensity: "minima" })
            ),
            toggle("intensity-max", "Estesa", "plus", settings.intensity === "completa", () =>
              patchSettings({ intensity: "completa" })
            ),
            toggle("mode-symbols", "Solo pittogrammi", "image", settings.mode === "solo_simboli", () =>
              patchSettings({ mode: "solo_simboli" })
            ),
            toggle("mode-text", "Pittogrammi e testo", "book-open", settings.mode === "sopra", () =>
              patchSettings({ mode: "sopra" })
            ),
            cmd("refresh-sentence", "Rivaluta frase", "refresh", () => {
              if (selected) editor?.refreshParagraph(selected.pid);
              else editor?.refreshAll();
            }),
            cmd("refresh-document", "Rivaluta documento", "refresh", () => editor?.refreshAll()),
          ]),
          group("search", "Ricerca", [
            cmd("search-symbol", "Cerca pittogramma", "search", () => openSearchForSelected()),
            cmd("search-affinity", "Cerca per affinità", "network", () => openSearchForSelected()),
            cmd("search-ai", "Cerca con IA", "sparkles", () => openSearchForSelected(true)),
            cmd("search-manual", "Inserisci manualmente", "scan-search", () => openSearchForSelected()),
            cmd("show-alternatives", "Mostra alternative", "list", () => openPanelTab("alternative"), {
              enabled: hasToken,
              ...unavailable("Seleziona una parola nel documento."),
            }),
          ]),
          group("manage", "Gestione", [
            cmd("change-symbol", "Cambia pittogramma", "image-plus", () => openSearchForSelected(), {
              enabled: hasToken,
              ...unavailable("Seleziona una parola nel documento."),
            }),
            cmd("merge-unit", "Unisci in un'unica unità CAA", "combine", () => {
              if (editor?.mergeSelection()) {
                toast("Parole unite in una sola unità CAA.", "info");
                syncSelectedSoon();
              } else {
                toast("Seleziona almeno due parole consecutive nello stesso paragrafo.", "warn");
              }
            }),
            cmd("split-unit", "Separa unità CAA", "split", () => {
              if (selected && editor?.splitUnit(selected.pid, selected.token.index)) {
                toast("Unità separata: ogni parola ha di nuovo il suo pittogramma.", "info");
                syncSelectedSoon();
              } else {
                toast("Seleziona un'espressione composta da separare.", "warn");
              }
            }, {
              enabled: Boolean(selected && selected.token.span > 1),
              ...unavailable("Seleziona un'espressione composta."),
            }),
            cmd("lock", "Blocca", "lock", () => void setUnitState({ locked: true }), {
              enabled: hasToken,
              ...unavailable("Seleziona una parola nel documento."),
            }),
            cmd("unlock", "Sblocca", "lock-open", () => void setUnitState({ locked: false }), {
              enabled: hasToken,
              ...unavailable("Seleziona una parola nel documento."),
            }),
            cmd("hide", "Nascondi", "eye-off", () => void setUnitState({ hidden: true }), {
              enabled: hasToken,
              ...unavailable("Seleziona una parola nel documento."),
            }),
            cmd("show", "Mostra", "eye", () => void setUnitState({ hidden: false }), {
              enabled: hasToken,
              ...unavailable("Seleziona una parola nel documento."),
            }),
            cmd("forget", "Elimina associazione", "eraser", () => void resetUnit(), {
              enabled: hasToken,
              ...unavailable("Seleziona una parola nel documento."),
            }),
            cmd("remember", "Ricorda associazione", "badge-check", rememberSelected, {
              enabled: Boolean(token?.symbol_id),
              ...unavailable("Serve un pittogramma già associato."),
            }),
            cmd("undo-association", "Annulla associazione", "undo", () => void undoAssociation(), {
              enabled: associationUndo.length > 0,
            }),
            cmd("redo-association", "Ripeti associazione", "redo", () => void redoAssociation(), {
              enabled: associationRedo.length > 0,
            }),
          ]),
          group("modifiers", "Modificatori", [
            toggle("mod-plural", "Plurale", "type", Boolean(token?.modifiers?.plural), () =>
              toggleModifier("plural")
            ),
            toggle("mod-negation", "Negazione", "minus", Boolean(token?.modifiers?.negation), () =>
              toggleModifier("negation")
            ),
            toggle("mod-past", "Passato", "clock", Boolean(token?.modifiers?.past), () =>
              toggleModifier("past")
            ),
            toggle("mod-future", "Futuro", "calendar", Boolean(token?.modifiers?.future), () =>
              toggleModifier("future")
            ),
            cmd("mod-auto", "Automatici", "refresh", () => {
              if (!selected) return;
              void setUnitState({ modifiers: { ...noModifiers, manual: false } });
            }),
            cmd("mod-manual", "Manuali", "sliders", () => {
              if (!selected) return;
              void setUnitState({
                modifiers: { ...(selected.token.modifiers ?? noModifiers), manual: true },
              });
            }),
          ]),
          group("grammar", "Parole grammaticali", [
            toggle("grammar-articles", "Articoli", "type", settings.symbolize_articles, () =>
              patchSettings({ symbolize_articles: !settings.symbolize_articles })
            ),
            toggle("grammar-prepositions", "Preposizioni", "link", settings.symbolize_prepositions, () =>
              patchSettings({ symbolize_prepositions: !settings.symbolize_prepositions })
            ),
            toggle("grammar-conjunctions", "Congiunzioni", "combine", settings.symbolize_conjunctions, () =>
              patchSettings({ symbolize_conjunctions: !settings.symbolize_conjunctions })
            ),
            { ...cmd("grammar-pronouns", "Pronomi", "message-square", () => {}), ...disabled("Categoria non ancora implementata.") },
            { ...cmd("grammar-adverbs", "Avverbi", "type", () => {}), ...disabled("Categoria non ancora implementata.") },
          ]),
          group("packs", "Repertori", [
            select(
              "pack",
              "Repertorio",
              settings.pack_id ?? "",
              [{ value: "", label: "Tutti" }, ...packs.map((pack) => ({ value: pack.id, label: pack.name }))],
              (value) => patchSettings({ pack_id: value || null }),
              { width: "150px" }
            ),
            cmd("manage-packs", "Gestione librerie", "settings", () => openSettingsPanel()),
          ]),
        ],
      },
      {
        id: "review",
        label: "Revisione",
        groups: [
          group("review-text", "Testo", [
            cmd("review-find", "Trova e sostituisci", "replace", openFind),
            cmd("word-count", "Conteggio parole", "calculator", showWordCount),
          ]),
          group("review-semantic", "Semantica", [
            cmd("analyze-sentence", "Analizza frase", "scan-search", () => {
              if (selected) editor?.refreshParagraph(selected.pid);
              else editor?.refreshAll();
            }),
            cmd("analyze-document", "Analizza documento", "refresh", () => editor?.refreshAll()),
            cmd("show-units", "Mostra unità semantiche", "list-tree", () => openPanelTab("alternative")),
            cmd("show-concepts", "Mostra concetti", "book-open", () => showReviewReport("semantic")),
            cmd("show-ambiguous", "Evidenzia ambiguità", "triangle-alert", () => showReviewReport("semantic")),
            cmd("show-approx", "Mostra approssimativi", "circle-help", () => showReviewReport("coverage")),
          ]),
          group("review-coverage", "Copertura CAA", [
            cmd("coverage-missing", "Parole senza simbolo", "search", () => showReviewReport("coverage")),
            cmd("coverage-manual", "Associazioni manuali", "badge-check", () => showReviewReport("coverage")),
            cmd("coverage-locked", "Simboli bloccati", "lock", () => showReviewReport("coverage")),
            cmd("coverage-modifiers", "Verifica modificatori", "sliders", () => showReviewReport("modifiers")),
          ]),
          group("review-a11y", "Accessibilità", [
            cmd("check-symbol-size", "Verifica dimensioni", "maximize", () =>
              report = {
                title: "Dimensioni dei pittogrammi",
                lines: [
                  `Dimensione attuale: ${settings.symbol_size} px`,
                  `Spaziatura: ${settings.symbol_gap} px`,
                  settings.touch_mode ? "Modalità touch attiva." : "Modalità touch disattivata.",
                ],
              }
            ),
            cmd("accessible-preview", "Anteprima accessibile", "printer", printDocument),
          ]),
        ],
      },
      {
        id: "ai",
        label: "Strumenti IA",
        groups: [
          group("ai-understanding", "Comprensione", [
            cmd("ai-clarity", "Analizza chiarezza", "brain", () =>
              void runAiReport("analyse_text_clarity", "Chiarezza del testo")
            ),
            cmd("ai-affinity", "Cerca concetti affini", "network", () => openSearchForSelected(true)),
            cmd("ai-refine", "Rivaluta con il modello", "refresh", () => editor?.refreshAll(), {
              enabled: neuralConfigured,
              ...unavailable("Nessun modello neurale configurato."),
            }),
          ]),
          group("ai-writing", "Scrittura", [
            cmd("ai-document", "Modifica con IA", "wand", () => (assistantOpen = true)),
            cmd("ai-simplify", "Semplifica testo", "wand", () =>
              void runAiReport(
                "simplify_sentence",
                "Semplificazione",
                editor?.getSelectedText().trim() || editor?.getPlainText()
              )
            ),
            cmd("ai-alternatives", "Suggerisci alternative", "sparkles", () => openSearchForSelected(true)),
          ]),
          group("ai-symbols", "Pittogrammi", [
            cmd("ai-missing", "Trova simboli mancanti", "search", () => showReviewReport("coverage")),
            cmd("ai-equivalents", "Cerca equivalenti", "network", () => openSearchForSelected()),
            cmd("ai-recheck", "Rivaluta frase", "refresh", () => {
              if (selected) editor?.refreshParagraph(selected.pid);
              else editor?.refreshAll();
            }),
          ]),
          group("ai-create", "Creazione", [
            cmd("create-grid", "Tabella comunicativa", "table", () => void createResource("grid")),
            cmd("create-schedule", "Agenda", "calendar", () => void createResource("schedule")),
            cmd("create-social", "Storia sociale", "book-open", () => void createResource("social")),
            cmd("create-sequence", "Sequenza visiva", "list-tree", () => void createResource("sequence")),
            cmd("create-teaching", "Materiale didattico", "layout-grid", () => void createResource("teaching")),
          ]),
          group("ai-model", "Modello neurale", [
            cmd("open-cli", "Console CLI", "terminal", () => (cliOpen = true)),
            cmd("manage-profiles", "Gestione utenti", "accessibility", () => (profileManagerOpen = true)),
            cmd("exit-creator", "Torna all'Utilizzatore", "arrow-right", () => void exitCreator()),
            cmd("ai-status", "Stato del modello", "badge-check", () =>
              report = {
                title: "Modello neurale",
                lines: [
                  aiStatus
                    ? `Stato: ${aiStatus.enabled ? "attivo" : "disattivato"}`
                    : "Stato non disponibile.",
                  aiStatus?.active_model
                    ? `Modello attivo: ${aiStatus.active_model}`
                    : "Nessun modello attivo.",
                  aiStatus?.note ?? "Nessuna nota dal servizio.",
                  `Ultimo errore: ${aiStatus?.last_error ?? "nessuno"}`,
                ],
              }
            ),
            cmd("ai-manage", "Gestione modelli", "settings", () => openSettingsPanel()),
            cmd("ai-settings", "Impostazioni IA", "sliders", () => openSettingsPanel()),
          ]),
        ],
      },
      {
        id: "view",
        label: "Visualizza",
        groups: [
          group("zoom", "Zoom", [
            cmd("zoom-in", "Zoom avanti", "zoom-in", () => setZoom(zoom + 0.1)),
            cmd("zoom-out", "Zoom indietro", "zoom-out", () => setZoom(zoom - 0.1)),
            cmd("zoom-100", "100%", "maximize", () => setZoom(1)),
            cmd("zoom-fit", "Adatta larghezza", "minimize", fitWidth),
          ]),
          group("panels", "Pannelli", [
            toggle("panel-toggle", "Pannello laterale", "panel-right", !panelClosed, () =>
              (panelClosed ? openSidebar() : closeSidebar())
            ),
            cmd("panel-settings", "Impostazioni", "settings", () => openSettingsPanel()),
            select(
              "theme",
              "Tema",
              settings.theme,
              [
                { value: "chiaro", label: "Chiaro" },
                { value: "scuro", label: "Scuro" },
                { value: "contrasto", label: "Alto contrasto" },
              ],
              (value) => patchSettings({ theme: value as never }),
              { width: "130px" }
            ),
          ]),
          group("workspace", "Area di lavoro", [
            select(
              "workspace-preset",
              "Preset",
              $workspace.preset,
              [
                { value: "", label: "Personalizzato" },
                ...(() => {
                  void $customPresets;
                  return presetNames().map((name) => ({ value: name, label: name }));
                })(),
              ],
              (value) => applyWorkspacePreset(value),
              { width: "170px" }
            ),
            cmd("save-workspace", "Salva preset", "save", () => (presetDialogOpen = true)),
            cmd("reset-workspace", "Reimposta area", "refresh", () => {
              resetWorkspace();
              toast("Area di lavoro ripristinata.", "info");
            }),
            toggle("focus-mode", "Concentrazione", "eye", $workspace.focusMode, toggleFocusMode, {
              shortcut: "Ctrl+Shift+F",
            }),
            toggle(
              "topbar-minimized",
              "Minimizza barra superiore",
              "chevron-up",
              $workspace.topbarMinimized,
              () => withStageScroll(() => updateWorkspace({ topbarMinimized: !$workspace.topbarMinimized })),
              { shortcut: "Ctrl+Shift+F1" }
            ),
          ]),
          group("modes", "Modalità", [
            toggle("touch-mode", "Modalità touch", "touchpad", settings.touch_mode, () =>
              patchSettings({ touch_mode: !settings.touch_mode })
            ),
            toggle("fullscreen", "Schermo intero", "fullscreen", fullscreen, () =>
              void toggleFullscreen()
            ),
          ]),
          group("preview", "Anteprima", [
            cmd("print-preview", "Anteprima di stampa / PDF", "printer", printDocument),
            cmd("caa-preview", "Anteprima CAA", "eye", () =>
              patchSettings({ mode: "sopra", border_style: "immagine_testo" })
            ),
            cmd("toggle-borders", "Mostra/nascondi bordi", "frame", () =>
              patchSettings({
                border_style: settings.border_style === "nessuno" ? "immagine_testo" : "nessuno",
              })
            ),
          ]),
        ],
      },
    ];

    if (creatorUnlocked) {
      tabs.push({
        id: "users",
        label: "Utenti",
        groups: [
          group("users-admin", "Profili e accessi", [
            cmd("manage-profiles", "Gestione utenti", "accessibility", () => (profileManagerOpen = true)),
            cmd("exit-creator", "Torna all'Utilizzatore", "arrow-right", () => void exitCreator()),
            cmd("manage-pin", "PIN Area Creator", "lock", () => (profileManagerOpen = true)),
          ]),
        ],
      });
    }

    if (selected) {
      tabs.push({
        id: "pictogram",
        label: "Pittogramma",
        contextual: true,
        groups: [
          group("pictogram-format", "Formato pittogramma", [
            cmd("pictogram-change", "Cambia immagine", "image-plus", () => openSearchForSelected()),
            cmd("pictogram-size", "Dimensioni", "maximize", () => (ribbonTab = "layout")),
            cmd("pictogram-border", "Bordi", "frame", () => (ribbonTab = "layout")),
            cmd("pictogram-align", "Allineamento", "align-center", () => (ribbonTab = "layout")),
            cmd("pictogram-lock", selected.token.locked ? "Sblocca" : "Blocca", "lock", () =>
              void setUnitState({ locked: !selected!.token.locked })
            ),
            cmd("pictogram-hide", selected.token.hidden ? "Mostra" : "Nascondi", "eye-off", () =>
              void setUnitState({ hidden: !selected!.token.hidden })
            ),
            cmd("pictogram-text", "Testo associato", "type", () => editor?.focus()),
          ]),
          group("pictogram-modifiers", "Modificatori", [
            toggle("pictogram-plural", "Plurale", "type", Boolean(selected.token.modifiers?.plural), () =>
              toggleModifier("plural")
            ),
            toggle("pictogram-negation", "Negazione", "minus", Boolean(selected.token.modifiers?.negation), () =>
              toggleModifier("negation")
            ),
            toggle("pictogram-past", "Passato", "clock", Boolean(selected.token.modifiers?.past), () =>
              toggleModifier("past")
            ),
            toggle("pictogram-future", "Futuro", "calendar", Boolean(selected.token.modifiers?.future), () =>
              toggleModifier("future")
            ),
          ]),
        ],
      });
    }

    if (formatState.inTable) {
      tabs.push({
        id: "table",
        label: "Tabella",
        contextual: true,
        groups: [
          group("table-structure", "Struttura tabella", [
            cmd("table-row", "Inserisci riga", "rows2", () => editor?.tableAction("addRowAfter")),
            cmd("table-column", "Inserisci colonna", "columns-2", () => editor?.tableAction("addColumnAfter")),
            cmd("table-delete-row", "Elimina riga", "minus", () => editor?.tableAction("deleteRow")),
            cmd("table-delete-column", "Elimina colonna", "x", () => editor?.tableAction("deleteColumn")),
            cmd("table-merge", "Unisci celle", "combine", () => editor?.tableAction("mergeCells")),
            cmd("table-split", "Dividi celle", "split", () => editor?.tableAction("splitCell")),
          ]),
        ],
      });
    }

    return tabs;
  }
</script>

<svelte:head>
  <title>{view === "editor" ? `${docTitle} · CAA Writer` : "CAA Writer"}</title>
</svelte:head>

{#if userMode && !boardPlayerId && !userEditorOpen && view !== "universal"}
  <UserHome
    profile={activeProfile!}
    boards={userBoards}
    onOpenBoard={(id) => void openBoardUniversal(id)}
    onOpenEditor={() => (userEditorOpen = true)}
    onOpenCreator={() => void openCreator()}
  />
{/if}

{#if userMode && userEditorOpen && activeProfile && !boardPlayerId && view !== "universal"}
  <UserEditor
    profile={activeProfile}
    onBack={() => (userEditorOpen = false)}
    onSpeak={(text) => void speak(text)}
    onToast={toast}
  />
{/if}

{#if boardPlayerId && activeProfile}
  <BoardPlayer boardId={boardPlayerId} profile={activeProfile} onClose={() => (boardPlayerId = null)} />
{/if}

{#if view === "home"}
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
{:else if view === "universal" && universalDoc}
  {#key `${universalDoc.id}:${universalPath ?? ""}`}
  <UniversalEditor
    document={universalDoc}
    path={universalPath}
    readOnly={universalReadOnly || userMode}
    profile={activeProfile}
    onSave={saveUniversal}
    onSavePlayer={saveUniversalPlayer}
    onClose={() => {
      view = "home";
      universalDoc = null;
    }}
    onToast={toast}
    onSpeak={(text) => void speak(text)}
    onExport={exportUniversal}
    canEdit={!userMode}
    onOpen={() => void openUniversalFile(true)}
    onImport={() => void importUniversalFile()}
    onEdit={editUniversalCopy}
    onToggleFullscreen={() => void toggleFullscreen()}
  />
  {/key}
{:else if view === "board"}
  <FreeBoard
    packId={settings.pack_id}
    commercialOnly={settings.commercial_only}
    initial={freeBoardData}
    onSave={saveFreeBoard}
    onToast={toast}
    onExit={() => (view = "home")}
  />
{:else}
<div class="app" class:app--focus={$workspace.focusMode}>
  {#if $workspace.focusMode}
    <div class="focus-zone" role="presentation" onmouseenter={() => (focusBarVisible = true)}>
      {#if focusBarVisible}
        <div class="focus-bar" role="toolbar" aria-label="Comandi modalità concentrazione">
          <span>Concentrazione</span>
          <button class="btn" onclick={() => void saveDocument(false)}>Salva</button>
          <button class="btn" onclick={speakParagraph}>Ascolta</button>
          <button class="btn" onclick={() => (findOpen = true)}>Cerca</button>
          <button class="btn" onclick={() => (view = "home")}>Esci</button>
          <button class="btn btn--solid" onclick={toggleFocusMode}>Esci da concentrazione (Esc)</button>
          <button class="btn btn--ghost" aria-label="Nascondi la barra" onclick={() => (focusBarVisible = false)}>×</button>
        </div>
      {/if}
    </div>
  {/if}
  <Ribbon
    tabs={ribbonTabs()}
    activeTab={ribbonTab}
    onTab={moveRibbonTab}
    fileCommands={fileCommands()}
    quickCommands={quickCommands()}
    quickOptions={quickOptions()}
    quickAccess={$workspace.quickAccess}
    onToggleQuick={toggleQuickAccess}
    quickLabels={$workspace.quickLabels}
    onToggleQuickLabels={toggleQuickLabels}
    onHome={() => (view = "home")}
    docTitle={docTitle}
    docPath={docPath}
    dirty={dirty}
    savedAt={autoSavedAt ?? ""}
    mode={ribbonMode}
    onMode={setRibbonMode}
    pinned={$workspace.ribbonPinned}
    onTogglePin={() => updateWorkspace({ ribbonPinned: !$workspace.ribbonPinned })}
    topbarMinimized={$workspace.topbarMinimized}
    onToggleTopbar={() => withStageScroll(() => updateWorkspace({ topbarMinimized: !$workspace.topbarMinimized }))}
    onSettings={openSettingsPanel}
  />
  {#if findOpen}
    <FindReplace
      onFind={(query, forward) => {
        if (!editor?.findNext(query, forward)) toast("Nessuna occorrenza trovata.", "warn");
      }}
      onReplace={(query, replacement) => {
        if (!editor?.replaceCurrent(query, replacement)) toast("Nessuna occorrenza trovata.", "warn");
      }}
      onReplaceAll={(query, replacement) => {
        const count = editor?.replaceAll(query, replacement) ?? 0;
        toast(count ? `${count} occorrenze sostituite.` : "Nessuna occorrenza trovata.", count ? "info" : "warn");
      }}
      countMatches={(query) => editor?.countMatches(query) ?? 0}
      onClose={() => (findOpen = false)}
    />
  {/if}

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
        class:paper--a5={settings.page === "a5"}
        data-uppercase={settings.uppercase ? "true" : "false"}
        style={`${settings.font_family ? `font-family:${settings.font_family};` : ""}zoom:${zoom}`}
      >
        {#if showEditorRuler}
          <div class="paper-ruler paper-ruler--top" aria-hidden="true"></div>
          <div class="paper-ruler paper-ruler--left" aria-hidden="true"></div>
          <div
            class="paper-margin-line paper-margin-line--top"
            style={`top:${settings.page_margin === "stretto" ? 24 : settings.page_margin === "ampio" ? 56 : 40}px`}
          ></div>
          <div
            class="paper-margin-line paper-margin-line--bottom"
            style={`bottom:${settings.page_margin === "stretto" ? 32 : settings.page_margin === "ampio" ? 72 : 56}px`}
          ></div>
          <div
            class="paper-margin-line paper-margin-line--left"
            style={`left:${settings.page_margin === "stretto" ? 30 : settings.page_margin === "ampio" ? 72 : 48}px`}
          ></div>
          <div
            class="paper-margin-line paper-margin-line--right"
            style={`right:${settings.page_margin === "stretto" ? 30 : settings.page_margin === "ampio" ? 72 : 48}px`}
          ></div>
        {/if}
        <CaaEditor
          bind:this={editor}
          mode={effectiveMode}
          autoEnabled={settings.auto_caa}
          neural={neuralReady}
          onlineAutoInsert={settings.online_auto_insert && settings.online_search !== "off"}
          symbolizeArticles={settings.symbolize_articles}
          symbolizePrepositions={settings.symbolize_prepositions}
          symbolizeConjunctions={settings.symbolize_conjunctions}
          onTokenMenu={(_pid, _token, at) => (unitMenu = at)}
          intensity={settings.intensity}
          documentKey={docKey}
          commercialOnly={settings.commercial_only}
          packId={settings.pack_id}
          selectedUnit={selected ? { pid: selected.pid, index: selected.token.index } : null}
          borderStyle={settings.border_style}
          onSelectToken={(pid, token) => (selected = { pid, token })}
          onError={(message) => toast(message, "error")}
          onAnalyzed={onAnalyzed}
          onFormattingChange={(state) => (formatState = state)}
          onDocChanged={() => (associationPending = false)}
          analysisDebounceMs={settings.analysis_debounce_ms}
          refineDebounceMs={settings.refine_debounce_ms}
          profileId={activeProfile?.id ?? null}
          onAnalysisState={(active) => (analyzing = active)}
          onMetrics={(metrics) => (editorMetrics = metrics)}
        />
      </div>

      {#if analyzing}
        <div class="analysis-pill" role="status" aria-live="polite" title="Il testo resta modificabile">
          <span class="analysis-pill__dot" aria-hidden="true"></span>
          Analisi in corso
        </div>
      {/if}

      {#if speechActive}
        <div class="analysis-pill analysis-pill--speech" role="status">
          <span class="analysis-pill__dot" aria-hidden="true"></span>
          Lettura in corso
          <button class="btn btn--ghost btn--tiny" onclick={() => void stopSpeech()}>Ferma</button>
        </div>
      {/if}

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
        onApplySpec={applyCommunicatorSpec}
        onSave={saveLayoutDocument}
      />
    {/if}

    {#if effectiveSidebarMode === "collapsed"}
      <SidebarRail
        activePanel={$workspace.activeSidebarPanel}
        onOpen={(panel) => openSidebar(panel)}
      />
    {:else}
      {#if effectiveSidebarMode === "overlay"}
        <button class="panel-backdrop" aria-label="Chiudi pannello laterale" onclick={closeSidebar}></button>
      {/if}
      <div
        class="panel-wrap"
        class:panel-wrap--overlay={effectiveSidebarMode === "overlay"}
        style={`width:${$workspace.sidebarWidth}px`}
      >
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
          collections={pictogramCollections}
          onRefreshCollections={refreshPictogramCollections}
          onImportImages={() => void importPictogramImages()}
          onImportFolder={() => void importPictogramFolder()}
          intent={sideIntent}
          overlay={effectiveSidebarMode === "overlay"}
          onCollapse={closeSidebar}
          onToggleDock={toggleSidebarDock}
          associationScope={$workspace.associationScope}
          onAssociationScope={(scope) => updateWorkspace({ associationScope: scope })}
          onSwitchTab={(tab) => updateWorkspace({ activeSidebarPanel: tab })}
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
          onOpenInEditor={openPlanInEditor}
          onOpenUniversalTemplate={(id) => openTemplate(id)}
          onPrintResource={printResource}
          onToast={toast}
          onClearHistory={clearHistory}
          onSetUnitState={(patch) => void setUnitState(patch)}
        />
      </div>
    {/if}
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

{#if setupOpen}
  <ProfileSetup onSave={saveFirstProfile} />
{/if}

{#if profileManagerOpen}
  <ProfileManager
    profiles={profiles}
    creatorPin={creatorPin}
    onClose={() => (profileManagerOpen = false)}
    onChanged={refreshProfileData}
  />
{/if}

{#if cliOpen}
  <CliConsole onClose={() => (cliOpen = false)} />
{/if}

{#if assistantOpen}
  <DocumentAssistant
    onClose={() => (assistantOpen = false)}
    onApply={applyDocumentPlan}
    onUndo={undoDocumentPlan}
    neuralReady={neuralConfigured}
  />
{/if}

{#if importPaths.length > 0}
  <ImportDialog
    paths={importPaths}
    onClose={() => (importPaths = [])}
    onImported={(report, collection) => void onPictogramsImported(report, collection)}
  />
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
    {editorMetrics}
    {diagnostics}
    onRefreshDiagnostics={refreshDiagnostics}
  />
{/if}

{#if presetDialogOpen}
  <PresetDialog
    onSave={(name) => {
      saveCurrentPreset(name);
      toast(`Preset «${name}» salvato.`, "info");
    }}
    onClose={() => (presetDialogOpen = false)}
  />
{/if}

{#if report}
  <ReportDialog title={report.title} lines={report.lines} onClose={() => (report = null)} />
{/if}

<div class="toast-area" aria-live="polite" aria-atomic="false">
  {#each toasts as item (item.id)}
    <div class="toast" class:toast--warn={item.kind === "warn"} class:toast--error={item.kind === "error"}>
      <button class="toast__close" aria-label="Chiudi avviso" onclick={() => (toasts = toasts.filter((t) => t.id !== item.id))}>×</button>
      {item.text}
    </div>
  {/each}
</div>
