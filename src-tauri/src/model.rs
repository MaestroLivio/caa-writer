//! Modelli dati condivisi tra backend e frontend.
//!
//! Tutti i DTO sono serializzati in `snake_case` per coerenza con i tipi
//! TypeScript in `src/document/types.ts`.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Intensity {
    Minima,
    Media,
    Completa,
}

impl Default for Intensity {
    fn default() -> Self {
        Intensity::Media
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CaaMode {
    Testo,
    Sopra,
    Sotto,
    SoloSimboli,
    Semplificata,
}

impl Default for CaaMode {
    fn default() -> Self {
        CaaMode::Sopra
    }
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct UnitModifiers {
    /// Plurale mostrato sul pittogramma (es. «bambini»).
    #[serde(default)]
    pub plural: bool,
    /// Negazione mostrata sul pittogramma (es. «non mangiare»).
    #[serde(default)]
    pub negation: bool,
    #[serde(default)]
    pub past: bool,
    #[serde(default)]
    pub future: bool,
    /// Vero quando la configurazione è stata fissata dall'utente: il motore
    /// non la ricalcola più finché non viene ripristinata.
    #[serde(default)]
    pub manual: bool,
}

impl UnitModifiers {
    pub fn any(&self) -> bool {
        self.plural || self.negation || self.past || self.future
    }
}

/// Intervallo scelto manualmente nel paragrafo. Gli offset sono in unità
/// UTF-16 (le posizioni di ProseMirror); il motore li converte in caratteri.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ManualRange {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolizeOptions {
    #[serde(default)]
    pub intensity: Intensity,
    #[serde(default)]
    pub mode: CaaMode,
    /// Percorso o identificativo del documento: serve per le scelte "solo qui".
    #[serde(default)]
    pub document_key: String,
    /// Esclude simboli con licenza non compatibile con uso commerciale.
    #[serde(default)]
    pub commercial_only: bool,
    /// Limita a un pacchetto simboli specifico (None = tutti).
    #[serde(default)]
    pub pack_id: Option<String>,
    #[serde(default)]
    pub paragraph_id: String,
    /// Simbolizzazione delle parole grammaticali, controllabile per categoria.
    #[serde(default = "default_true")]
    pub symbolize_articles: bool,
    #[serde(default = "default_true")]
    pub symbolize_prepositions: bool,
    #[serde(default = "default_true")]
    pub symbolize_conjunctions: bool,
    /// Espressioni unite manualmente dall'utente: una sola unità CAA.
    #[serde(default)]
    pub manual_units: Vec<ManualRange>,
    /// Intervalli in cui l'utente ha separato un'espressione automatica.
    #[serde(default)]
    pub split_ranges: Vec<ManualRange>,
}

impl Default for SymbolizeOptions {
    fn default() -> Self {
        Self {
            intensity: Intensity::default(),
            mode: CaaMode::default(),
            document_key: String::new(),
            commercial_only: false,
            pack_id: None,
            paragraph_id: String::new(),
            symbolize_articles: false,
            symbolize_prepositions: false,
            symbolize_conjunctions: false,
            manual_units: Vec::new(),
            split_ranges: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Strategy {
    /// Corrispondenza esatta tra concetto e simbolo.
    Exact,
    /// Sinonimo equivalente (auto / macchina).
    Synonym,
    /// Forma base di una parola flessa (camminavano -> camminare).
    BaseForm,
    /// Concetto più generale (bassotto -> cane).
    Broader,
    /// Composizione di più pittogrammi (monopattino elettrico).
    Composition,
    /// Concetto semanticamente vicino, dichiarato come approssimazione.
    Related,
    /// Nessun simbolo adeguato: resta solo il testo.
    None,
}

impl Strategy {
    pub fn as_str(&self) -> &'static str {
        match self {
            Strategy::Exact => "exact",
            Strategy::Synonym => "synonym",
            Strategy::BaseForm => "base_form",
            Strategy::Broader => "broader",
            Strategy::Composition => "composition",
            Strategy::Related => "related",
            Strategy::None => "none",
        }
    }

    pub fn is_approximate(&self) -> bool {
        matches!(self, Strategy::Broader | Strategy::Related)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolPart {
    pub concept_id: String,
    pub label: String,
    pub symbol_id: Option<String>,
    pub svg: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alternative {
    pub symbol_id: String,
    pub concept_id: Option<String>,
    pub label: String,
    pub meaning: String,
    pub pack_id: String,
    pub pack_name: String,
    pub format: String,
    pub svg: Option<String>,
    pub approximate: bool,
    /// Vero se l'alternativa è stata proposta dal modello neurale (parole
    /// affini o campo semantico della frase), non dal lessico deterministico.
    /// Resta sempre approssimativa finché l'utente non la sceglie.
    #[serde(default)]
    pub neural: bool,
    pub license: String,
    pub source_url: Option<String>,
    pub author: Option<String>,
}

/// Livello di affinità di un pittogramma rispetto alla parola cercata.
/// L'ordine della variante è anche l'ordine di presentazione: dal più forte
/// (la parola scritta) al più debole (somiglianza dichiarata).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AffinityLevel {
    /// L'etichetta del pittogramma è esattamente la parola.
    Exact,
    /// Il pittogramma è la forma base (lemma) della parola scritta.
    Lemma,
    /// Il pittogramma è un'altra forma flessa della stessa parola.
    Variant,
    /// Sinonimo dichiarato nel lessico o concetto equivalente.
    Synonym,
    /// Concetto più generale (per esempio «frutta» per «pesca»).
    General,
    /// Concetto collegato o somiglianza nel testo: sempre approssimativo.
    Related,
}

impl AffinityLevel {
    pub fn title(&self) -> &'static str {
        match self {
            AffinityLevel::Exact => "Corrispondenza esatta",
            AffinityLevel::Lemma => "Forma base",
            AffinityLevel::Variant => "Varianti della parola",
            AffinityLevel::Synonym => "Sinonimi",
            AffinityLevel::General => "Concetto più generale",
            AffinityLevel::Related => "Affini",
        }
    }

    /// Indice di forza: serve per ordinare e per verificare la deduplica.
    pub fn rank(&self) -> u8 {
        match self {
            AffinityLevel::Exact => 0,
            AffinityLevel::Lemma => 1,
            AffinityLevel::Variant => 2,
            AffinityLevel::Synonym => 3,
            AffinityLevel::General => 4,
            AffinityLevel::Related => 5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AffinityCandidate {
    /// Dati del pittogramma, compatibili con `Alternative` (l'interfaccia lo
    /// applica con lo stesso percorso delle alternative normali).
    pub symbol: Alternative,
    /// Punteggio interno 0–100: non è una probabilità calibrata.
    pub score: u32,
    /// Vero quando il candidato non è una corrispondenza diretta della parola.
    pub approximate: bool,
    /// Spiegazione leggibile del perché il pittogramma è stato proposto.
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AffinityGroup {
    pub level: AffinityLevel,
    pub title: String,
    pub approximate: bool,
    pub candidates: Vec<AffinityCandidate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WordAffinity {
    /// Parola scritta dall'utente.
    pub word: String,
    /// Forma normalizzata usata per la ricerca.
    pub normalized: String,
    /// Lemma riconosciuto dal lessico, se esiste.
    pub lemma: Option<String>,
    /// Vero se la parola è stata riconosciuta dal lessico o dalla libreria.
    pub known: bool,
    pub explanation: String,
    /// Solo i gruppi con almeno un candidato, in ordine di forza.
    pub groups: Vec<AffinityGroup>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaaToken {
    pub index: usize,
    pub surface: String,
    pub start: usize,
    pub end: usize,
    pub lemma: Option<String>,
    pub pos: String,
    /// Concetto scelto (se esiste).
    pub concept_id: Option<String>,
    pub concept_label: Option<String>,
    pub meaning: Option<String>,
    pub category: Option<String>,
    pub symbol_id: Option<String>,
    pub pack_id: Option<String>,
    /// Testo mostrato sotto il pittogramma: è sempre la parola scritta
    /// dall'utente, non l'etichetta del repertorio.
    pub caption: String,
    /// Etichetta originale del pittogramma nel repertorio. Resta nei metadati
    /// (tooltip, pannello, registro licenze) e non viene duplicata a schermo.
    #[serde(default)]
    pub symbol_label: Option<String>,
    pub svg: Option<String>,
    pub composition: Vec<SymbolPart>,
    pub strategy: String,
    pub confidence: f32,
    pub approximate: bool,
    pub locked: bool,
    pub negated: bool,
    pub key: bool,
    pub is_punct: bool,
    pub is_function: bool,
    pub span: usize,
    pub sentence_index: usize,
    /// Vero se l'utente ha nascosto il pittogramma: il testo resta, il simbolo
    /// non compare nel foglio, nella stampa né nelle esportazioni visive.
    #[serde(default)]
    pub hidden: bool,
    /// Modificatori grafici associati al pittogramma.
    #[serde(default)]
    pub modifiers: UnitModifiers,
    /// Alternative ordinate per punteggio (massimo 12 per non pesare sul JSON).
    pub alternatives: Vec<Alternative>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParagraphResult {
    pub paragraph_id: String,
    pub tokens: Vec<CaaToken>,
    pub domains: Vec<String>,
    pub symbolized: usize,
    pub approximate: usize,
    pub missing: usize,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiStatus {
    pub enabled: bool,
    pub binary_present: bool,
    pub model_present: bool,
    pub binary_path: String,
    pub model_path: String,
    pub backend: String,
    pub threads: u32,
    pub context_size: u32,
    pub timeout_ms: u64,
    pub last_error: Option<String>,
    pub note: String,
    /// Hardware misurato e profilo scelto al momento della richiesta.
    pub hardware: crate::ai_engine::hardware::HardwareProfile,
    /// Metriche del servizio neurale condiviso (richieste, cache, tempi).
    pub metrics: crate::ai_engine::service::MetricsSnapshot,
    /// Vero se l'istanza condivisa (llama-server) è attiva.
    pub server_running: bool,
    /// Nome leggibile del modello attivo (dal catalogo o nome del file).
    #[serde(default)]
    pub active_model: String,
    /// Vero se il modello attivo è quello incluso nel pacchetto.
    #[serde(default)]
    pub active_model_bundled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub model: String,
    pub runs: u32,
    pub failures: u32,
    pub average_ms: u64,
    pub tokens_per_second: f32,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    pub enabled: bool,
    pub binary_path: String,
    pub model_path: String,
    pub threads: u32,
    pub context_size: u32,
    pub timeout_ms: u64,
    /// `auto` | `cpu` | `metal` (macOS Apple Silicon) | `cuda`
    pub backend: String,
    /// Argomenti aggiuntivi per versioni diverse di llama.cpp.
    #[serde(default)]
    pub extra_args: Vec<String>,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            binary_path: String::new(),
            model_path: String::new(),
            threads: 2,
            context_size: 1024,
            timeout_ms: 8000,
            backend: "auto".to_string(),
            extra_args: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryStats {
    pub symbols: i64,
    pub concepts: i64,
    pub lemmas: i64,
    pub packs: i64,
    pub expressions: i64,
    pub packs_detail: Vec<PackInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub license: String,
    pub license_url: Option<String>,
    pub source_url: Option<String>,
    pub author: Option<String>,
    pub commercial_ok: bool,
    pub symbol_count: i64,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributionEntry {
    pub symbol_id: String,
    pub label: String,
    pub pack_id: String,
    pub pack_name: String,
    pub author: Option<String>,
    pub rights_holder: Option<String>,
    pub license: String,
    pub license_url: Option<String>,
    pub source_url: Option<String>,
    pub constraints: Option<String>,
    pub modifications: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preference {
    pub scope: String,
    pub lemma: String,
    pub context_key: Option<String>,
    pub symbol_id: String,
    pub concept_id: Option<String>,
    pub locked: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResultPage {
    pub items: Vec<Alternative>,
    pub total: i64,
    pub page: u32,
    pub page_size: u32,
}

/// Documento inviato al backend per il salvataggio in `.caadoc`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentPayload {
    pub title: String,
    /// JSON del documento (ProseMirror + impostazioni CAA), opaco per il backend.
    pub content: serde_json::Value,
    #[serde(default)]
    pub plain_text: String,
    #[serde(default)]
    pub settings: serde_json::Value,
    /// Coppie (symbol_id, svg) usate nel documento, salvate nel contenitore.
    #[serde(default)]
    pub assets: Vec<DocumentAsset>,
    #[serde(default)]
    pub preferences: Vec<DocumentPreference>,
    /// Specifica di layout del materiale (Word Processor o Studio). Il backend
    /// la conserva opaca: il motore la interpreta, il documento la conserva.
    #[serde(default)]
    pub layout: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentAsset {
    pub symbol_id: String,
    pub svg: Option<String>,
    pub png_base64: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentPreference {
    pub unit_key: String,
    pub symbol_id: Option<String>,
    pub concept_id: Option<String>,
    pub locked: bool,
    /// Stato di visibilità e modificatori: salvati con il documento.
    #[serde(default)]
    pub hidden: bool,
    #[serde(default)]
    pub modifiers: UnitModifiers,
}

/// Documento caricato da `.caadoc`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadedDocument {
    pub title: String,
    pub content: serde_json::Value,
    pub plain_text: String,
    pub settings: serde_json::Value,
    pub assets: Vec<DocumentAsset>,
    pub preferences: Vec<DocumentPreference>,
    pub format_version: u32,
    pub warnings: Vec<String>,
    pub path: String,
    /// Layout salvato nel documento (None per i documenti solo testuali).
    #[serde(default)]
    pub layout: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportUnit {
    pub surface: String,
    pub caption: String,
    pub symbol_id: Option<String>,
    pub svg: Option<String>,
    pub png_base64: Option<String>,
    pub approximate: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportParagraph {
    pub plain_text: String,
    pub style: String,
    pub units: Vec<ExportUnit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportDoc {
    pub title: String,
    pub paragraphs: Vec<ExportParagraph>,
    pub include_symbols: bool,
    /// Didascalia del repertorio sotto il pittogramma. Spenta per default:
    /// la parola scritta è già nel testo del paragrafo.
    #[serde(default)]
    pub include_captions: bool,
    pub mode: CaaMode,
    pub attributions: Vec<AttributionEntry>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportResult {
    pub path: String,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceInfo {
    pub id: String,
    pub name: String,
    pub language: String,
}
