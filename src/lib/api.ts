// Ponte tipizzato verso il backend Rust.
//
// In anteprima browser (senza Tauri) le funzioni che richiedono il motore
// nativo lanciano un errore esplicito: nessuna funzione è simulata.
// `demoAnalyze` serve solo a mostrare l'interfaccia e produce token SENZA
// pittogrammi, dichiarandolo nell'interfaccia.

import { invoke } from "@tauri-apps/api/core";
import { noModifiers } from "./types";
import type {
  AiProviderConfig,
  AiProviderStatus,
  AiProviderTest,
  CatalogItem,
  CatalogOverview,
  CatalogPage,
  CatalogQuery,
  DocumentApplyResult,
  LibraryManifest,
  LibraryQualityReport,
  OpenedUniversalDocument,
  RenderedDocument,
  RuntimeDiagnostics,
  UniversalCommand,
  UniversalDocument,
  AiConfig,
  LayoutDocument,
  LayoutEditResult,
  LayoutOperation,
  LayoutSpecification,
  AiStatus,
  BenchmarkResult,
  InstallStatus,
  ModelEntry,
  ToolOutcome,
  Alternative,
  ResolvedSymbol,
  AppInfo,
  AttributionEntry,
  CaaToken,
  DocumentPayload,
  ExportDoc,
  ExportResult,
  LibraryStats,
  LoadedDocument,
  PackInfo,
  ParagraphResult,
  Preference,
  ResourcePlan,
  SearchResultPage,
  SymbolizeOptions,
  TemplateInfo,
  UnitModifiers,
  VoiceInfo,
  WordAffinity,
  ImportPreview,
  ImportItemInput,
  ImportLicense,
  ImportReport,
  CollectionInfo,
  OnlineResult,
  OnlineResponse,
  OnlineProviderInfo,
  OnlineCustomProvider,
  DocumentEditPlan,
  EnglishAlternatives,
  ImageConfig,
  AiChatMessage,
  AiChatReply,
  ImageStatus,
  GeneratedImage,
  CliRunResult,
  Profile,
  ProfileBoard,
  BoardMeta,
  BoardSystem,
} from "./types";

export const isTauri = (): boolean =>
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauri()) {
    throw new Error(
      "Anteprima browser: il motore nativo non è disponibile. Avvia l'app desktop con `npm run tauri dev`."
    );
  }
  return invoke<T>(command, args);
}

