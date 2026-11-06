// Tipi condivisi con il backend Rust (`src-tauri/src/model.rs`).
// I nomi dei campi restano in snake_case per non duplicare mappature.

export type Intensity = "minima" | "media" | "completa";
export type CaaMode = "testo" | "sopra" | "sotto" | "solo_simboli" | "semplificata";
/// Bordo del pittogramma: nessuno, solo immagine, immagine e testo,
/// bordi separati, bordo con divisore fra immagine e didascalia.
export type BorderStyle =
  | "nessuno"
  | "immagine"
  | "immagine_testo"
  | "separato"
  | "divisore";

/// Modalità di creazione scelte nella schermata iniziale.
export type CreationMode = "word" | "grid" | "schedule" | "aba" | "teacch" | "smart" | "free";

export interface ManualRange {
  start: number;
  end: number;
}

export interface SymbolizeOptions {
  intensity: Intensity;
  mode: CaaMode;
  document_key: string;
  commercial_only: boolean;
  pack_id: string | null;
  paragraph_id: string;
  /// Parole grammaticali simbolizzate solo se la categoria è attiva.
  symbolize_articles: boolean;
  symbolize_prepositions: boolean;
  symbolize_conjunctions: boolean;
  /// Espressioni unite manualmente (offset UTF-16 nel paragrafo).
  manual_units: ManualRange[];
  /// Intervalli in cui non unire automaticamente un'espressione.
  split_ranges: ManualRange[];
}

/// Modificatori grafici del pittogramma (stile ARASAAC).
/// `manual` significa che la configurazione è stata fissata dall'utente: il
/// motore non la ricalcola più finché non viene ripristinata.
export interface UnitModifiers {
  plural: boolean;
  negation: boolean;
  past: boolean;
  future: boolean;
  manual: boolean;
}

export const noModifiers: UnitModifiers = {
  plural: false,
  negation: false,
  past: false,
  future: false,
  manual: false,
};

export interface SymbolPart {
  concept_id: string;
  label: string;
  symbol_id: string | null;
  svg: string | null;
}

export interface Alternative {
  symbol_id: string;
  concept_id: string | null;
  label: string;
  meaning: string;
  pack_id: string;
  pack_name: string;
  format: string;
  svg: string | null;
  approximate: boolean;
  /// Vero se l'alternativa viene dal modello neurale (parole affini o campo
  /// semantico della frase): è un suggerimento, non una scelta automatica.
  neural?: boolean;
  license: string;
  source_url: string | null;
  author: string | null;
}

/// Livello di affinità, dal più forte al più debole.
export type AffinityLevel =
  | "exact"
  | "lemma"
  | "variant"
  | "synonym"
  | "general"
  | "related";

export interface AffinityCandidate {
  symbol: Alternative;
  /// Indice interno 0–100: non è una probabilità calibrata.
  score: number;
  approximate: boolean;
  reason: string;
}

export interface AffinityGroup {
  level: AffinityLevel;
  title: string;
  approximate: boolean;
  candidates: AffinityCandidate[];
}

export interface WordAffinity {
  word: string;
  normalized: string;
  lemma: string | null;
  known: boolean;
  explanation: string;
  groups: AffinityGroup[];
}

export interface CaaToken {
  index: number;
  surface: string;
  start: number;
  end: number;
  lemma: string | null;
  pos: string;
  concept_id: string | null;
  concept_label: string | null;
  meaning: string | null;
  category: string | null;
  symbol_id: string | null;
  pack_id: string | null;
  caption: string;
  /// Etichetta originale del pittogramma (metadato, non riga visibile).
  symbol_label?: string | null;
  svg: string | null;
  composition: SymbolPart[];
  strategy: string;
  confidence: number;
  approximate: boolean;
  locked: boolean;
  negated: boolean;
  key: boolean;
  is_punct: boolean;
  is_function: boolean;
  span: number;
  sentence_index: number;
  /// Pittogramma nascosto: il testo resta, il simbolo non compare nel foglio,
  /// nella stampa e nelle esportazioni visive.
  hidden: boolean;
  modifiers: UnitModifiers;
  alternatives: Alternative[];
}

export interface ParagraphResult {
  paragraph_id: string;
  tokens: CaaToken[];
  domains: string[];
  symbolized: number;
  approximate: number;
  missing: number;
  elapsed_ms: number;
}

export interface SearchResultPage {
  items: Alternative[];
  total: number;
  page: number;
  page_size: number;
}

export interface Preference {
  scope: "global" | "context" | "document" | string;
  lemma: string;
  context_key: string | null;
  symbol_id: string;
  concept_id: string | null;
  locked: boolean;
  created_at: string;
}