export const api = {
  appInfo: () => call<AppInfo>("app_info"),
  printWindow: () => call<void>("print_window"),
  libraryStats: () => call<LibraryStats>("library_stats"),
  listPacks: () => call<PackInfo[]>("list_packs"),

  symbolizeParagraph: (text: string, options: SymbolizeOptions) =>
    call<ParagraphResult>("symbolize_paragraph", { text, options }),
  refineParagraph: (text: string, options: SymbolizeOptions, requestId: string | null = null) =>
    call<ParagraphResult>("refine_paragraph", { text, options, requestId }),
  cancelRefine: (requestId: string) => call<void>("cancel_refine", { requestId }),

  searchSymbols: (
    query: string,
    limit = 40,
    offset = 0,
    packId: string | null = null,
    commercialOnly = false,
    collectionId: string | null = null
  ) =>
    call<SearchResultPage>("search_symbols", {
      query,
      limit,
      offset,
      packId,
      commercialOnly,
      collectionId,
    }),
  scanPictograms: (paths: string[]) => call<ImportPreview>("scan_pictograms", { paths }),
  importPictograms: (entries: ImportItemInput[], collection: string, license: ImportLicense) =>
    call<ImportReport>("import_pictograms", { entries, collection, license }),
  listPictogramCollections: () => call<CollectionInfo[]>("list_pictogram_collections"),
  renamePictogramCollection: (id: string, name: string) =>
    call<void>("rename_pictogram_collection", { id, name }),
  deletePictogramCollection: (id: string) =>
    call<number>("delete_pictogram_collection", { id }),
  deletePictogramSymbols: (ids: string[]) =>
    call<number>("delete_pictogram_symbols", { ids }),
  movePictogramSymbols: (ids: string[], collectionId: string) =>
    call<number>("move_pictogram_symbols", { ids, collectionId }),
  listOnlineProviders: () => call<OnlineProviderInfo[]>("list_online_providers"),
  saveOnlineProvider: (provider: OnlineCustomProvider) =>
    call<OnlineProviderInfo[]>("save_online_provider", { provider }),
  deleteOnlineProvider: (id: string) =>
    call<OnlineProviderInfo[]>("delete_online_provider", { id }),
  searchOnlinePictograms: (terms: string[], forceRefresh = false) =>
    call<OnlineResponse>("search_online_pictograms", { terms, forceRefresh }),
  onlineThumbnail: (url: string) => call<string>("online_thumbnail", { url }),
  ensureOnlineSymbols: (words: string[]) =>
    call<string[]>("ensure_online_symbols", { words }),
  onlineAutoFill: (words: string[]) => call<string[]>("online_autofill", { words }),
  hydrateSymbols: (ids: string[]) =>
    call<Record<string, string>>("hydrate_symbols", { ids }),
  saveOnlinePictogram: (result: OnlineResult, lemma: string, collection: string) =>
    call<Alternative>("save_online_pictogram", { result, lemma, collection }),
  interpretDocumentPrompt: (prompt: string, useNeural = true) =>
    call<DocumentEditPlan>("interpret_document_prompt", { prompt, useNeural }),
  runCli: (method: string, params: unknown) =>
    call<CliRunResult>("run_cli", { method, params }),
  profilesList: () => call<Profile[]>("profiles_list"),
  profileSave: (profile: Profile, creatorPin: string | null = null) =>
    call<void>("profile_save", { profile, creatorPin }),
  profileDelete: (id: string, creatorPin: string | null = null) =>
    call<void>("profile_delete", { id, creatorPin }),
  profileSetActive: (id: string) => call<void>("profile_set_active", { id }),
  profileActive: () => call<Profile | null>("profile_active"),
  profileBoards: (profileId: string) =>
    call<ProfileBoard[]>("profile_boards", { profileId }),
  profileAssignBoard: (
    profileId: string,
    boardId: string,
    enabled: boolean,
    position: number,
    creatorPin: string | null = null
  ) =>
    call<void>("profile_assign_board", {
      profileId,
      boardId,
      enabled,
      position,
      creatorPin,
    }),
  profileRemoveBoard: (profileId: string, boardId: string, creatorPin: string | null = null) =>
    call<void>("profile_remove_board", { profileId, boardId, creatorPin }),
  boardsList: () => call<BoardMeta[]>("boards_list"),
  boardBuild: (id: string) => call<BoardSystem>("board_build", { id }),
  creatorPinStatus: () => call<boolean>("creator_pin_status"),
  creatorSetPin: (pin: string | null, currentPin: string | null = null) =>
    call<void>("creator_set_pin", { pin, currentPin }),
  creatorVerifyPin: (pin: string) => call<boolean>("creator_verify_pin", { pin }),
  conceptAlternatives: (conceptId: string, packId: string | null = null) =>
    call<Alternative[]>("concept_alternatives", { conceptId, packId }),
  wordAffinities: (word: string, packId: string | null = null, commercialOnly = false) =>
    call<WordAffinity>("word_affinities", { word, packId, commercialOnly }),
  suggestSymbols: (text: string, commercialOnly = false) =>
    call<Alternative[]>("suggest_symbols", { text, commercialOnly }),
  commonSymbols: (limit = 48) => call<Alternative[]>("common_symbols", { limit }),

  documentValidate: (document: UniversalDocument) =>
    call<Record<string, unknown>>("document_validate", { document }),
  documentRender: (document: UniversalDocument) =>
    call<RenderedDocument>("document_render", { document }),
  documentApply: (document: UniversalDocument, commands: UniversalCommand[]) =>
    call<DocumentApplyResult>("document_apply", { document, commands }),
  universalLibrary: () => call<LibraryManifest>("universal_library"),
  libraryCatalog: () => call<CatalogOverview>("library_catalog"),
  librarySearch: (query: CatalogQuery) => call<CatalogPage>("library_search", { query }),
  libraryPreview: (id: string, sourceType: string | null) =>
    call<string>("library_preview", { id, sourceType }),
  libraryByIds: (ids: string[]) => call<CatalogItem[]>("library_by_ids", { ids }),
  libraryValidate: () => call<LibraryQualityReport>("library_validate"),
  universalOpen: (id: string, sourceType: string | null = null) =>
    call<OpenedUniversalDocument>("universal_open", { id, sourceType }),
  takePendingOpen: () => call<string | null>("take_pending_open"),
  universalImport: (source: string) =>
    call<OpenedUniversalDocument>("universal_import", { source }),
  documentHtml: (document: UniversalDocument) =>
    call<string>("document_html", { document }),
  printUniversal: (document: UniversalDocument) =>
    call<string>("print_universal", { document }),
  documentExport: (document: UniversalDocument, format: "html" | "svg" | "pdf" | "png" | "docx" | "pptx", path: string) =>
    call<{ path: string; warnings: string[] }>("document_export", { document, format, path }),

  setPreference: (preference: Preference) => call<void>("set_preference", { preference }),
  deletePreference: (scope: string, lemma: string, contextKey: string | null) =>
    call<void>("delete_preference", { scope, lemma, contextKey }),
  listPreferences: () => call<Preference[]>("list_preferences"),
  setDocumentOverride: (
    documentKey: string,
    unitKey: string,
    symbolId: string | null,
    conceptId: string | null
  ) =>
    call<void>("set_document_override", {
      documentKey,
      unitKey,
      symbolId,
      conceptId,
    }),
  clearDocumentOverride: (documentKey: string, unitKey: string) =>
    call<void>("clear_document_override", { documentKey, unitKey }),
  setUnitState: (
    documentKey: string,
    unitKey: string,
    locked: boolean,
    hidden: boolean,
    modifiers: UnitModifiers
  ) =>
    call<void>("set_unit_state", {
      documentKey,
      unitKey,
      locked,
      hidden,
      modifiers,
    }),

  saveDocument: (path: string, payload: DocumentPayload) =>
    call<ExportResult>("save_document", { path, payload }),
  loadDocument: (path: string) => call<LoadedDocument>("load_document", { path }),
  importDocumentText: (path: string) => call<string>("import_document_text", { path }),

  exportHtml: (path: string, doc: ExportDoc) => call<ExportResult>("export_html", { path, doc }),
  exportDocx: (path: string, doc: ExportDoc) => call<ExportResult>("export_docx", { path, doc }),
  exportAttributions: (path: string) =>
    call<ExportResult>("export_attributions", { path }),
  attributions: () => call<AttributionEntry[]>("attributions"),

  listVoices: () => call<VoiceInfo[]>("list_voices"),
  speak: (text: string, voice: string | null, rate: number, language: string | null = null) =>
    call<void>("speak", { text, voice, rate, language }),
  stopSpeaking: () => call<void>("stop_speaking"),
  speechActive: () => call<boolean>("speech_active"),
  runtimeDiagnostics: () => call<RuntimeDiagnostics>("runtime_diagnostics"),

  aiStatus: () => call<AiStatus>("ai_status"),
  aiProviderStatus: () => call<AiProviderStatus>("ai_provider_status"),
  aiProviderSave: (config: AiProviderConfig, apiKey: string | null = null) =>
    call<AiProviderStatus>("ai_provider_save", { config, apiKey }),
  aiProviderTest: () => call<AiProviderTest>("ai_provider_test"),
  aiProviderClearKey: () => call<AiProviderStatus>("ai_provider_clear_key"),
  setAiConfig: (config: AiConfig) => call<AiStatus>("set_ai_config", { config }),
  aiModels: () => call<ModelEntry[]>("ai_models"),
  aiInstallModel: (modelId: string) => call<InstallStatus>("ai_install_model", { modelId }),
  aiInstallStatus: () => call<InstallStatus>("ai_install_status"),
  aiCancelModelInstall: () => call<InstallStatus>("ai_cancel_model_install"),
  aiRemoveModel: (modelId: string) => call<AiStatus>("ai_remove_model", { modelId }),
  aiImportModel: (path: string) => call<AiStatus>("ai_import_model", { path }),
  aiRelatedSymbols: (text: string) => call<Alternative[]>("ai_related_symbols", { text }),
  /// Sinonimi/concetti affini dal modello locale, in background.
  aiAffinitySymbols: (word: string) => call<Alternative[]>("ai_affinity_symbols", { word }),
  aiSelectModel: (modelId: string) => call<AiStatus>("ai_select_model", { modelId }),
  aiBenchmark: () => call<BenchmarkResult>("ai_benchmark"),
  aiRunTool: (tool: string, input: unknown) => call<ToolOutcome>("ai_run_tool", { tool, input }),
  translateSearchTerm: (text: string) => call<string>("translate_search_term", { text }),
  aiChat: (messages: AiChatMessage[]) => call<AiChatReply>("ai_chat", { messages }),
  imageStatus: () => call<ImageStatus>("image_status"),
  imageSetConfig: (config: ImageConfig) => call<ImageStatus>("image_set_config", { config }),
  imageSetKey: (key: string) => call<ImageStatus>("image_set_key", { key }),
  imageClearKey: () => call<ImageStatus>("image_clear_key"),
  imageGenerate: (prompt: string, negative: string | null = null) =>
    call<GeneratedImage>("image_generate", { prompt, negative }),
  imageInstallRuntime: () => call<string>("image_install_runtime"),
  imageDownloadModel: (model: string) => call<string>("image_download_model", { model }),
  resolveMissingSymbols: (words: string[]) =>
    call<ResolvedSymbol[]>("resolve_missing_symbols", { words }),
  englishAlternatives: (word: string, limit = 4) =>
    call<EnglishAlternatives>("english_alternatives", { word, limit }),
  storyCreate: (spec: Record<string, unknown>) => call<OpenedUniversalDocument>("story_create", { spec }),

  listTemplates: () => call<TemplateInfo[]>("list_templates"),
  planResource: (prompt: string, templateId: string | null) =>
    call<ResourcePlan>("plan_resource", { prompt, templateId }),
  universalFromPlan: (plan: ResourcePlan) =>
    call<UniversalDocument>("universal_from_plan", { plan }),
  buildLayout: (spec: LayoutSpecification) =>
    call<LayoutDocument>("build_layout", { spec }),
  editLayout: (
    spec: LayoutSpecification,
    prompt: string | null,
    operations: LayoutOperation[] | null = null
  ) => call<LayoutEditResult>("edit_layout", { spec, prompt, operations }),

  importSvgSymbol: (
    name: string,
    svg: string,
    lemma: string,
    meaning: string | null,
    conceptId: string | null
  ) => call<string>("import_svg_symbol", { name, svg, lemma, meaning, conceptId }),
  /// Importa un'immagine raster (PNG/JPEG/WebP/...) come pittogramma.
  importImageSymbol: (
    name: string,
    dataBase64: string,
    lemma: string,
    meaning: string | null,
    collection: string | null = null
  ) => call<string>("import_image_symbol", { name, dataBase64, lemma, meaning, collection }),
  hideSymbol: (symbolId: string, hide: boolean) =>
    call<void>("hide_symbol", { symbolId, hide }),

  clearHistory: () => call<void>("clear_history"),
  getSetting: (key: string) => call<string | null>("get_setting", { key }),
  setSetting: (key: string, value: string) => call<void>("set_setting", { key, value }),
};