export interface PackInfo {
  id: string;
  name: string;
  version: string;
  license: string;
  license_url: string | null;
  source_url: string | null;
  author: string | null;
  commercial_ok: boolean;
  symbol_count: number;
  description: string | null;
}

export interface LibraryStats {
  symbols: number;
  concepts: number;
  lemmas: number;
  packs: number;
  expressions: number;
  packs_detail: PackInfo[];
}

export interface AttributionEntry {
  symbol_id: string;
  label: string;
  pack_id: string;
  pack_name: string;
  author: string | null;
  rights_holder: string | null;
  license: string;
  license_url: string | null;
  source_url: string | null;
  constraints: string | null;
  modifications: string | null;
}

export interface VoiceInfo {
  id: string;
  name: string;
  language: string;
}

export interface AiConfig {
  enabled: boolean;
  binary_path: string;
  model_path: string;
  threads: number;
  context_size: number;
  timeout_ms: number;
  backend: string;
  extra_args: string[];
}

export interface AiStatus {
  enabled: boolean;
  binary_present: boolean;
  model_present: boolean;
  binary_path: string;
  model_path: string;
  backend: string;
  threads: number;
  context_size: number;
  timeout_ms: number;
  last_error: string | null;
  note: string;
  hardware: {
    os: string;
    arch: string;
    cpu_cores: number;
    total_ram_mb: number;
    available_ram_mb: number;
    accelerated: boolean;
    profile: string;
    max_context: number;
    max_threads: number;
    concurrent_requests: number;
    note: string;
  };
  metrics: {
    requests: number;
    completed: number;
    cache_hits: number;
    cancelled: number;
    failures: number;
    total_ms: number;
    last_ms: number;
    executor: string;
  };
  server_running: boolean;
  active_model?: string;
  active_model_bundled?: boolean;
}

export type InstallPhase =
  | "idle"
  | "downloading"
  | "verifying"
  | "ready"
  | "cancelled"
  | "error";

export interface InstallStatus {
  model_id: string;
  phase: InstallPhase;
  downloaded_bytes: number;
  total_bytes: number;
  percent: number;
  message: string;
}

export interface ModelEntry {
  id: string;
  name: string;
  parameters: string;
  quantization: string;
  format: string;
  file_name: string;
  size_bytes: number;
  minimum_ram_mb: number;
  recommended_ram_mb: number;
  context_length: number;
  supported_backends: string[];
  languages: string[];
  capabilities: string[];
  license: string;
  download_url: string | null;
  sha256: string | null;
  verified: boolean;
  bundled: boolean;
  estimated_italian_quality: number | null;
  notes: string;
  installed?: boolean;
  local_path?: string | null;
}

export interface ToolOutcome {
  tool: string;
  result: unknown;
  source: string;
  notes: string[];
}

export interface BenchmarkResult {
  model: string;
  runs: number;
  failures: number;
  average_ms: number;
  tokens_per_second: number;
  note: string;
}

export interface AppInfo {
  version: string;
  schema_version: number;
  lexicon_concepts: number;
  lexicon_lemmas: number;
  data_dir: string;
  resources_dir: string;
  warnings: string[];
  platform: string;
}

export interface DocumentAsset {
  symbol_id: string;
  svg: string | null;
  png_base64: string | null;
}

export interface DocumentPreference {
  unit_key: string;
  symbol_id: string | null;
  concept_id: string | null;
  locked: boolean;
  hidden: boolean;
  modifiers: UnitModifiers;
}

export interface DocumentPayload {
  title: string;
  content: unknown;
  plain_text: string;
  settings: unknown;
  assets: DocumentAsset[];
  preferences: DocumentPreference[];
  layout?: LayoutSpecification | null;
}

export interface LoadedDocument {
  title: string;
  content: unknown;
  plain_text: string;
  settings: unknown;
  assets: DocumentAsset[];
  preferences: DocumentPreference[];
  format_version: number;
  warnings: string[];
  path: string;
  layout?: LayoutSpecification | null;
}

export interface ExportUnit {
  surface: string;
  caption: string;
  symbol_id: string | null;
  svg: string | null;
  png_base64: string | null;
  approximate: boolean;
}

export interface ExportParagraph {
  plain_text: string;
  style: string;
  units: ExportUnit[];
}

export interface ExportDoc {
  title: string;
  paragraphs: ExportParagraph[];
  include_symbols: boolean;
  /// Didascalia del repertorio sotto il pittogramma (spenta per default).
  include_captions: boolean;
  mode: CaaMode;
  attributions: AttributionEntry[];
  metadata: unknown;
}

export interface ExportResult {
  path: string;
  warnings: string[];
}

export interface TemplateItem {
  label: string;
  concept?: string | null;
  note?: string | null;
}

export interface TemplateField {
  key: string;
  label: string;
  kind: string;
  default_value?: string | null;
}

export interface GridSpec {
  columns: number;
  rows: number;
  caption: boolean;
  symbol: boolean;
  border: boolean;
  color_coding: string;
}

export interface TemplateInfo {
  id: string;
  name: string;
  category: string;
  description: string;
  kind: string;
  version: string;
  license: string;
  grid: GridSpec | null;
  fields: TemplateField[];
  items: TemplateItem[];
  complexity?: string | null;
  pages?: number | null;
  orientation?: string | null;
  printable?: boolean | null;
  interactive?: boolean | null;
}

export interface PlannedItem {
  label: string;
  concept_id: string | null;
  strategy: string;
  symbol_id: string | null;
  svg: string | null;
  approximate: boolean;
}

export interface ResourcePlan {
  template_id: string;
  title: string;
  kind: string;
  grid: GridSpec;
  items: PlannedItem[];
  notes: string[];
  source: string;
}

export interface DocumentSettings {
  mode: CaaMode;
  intensity: Intensity;
  commercial_only: boolean;
  pack_id: string | null;
  font_size: number;
  show_captions: boolean;
  border_style: BorderStyle;
  animations: "full" | "reduced" | "off";
  online_search: "off" | "suggest" | "auto";
  typing_sound: boolean;
  speak_words: boolean;
  speak_sentences: boolean;
  touch_mode: boolean;
  theme: "chiaro" | "scuro" | "contrasto";
  page: "a4" | "a4-orizzontale" | "a3" | "a5";
  page_margin: "stretto" | "normale" | "ampio";
  page_color: string;
  symbol_size: number;
  symbol_gap: number;
  chip_border_color: string;
  chip_border_width: number;
  chip_border_style: "solid" | "dashed" | "dotted";
  chip_radius: number;
  chip_padding: number;
  chip_background: string;
  pictogram_align: "left" | "center" | "right";
  mono: boolean;
  color_coding: boolean;
  /// Simbolizzazione automatica durante la digitazione (predefinita: attiva).
  auto_caa: boolean;
  /// Reranking contestuale con il modello locale, se configurato.
  neural_context: boolean;
  /// Visualizzazione in maiuscolo: cambia solo il testo mostrato, non quello
  /// analizzato dal motore né i pittogrammi associati.
  uppercase: boolean;
  /// Famiglia tipografica del documento; vuoto = carattere di sistema.
  font_family: string;
  /// Simbolizzazione delle singole categorie grammaticali.
  symbolize_articles: boolean;
  symbolize_prepositions: boolean;
  symbolize_conjunctions: boolean;
}

export const defaultSettings: DocumentSettings = {
  mode: "sopra",
  intensity: "media",
  commercial_only: false,
  pack_id: null,
  font_size: 15,
  // La parola è già nel testo sotto la striscia: la didascalia del repertorio
  // sarebbe una terza riga. Chi la vuole può attivarla nelle impostazioni.
  show_captions: false,
  // Bordo attorno a immagine e testo: è l'aspetto storico dei chip.
  border_style: "immagine_testo",
  animations: "full",
  online_search: "off",
  typing_sound: false,
  speak_words: false,
  speak_sentences: false,
  touch_mode: false,
  theme: "chiaro",
  page: "a4",
  page_margin: "normale",
  page_color: "#ffffff",
  symbol_size: 56,
  symbol_gap: 6,
  chip_border_color: "",
  chip_border_width: 1,
  chip_border_style: "solid",
  chip_radius: 10,
  chip_padding: 4,
  chip_background: "",
  pictogram_align: "left",
  mono: false,
  color_coding: true,
  auto_caa: true,
  neural_context: true,
  uppercase: false,
  font_family: "",
  symbolize_articles: false,
  symbolize_prepositions: false,
  symbolize_conjunctions: false,
};

// ---------------------------------------------------------------------------
// Universal Layout Engine: la specifica arriva al backend, che restituisce la
// geometria in millimetri. Il frontend non inventa mai misure.
// ---------------------------------------------------------------------------

export interface LayoutPageSpec {
  size: "A4" | "A3" | "A5" | string;
  orientation: "portrait" | "landscape" | string;
  margin_mm: number;
}

export interface LayoutGridSpec {
  kind: string;
  columns: number;
  rows: number;
  gap_mm: number;
}

export interface LayoutElementSpec {
  id: string;
  kind: "symbol" | "text" | "label" | "checkbox" | "box" | string;
  label: string;
  concept_id?: string | null;
  symbol_id?: string | null;
  svg?: string | null;
  page?: number;
  row?: number;
  column?: number;
  row_span?: number;
  column_span?: number;
  hidden?: boolean;
  locked?: boolean;
}

export interface LayoutAccessibility {
  large_symbols: boolean;
  high_contrast: boolean;
}