/// Token dimostrativi per l'anteprima browser: nessun pittogramma, solo testo.
/// Serve a valutare layout e accessibilità dell'interfaccia, non il motore.
export function demoAnalyze(text: string, paragraphId: string): ParagraphResult {
  const tokens: CaaToken[] = [];
  const regex = /[\p{L}\p{N}']+|[.,!?;:]/gu;
  let match: RegExpExecArray | null;
  let index = 0;
  while ((match = regex.exec(text)) !== null) {
    const surface = match[0];
    const isPunct = /^[.,!?;:]$/.test(surface);
    tokens.push({
      index: index++,
      surface,
      start: match.index,
      end: match.index + surface.length,
      lemma: isPunct ? null : surface.toLowerCase(),
      pos: isPunct ? "punct" : "noun",
      concept_id: null,
      concept_label: null,
      meaning: null,
      category: null,
      symbol_id: null,
      pack_id: null,
      caption: surface,
      svg: null,
      composition: [],
      strategy: "none",
      confidence: 0,
      approximate: false,
      locked: false,
      negated: false,
      key: !isPunct,
      is_punct: isPunct,
      is_function: false,
      span: 1,
      sentence_index: 0,
      hidden: false,
      modifiers: { ...noModifiers },
      alternatives: [],
    });
  }
  return {
    paragraph_id: paragraphId,
    tokens,
    domains: [],
    symbolized: 0,
    approximate: 0,
    missing: tokens.filter((t) => !t.is_punct).length,
    elapsed_ms: 0,
  };
}