export interface LayoutSpecification {
  schema_version: number;
  document_type: string;
  title: string;
  page: LayoutPageSpec;
  layout: LayoutGridSpec;
  elements: LayoutElementSpec[];
  accessibility: LayoutAccessibility;
}

export interface PlacedElement {
  id: string;
  kind: string;
  label: string;
  concept_id: string | null;
  symbol_id: string | null;
  svg: string | null;
  hidden: boolean;
  locked: boolean;
  x_mm: number;
  y_mm: number;
  w_mm: number;
  h_mm: number;
  font_mm: number;
}

export interface LayoutPage {
  index: number;
  width_mm: number;
  height_mm: number;
  elements: PlacedElement[];
}

export interface LayoutDocument {
  spec: LayoutSpecification;
  pages: LayoutPage[];
  notes: string[];
}

/// Operazioni validate: le stesse per il comando manuale e per la richiesta in
/// linguaggio naturale. Gli elementi bloccati vengono rifiutati dal backend.
export type LayoutOperation =
  | { type: "set_page_orientation"; value: string }
  | { type: "set_page_size"; value: string }
  | { type: "set_margin"; value_mm: number }
  | { type: "set_grid_columns"; value: number }
  | { type: "set_grid_rows"; value: number }
  | { type: "set_gap"; value_mm: number }
  | { type: "set_large_symbols"; value: boolean }
  | { type: "set_title"; value: string }
  | { type: "move_element"; id: string; row: number; column: number }
  | { type: "move_element_to_page"; id: string; page: number }
  | { type: "hide_element"; id: string; hidden: boolean }
  | { type: "lock_element"; id: string; locked: boolean }
  | { type: "distribute_pages"; pages: number };

export interface LayoutEditResult {
  spec: LayoutSpecification;
  pages: LayoutPage[];
  notes: string[];
  operations: string[];
}

/// Anteprima di un file selezionato per l'importazione.
export interface ImportPreviewEntry {
  path: string;
  relative_path: string;
  original_name: string;
  suggested_lemma: string;
  format: string;
  supported: boolean;
  reason: string | null;
  duplicate: boolean;
  width: number | null;
  height: number | null;
  checksum: string | null;
  preview: string | null;
}

export interface ImportPreview {
  suggested_collection: string;
  root: string | null;
  entries: ImportPreviewEntry[];
  total: number;
  supported: number;
  duplicates: number;
  truncated: boolean;
  warnings: string[];
}

export interface ImportItemInput {
  path: string;
  lemma: string;
  collection: string | null;
  category: string | null;
  synonyms: string[];
  relative_path: string | null;
  original_name: string | null;
}

export interface ImportLicense {
  name: string | null;
  url: string | null;
  author: string | null;
  source_url: string | null;
  commercial_ok: boolean;
}

export interface ImportOutcome {
  path: string;
  symbol_id: string | null;
  status: "imported" | "duplicate" | "failed";
  reason: string | null;
}

export interface ImportReport {
  imported: number;
  skipped: number;
  failed: number;
  items: ImportOutcome[];
  warnings: string[];
}

export interface CollectionInfo {
  id: string;
  name: string;
  description: string | null;
  symbols: number;
  updated_at: string;
}

export interface OnlineResult {
  provider: string;
  id: string;
  title: string;
  thumbnail_url: string | null;
  image_url: string;
  page_url: string | null;
  mime: string;
  width: number | null;
  height: number | null;
  author: string | null;
  license: string;
  license_url: string | null;
  source: string;
  commercial_ok: boolean;
  kind: "pittogramma" | "fotografia";
}

export interface OnlineResponse {
  provider: string;
  query: string;
  results: OnlineResult[];
  cached: boolean;
  warnings: string[];
}

export interface OnlineProviderInfo {
  id: string;
  name: string;
  enabled: boolean;
  note: string;
}

export type DocumentOperation =
  | { type: "set_uppercase"; value: boolean }
  | { type: "scale_font_size"; factor: number }
  | { type: "set_font_size"; value: number }
  | { type: "scale_symbol_size"; factor: number }
  | { type: "set_symbol_gap"; value: number }
  | { type: "set_chip_border_color"; value: string }
  | { type: "set_alignment"; value: "left" | "center" | "right" | "justify" }
  | { type: "insert_table"; rows: number; columns: number }
  | { type: "add_table_row" }
  | { type: "add_table_column" }
  | { type: "generate_board"; prompt: string };

export interface DocumentEditPlan {
  operations: DocumentOperation[];
  descriptions: string[];
  warnings: string[];
  source: string;
}

export interface CliRunResult {
  ok: boolean;
  stdout: string;
  stderr: string;
  json: unknown;
  binary: string;
}
