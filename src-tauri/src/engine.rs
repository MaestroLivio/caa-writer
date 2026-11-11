//! Motore applicativo: unisce analisi linguistica, ricerca simbolica,
//! preferenze utente e (facoltativamente) il modello locale.
//!
//! Ogni paragrafo è analizzato in modo indipendente e incrementale: modificare
//! una frase non rigenera l'intero documento.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::ai_engine;
use crate::asset_manager::{alternative_from_record, SvgCache};
use crate::database::{seed_core_pack, Database, DbResult, SymbolFilters};
use crate::linguistics::analyzer::TokenAnalysis;
use crate::linguistics::lexicon::Lexicon;
use crate::linguistics::{morphology, Analyzer};
use crate::model::*;
use crate::paths::AppPaths;
use crate::symbol_search;
use crate::util::now_iso;

const CACHE_LIMIT: usize = 96;
const MAX_ALTERNATIVES: usize = 12;

/// Glossario deterministico per la ricerca inglese. Non sostituisce il motore
/// neurale: copre le parole CAA più frequenti, così la ricerca funziona anche
/// quando il modello locale è spento o è troppo piccolo per tradurre bene.
fn translation_glossary(term: &str) -> Option<&'static str> {
    let key = term
        .trim()
        .to_lowercase()
        .replace(['’', '\''], "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let value = match key.as_str() {
        "dog" => "cane",
        "cat" => "gatto",
        "bird" => "uccello",
        "fish" => "pesce",
        "horse" => "cavallo",
        "cow" => "mucca",
        "pig" => "maiale",
        "sheep" => "pecora",
        "mouse" => "topo",
        "butterfly" => "farfalla",
        "eat" | "eating" => "mangiare",
        "drink" | "drinking" => "bere",
        "school" => "scuola",
        "class" | "classroom" => "classe",
        "test" | "tests" | "quiz" => "test",
        "exam" => "esame",
        "exercise" => "esercizio",
        "homework" => "compiti",
        "teacher" => "maestra",
        "friend" => "amico",
        "home" | "house" => "casa",
        "garden" | "park" => "giardino",
        "mum" | "mom" | "mother" => "mamma",
        "dad" | "father" => "papà",
        "brother" => "fratello",
        "sister" => "sorella",
        "grandmother" | "grandma" => "nonna",
        "grandfather" | "grandpa" => "nonno",
        "water" => "acqua",
        "milk" => "latte",
        "bread" => "pane",
        "apple" => "mela",
        "banana" => "banana",
        "pasta" => "pasta",
        "pizza" => "pizza",
        "ball" => "palla",
        "book" => "libro",
        "toy" | "toys" => "giocattolo",
        "car" => "macchina",
        "bus" => "autobus",
        "train" => "treno",
        "plane" | "airplane" => "aereo",
        "helicopter" => "elicottero",
        "ambulance" => "ambulanza",
        "hospital" => "ospedale",
        "doctor" => "dottore",
        "police" => "polizia",
        "bike" | "bicycle" => "bicicletta",
        "play" | "playing" => "giocare",
        "sleep" | "sleeping" => "dormire",
        "run" | "running" => "correre",
        "walk" | "walking" => "camminare",
        "jump" | "jumping" => "saltare",
        "draw" | "drawing" => "disegnare",
        "write" | "writing" => "scrivere",
        "read" | "reading" => "leggere",
        "listen" | "listening" => "ascoltare",
        "speak" | "speaking" | "talk" => "parlare",
        "help" => "aiuto",
        "stop" => "fermare",
        "go" | "going" => "andare",
        "come" | "coming" => "venire",
        "yes" => "sì",
        "no" => "no",
        "more" => "ancora",
        "finished" | "finish" => "finito",
        "toilet" | "bathroom" => "bagno",
        "wash" | "washing" => "lavare",
        "hands" => "mani",
        "head" => "testa",
        "happy" => "felice",
        "sad" => "triste",
        "angry" => "arrabbiato",
        "scared" | "afraid" => "spaventato",
        "tired" => "stanco",
        "hurt" | "pain" => "dolore",
        "hot" => "caldo",
        "cold" => "freddo",
        "big" | "large" => "grande",
        "small" | "little" => "piccolo",
        "good" => "bene",
        "bad" => "male",
        "wait" | "waiting" => "aspettare",
        "again" => "ancora",
        "open" => "aprire",
        "close" | "shut" => "chiudere",
        "give" => "dare",
        "take" => "prendere",
        "look" | "see" => "guardare",
        "make" | "do" => "fare",
        "want" => "volere",
        "need" => "bisogno",
        "like" => "piacere",
        "love" => "amore",
        "thank you" | "thanks" => "grazie",
        "good morning" => "buongiorno",
        "goodbye" | "bye" => "arrivederci",
        "excuse me" | "sorry" => "scusa",
        "wash hands" => "lavarsi le mani",
        "go home" => "andare a casa",
        "sit down" => "sedersi",
        "stand up" => "alzarsi",
        "i want" => "voglio",
        "i need" => "ho bisogno",
        "how are you" => "come stai",
        _ => return None,
    };
    Some(value)
}

/// Glossario deterministico italiano → inglese per la ricerca dei pittogrammi.
/// Copre il vocabolario CAA di base; per il resto interviene il modello locale.
fn translation_to_english_glossary(term: &str) -> Option<&'static str> {
    let key = term
        .trim()
        .to_lowercase()
        .replace(['’', '\''], "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let value = match key.as_str() {
        "cane" => "dog",
        "gatto" => "cat",
        "uccello" => "bird",
        "pesce" => "fish",
        "cavallo" => "horse",
        "mucca" => "cow",
        "maiale" => "pig",
        "pecora" => "sheep",
        "topo" => "mouse",
        "farfalla" => "butterfly",
        "mangiare" => "eat",
        "bere" => "drink",
        "scuola" => "school",
        "classe" => "class",
        "test" => "test",
        "esame" => "exam",
        "esercizio" => "exercise",
        "compiti" => "homework",
        "maestra" => "teacher",
        "amico" => "friend",
        "casa" => "home",
        "giardino" => "garden",
        "mamma" => "mum",
        "papà" => "dad",
        "fratello" => "brother",
        "sorella" => "sister",
        "nonna" => "grandmother",
        "nonno" => "grandfather",
        "acqua" => "water",
        "latte" => "milk",
        "pane" => "bread",
        "mela" => "apple",
        "banana" => "banana",
        "pasta" => "pasta",
        "pizza" => "pizza",
        "palla" => "ball",
        "libro" => "book",
        "giocattolo" => "toy",
        "macchina" => "car",
        "autobus" => "bus",
        "treno" => "train",
        "aereo" => "plane",
        "elicottero" => "helicopter",
        "ambulanza" => "ambulance",
        "ospedale" => "hospital",
        "dottore" => "doctor",
        "polizia" => "police",
        "bicicletta" => "bike",
        "giocare" => "play",
        "dormire" => "sleep",
        "correre" => "run",
        "camminare" => "walk",
        "saltare" => "jump",
        "disegnare" => "draw",
        "scrivere" => "write",
        "leggere" => "read",
        "ascoltare" => "listen",
        "parlare" => "speak",
        "aiuto" => "help",
        "fermare" => "stop",
        "andare" => "go",
        "venire" => "come",
        "sì" => "yes",
        "no" => "no",
        "ancora" => "more",
        "finito" => "finished",
        "bagno" => "toilet",
        "lavare" => "wash",
        "mani" => "hands",
        "testa" => "head",
        "felice" => "happy",
        "triste" => "sad",
        "arrabbiato" => "angry",
        "spaventato" => "scared",
        "stanco" => "tired",
        "dolore" => "hurt",
        "caldo" => "hot",
        "freddo" => "cold",
        "grande" => "big",
        "piccolo" => "small",
        "bene" => "good",
        "male" => "bad",
        "aspettare" => "wait",
        "aprire" => "open",
        "chiudere" => "close",
        "dare" => "give",
        "prendere" => "take",
        "guardare" => "look",
        "fare" => "make",
        "volere" => "want",
        "bisogno" => "need",
        "piacere" => "like",
        "amore" => "love",
        "grazie" => "thank you",
        "buongiorno" => "good morning",
        "arrivederci" => "goodbye",
        "scusa" => "excuse me",
        "lavarsi le mani" => "wash hands",
        "andare a casa" => "go home",
        "sedersi" => "sit down",
        "alzarsi" => "stand up",
        "voglio" => "i want",
        "ho bisogno" => "i need",
        "come stai" => "how are you",
        _ => return None,
    };
    Some(value)
}

/// Metriche misurate della pipeline di scrittura. Non sono stime: ogni valore
/// viene incrementato nel punto in cui l'operazione avviene davvero.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SymbolMetrics {
    pub paragraphs: u64,
    pub total_ms: u64,
    pub last_ms: u64,
    pub worst_ms: u64,
    pub cache_hits: u64,
    pub stale_results: u64,
    pub neural_paragraphs: u64,
    pub neural_ms: u64,
}

/// Cancellazione cooperativa di una richiesta di rifinitura neurale.
pub struct CancelToken {
    id: Option<String>,
    flag: Arc<AtomicBool>,
}

/// Dati misurati mostrati nel pannello Diagnostica del Creator.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RuntimeDiagnostics {
    pub version: String,
    pub platform: String,
    pub app_process: crate::diagnostics::ProcessSample,
    pub model_process: Option<crate::diagnostics::ProcessSample>,
    pub server_running: bool,
    pub speech_active: bool,
    pub ai: ai_engine::service::MetricsSnapshot,
    pub symbols: SymbolMetrics,
    pub active_model: String,
}

/// Converte gli intervalli UTF-16 di ProseMirror in offset di caratteri,
/// che sono le posizioni usate dal tokenizer.
fn char_ranges(text: &str, ranges: &[ManualRange]) -> Vec<(usize, usize)> {
    fn to_char(text: &str, utf16: usize) -> usize {
        let mut units = 0usize;
        let mut chars = 0usize;
        for c in text.chars() {
            if units >= utf16 {
                break;
            }
            units += c.len_utf16();
            chars += 1;
        }
        chars
    }
    ranges
        .iter()
        .map(|range| {
            let start = to_char(text, range.start);
            let end = to_char(text, range.end);
            (start.min(end), start.max(end))
        })
        .filter(|(start, end)| start < end)
        .collect()
}

/// Unisce le unità comprese negli intervalli scelti dall'utente. Il testo
/// originale non cambia: la superficie dell'unità è lo stesso intervallo.
fn apply_manual_units(
    lexicon: &Lexicon,
    text: &str,
    ranges: &[(usize, usize)],
    tokens: &mut [TokenAnalysis],
) {
    if ranges.is_empty() {
        return;
    }
    let chars: Vec<char> = text.chars().collect();
    let mut sorted = ranges.to_vec();
    sorted.sort_by_key(|(start, _)| *start);
    for (start, end) in sorted {
        if end > chars.len() {
            continue;
        }
        let indices: Vec<usize> = tokens
            .iter()
            .enumerate()
            .filter(|(_, token)| {
                token.is_word()
                    && token.span > 0
                    && token.start >= start
                    && token.end <= end
            })
            .map(|(index, _)| index)
            .collect();
        if indices.len() < 2 {
            continue;
        }
        let first = indices[0];
        let last = *indices.last().unwrap();
        // L'unità copre parole intere, anche se la selezione era parziale.
        let unit_start = tokens[first].start;
        let unit_end = tokens[last].end;
        let surface = chars[unit_start..unit_end]
            .iter()
            .collect::<String>()
            .trim()
            .to_string();
        let normalized = crate::linguistics::normalize::normalize_word(&surface);
        let mut merged = tokens[first].clone();
        merged.surface = surface;
        merged.start = unit_start;
        merged.end = unit_end;
        merged.span = indices.len();
        merged.key = true;
        merged.pos = "mwe".to_string();
        merged.is_function = false;
        merged.is_punct = false;
        merged.negated = false;
        merged.normalized = normalized.clone();
        merged.lemma = Some(normalized.clone());
        merged.concepts = lexicon
            .mwe
            .get(&normalized)
            .cloned()
            .into_iter()
            .collect();
        merged.mwe_concept = merged.concepts.first().cloned();
        tokens[first] = merged;
        for index in indices.iter().skip(1) {
            let token = &mut tokens[*index];
            token.span = 0;
            token.is_punct = true;
            token.key = false;
            token.concepts.clear();
            token.mwe_concept = None;
        }
    }
}

/// Vero se l'etichetta corrisponde alla parola cercata: identica, oppure la
/// contiene come parola intera («lavarsi le mani» per «mani»). Le etichette
/// ARASAAC sono spesso frasi: la ricerca manuale le trovava, la simbolizzazione
/// automatica no, perché pretendeva l'uguaglianza esatta.
fn label_matches_word(label: &str, word: &str) -> bool {
    let normalized = crate::linguistics::normalize::normalize_word(label);
    if normalized == word {
        return true;
    }
    // Sotto le tre lettere la corrispondenza parziale produrrebbe solo rumore.
    if word.chars().count() < 3 {
        return false;
    }
    normalized.split_whitespace().any(|part| part == word)
}

/// Ipotesi di forma singolare per una parola fuori lessico, limitate a regole
/// italiane non ambigue. Servono solo a cercare un'etichetta che esiste già:
/// se il singolare ipotizzato non è nel repertorio, non viene inventato nulla.
/// La regola generica «-e → -a» è esclusa di proposito («pesce» diventerebbe
/// «pesca», un pittogramma fuorviante).
fn singular_guesses(word: &str) -> Vec<(String, &'static str)> {
    let mut out = Vec::new();
    if let Some(stem) = word.strip_suffix("esse") {
        out.push((format!("{stem}essa"), "plurale femminile in -esse"));
    }
    if let Some(stem) = word.strip_suffix("iste") {
        out.push((format!("{stem}ista"), "plurale in -iste"));
    }
    if let Some(stem) = word.strip_suffix("trici") {
        out.push((format!("{stem}trice"), "plurale in -trici"));
    }
    if let Some(stem) = word.strip_suffix("che") {
        out.push((format!("{stem}ca"), "plurale in -che"));
    }
    if let Some(stem) = word.strip_suffix("ghe") {
        out.push((format!("{stem}ga"), "plurale in -ghe"));
    }
    out.retain(|(guess, _)| guess.chars().count() >= 3);
    out
}

pub struct Engine {
    pub db: Mutex<Database>,
    pub lexicon: Lexicon,
    pub paths: AppPaths,
    svg_cache: Mutex<SvgCache>,
    ai_config: Mutex<AiConfig>,
    ai_error: Mutex<Option<String>>,
    /// Istanza condivisa del modello e coda con priorità: tutti i moduli
    /// passano da qui, nessuno avvia il proprio processo.
    ai_model: Arc<ai_engine::service::LocalModel>,
    ai_service: Arc<ai_engine::service::Service>,
    hardware: ai_engine::hardware::HardwareProfile,
    /// Download e importazione dei modelli GGUF (azione esplicita dell'utente).
    installer: ai_engine::installer::ModelInstaller,
    cache: Mutex<HashMap<String, ParagraphResult>>,
    /// Richieste neurali annullabili: il frontend assegna un id e, quando il
    /// testo cambia, chiede l'annullamento. Il lavoro già in esecuzione in
    /// llama.cpp non viene interrotto a forza (non è sicuro), ma il risultato
    /// viene scartato e non tocca il documento.
    cancellations: Mutex<HashMap<String, Arc<AtomicBool>>>,
    metrics: Mutex<SymbolMetrics>,
    pub startup_warnings: Vec<String>,
}

impl Engine {
    /// Apre il database, applica le migrazioni e importa le risorse se mancano.
    pub fn new(paths: AppPaths) -> Result<Self, String> {
        let mut db = Database::open(&paths.db_path).map_err(|e| e.to_string())?;
        db.migrate().map_err(|e| e.to_string())?;

        let lexicon = Lexicon::load(&paths.dictionary_path())?;
        let mut warnings = Vec::new();

        // Ogni pacchetto presente in resources/symbol-packs viene importato una
        // volta sola: un pacchetto nuovo (per esempio ARASAAC) entra nel
        // database esistente senza duplicare i simboli già presenti.
        let symbol_count = db.statistics().map(|s| s.symbols).unwrap_or(0);
        // La versione del lessico è versionata come il pacchetto: se il file
        // delle risorse è più recente del seed, i concetti e i lemmi nuovi
        // devono entrare anche nel database già esistente.
        let lexicon_version = std::fs::read_to_string(paths.dictionary_path())
            .ok()
            .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
            .and_then(|value| value.get("version").and_then(|v| v.as_u64()));
        let installed: std::collections::HashMap<String, String> = db
            .list_packs()
            .unwrap_or_default()
            .into_iter()
            .map(|pack| (pack.id, pack.version))
            .collect();
        let mut pack_dirs: Vec<std::path::PathBuf> = std::fs::read_dir(paths.packs_dir())
            .map(|entries| {
                entries
                    .flatten()
                    .map(|entry| entry.path())
                    .filter(|dir| dir.join("manifest.json").exists())
                    .collect()
            })
            .unwrap_or_default();
        pack_dirs.sort();
        for dir in pack_dirs {
            let pack_id = dir
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default()
                .to_string();
            // Un pacchetto già importato si salta, ma se il manifest dichiara
            // una versione diversa (per esempio ARASAAC esteso all'intero
            // catalogo) i suoi simboli vengono reimportati.
            let manifest_version = std::fs::read_to_string(dir.join("manifest.json"))
                .ok()
                .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
                .and_then(|value| {
                    value
                        .get("pack")
                        .and_then(|pack| pack.get("version"))
                        .and_then(|version| version.as_str())
                        .map(str::to_string)
                });
            let seeded_lexicon = db
                .setting("seed.lexicon_version")
                .and_then(|raw| raw.parse::<u64>().ok());
            if symbol_count > 0
                && installed.get(&pack_id) == manifest_version.as_ref()
                && seeded_lexicon == lexicon_version
            {
                continue;
            }
            match seed_core_pack(&mut db, &paths.dictionary_path(), &dir) {
                Ok(stats) => {
                    if !stats.warnings.is_empty() {
                        warnings.push(format!(
                            "Importazione simboli ({pack_id}) con {} avvisi.",
                            stats.warnings.len()
                        ));
                    }
                }
                Err(e) => warnings.push(format!("Importazione del pacchetto {pack_id} non riuscita: {e}")),
            }
        }

        // Il database può essere stato creato da una copia dell'app in un
        // percorso diverso: riallinea i file dei simboli alla cartella risorse
        // corrente prima di usarli.
        if let Ok(repaired) = db.repair_asset_paths(&paths.resources_dir, &paths.assets_dir) {
            if repaired > 0 {
                warnings.push(format!(
                    "Riparati {repaired} percorsi di pittogrammi dopo lo spostamento dell'app."
                ));
            }
        }

        let mut ai_config = db
            .setting("ai.config")
            .and_then(|raw| serde_json::from_str::<AiConfig>(&raw).ok())
            .unwrap_or_default();

        // Il motore neurale è incluso nell'applicazione: se l'utente non ha
        // scelto altri percorsi, si usano binario e modello del pacchetto.
        let ai_dir = paths.ai_dir();
        let mut bundled = false;
        if ai_config.binary_path.trim().is_empty() {
            for name in ["llama-cli", "llama-cli.exe"] {
                for directory in [ai_dir.clone(), ai_dir.join("runtime")] {
                    let candidate = directory.join(name);
                    if candidate.exists() {
                        ai_config.binary_path = candidate.display().to_string();
                        bundled = true;
                        break;
                    }
                }
                if bundled {
                    break;
                }
            }
        }
        if ai_config.model_path.trim().is_empty() {
            let candidate = ai_dir.join("model.gguf");
            if candidate.exists() {
                ai_config.model_path = candidate.display().to_string();
                bundled = true;
            }
        }
        // Il progetto è stato spostato (o l'app è stata reinstallata): i
        // percorsi salvati possono puntare a una cartella non più esistente.
        // Si riallineano al runtime incluso, senza toccare un percorso
        // personalizzato ancora valido.
        let mut repaired_ai = false;
        if !ai_config.binary_path.trim().is_empty()
            && !std::path::Path::new(&ai_config.binary_path).exists()
        {
            for name in ["llama-cli", "llama-cli.exe"] {
                let candidate = ai_dir.join(name);
                if candidate.exists() {
                    ai_config.binary_path = candidate.display().to_string();
                    repaired_ai = true;
                    break;
                }
            }
        }
        if !ai_config.model_path.trim().is_empty()
            && !std::path::Path::new(&ai_config.model_path).exists()
        {
            let candidate = ai_dir.join("model.gguf");
            if candidate.exists() {
                ai_config.model_path = candidate.display().to_string();
                repaired_ai = true;
            }
        }
        if repaired_ai {
            if let Ok(raw) = serde_json::to_string(&ai_config) {
                let _ = db.set_setting("ai.config", &raw);
            }
        }
        if bundled && !ai_config.enabled {
            ai_config.enabled = true;
        }

        let hardware = ai_engine::hardware::detect();
        let ai_model = Arc::new(ai_engine::service::LocalModel::new(
            paths.ai_dir(),
            ai_config.clone(),
        ));
        let ai_service = Arc::new(ai_engine::service::Service::new(
            ai_model.clone(),
            std::time::Duration::from_millis(ai_config.timeout_ms.clamp(1000, 60_000) + 5_000),
        ));
        let installer = ai_engine::installer::ModelInstaller::new(paths.models_dir.clone());

        Ok(Self {
            db: Mutex::new(db),
            lexicon,
            paths,
            svg_cache: Mutex::new(SvgCache::new(160)),
            ai_config: Mutex::new(ai_config),
            ai_error: Mutex::new(None),
            ai_model,
            ai_service,
            hardware,
            installer,
            cache: Mutex::new(HashMap::new()),
            cancellations: Mutex::new(HashMap::new()),
            metrics: Mutex::new(SymbolMetrics::default()),
            startup_warnings: warnings,
        })
    }

    pub fn filters(&self, opts: &SymbolizeOptions) -> SymbolFilters {
        SymbolFilters {
            pack_id: opts.pack_id.clone(),
            commercial_only: opts.commercial_only,
            collection_id: None,
        }
    }

    fn cache_key(opts: &SymbolizeOptions, text: &str) -> String {
        format!(
            "{}\u{1}{:?}\u{1}{:?}\u{1}{}\u{1}{}\u{1}{:?}\u{1}{}\u{1}{:?}\u{1}{:?}\u{1}{:?}",
            opts.paragraph_id,
            opts.intensity,
            opts.mode,
            opts.document_key,
            opts.commercial_only,
            opts.pack_id,
            text,
            opts.manual_units
                .iter()
                .map(|range| (range.start, range.end))
                .collect::<Vec<_>>(),
            opts.split_ranges
                .iter()
                .map(|range| (range.start, range.end))
                .collect::<Vec<_>>(),
            // Le preferenze di un profilo non devono essere servite dalla
            // cache a un altro profilo.
            opts.profile_id
        )
    }

    /// Simbolizza un paragrafo. Ritorna token allineati al testo originale.
    pub fn symbolize(&self, text: &str, opts: &SymbolizeOptions) -> Result<ParagraphResult, String> {
        self.symbolize_internal(text, opts, false, None)
    }

    /// Come `symbolize`, ma con il reranking del modello locale sui soli
    /// token ambigui. Da invocare in background, mai a ogni tasto.
    pub fn refine(&self, text: &str, opts: &SymbolizeOptions) -> Result<ParagraphResult, String> {
        self.symbolize_internal(text, opts, true, None)
    }

    /// Rifinitura annullabile: `request_id` identifica la versione del testo a
    /// cui il risultato appartiene. Se nel frattempo arriva `cancel_request`,
    /// il risultato viene scartato anche se il modello ha già risposto.
    pub fn refine_cancellable(
        &self,
        text: &str,
        opts: &SymbolizeOptions,
        request_id: Option<&str>,
    ) -> Result<ParagraphResult, String> {
        let token = self.register_cancel(request_id);
        let result = self.symbolize_internal(text, opts, true, Some(token.flag.clone()));
        self.release_cancel(&token);
        result
    }

    /// Registra un gettone di annullamento. Senza id non c'è nulla da
    /// annullare in modo mirato: resta un gettone non registrato.
    fn register_cancel(&self, id: Option<&str>) -> CancelToken {
        let flag = Arc::new(AtomicBool::new(false));
        let id = id.map(str::to_string);
        if let (Some(key), Ok(mut map)) = (id.as_ref(), self.cancellations.lock()) {
            // Evita che una mappa di richieste abbandonate cresca senza limite.
            if map.len() > 64 {
                map.retain(|_, value| !value.load(Ordering::SeqCst));
            }
            // Se l'annullamento è arrivato prima della registrazione (il
            // comando async non ha ancora raggiunto il motore), il gettone
            // esiste già ed è positivo: il risultato non deve essere applicato.
            if let Some(existing) = map.get(key) {
                return CancelToken {
                    id,
                    flag: existing.clone(),
                };
            }
            map.insert(key.clone(), flag.clone());
        }
        CancelToken { id, flag }
    }

    fn release_cancel(&self, token: &CancelToken) {
        if let (Some(id), Ok(mut map)) = (token.id.as_ref(), self.cancellations.lock()) {
            map.remove(id);
        }
    }

    /// Annulla la rifinitura neurale associata a un id. Il flag è controllato
    /// prima della coda, tra un token e l'altro e dopo la risposta del modello.
    pub fn cancel_request(&self, id: &str) {
        if let Ok(mut map) = self.cancellations.lock() {
            if map.len() > 64 {
                map.retain(|_, value| !value.load(Ordering::SeqCst));
            }
            match map.get(id) {
                Some(flag) => flag.store(true, Ordering::SeqCst),
                None => {
                    map.insert(id.to_string(), Arc::new(AtomicBool::new(true)));
                }
            }
        }
    }

    pub fn metrics_snapshot(&self) -> SymbolMetrics {
        self.metrics
            .lock()
            .map(|value| value.clone())
            .unwrap_or_default()
    }

    fn record_metrics(&self, elapsed_ms: u64, cache_hit: bool, neural: bool, neural_ms: u64) {
        if let Ok(mut metrics) = self.metrics.lock() {
            metrics.paragraphs += 1;
            metrics.total_ms = metrics.total_ms.saturating_add(elapsed_ms);
            metrics.last_ms = elapsed_ms;
            metrics.worst_ms = metrics.worst_ms.max(elapsed_ms);
            if cache_hit {
                metrics.cache_hits += 1;
            }
            if neural {
                metrics.neural_paragraphs += 1;
                metrics.neural_ms = metrics.neural_ms.saturating_add(neural_ms);
            }
        }
    }

    fn mark_stale(&self) {
        if let Ok(mut metrics) = self.metrics.lock() {
            metrics.stale_results += 1;
        }
    }

    fn symbolize_internal(
        &self,
        text: &str,
        opts: &SymbolizeOptions,
        use_ai: bool,
        cancel: Option<Arc<AtomicBool>>,
    ) -> Result<ParagraphResult, String> {
        let started = std::time::Instant::now();
        let key = Self::cache_key(opts, text);
        if !use_ai {
            if let Ok(cache) = self.cache.lock() {
                if let Some(hit) = cache.get(&key) {
                    let elapsed = started.elapsed().as_millis() as u64;
                    self.record_metrics(elapsed, true, false, 0);
                    return Ok(hit.clone());
                }
            }
        }

        let split_ranges = char_ranges(text, &opts.split_ranges);
        let manual_ranges = char_ranges(text, &opts.manual_units);
        let mut analysis = Analyzer::with_skips(&self.lexicon, split_ranges).analyze(text);
        for sentence in &mut analysis.sentences {
            apply_manual_units(&self.lexicon, text, &manual_ranges, &mut sentence.tokens);
        }
        let domains = analysis.domains.clone();
        let filters = self.filters(opts);

        // Il blocco del database e della cache SVG resta limitato alla
        // risoluzione lessicale. Prima veniva tenuto anche durante l'inferenza
        // neurale: una rifinitura da secondi bloccava ogni altra lettura.
        let (mut tokens, mut symbolized, mut approximate, mut missing) = {
            let db = self.db.lock().map_err(|_| "database non disponibile".to_string())?;
            let mut svg_cache = self
                .svg_cache
                .lock()
                .map_err(|_| "cache simboli non disponibile".to_string())?;

            let mut tokens: Vec<CaaToken> = Vec::new();
            let mut symbolized = 0usize;
            let mut approximate = 0usize;
            let mut missing = 0usize;
            let mut index = 0usize;

            for sentence in &analysis.sentences {
                for token in &sentence.tokens {
                    if token.span == 0 {
                        continue;
                    }
                    let unit_key = format!("{}:{}", opts.paragraph_id, index);
                    let caa = self.resolve_token(
                        &db,
                        &filters,
                        opts,
                        &analysis.domains,
                        sentence.index,
                        &sentence.text,
                        token,
                        index,
                        &unit_key,
                        &mut svg_cache,
                    )?;
                    if caa.symbol_id.is_some() || !caa.composition.is_empty() {
                        symbolized += 1;
                        if caa.approximate {
                            approximate += 1;
                        }
                    } else if caa.key && !caa.is_punct {
                        missing += 1;
                    }
                    tokens.push(caa);
                    index += 1;
                }
            }

            // Pittogrammi arrivati dalle API e salvati nella libreria: valgono
            // anche per parole fuori lessico. La corrispondenza è esatta su
            // etichetta/parola chiave, mai per somiglianza, così la rianalisi
            // non cancella il simbolo automatico né ne applica uno sbagliato.
            if opts.mode != CaaMode::Testo {
                // Solo la libreria personale: i simboli ARASAAC salvati dalle
                // API (anche automaticamente) e le scelte manuali. Il catalogo
                // completo resta gestito dal percorso lessicale, altrimenti
                // parole che il corpus considera senza simbolo verrebbero
                // collegate solo perché esiste un'etichetta omonima.
                let personal_filters = SymbolFilters {
                    pack_id: Some("personal".to_string()),
                    commercial_only: filters.commercial_only,
                    collection_id: None,
                };
                for caa in tokens.iter_mut() {
                    if caa.symbol_id.is_some()
                        || !caa.composition.is_empty()
                        || caa.is_punct
                        || caa.is_function
                    {
                        continue;
                    }
                    let query = caa.lemma.clone().unwrap_or_else(|| caa.surface.clone());
                    let normalized = crate::linguistics::normalize::normalize_word(&query);
                    if normalized.is_empty() {
                        continue;
                    }
                    let Ok(Some(symbol)) = db.symbol_by_label(&normalized, &personal_filters) else {
                        continue;
                    };
                    caa.symbol_id = Some(symbol.id.clone());
                    caa.pack_id = Some(symbol.pack_id.clone());
                    caa.concept_id = symbol.concept_id.clone();
                    caa.caption = caa.surface.clone();
                    caa.symbol_label = Some(symbol.label.clone());
                    if caa.concept_label.is_none() {
                        caa.concept_label = Some(symbol.label.clone());
                    }
                    caa.meaning = symbol.meaning.clone();
                    caa.strategy = Strategy::Exact.as_str().to_string();
                    caa.confidence = 0.9;
                    caa.image = symbol.image.clone();
                    caa.svg = self.svg_for(&db, &symbol.id, symbol.svg.clone(), &mut svg_cache);
                }
                symbolized = tokens
                    .iter()
                    .filter(|t| t.symbol_id.is_some() || !t.composition.is_empty())
                    .count();
                approximate = tokens.iter().filter(|t| t.approximate).count();
                missing = tokens
                    .iter()
                    .filter(|t| t.key && t.symbol_id.is_none() && t.composition.is_empty())
                    .count();
            }
            (tokens, symbolized, approximate, missing)
        };

        let mut neural_ms = 0u64;
        if use_ai {
            if cancel
                .as_ref()
                .map(|flag| flag.load(Ordering::SeqCst))
                .unwrap_or(false)
            {
                self.mark_stale();
                return Err("richiesta superata: il testo è cambiato".to_string());
            }
            let neural_started = Instant::now();
            self.rerank_with_ai(&filters, &mut tokens, text, cancel.as_ref())?;
            neural_ms = neural_started.elapsed().as_millis() as u64;
            if cancel
                .as_ref()
                .map(|flag| flag.load(Ordering::SeqCst))
                .unwrap_or(false)
            {
                self.mark_stale();
                return Err("richiesta superata: il testo è cambiato".to_string());
            }
            symbolized = tokens
                .iter()
                .filter(|t| t.symbol_id.is_some() || !t.composition.is_empty())
                .count();
            approximate = tokens.iter().filter(|t| t.approximate).count();
            missing = tokens
                .iter()
                .filter(|t| t.key && t.symbol_id.is_none() && t.composition.is_empty())
                .count();
        }

        let elapsed = started.elapsed().as_millis() as u64;
        let result = ParagraphResult {
            paragraph_id: opts.paragraph_id.clone(),
            tokens,
            domains,
            symbolized,
            approximate,
            missing,
            elapsed_ms: elapsed,
        };

        if !use_ai {
            if let Ok(mut cache) = self.cache.lock() {
                if cache.len() >= CACHE_LIMIT {
                    cache.clear();
                }
                cache.insert(key, result.clone());
            }
        }
        self.record_metrics(elapsed, false, use_ai, neural_ms);
        Ok(result)
    }

    #[allow(clippy::too_many_arguments)]
    fn resolve_token(
        &self,
        db: &Database,
        filters: &SymbolFilters,
        opts: &SymbolizeOptions,
        paragraph_domains: &[String],
        sentence_index: usize,
        sentence_text: &str,
        token: &TokenAnalysis,
        index: usize,
        unit_key: &str,
        svg_cache: &mut SvgCache,
    ) -> Result<CaaToken, String> {
        let mut out = CaaToken {
            index,
            surface: token.surface.clone(),
            start: token.start,
            end: token.end,
            lemma: token.lemma.clone(),
            pos: token.pos.clone(),
            concept_id: None,
            concept_label: None,
            meaning: None,
            category: None,
            symbol_id: None,
            pack_id: None,
            caption: token.surface.clone(),
            symbol_label: None,
            svg: None,
            image: None,
            composition: Vec::new(),
            strategy: Strategy::None.as_str().to_string(),
            confidence: 0.0,
            approximate: false,
            locked: false,
            negated: token.negated,
            key: token.key,
            is_punct: token.is_punct,
            is_function: token.is_function,
            span: token.span.max(1),
            sentence_index,
            hidden: false,
            modifiers: UnitModifiers::default(),
            alternatives: Vec::new(),
        };

        if token.is_punct {
            out.strategy = "none".to_string();
            return Ok(out);
        }
        if opts.mode == CaaMode::Testo {
            return Ok(out);
        }
        // Preferenze: documento > contesto > globale (mai sovrascritte).
        // Vanno lette prima del filtro di intensità: una scelta manuale deve
        // valere anche per le parole che l'analisi automatica salterebbe
        // (pos sconosciuta, intensità minima). Sono proprio quelle che
        // l'utente corregge a mano dal pannello.
        let preferences = self.preferences_for(db, token, unit_key, paragraph_domains, opts)?;
        let manual_choice = preferences
            .as_ref()
            .map(|pref| !pref.symbol_id.is_empty())
            .unwrap_or(false);
        // Una parola fuori lessico può comunque avere un pittogramma ARASAAC
        // esatto (es. «volare»): in quel caso non si scarta.
        let arasaac_filters = SymbolFilters {
            pack_id: Some("arasaac".to_string()),
            ..filters.clone()
        };
        let mut arasaac_exact_available = false;
        if !should_symbolize(token, opts)
            && !manual_choice
            && !token.is_function
            && token.concepts.is_empty()
        {
            let mut labels: Vec<&str> = vec![token.surface.as_str()];
            if let Some(lemma) = token.lemma.as_deref() {
                if lemma != token.surface {
                    labels.push(lemma);
                }
            }
            for label in labels {
                if db
                    .symbol_by_label(label, &arasaac_filters)
                    .map_err(|e| e.to_string())?
                    .is_some()
                {
                    arasaac_exact_available = true;
                    break;
                }
            }
        }
        if !should_symbolize(token, opts) && !manual_choice && !arasaac_exact_available {
            return Ok(out);
        }

        // L'override della singola occorrenza (scelta fatta nel pannello) vince
        // su tutto: si applica subito, anche quando la parola non ha un
        // concetto nel lessico o l'analisi automatica produrrebbe un simbolo
        // diverso. Le preferenze globali restano invece subordinate alla
        // disambiguazione contestuale, come prima.
        if let Some(pref) = &preferences {
            if pref.scope == "document"
                && self.apply_preference_symbol(
                    db,
                    filters,
                    &token.surface,
                    pref,
                    svg_cache,
                    &mut out,
                )?
            {
                return Ok(out);
            }
        }

        // Stato dell'unità: visibilità, blocco e modificatori sopravvivono alle
        // rianalisi e alle modifiche del testo. Il blocco non viene mai
        // sovrascritto dall'IA né dalla rivalutazione della frase.
        if !opts.document_key.is_empty() {
            match db.unit_state(&opts.document_key, unit_key) {
                Ok(Some((hidden, modifiers))) => {
                    out.hidden = hidden;
                    out.modifiers = if modifiers.manual {
                        modifiers
                    } else {
                        merge_modifiers(auto_modifiers(token), &modifiers)
                    };
                }
                _ => out.modifiers = auto_modifiers(token),
            }
        } else {
            out.modifiers = auto_modifiers(token);
        }

        // Priorità ARASAAC: un'etichetta esatta ARASAAC (parola o lemma) vince
        // sui concetti lessicali approssimativi. Le scelte esplicite
        // dell'utente (override/preferenze) hanno già vinto sopra.
        if !manual_choice && token.concepts.is_empty() {
            let arasaac = SymbolFilters {
                pack_id: Some("arasaac".to_string()),
                ..filters.clone()
            };
            let mut labels: Vec<&str> = vec![token.surface.as_str()];
            if let Some(lemma) = token.lemma.as_deref() {
                if lemma != token.surface {
                    labels.push(lemma);
                }
            }
            for label in labels {
                if let Some(symbol) = db
                    .symbol_by_label(label, &arasaac)
                    .map_err(|e| e.to_string())?
                {
                    let preference = Preference {
                        scope: "auto-arasaac".to_string(),
                        lemma: token
                            .lemma
                            .clone()
                            .unwrap_or_else(|| token.surface.clone()),
                        context_key: None,
                        symbol_id: symbol.id.clone(),
                        concept_id: symbol.concept_id.clone(),
                        locked: false,
                        created_at: now_iso(),
                    };
                    if self.apply_preference_symbol(
                        db,
                        filters,
                        &token.surface,
                        &preference,
                        svg_cache,
                        &mut out,
                    )? {
                        return Ok(out);
                    }
                }
            }
        }

        // Livello 2-4: candidati lessicali e punteggio contestuale.
        let lemma_for_frequency = token.lemma.clone().unwrap_or_else(|| token.surface.clone());
        let frequency = |concept_id: &str| -> f32 {
            let hits = db.frequency(&format!("concept:{concept_id}")) as f32;
            let lemma_hits = db.frequency(&lemma_for_frequency) as f32;
            ((hits * 0.02) + (lemma_hits * 0.01)).min(0.12)
        };

        let mut ranked = symbol_search::rank_concepts(
            &token.concepts,
            &self.lexicon,
            sentence_text,
            &frequency,
        );

        // Priorità alle collezioni personali: se l'utente ha importato un
        // pittogramma per questa parola, il concetto personale entra in testa
        // ai candidati. Resta un candidato semantico, non una sostituzione
        // cieca: la scelta manuale e le preferenze continuano a vincere.
        let personal_slug = crate::pictogram_import::slugify(
            &crate::linguistics::normalize::normalize_word(&token.surface),
        );
        if !personal_slug.is_empty() {
            let personal_concept = format!("custom.{personal_slug}");
            if !ranked.iter().any(|item| item.concept_id == personal_concept) {
                if let Ok(found) = db.symbols_for_concept(&personal_concept, filters) {
                    if !found.is_empty() {
                        ranked.insert(
                            0,
                            symbol_search::ScoredConcept {
                                concept_id: personal_concept,
                                score: 100.0,
                            },
                        );
                    }
                }
            }
        }

        if let Some(pref) = &preferences {
            if let Some(concept_id) = &pref.concept_id {
                if let Some(pos) = ranked.iter().position(|c| &c.concept_id == concept_id) {
                    let item = ranked.remove(pos);
                    ranked.insert(0, item);
                } else if self.lexicon.concept(concept_id).is_some() {
                    ranked.insert(
                        0,
                        symbol_search::ScoredConcept {
                            concept_id: concept_id.clone(),
                            score: 100.0,
                        },
                    );
                }
            }
        }

        let chosen = ranked.first().map(|c| c.concept_id.clone());
        out.confidence = ranked
            .first()
            .map(|c| confidence_from_score(c.score))
            .unwrap_or(0.0);

        // Nessun concetto nel lessico: prova un simbolo personalizzato.
        let concept_id = match chosen {
            Some(id) => id,
            None => {
                // Anche senza un concetto nel lessico la scelta manuale vince:
                // «mondo» e «governa» non hanno un concept lessicale, ma un
                // override documento/occorrenza deve sopravvivere alla
                // rianalisi invece di essere sostituito dalla ricerca per
                // etichetta.
                if let Some(pref) = &preferences {
                    if self.apply_preference_symbol(
                        db,
                        filters,
                        &token.surface,
                        pref,
                        svg_cache,
                        &mut out,
                    )? {
                        return Ok(out);
                    }
                }
                // Nessun concetto nel lessico: si cerca nella libreria dei
                // pittogrammi (per esempio l'intero catalogo ARASAAC) usando la
                // parola scritta e il suo lemma. Si accetta solo una
                // corrispondenza esatta dell'etichetta: un simbolo sbagliato è
                // peggio di nessun simbolo.
                let mut words = vec![token.normalized.clone()];
                if let Some(lemma) = &token.lemma {
                    let normalized = crate::linguistics::normalize::normalize_word(lemma);
                    if !normalized.is_empty() && !words.contains(&normalized) {
                        words.push(normalized);
                    }
                }
                for word in words {
                    let Some(first) = self.library_match(db, filters, &word) else {
                        continue;
                    };
                    let Ok(Some(symbol)) = db.symbol_by_id(&first.symbol_id) else {
                        continue;
                    };
                    out.symbol_id = Some(symbol.id.clone());
                    out.pack_id = Some(symbol.pack_id.clone());
                    out.concept_id = symbol.concept_id.clone();
                    // La didascalia visibile è la parola scritta; l'etichetta
                    // del repertorio resta in `symbol_label`/tooltip.
                    out.caption = token.surface.clone();
                    out.symbol_label = Some(symbol.label.clone());
                    if out.concept_label.is_none() {
                        out.concept_label = Some(symbol.label.clone());
                    }
                    out.meaning = symbol.meaning.clone();
                    out.strategy = Strategy::Exact.as_str().to_string();
                    out.confidence = 0.4;
                    // Senza questa riga il pittogramma restava invisibile
                    // nell'editor pur essendo presente nella ricerca manuale.
                    out.image = symbol.image.clone();
                    out.svg = self.svg_for(db, &symbol.id, symbol.svg.clone(), svg_cache);
                    out.alternatives = self.build_alternatives(
                        db,
                        filters,
                        &[],
                        symbol.concept_id.as_deref(),
                        svg_cache,
                    )?;
                    for neighbour in
                        self.library_alternatives(db, filters, &symbol.label, &symbol.id, svg_cache)
                    {
                        if out.alternatives.iter().all(|a| a.symbol_id != neighbour.symbol_id) {
                            out.alternatives.push(neighbour);
                        }
                    }
                    return Ok(out);
                }
                // Fallback morfologico prudente: una forma fuori lessico può
                // essere ricondotta a un lemma italiano (principesse →
                // principessa, governa → governare). Il simbolo viene accettato
                // solo se l'etichetta del lemma esiste davvero nel repertorio:
                // nessuna ricerca per prefisso, nessuna somiglianza di lettere.
                // «sento» non può diventare «sentiero», «bravo» non può
                // diventare «patatas bravas».
                if !token.is_function {
                    let normalized = crate::linguistics::normalize::normalize_word(&token.surface);
                    let mut guesses = singular_guesses(&normalized);
                    guesses.extend(morphology::lemma_guesses(&normalized));
                    let mut seen_guesses = HashSet::new();
                    for (guess, rule) in guesses {
                        if guess == normalized || !seen_guesses.insert(guess.clone()) {
                            continue;
                        }
                        let Some(first) = self.library_match(db, filters, &guess) else {
                            continue;
                        };
                        let Ok(Some(symbol)) = db.symbol_by_id(&first.symbol_id) else {
                            continue;
                        };
                        out.symbol_id = Some(symbol.id.clone());
                        out.pack_id = Some(symbol.pack_id.clone());
                        out.concept_id = symbol.concept_id.clone();
                        out.caption = token.surface.clone();
                        out.symbol_label = Some(symbol.label.clone());
                        out.concept_label = Some(symbol.label.clone());
                        out.meaning = Some(format!(
                            "Forma ricondotta a «{}» ({}). Pittogramma approssimativo: verificare il significato.",
                            guess, rule
                        ));
                        out.strategy = Strategy::Related.as_str().to_string();
                        out.approximate = true;
                        out.confidence = 0.3;
                        out.image = symbol.image.clone();
                        out.svg = self.svg_for(db, &symbol.id, symbol.svg.clone(), svg_cache);
                        out.alternatives = self.library_alternatives(
                            db,
                            filters,
                            &symbol.label,
                            &symbol.id,
                            svg_cache,
                        );
                        return Ok(out);
                    }
                }
                out.strategy = "none".to_string();
                return Ok(out);
            }
        };

        let mut resolved = symbol_search::resolve(db, &self.lexicon, filters, &concept_id)?;

        // La scelta manuale vince sempre.
        if let Some(pref) = &preferences {
            let symbol_id = pref.symbol_id.clone();
            if !symbol_id.is_empty() {
                if let Ok(Some(symbol)) = db.symbol_by_id(&symbol_id) {
                    resolved.symbol = Some(symbol);
                    resolved.strategy = Strategy::Exact;
                    resolved.parts.clear();
                    out.locked = pref.locked;
                }
            }
        }

        if let Some(concept) = self.lexicon.concept(&concept_id) {
            out.concept_label = Some(concept.label.clone());
            out.category = Some(concept.category.clone());
            if resolved.strategy == Strategy::Exact {
                out.meaning = Some(concept.meaning.clone());
            }
        }

        out.concept_id = resolved.concept_id.clone();
        out.strategy = resolved.strategy.as_str().to_string();
        out.approximate = resolved.strategy.is_approximate();
        out.composition = resolved.parts.clone();

        if let Some(symbol) = &resolved.symbol {
            out.symbol_id = Some(symbol.id.clone());
            out.pack_id = Some(symbol.pack_id.clone());
            out.caption = token.surface.clone();
            out.symbol_label = Some(symbol.label.clone());
            out.meaning = symbol.meaning.clone().or(out.meaning);
            out.image = symbol.image.clone();
            let svg = self.svg_for(db, &symbol.id, symbol.svg.clone(), svg_cache);
            out.svg = svg;
        } else if !resolved.parts.is_empty() {
            out.caption = token.surface.clone();
        } else {
            // Strategia G: testo senza pittogramma, ma con spiegazione.
            out.strategy = Strategy::None.as_str().to_string();
            out.meaning = resolved.note.clone().or(Some(
                "Nessun pittogramma adeguato: la parola resta senza simbolo.".to_string(),
            ));
        }

        if let Some(note) = &resolved.note {
            if resolved.strategy != Strategy::Exact {
                out.meaning = Some(match &out.meaning {
                    Some(existing) if existing != note => format!("{existing} — {note}"),
                    _ => note.clone(),
                });
            }
        }

        out.alternatives = self.build_alternatives(
            db,
            filters,
            &token.concepts,
            Some(&concept_id),
            svg_cache,
        )?;

        // Concetti nuovi o senza un pittogramma dedicato: cerca l'etichetta
        // del concetto nel repertorio (es. «capire» → «comprendere»). Senza
        // questo ramo la parola restava selezionabile ma senza alcun candidato.
        if out.alternatives.is_empty() {
            if let Some(concept) = self.lexicon.concept(&concept_id) {
                for neighbour in
                    self.library_alternatives(db, filters, &concept.label, "", svg_cache)
                {
                    if out.alternatives.iter().all(|a| a.symbol_id != neighbour.symbol_id) {
                        out.alternatives.push(neighbour);
                    }
                }
            }
        }

        // Vicini della libreria: altri pittogrammi legati alla stessa parola.
        // Il lessico ha 102 concetti; le parole chiave ARASAAC sono l'unica
        // sorgente di somiglianza per tutto il resto del vocabolario.
        if let Some(symbol_id) = out.symbol_id.clone() {
            let caption = out
                .symbol_label
                .clone()
                .unwrap_or_else(|| out.caption.clone());
            for neighbour in
                self.library_alternatives(db, filters, &caption, &symbol_id, svg_cache)
            {
                if out.alternatives.len() >= MAX_ALTERNATIVES {
                    break;
                }
                if out.alternatives.iter().all(|a| a.symbol_id != neighbour.symbol_id) {
                    out.alternatives.push(neighbour);
                }
            }
        }

        if out.approximate {
            out.confidence = (out.confidence * 0.6).clamp(0.0, 1.0);
        }
        Ok(out)
    }

    /// Primo pittogramma della libreria la cui etichetta corrisponde alla
    /// parola (esatta o come parola intera). Usa lo stesso indice FTS della
    /// ricerca manuale, così i due percorsi vedono gli stessi simboli.
    fn library_match(
        &self,
        db: &Database,
        filters: &SymbolFilters,
        word: &str,
    ) -> Option<Alternative> {
        let page = db.search_symbols(word, 8, 0, filters).ok()?;
        let mut hit = page
            .items
            .into_iter()
            .find(|item| label_matches_word(&item.label, word))?;
        if hit.svg.is_none() && hit.image.is_none() {
            // Lettura diretta dal database: la cache dei simboli è già
            // impegnata dal chiamante, bloccarla di nuovo causerebbe uno
            // stallo.
            if let Ok(Some(record)) = db.symbol_by_id(&hit.symbol_id) {
                hit.svg = record.svg;
                hit.image = record.image;
            }
        }
        Some(hit)
    }

    fn svg_for(
        &self,
        _db: &Database,
        symbol_id: &str,
        from_row: Option<String>,
        cache: &mut SvgCache,
    ) -> Option<String> {
        if let Some(cached) = cache.get(symbol_id) {
            // Un None può essere stato memorizzato prima che il file fosse
            // idratato: si accetta solo un valore positivo.
            if cached.is_some() || from_row.is_none() {
                return cached;
            }
        }
        // Il chiamante tiene già il lock della cache (passata come `cache`):
        // riacquisire `self.svg_cache` qui provocava un deadlock quando il
        // simbolo aveva un file non leggibile (per esempio un percorso della
        // vecchia installazione). La cache è la stessa: si usa direttamente.
        let value = from_row;
        if value.is_some() {
            cache.insert(symbol_id.to_string(), value.clone());
        }
        value
    }

    /// Simboli della libreria legati alla stessa parola (parole chiave
    /// ARASAAC): servono come «vicini» quando il lessico non ha concetti.
    fn library_alternatives(
        &self,
        db: &Database,
        filters: &SymbolFilters,
        label: &str,
        exclude: &str,
        svg_cache: &mut SvgCache,
    ) -> Vec<Alternative> {
        const LIMIT: usize = 6;
        let mut out = Vec::new();
        if label.trim().is_empty() {
            return out;
        }
        if let Ok(page) = db.search_symbols(label, (LIMIT + 1) as u32, 0, filters) {
            for mut item in page.items {
                if item.symbol_id == exclude {
                    continue;
                }
                item.approximate = true;
                if item.svg.is_none() {
                    item.svg = self.svg_for(db, &item.symbol_id, None, svg_cache);
                }
                out.push(item);
                if out.len() >= LIMIT {
                    break;
                }
            }
        }
        out
    }

    fn build_alternatives(
        &self,
        db: &Database,
        filters: &SymbolFilters,
        concept_ids: &[String],
        chosen: Option<&str>,
        cache: &mut SvgCache,
    ) -> Result<Vec<Alternative>, String> {
        let mut out: Vec<Alternative> = Vec::new();
        let mut push_from_concept = |concept_id: &str, approximate: bool| -> DbResult<()> {
            for symbol in db.symbols_for_concept(concept_id, filters)? {
                if out.len() >= MAX_ALTERNATIVES {
                    return Ok(());
                }
                if out.iter().any(|a| a.symbol_id == symbol.id) {
                    continue;
                }
                let mut alt = alternative_from_record(&symbol);
                if alt.svg.is_none() {
                    alt.svg = self.svg_for(db, &symbol.id, None, cache);
                }
                alt.approximate = approximate;
                out.push(alt);
            }
            Ok(())
        };

        if let Some(concept_id) = chosen {
            push_from_concept(concept_id, false).map_err(|e| e.to_string())?;
        }
        for concept_id in concept_ids {
            push_from_concept(concept_id, false).map_err(|e| e.to_string())?;
        }
        if let Some(concept_id) = chosen {
            if let Some(broader) = db.broader_of(concept_id).map_err(|e| e.to_string())? {
                push_from_concept(&broader, true).map_err(|e| e.to_string())?;
            }
            for related in db
                .related_concepts(concept_id, 3)
                .map_err(|e| e.to_string())?
            {
                push_from_concept(&related, true).map_err(|e| e.to_string())?;
            }
        }
        Ok(out)
    }

    /// Aggiunge un candidato a un gruppo, evitando i duplicati: lo stesso
    /// pittogramma appartiene al livello più forte in cui compare.
    #[allow(clippy::too_many_arguments)]
    fn push_affinity(
        &self,
        db: &Database,
        cache: &mut SvgCache,
        seen: &mut HashSet<String>,
        out: &mut Vec<AffinityCandidate>,
        mut symbol: Alternative,
        score: u32,
        approximate: bool,
        reason: String,
    ) {
        if symbol.symbol_id.is_empty() || seen.contains(&symbol.symbol_id) {
            return;
        }
        seen.insert(symbol.symbol_id.clone());
        symbol.approximate = approximate;
        symbol.neural = false;
        if symbol.svg.is_none() {
            symbol.svg = self.svg_for(db, &symbol.symbol_id, None, cache);
        }
        out.push(AffinityCandidate {
            symbol,
            score: score.min(100),
            approximate,
            reason,
        });
    }

    /// Candidati raggruppati per livello di affinità a partire da una parola
    /// grezza. Il percorso è deterministico (lessico, morfologia, indice
    /// FTS): nessun candidato viene inventato e nessuno viene applicato al
    /// documento. Per le parole fuori lessico si usano soltanto etichette
    /// realmente presenti nel repertorio, dichiarate approssimative.
    pub fn word_affinities(
        &self,
        word: &str,
        pack_id: Option<String>,
        commercial_only: bool,
    ) -> Result<WordAffinity, String> {
        let trimmed = word.trim();
        if trimmed.is_empty() {
            return Err("Scrivi una parola da cercare.".to_string());
        }
        if trimmed.chars().count() > 80 {
            return Err("La parola è troppo lunga: cerca un singolo termine.".to_string());
        }
        let normalized = crate::linguistics::normalize::normalize_word(trimmed);
        if normalized.is_empty() {
            return Err("La parola non contiene lettere o numeri.".to_string());
        }

        let filters = SymbolFilters {
            pack_id,
            commercial_only,
            collection_id: None,
        };
        let db = self.db()?;
        let mut cache = SvgCache::new(64);
        let mut seen: HashSet<String> = HashSet::new();

        let mut exact: Vec<AffinityCandidate> = Vec::new();
        let mut lemma_candidates: Vec<AffinityCandidate> = Vec::new();
        let mut variants: Vec<AffinityCandidate> = Vec::new();
        let mut synonyms: Vec<AffinityCandidate> = Vec::new();
        let mut general: Vec<AffinityCandidate> = Vec::new();
        let mut related: Vec<AffinityCandidate> = Vec::new();

        let mut matches = self.lexicon.lookup(trimmed);
        if matches.is_empty() {
            matches = self.lexicon.lookup_folded(&normalized);
        }
        let best = matches
            .iter()
            .max_by_key(|m| (m.concepts.len(), m.from_lemma))
            .cloned();

        let synonym_concepts = self
            .lexicon
            .synonyms
            .get(&normalized)
            .cloned()
            .unwrap_or_default();
        let via_synonym = !synonym_concepts.is_empty();
        // Espressioni come «non so» o «da sole» non sono lemmi singoli: il
        // lessico le mappa a un concetto. Senza questo ramo la ricerca per
        // affinità non le riconoscerebbe affatto.
        let mwe_concept = self.lexicon.mwe.get(&normalized).cloned();
        let via_mwe = mwe_concept.is_some();
        let mut concepts = best
            .as_ref()
            .map(|m| m.concepts.clone())
            .unwrap_or_default();
        if concepts.is_empty() {
            if !synonym_concepts.is_empty() {
                concepts = synonym_concepts.clone();
            } else if let Some(concept) = &mwe_concept {
                concepts.push(concept.clone());
            }
        }

        let lemma = best.as_ref().map(|m| m.lemma.clone());
        let lemma_norm = lemma
            .as_deref()
            .map(crate::linguistics::normalize::normalize_word)
            .filter(|value| !value.is_empty());

        // 1. Etichette esattamente uguali alla parola: la corrispondenza più
        //    forte, indipendentemente dal lessico.
        if let Ok(page) = db.search_symbols(&normalized, 60, 0, &filters) {
            for alt in page.items {
                if crate::linguistics::normalize::normalize_word(&alt.label) != normalized {
                    continue;
                }
                let reason = if alt.meaning.trim().is_empty() {
                    format!("L'etichetta del pittogramma è «{}».", alt.label)
                } else {
                    format!("L'etichetta è «{}» — {}.", alt.label, alt.meaning.trim())
                };
                self.push_affinity(
                    &db,
                    &mut cache,
                    &mut seen,
                    &mut exact,
                    alt,
                    100,
                    false,
                    reason,
                );
            }
        }

        // 2. Concetti collegati dal lessico alla parola. Il livello dipende
        //    dal rapporto tra etichetta del concetto, parola e lemma.
        for concept_id in &concepts {
            let Some(concept) = self.lexicon.concept(concept_id) else {
                continue;
            };
            let label_norm = crate::linguistics::normalize::normalize_word(&concept.label);
            let (level, score, approximate, reason) = if label_norm == normalized {
                (
                    AffinityLevel::Exact,
                    100u32,
                    false,
                    format!(
                        "«{}» è il concetto della parola: {}.",
                        concept.label, concept.meaning
                    ),
                )
            } else if via_mwe {
                (
                    AffinityLevel::Variant,
                    80,
                    true,
                    format!(
                        "Espressione «{}» ricondotta al concetto «{}».",
                        trimmed, concept.label
                    ),
                )
            } else if via_synonym {
                (
                    AffinityLevel::Synonym,
                    70,
                    false,
                    format!(
                        "«{}» è un sinonimo dichiarato nel lessico di «{}».",
                        trimmed, concept.label
                    ),
                )
            } else if lemma_norm.as_deref() == Some(label_norm.as_str()) {
                (
                    AffinityLevel::Lemma,
                    90,
                    false,
                    format!("Forma base «{}»: {}.", concept.label, concept.meaning),
                )
            } else if lemma.is_some() {
                (
                    AffinityLevel::Lemma,
                    85,
                    false,
                    format!(
                        "Il lessico collega «{}» al concetto «{}».",
                        trimmed, concept.label
                    ),
                )
            } else {
                (
                    AffinityLevel::Synonym,
                    65,
                    true,
                    format!("Concetto collegato nel lessico: «{}».", concept.label),
                )
            };
            let target = match level {
                AffinityLevel::Exact => &mut exact,
                AffinityLevel::Lemma => &mut lemma_candidates,
                AffinityLevel::Variant => &mut variants,
                AffinityLevel::Synonym => &mut synonyms,
                AffinityLevel::General => &mut general,
                AffinityLevel::Related => &mut related,
            };
            let symbols = db
                .symbols_for_concept(concept_id, &filters)
                .map_err(|e| e.to_string())?;
            for record in symbols {
                self.push_affinity(
                    &db,
                    &mut cache,
                    &mut seen,
                    target,
                    alternative_from_record(&record),
                    score,
                    approximate,
                    reason.clone(),
                );
            }
        }

        // 3. Varianti. Per le parole nel lessico si usa il paradigma
        //    morfologico reale; fuori lessico solo poche regole non ambigue,
        //    accettate soltanto se l'etichetta esiste davvero.
        let mut variant_forms: Vec<(String, String, bool)> = Vec::new();
        if let (Some(lemma), Some(pos)) = (
            lemma.as_ref(),
            best.as_ref().map(|m| m.pos.clone()),
        ) {
            let gender = best.as_ref().and_then(|m| m.gender.clone());
            let mut forms: Vec<String> =
                morphology::generate_forms(lemma, &pos, gender.as_deref())
                    .into_iter()
                    .map(|form| crate::linguistics::normalize::normalize_word(&form.form))
                    .filter(|form| {
                        !form.is_empty()
                            && form != &normalized
                            && Some(form) != lemma_norm.as_ref()
                    })
                    .collect();
            forms.sort();
            forms.dedup();
            for form in forms.into_iter().take(8) {
                variant_forms.push((
                    form.clone(),
                    format!("Forma flessa «{}» del lemma «{}».", form, lemma),
                    false,
                ));
            }
        } else {
            for (guess, rule) in singular_guesses(&normalized).into_iter().take(4) {
                variant_forms.push((
                    guess.clone(),
                    format!(
                        "Forma singolare probabile «{}» ({}), non confermata dal lessico.",
                        guess, rule
                    ),
                    true,
                ));
            }
        }
        for (form, reason, approximate) in variant_forms {
            let Ok(page) = db.search_symbols(&form, 8, 0, &filters) else {
                continue;
            };
            for alt in page.items {
                if crate::linguistics::normalize::normalize_word(&alt.label) != form {
                    continue;
                }
                self.push_affinity(
                    &db,
                    &mut cache,
                    &mut seen,
                    &mut variants,
                    alt,
                    if approximate { 75 } else { 80 },
                    approximate,
                    reason.clone(),
                );
            }
        }

        // 4. Sinonimi dichiarati: altri termini che puntano agli stessi
        //    concetti, più i concetti equivalenti del database e del lessico.
        for concept_id in &concepts {
            let concept_label = self
                .lexicon
                .concept(concept_id)
                .map(|concept| concept.label.clone())
                .unwrap_or_else(|| concept_id.clone());
            for (term, mapped) in &self.lexicon.synonyms {
                if term == &normalized || !mapped.iter().any(|id| id == concept_id) {
                    continue;
                }
                let Ok(page) = db.search_symbols(term, 8, 0, &filters) else {
                    continue;
                };
                for alt in page.items {
                    if crate::linguistics::normalize::normalize_word(&alt.label) != *term {
                        continue;
                    }
                    self.push_affinity(
                        &db,
                        &mut cache,
                        &mut seen,
                        &mut synonyms,
                        alt,
                        70,
                        false,
                        format!("«{}» è un sinonimo di «{}».", term, concept_label),
                    );
                }
            }

            let mut equivalents: Vec<String> =
                db.equivalent_concepts(concept_id, 8).unwrap_or_default();
            equivalents.extend(
                self.lexicon
                    .relations
                    .iter()
                    .filter(|(from, _, kind, _)| from == concept_id && kind == "equivalent")
                    .map(|(_, to, _, _)| to.clone()),
            );
            equivalents.sort();
            equivalents.dedup();
            for equivalent in equivalents {
                let label = self
                    .lexicon
                    .concept(&equivalent)
                    .map(|concept| concept.label.clone())
                    .unwrap_or_else(|| equivalent.clone());
                let Ok(records) = db.symbols_for_concept(&equivalent, &filters) else {
                    continue;
                };
                for record in records {
                    self.push_affinity(
                        &db,
                        &mut cache,
                        &mut seen,
                        &mut synonyms,
                        alternative_from_record(&record),
                        70,
                        false,
                        format!(
                            "«{}» è un concetto equivalente a «{}».",
                            label, concept_label
                        ),
                    );
                }
            }
        }

        // 5. Concetto più generale: un solo livello, quello dichiarato dal
        //    lessico. Risalire più livelli produrrebbe proposte vaghe.
        for concept_id in &concepts {
            let broader = db
                .broader_of(concept_id)
                .ok()
                .flatten()
                .or_else(|| {
                    self.lexicon
                        .concept(concept_id)
                        .and_then(|concept| concept.broader.clone())
                });
            let Some(broader) = broader else {
                continue;
            };
            let broader_label = self
                .lexicon
                .concept(&broader)
                .map(|concept| concept.label.clone())
                .unwrap_or_else(|| broader.clone());
            let concept_label = self
                .lexicon
                .concept(concept_id)
                .map(|concept| concept.label.clone())
                .unwrap_or_else(|| concept_id.clone());
            let Ok(records) = db.symbols_for_concept(&broader, &filters) else {
                continue;
            };
            for record in records {
                self.push_affinity(
                    &db,
                    &mut cache,
                    &mut seen,
                    &mut general,
                    alternative_from_record(&record),
                    60,
                    true,
                    format!(
                        "«{}» è un concetto più generale di «{}».",
                        broader_label, concept_label
                    ),
                );
            }
        }

        // 6. Concetti collegati e somiglianze nel testo dell'etichetta. Sono
        //    i candidati più deboli: sempre dichiarati approssimativi.
        for concept_id in &concepts {
            let concept_label = self
                .lexicon
                .concept(concept_id)
                .map(|concept| concept.label.clone())
                .unwrap_or_else(|| concept_id.clone());
            let mut linked: Vec<(String, f32)> = db
                .related_concepts(concept_id, 4)
                .unwrap_or_default()
                .into_iter()
                .map(|id| (id, 1.0))
                .collect();
            linked.extend(
                self.lexicon
                    .relations
                    .iter()
                    .filter(|(from, _, kind, _)| from == concept_id && kind == "related")
                    .map(|(_, to, _, weight)| (to.clone(), *weight)),
            );
            for (linked_id, weight) in linked {
                let linked_label = self
                    .lexicon
                    .concept(&linked_id)
                    .map(|concept| concept.label.clone())
                    .unwrap_or_else(|| linked_id.clone());
                let score = (50.0 * weight).round().clamp(35.0, 55.0) as u32;
                let Ok(records) = db.symbols_for_concept(&linked_id, &filters) else {
                    continue;
                };
                for record in records {
                    self.push_affinity(
                        &db,
                        &mut cache,
                        &mut seen,
                        &mut related,
                        alternative_from_record(&record),
                        score,
                        true,
                        format!(
                            "«{}» è un concetto collegato a «{}».",
                            linked_label, concept_label
                        ),
                    );
                }
            }
        }

        // Nessun candidato viene creato dalla sola somiglianza ortografica o
        // dall'indice FTS. «sento» non è «sentiero», «bravo» non è «patatas
        // bravas», «non» non è «nonna»: la vicinanza di lettere non è una
        // relazione semantica. Gli unici gruppi ammessi sono quelli costruiti
        // sopra (etichetta esatta, lemma, variante morfologica, sinonimo
        // dichiarato, concetto equivalente/generale/collegato nel lessico).

        let mut groups = Vec::new();
        for (level, mut candidates) in [
            (AffinityLevel::Exact, exact),
            (AffinityLevel::Lemma, lemma_candidates),
            (AffinityLevel::Variant, variants),
            (AffinityLevel::Synonym, synonyms),
            (AffinityLevel::General, general),
            (AffinityLevel::Related, related),
        ] {
            if candidates.is_empty() {
                continue;
            }
            candidates.sort_by(|a, b| {
                b.score
                    .cmp(&a.score)
                    .then_with(|| a.symbol.label.to_lowercase().cmp(&b.symbol.label.to_lowercase()))
            });
            groups.push(AffinityGroup {
                level,
                title: level.title().to_string(),
                approximate: candidates.iter().any(|candidate| candidate.approximate),
                candidates,
            });
        }

        let known = best.is_some() || via_mwe || !groups.is_empty();
        let explanation = if groups.is_empty() {
            format!(
                "Nessun pittogramma del repertorio corrisponde a «{}». La parola resta senza \
                 simbolo: puoi importare un SVG dalla scheda Alternative.",
                trimmed
            )
        } else if via_mwe {
            format!(
                "«{}» è un'espressione riconosciuta dal lessico. I gruppi sono ordinati dal più \
                 forte al più debole; i punteggi sono un indice interno, non una probabilità.",
                trimmed
            )
        } else if via_synonym && lemma_norm.as_deref() == Some(normalized.as_str()) {
            format!(
                "«{}» è un sinonimo dichiarato nel lessico: i candidati sono i pittogrammi del \
                 concetto corrispondente.",
                trimmed
            )
        } else if let Some(base) = lemma
            .as_deref()
            .filter(|base| crate::linguistics::normalize::normalize_word(base) != normalized)
        {
            format!(
                "La parola è stata ricondotta alla forma base «{}». I gruppi sono ordinati dal \
                 più forte al più debole; i punteggi sono un indice interno, non una probabilità.",
                base
            )
        } else {
            "I gruppi sono ordinati dal più forte al più debole; i punteggi sono un indice \
             interno, non una probabilità. I gruppi marcati approssimati vanno verificati."
                .to_string()
        };

        Ok(WordAffinity {
            word: trimmed.to_string(),
            normalized,
            lemma,
            known,
            explanation,
            groups,
        })
    }

    /// Applica alla risposta il pittogramma scelto esplicitamente dall'utente
    /// (override del documento o preferenza). Restituisce `false` se la
    /// preferenza non ha un simbolo utilizzabile, così il chiamante può
    /// proseguire con la risoluzione automatica.
    #[allow(clippy::too_many_arguments)]
    fn apply_preference_symbol(
        &self,
        db: &Database,
        filters: &SymbolFilters,
        surface: &str,
        preference: &Preference,
        svg_cache: &mut SvgCache,
        out: &mut CaaToken,
    ) -> Result<bool, String> {
        if preference.symbol_id.is_empty() {
            return Ok(false);
        }
        let Ok(Some(symbol)) = db.symbol_by_id(&preference.symbol_id) else {
            return Ok(false);
        };
        out.symbol_id = Some(symbol.id.clone());
        out.pack_id = Some(symbol.pack_id.clone());
        out.concept_id = symbol.concept_id.clone();
        out.caption = surface.to_string();
        out.symbol_label = Some(symbol.label.clone());
        out.concept_label = Some(symbol.label.clone());
        out.meaning = symbol.meaning.clone();
        out.strategy = Strategy::Exact.as_str().to_string();
        out.confidence = 1.0;
        out.locked = preference.locked;
        out.image = symbol.image.clone();
        out.svg = self.svg_for(db, &symbol.id, symbol.svg.clone(), svg_cache);
        out.alternatives =
            self.build_alternatives(db, filters, &[], symbol.concept_id.as_deref(), svg_cache)?;
        for neighbour in self.library_alternatives(
            db,
            filters,
            &symbol.label,
            &symbol.id,
            svg_cache,
        ) {
            if out.alternatives.iter().all(|a| a.symbol_id != neighbour.symbol_id) {
                out.alternatives.push(neighbour);
            }
        }
        Ok(true)
    }

    fn preferences_for(
        &self,
        db: &Database,
        token: &TokenAnalysis,
        unit_key: &str,
        domains: &[String],
        opts: &SymbolizeOptions,
    ) -> Result<Option<Preference>, String> {
        let lemma = token
            .lemma
            .clone()
            .unwrap_or_else(|| token.normalized.clone());

        if !opts.document_key.is_empty() {
            if let Some((symbol_id, concept_id)) = db
                .document_override(&opts.document_key, unit_key)
                .map_err(|e| e.to_string())?
            {
                return Ok(Some(Preference {
                    scope: "document".to_string(),
                    lemma,
                    context_key: None,
                    symbol_id: symbol_id.unwrap_or_default(),
                    concept_id,
                    locked: true,
                    created_at: now_iso(),
                }));
            }
        }

        // Preferenza del profilo attivo: vale solo per questo Utilizzatore.
        // Prima del contesto, perché è una scelta esplicita e circoscritta.
        if let Some(profile_id) = opts.profile_id.as_deref().filter(|id| !id.is_empty()) {
            for concept_id in token.concepts.iter().map(Some).chain(std::iter::once(None)) {
                let key = match concept_id {
                    Some(id) => format!("profile:{profile_id}:concept:{id}"),
                    None => format!("profile:{profile_id}"),
                };
                if let Some(pref) = db
                    .preference("profile", &lemma, &key)
                    .map_err(|e| e.to_string())?
                {
                    return Ok(Some(pref));
                }
            }
        }

        for domain in domains {
            if let Some(pref) = db
                .preference("context", &lemma, domain)
                .map_err(|e| e.to_string())?
            {
                return Ok(Some(pref));
            }
        }

        // Preferenza legata al concetto: «penna» strumento e «penna» piuma
        // restano associazioni distinte. La chiave usa il concetto, non solo
        // la stringa del lemma.
        for concept_id in &token.concepts {
            let key = format!("concept:{concept_id}");
            if let Some(pref) = db
                .preference("global", &lemma, &key)
                .map_err(|e| e.to_string())?
            {
                return Ok(Some(pref));
            }
        }

        if let Some(pref) = db
            .preference("global", &lemma, "")
            .map_err(|e| e.to_string())?
        {
            return Ok(Some(pref));
        }

        // Vocabolario condiviso del Creator: l'ultima risorsa, dopo le scelte
        // del singolo profilo e le associazioni globali.
        for concept_id in &token.concepts {
            let key = format!("concept:{concept_id}");
            if let Some(pref) = db
                .preference("vocabulary", &lemma, &key)
                .map_err(|e| e.to_string())?
            {
                return Ok(Some(pref));
            }
        }
        db.preference("vocabulary", &lemma, "")
            .map_err(|e| e.to_string())
    }

    /// Passata neurale di scrittura. Tre usi, in ordine di valore:
    ///
    /// 1. parola senza pittogramma: il modello propone parole affini **nel
    ///    contesto della frase** e si cerca un simbolo per quelle;
    /// 2. parole chiave già simbolizzate: si aggiungono i pittogrammi del
    ///    campo semantico della frase, così l'utente vede più affini;
    /// 3. token ambigui con confidenza bassa: il modello può correggere la
    ///    scelta deterministica, mai ribaltare una scelta sicura.
    ///
    /// Il budget limita le inferenze per paragrafo: la digitazione non deve
    /// bloccarsi. Tutto ciò che arriva dal modello è marcato `neural` e
    /// `approximate`; nessuna preferenza bloccata viene toccata.
    fn rerank_with_ai(
        &self,
        filters: &SymbolFilters,
        tokens: &mut [CaaToken],
        text: &str,
        cancel: Option<&Arc<AtomicBool>>,
    ) -> Result<(), String> {
        let cancelled = |flag: Option<&Arc<AtomicBool>>| {
            flag.map(|value| value.load(Ordering::SeqCst)).unwrap_or(false)
        };
        if cancelled(cancel) {
            return Err("richiesta superata: il testo è cambiato".to_string());
        }
        // La comunicazione vocale ha priorità sulle elaborazioni differibili:
        // mentre la voce legge, non si avvia una seconda inferenza pesante.
        // La simbolizzazione lessicale già fatta resta valida; la rifinitura
        // riparte alla prossima pausa di scrittura.
        if crate::speech::is_speaking() {
            return Ok(());
        }
        let config = self
            .ai_config
            .lock()
            .map(|c| c.clone())
            .map_err(|_| "configurazione IA non disponibile".to_string())?;
        if !config.enabled || !ai_engine::is_available(&config) {
            return Ok(());
        }

        // Budget: una richiesta per il campo semantico, le restanti per le
        // parole senza simbolo e per le ambiguità. Otto inferenze al massimo
        // per paragrafo, sempre in background.
        let mut budget: usize = 8;
        let mut hard_error: Option<String> = None;

        // (1) Campo semantico della frase: una sola inferenza condivisa.
        let mut field_symbols: Vec<Alternative> = Vec::new();
        if budget > 0 {
            budget -= 1;
            match self.ask_neural(&ai_engine::semantic_field_prompt(text), 96, cancel) {
                Ok(raw) => {
                    let words = ai_engine::parse_semantic_field(&raw);
                    // Il database viene bloccato solo per la lettura dei
                    // simboli, mai durante l'inferenza.
                    if let Ok(db) = self.db.lock() {
                        field_symbols = self.library_alternatives_for_words(&db, filters, &words, 6);
                    }
                }
                Err(err) => hard_error = Some(err),
            }
        }

        for token in tokens.iter_mut() {
            if cancelled(cancel) {
                return Err("richiesta superata: il testo è cambiato".to_string());
            }
            if token.locked || token.is_punct {
                continue;
            }
            if hard_error.is_some() {
                break;
            }

            // (2) Parola senza simbolo: ricerca per significato, con il
            // contesto della frase. Resta dichiarata come approssimazione.
            if token.symbol_id.is_none() && token.composition.is_empty() {
                if !token.key || budget == 0 {
                    continue;
                }
                budget -= 1;
                let raw = match self.ask_neural(
                    &ai_engine::related_words_prompt(&token.surface, Some(text)),
                    64,
                    cancel,
                ) {
                    Ok(raw) => raw,
                    Err(err) => {
                        hard_error = Some(err);
                        break;
                    }
                };
                let words = ai_engine::parse_related_words(&raw, ai_engine::RELATED_WORDS_LIMIT);
                let found = {
                    let db = self
                        .db
                        .lock()
                        .map_err(|_| "database non disponibile".to_string())?;
                    self.library_alternatives_for_words(&db, filters, &words, 4)
                };
                if let Some(best) = found.first() {
                    token.symbol_id = Some(best.symbol_id.clone());
                    token.pack_id = Some(best.pack_id.clone());
                    token.svg = best.svg.clone();
                    token.image = best.image.clone();
                    token.symbol_label = Some(best.label.clone());
                    token.concept_id = best.concept_id.clone();
                    token.strategy = Strategy::Related.as_str().to_string();
                    token.approximate = true;
                    token.confidence = 0.35;
                    token.meaning = Some(format!(
                        "simbolo vicino a «{}» proposto dal modello locale nel contesto della frase",
                        token.surface
                    ));
                }
                for alt in found {
                    if token.alternatives.len() >= MAX_ALTERNATIVES {
                        break;
                    }
                    if token.alternatives.iter().all(|a| a.symbol_id != alt.symbol_id) {
                        token.alternatives.push(alt);
                    }
                }
                continue;
            }

            // (3) Simbolo già scelto: si aggiungono gli affini del campo
            // semantico dove il repertorio deterministico è più povero.
            if token.key
                && !token.is_function
                && token.alternatives.len() < 4
                && !field_symbols.is_empty()
            {
                let surface = crate::linguistics::normalize::normalize_word(&token.surface);
                let caption = crate::linguistics::normalize::normalize_word(
                    token.symbol_label.as_deref().unwrap_or(""),
                );
                let mut added = 0;
                for alt in &field_symbols {
                    if added >= 3 || token.alternatives.len() >= MAX_ALTERNATIVES {
                        break;
                    }
                    if token.alternatives.iter().any(|a| a.symbol_id == alt.symbol_id) {
                        continue;
                    }
                    let label = crate::linguistics::normalize::normalize_word(&alt.label);
                    if label == surface || label == caption {
                        continue;
                    }
                    let mut alt = alt.clone();
                    alt.neural = true;
                    alt.approximate = true;
                    token.alternatives.push(alt);
                    added += 1;
                }
            }

            // (4) Correzione contestuale solo dove la scelta è incerta.
            if budget == 0 || token.alternatives.len() < 2 || token.confidence >= 0.55 {
                continue;
            }
            let candidates: Vec<ai_engine::Candidate> = token
                .alternatives
                .iter()
                .filter_map(|alt| {
                    alt.concept_id.as_ref().map(|concept_id| ai_engine::Candidate {
                        concept_id: concept_id.clone(),
                        label: alt.label.clone(),
                        meaning: alt.meaning.clone(),
                        category: String::new(),
                    })
                })
                .collect();
            if candidates.len() < 2 {
                continue;
            }
            budget -= 1;
            let prompt = ai_engine::build_prompt(text, &token.surface, &candidates);
            match self
                .ask_neural(&prompt, 32, cancel)
                .and_then(|raw| ai_engine::validate_answer(&raw, &candidates, ""))
            {
                Ok(choice) => {
                    if choice.concept_id != token.concept_id.clone().unwrap_or_default() {
                        token.concept_id = Some(choice.concept_id.clone());
                        token.strategy = Strategy::Exact.as_str().to_string();
                        token.approximate = false;
                        if let Some(alt) = token
                            .alternatives
                            .iter()
                            .find(|a| a.concept_id.as_deref() == Some(&choice.concept_id))
                        {
                            token.symbol_id = Some(alt.symbol_id.clone());
                            token.pack_id = Some(alt.pack_id.clone());
                            token.svg = alt.svg.clone();
                            token.image = alt.image.clone();
                            token.symbol_label = Some(alt.label.clone());
                            token.meaning = Some(format!(
                                "{} (scelta contestuale del modello locale)",
                                alt.meaning
                            ));
                            token.confidence = choice.confidence;
                        }
                    }
                }
                Err(err) => {
                    hard_error = Some(err);
                    break;
                }
            }
        }

        if let Some(err) = hard_error {
            if let Ok(mut slot) = self.ai_error.lock() {
                *slot = Some(err);
            }
        } else if let Ok(mut slot) = self.ai_error.lock() {
            // Una passata riuscita cancella l'errore precedente: lo stato
            // mostrato nell'interfaccia deve dire la verità.
            *slot = None;
        }
        Ok(())
    }

    /// Una richiesta al servizio condiviso con la priorità della scrittura.
    fn ask_neural(
        &self,
        prompt: &str,
        max_tokens: u32,
        cancel: Option<&Arc<AtomicBool>>,
    ) -> Result<String, String> {
        let cancel = cancel
            .cloned()
            .unwrap_or_else(|| self.ai_service.cancel_handle());
        self.ai_service
            .complete(prompt.to_string(), max_tokens, 7, cancel)
    }

    /// Cerca nella libreria i pittogrammi delle parole indicate, senza
    /// duplicati. Le alternative prodotte sono marcate come neurali: sono
    /// suggerimenti, non sostituzioni automatiche.
    /// Come `library_match`, ma prova anche le forme base (fissare →
    /// assicurare) per le parole proposte dal modello. Accetta solo
    /// un'etichetta reale del repertorio: nessuna somiglianza inventata.
    fn library_match_neural(
        &self,
        db: &Database,
        filters: &SymbolFilters,
        word: &str,
    ) -> Option<Alternative> {
        if let Some(hit) = self.library_match(db, filters, word) {
            return Some(hit);
        }
        let normalized = crate::linguistics::normalize::normalize_word(word);
        if normalized.chars().count() < 3 {
            return None;
        }
        let mut guesses = singular_guesses(&normalized);
        guesses.extend(crate::linguistics::morphology::lemma_guesses(&normalized));
        let mut seen = HashSet::new();
        for (guess, _) in guesses {
            if guess == normalized || !seen.insert(guess.clone()) {
                continue;
            }
            if let Some(hit) = self.library_match(db, filters, &guess) {
                return Some(hit);
            }
        }
        None
    }

    fn library_alternatives_for_words(
        &self,
        db: &Database,
        filters: &SymbolFilters,
        words: &[String],
        limit: usize,
    ) -> Vec<Alternative> {
        let mut out: Vec<Alternative> = Vec::new();
        for word in words {
            if out.len() >= limit {
                break;
            }
            let Some(mut alt) = self.library_match_neural(db, filters, word) else {
                continue;
            };
            if out.iter().any(|item| item.symbol_id == alt.symbol_id) {
                continue;
            }
            alt.neural = true;
            alt.approximate = true;
            out.push(alt);
        }
        out
    }

    /// Suggerimenti di simboli affini a una parola o a una frase, su richiesta
    /// esplicita dell'utente (scheda «Cerca»). Restano dichiarati come
    /// approssimativi e non modificano il documento.
    pub fn ai_related_symbols(&self, text: &str) -> Result<Vec<Alternative>, String> {
        let text = text.trim();
        if text.is_empty() {
            return Err("scrivi una parola o una frase da cercare".to_string());
        }
        let config = self.ai_config();
        if !config.enabled || !ai_engine::is_available(&config) {
            return Err(
                "motore neurale non disponibile: attivalo in Impostazioni → Gestione modelli"
                    .to_string(),
            );
        }
        // Una parola chiede i suoi affini; una frase chiede direttamente le
        // parole concrete che contiene, come fa la passata di scrittura.
        let is_phrase = text.split_whitespace().count() > 1;
        let prompt = if is_phrase {
            ai_engine::semantic_field_prompt(text)
        } else {
            ai_engine::related_words_prompt(text, None)
        };
        let raw = self
            .ai_service
            .complete(prompt, 96, 10, self.ai_service.cancel_handle())
            .map_err(|e| format!("il modello non ha risposto: {e}"))?;
        let limit = if is_phrase {
            8
        } else {
            ai_engine::RELATED_WORDS_LIMIT
        };
        let words = ai_engine::parse_related_words(&raw, limit);
        if words.is_empty() {
            return Err("il modello non ha proposto parole affini utilizzabili".to_string());
        }
        let db = self.db.lock().map_err(|_| "database non disponibile".to_string())?;
        let filters = SymbolFilters {
            pack_id: None,
            commercial_only: false,
            collection_id: None,
        };
        let found = self.library_alternatives_for_words(&db, &filters, &words, 8);
        if found.is_empty() {
            return Ok(Vec::new());
        }
        Ok(found)
    }

    /// Sinonimi e concetti affini proposti dal modello locale, usati in
    /// background per arricchire le alternative del pannello. Non modifica il
    /// documento, non richiede rete e non blocca la digitazione: la richiesta
    /// ha priorità bassa nella coda del motore. Se il modello non è attivo
    /// restituisce un elenco vuoto, senza errori.
    pub fn ai_affinity_symbols(&self, word: &str) -> Vec<Alternative> {
        let word = crate::linguistics::normalize::normalize_word(word);
        if word.is_empty() || word.split_whitespace().count() > 1 {
            return Vec::new();
        }
        let config = self.ai_config();
        if !config.enabled || !ai_engine::is_available(&config) {
            return Vec::new();
        }
        use std::sync::OnceLock;
        static CACHE: OnceLock<Mutex<HashMap<String, Vec<Alternative>>>> = OnceLock::new();
        let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
        if let Ok(guard) = cache.lock() {
            if let Some(value) = guard.get(&word) {
                return value.clone();
            }
        }
        let found = self.ai_affinity_symbols_uncached(&word);
        if let Ok(mut guard) = cache.lock() {
            if guard.len() >= 128 {
                guard.clear();
            }
            guard.insert(word, found.clone());
        }
        found
    }

    /// Parole e brevi espressioni proposte dal modello per una parola.
    /// Estratto a parte per poterle ispezionare dalla CLI senza toccare il
    /// documento né il database.
    pub fn ai_suggested_words(&self, word: &str) -> Vec<String> {
        let prompt = ai_engine::related_words_prompt(word, None);
        // Priorità 1: sotto le richieste interattive (priorità 10). La coda
        // del motore serve prima la scrittura e solo quando è libera i
        // suggerimenti di sinonimi.
        let Ok(raw) = self
            .ai_service
            .complete(prompt, 40, 1, self.ai_service.cancel_handle())
        else {
            return Vec::new();
        };
        // Fino a quattro parole: il modello può proporre espressioni come
        // «gettare l'ancora», che il repertorio può contenere come etichetta.
        ai_engine::parse_synonyms(&raw, 12)
    }

    /// Sinonimi inglesi proposti dal modello locale, usati solo come ripiego
    /// per Global Symbols. Non modificano il documento e non richiedono rete
    /// oltre alla ricerca successiva.
    pub fn ai_english_synonyms(&self, word: &str) -> Vec<String> {
        let word = word.trim();
        if word.is_empty() || word.split_whitespace().count() > 4 {
            return Vec::new();
        }
        let config = self.ai_config();
        if !config.enabled || !ai_engine::is_available(&config) {
            return Vec::new();
        }
        let prompt = ai_engine::english_synonyms_prompt(word);
        let Ok(raw) = self
            .ai_service
            .complete(prompt, 40, 1, self.ai_service.cancel_handle())
        else {
            return Vec::new();
        };
        ai_engine::parse_synonyms(&raw, 8)
            .into_iter()
            .filter(|candidate| {
                let normalized = crate::linguistics::normalize::normalize_word(candidate);
                !normalized.is_empty() && normalized != crate::linguistics::normalize::normalize_word(word)
            })
            .collect()
    }

    fn ai_affinity_symbols_uncached(&self, word: &str) -> Vec<Alternative> {
        let words: Vec<String> = self
            .ai_suggested_words(word)
            .into_iter()
            .filter(|candidate| {
                let normalized = crate::linguistics::normalize::normalize_word(candidate);
                !normalized.is_empty() && normalized != word
            })
            .collect();
        if words.is_empty() {
            return Vec::new();
        }
        let Ok(db) = self.db.lock() else {
            return Vec::new();
        };
        let filters = SymbolFilters {
            pack_id: None,
            commercial_only: false,
            collection_id: None,
        };
        let mut found = self.library_alternatives_for_words(&db, &filters, &words, 10);
        found.retain(|alternative| {
            crate::linguistics::normalize::normalize_word(&alternative.label) != word
        });
        found
    }

    pub fn ai_status(&self) -> AiStatus {
        let config = self.ai_config.lock().map(|c| c.clone()).unwrap_or_default();
        let last_error = self.ai_error.lock().ok().and_then(|e| e.clone());
        let ready = config.enabled
            && std::path::Path::new(&config.binary_path).exists()
            && std::path::Path::new(&config.model_path).exists();
        let active_model = self.active_model_label(&config);
        let active_model_bundled = self
            .installed_entry_for(&config.model_path)
            .map(|entry| entry.bundled)
            .unwrap_or(false);
        AiStatus {
            enabled: config.enabled,
            binary_present: std::path::Path::new(&config.binary_path).exists(),
            model_present: std::path::Path::new(&config.model_path).exists(),
            binary_path: config.binary_path.clone(),
            model_path: config.model_path.clone(),
            backend: config.backend.clone(),
            threads: config.threads,
            context_size: config.context_size,
            timeout_ms: config.timeout_ms,
            last_error,
            hardware: self.hardware.clone(),
            metrics: self.ai_service.metrics(),
            server_running: self.ai_model.is_running(),
            active_model,
            active_model_bundled,
            note: if ready {
                format!(
                    "Motore neurale attivo con «{}» (llama.cpp): durante la scrittura propone \
                     simboli affini e può correggere le scelte incerte. Le preferenze bloccate non \
                     vengono mai sovrascritte e il motore resta disattivabile. Senza modello la \
                     selezione deterministica continua a funzionare.",
                    self.active_model_label(&config)
                )
            } else {
                "Motore neurale non disponibile: manca il runtime llama.cpp o un modello. \
                 Puoi scaricarlo dalla Gestione modelli o importare un file GGUF; l'applicazione \
                 usa la selezione deterministica e la scrittura resta completa."
                    .to_string()
            },
        }
    }

    /// Misure reali dei processi e della pipeline. La memoria viene letta dal
    /// sistema operativo solo quando il Creator apre il pannello Diagnostica.
    pub fn diagnostics(&self) -> RuntimeDiagnostics {
        let model_pid = self.ai_model.server_pid();
        let config = self.ai_config.lock().map(|c| c.clone()).unwrap_or_default();
        RuntimeDiagnostics {
            version: env!("CARGO_PKG_VERSION").to_string(),
            platform: format!("{} / {}", std::env::consts::OS, std::env::consts::ARCH),
            app_process: crate::diagnostics::current_process(),
            model_process: model_pid.map(crate::diagnostics::sample),
            server_running: self.ai_model.is_running(),
            speech_active: crate::speech::is_speaking(),
            ai: self.ai_service.metrics(),
            symbols: self.metrics_snapshot(),
            active_model: self.active_model_label(&config),
        }
    }

    /// Ferma il processo `llama-server` all'uscita dell'applicazione. Tauri
    /// non esegue i distruttori dello stato gestito: senza questa chiamata il
    /// modello resterebbe in memoria dopo la chiusura della finestra.
    pub fn shutdown(&self) {
        self.ai_service.shutdown();
        crate::speech::stop();
    }

    pub fn set_ai_config(&self, config: AiConfig) -> Result<(), String> {
        let db = self.db.lock().map_err(|_| "database non disponibile".to_string())?;
        db.set_setting(
            "ai.config",
            &serde_json::to_string(&config).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        if let Ok(mut slot) = self.ai_config.lock() {
            *slot = config.clone();
        }
        // Il cambio di modello scarica l'istanza precedente: non si tengono due
        // modelli in memoria su una macchina da 4 GB.
        self.ai_model.set_config(config);
        // Le risposte in cache appartenevano al modello precedente.
        self.ai_service.invalidate_cache();
        Ok(())
    }

    pub fn ai_hardware(&self) -> ai_engine::hardware::HardwareProfile {
        self.hardware.clone()
    }

    pub fn ai_models(&self) -> Vec<ai_engine::catalog::ModelEntry> {
        ai_engine::catalog::catalog()
            .into_iter()
            .map(|mut entry| {
                let path = self.model_path_for(&entry);
                entry.installed = path.is_file();
                entry.local_path = entry.installed.then(|| path.display().to_string());
                entry
            })
            .collect()
    }

    /// Percorso atteso di un modello del catalogo: incluso in `resources/ai`,
    /// scaricato o importato in `models/` dei dati utente.
    pub fn model_path_for(&self, entry: &ai_engine::catalog::ModelEntry) -> std::path::PathBuf {
        if entry.bundled {
            self.paths.ai_dir().join(&entry.file_name)
        } else {
            self.installer.models_dir().join(&entry.file_name)
        }
    }

    fn installed_entry_for(
        &self,
        model_path: &str,
    ) -> Option<ai_engine::catalog::ModelEntry> {
        let target = std::path::Path::new(model_path);
        let canonical = target.canonicalize().ok();
        ai_engine::catalog::catalog().into_iter().find(|entry| {
            let candidate = self.model_path_for(entry);
            candidate == target
                || (canonical.is_some() && candidate.canonicalize().ok() == canonical)
        })
    }

    fn active_model_label(&self, config: &AiConfig) -> String {
        if let Some(entry) = self.installed_entry_for(&config.model_path) {
            return entry.name;
        }
        std::path::Path::new(&config.model_path)
            .file_stem()
            .and_then(|stem| stem.to_str())
            .filter(|stem| !stem.is_empty())
            .unwrap_or("nessun modello")
            .to_string()
    }

    /// Attiva un modello del catalogo. Sono selezionabili solo i modelli
    /// inclusi o già verificati e presenti sul disco: un modello elencato ma
    /// non scaricato viene rifiutato con un messaggio che dice cosa fare.
    pub fn ai_select_model(&self, model_id: &str) -> Result<(), String> {
        let entry = ai_engine::catalog::find(model_id)
            .ok_or_else(|| format!("modello sconosciuto: {model_id}"))?;
        if !(entry.bundled || entry.installable()) {
            return Err(
                "modello non installabile: URL e impronta sha256 non sono stati verificati da questo progetto"
                    .to_string(),
            );
        }
        let model = self.model_path_for(&entry);
        if !model.is_file() {
            return Err(format!(
                "«{}» non è ancora installato. Scaricalo dalla Gestione modelli oppure importa un \
                 file GGUF già presente sul computer.",
                entry.name
            ));
        }
        let mut config = self.ai_config();
        config.model_path = model.display().to_string();
        config.enabled = true;
        self.set_ai_config(config)
    }

    /// Avvia il download verificato di un modello. Ritorna subito lo stato
    /// iniziale: l'interfaccia interroga `ai_install_status` per il progresso.
    pub fn ai_install_model(
        &self,
        model_id: &str,
    ) -> Result<ai_engine::installer::InstallStatus, String> {
        let entry = ai_engine::catalog::find(model_id)
            .ok_or_else(|| format!("modello sconosciuto: {model_id}"))?;
        if entry.bundled {
            return Err("questo modello è già incluso nell'applicazione".to_string());
        }
        if !entry.installable() {
            return Err(format!(
                "«{}» non è installabile: URL o impronta sha256 non verificati",
                entry.name
            ));
        }
        if self.model_path_for(&entry).is_file() {
            return Ok(ai_engine::installer::InstallStatus {
                model_id: entry.id.clone(),
                phase: ai_engine::installer::InstallPhase::Ready,
                downloaded_bytes: entry.size_bytes,
                total_bytes: entry.size_bytes,
                percent: 100.0,
                message: "Modello già presente sul disco.".to_string(),
            });
        }
        self.installer.start(&entry)?;
        Ok(self.installer.status())
    }

    pub fn ai_install_status(&self) -> ai_engine::installer::InstallStatus {
        self.installer.status()
    }

    pub fn ai_cancel_install(&self) {
        self.installer.cancel();
    }

    /// Rimuove un modello scaricato o importato. Se era quello attivo, si
    /// torna al modello incluso nell'applicazione quando esiste.
    pub fn ai_remove_model(&self, model_id: &str) -> Result<AiStatus, String> {
        let entry = ai_engine::catalog::find(model_id)
            .ok_or_else(|| format!("modello sconosciuto: {model_id}"))?;
        let removed_path = self.model_path_for(&entry);
        self.installer.remove(&entry)?;
        let config = self.ai_config();
        let active = std::path::Path::new(&config.model_path);
        let same_file = active == removed_path
            || (active.canonicalize().is_ok()
                && active.canonicalize().ok() == removed_path.canonicalize().ok());
        if same_file {
            let bundled = ai_engine::catalog::catalog()
                .into_iter()
                .find(|candidate| candidate.bundled)
                .map(|candidate| self.model_path_for(&candidate));
            let mut next = config;
            match bundled.filter(|path| path.is_file()) {
                Some(path) => next.model_path = path.display().to_string(),
                None => {
                    next.enabled = false;
                    next.model_path.clear();
                }
            }
            self.set_ai_config(next)?;
        }
        Ok(self.ai_status())
    }

    /// Importa un modello GGUF già scaricato dall'utente (per esempio da un
    /// altro computer, senza rete). Il file viene validato e copiato nella
    /// cartella modelli, poi attivato.
    pub fn ai_import_model(&self, path: &str) -> Result<AiStatus, String> {
        let source = std::path::Path::new(path);
        let target = self.installer.import(source)?;
        let mut config = self.ai_config();
        config.model_path = target.display().to_string();
        config.enabled = true;
        self.set_ai_config(config)?;
        Ok(self.ai_status())
    }

    /// Prova rapida sul modello attivo: misura latenza reale, non stime.
    pub fn ai_benchmark(&self) -> Result<BenchmarkResult, String> {
        let config = self.ai_config();
        if !ai_engine::is_available(&config) {
            return Err("modello non disponibile: configura runtime e GGUF".to_string());
        }
        let prompts = [
            "Scegli il significato di «pesca» in: Il nonno va a pesca. Opzioni: 1. pesca — frutto 2. pesca — attività. Rispondi SOLO con il numero.\nRisposta:",
            "Scegli il significato di «banco» in: Il bambino si siede al banco. Opzioni: 1. banco — scuola 2. banco — banca. Rispondi SOLO con il numero.\nRisposta:",
            "Riscrivi in italiano semplice: Il bambino che ha finito i compiti corre nel parco perché è contento.\nRisposta:",
        ];
        let mut total_ms = 0u128;
        let mut failures = 0u32;
        let mut runs = 0u32;
        for prompt in prompts {
            let started = Instant::now();
            let cancel = self.ai_service.cancel_handle();
            match self.ai_service.complete(prompt.to_string(), 24, 1, cancel) {
                Ok(_) => {
                    total_ms += started.elapsed().as_millis();
                    runs += 1;
                }
                Err(_) => failures += 1,
            }
        }
        let average_ms = if runs == 0 { 0 } else { (total_ms / runs as u128) as u64 };
        Ok(BenchmarkResult {
            model: config.model_path.clone(),
            runs,
            failures,
            average_ms,
            // I token non sono contati dal server: si dichiara la stima solo
            // quando esiste una misura, altrimenti resta 0.
            tokens_per_second: 0.0,
            note: if runs == 0 {
                "Nessuna prova completata: il modello non ha risposto.".to_string()
            } else {
                "Latenza misurata su tre richieste reali; i token/s non sono misurati dal server."
                    .to_string()
            },
        })
    }

    pub fn run_ai_tool(
        &self,
        tool: &str,
        input: &serde_json::Value,
    ) -> Result<ai_engine::tools::ToolOutcome, String> {
        ai_engine::tools::run(self.ai_service.clone(), tool, input)
    }

    /// Traduce una parola o una breve espressione di ricerca in italiano.
    /// Il risultato è usato solo per interrogare l'indice locale: il testo
    /// digitato dall'utente non viene modificato. Se il modello non è
    /// disponibile si restituisce un errore esplicito, non una traduzione
    /// inventata.
    pub fn translate_search_term(&self, text: &str) -> Result<String, String> {
        let term = text.trim();
        if term.is_empty() {
            return Err("serve una parola o una breve espressione da tradurre".to_string());
        }
        if term.chars().count() > 80 || term.lines().count() > 1 {
            return Err("la traduzione accetta una parola o una breve espressione".to_string());
        }
        // Il glossario copre il vocabolario CAA di base e rende la ricerca
        // inglese affidabile anche con modelli piccoli. Per il resto si usa
        // il modello locale, senza inventare risultati.
        if let Some(translated) = translation_glossary(term) {
            return Ok(translated.to_string());
        }
        let config = self.ai_config();
        if !ai_engine::is_available(&config) {
            return Err(
                "La traduzione inglese richiede un modello neurale attivo (Impostazioni → Gestione modelli)."
                    .to_string(),
            );
        }
        let prompt = format!(
            "Sei un traduttore per un vocabolario CAA. Traduci in italiano la parola o la breve \
             espressione inglese. Rispondi SOLO con la traduzione italiana, senza virgolette, \
             senza spiegazioni e senza punto finale. Se il testo è già italiano, ripetilo. \
             Se non è traducibile, ripeti il testo originale.\nTesto: {term}\nTraduzione:"
        );
        let cancel = self.ai_service.cancel_handle();
        // Priorità bassa: la traduzione serve alle alternative inglesi, non
        // deve precedere la simbolizzazione mentre si scrive.
        let raw = self.ai_service.complete(prompt, 24, 2, cancel)?;
        let first = raw.lines().next().unwrap_or("").trim();
        let cleaned = first
            .trim_matches(|ch| matches!(ch, '"' | '\'' | '«' | '»' | '“' | '”'))
            .trim()
            .trim_end_matches(['.', '!', '?', ';', ':'])
            .trim();
        let cleaned = cleaned
            .strip_prefix("Traduzione:")
            .or_else(|| cleaned.strip_prefix("traduzione:"))
            .unwrap_or(cleaned)
            .trim();
        if cleaned.is_empty() || cleaned.chars().count() > 80 {
            return Err("il modello non ha prodotto una traduzione utilizzabile".to_string());
        }
        Ok(cleaned.to_string())
    }

    /// Traduce in inglese il termine selezionato per cercare pittogrammi nei
    /// cataloghi in inglese. Usa il glossario quando copre la parola, altrimenti
    /// il modello neurale locale. Se nessuno dei due è disponibile restituisce
    /// un errore esplicito: non si inventa una traduzione.
    pub fn translate_to_english(&self, text: &str) -> Result<String, String> {
        let term = text.trim();
        if term.is_empty() {
            return Err("serve una parola o una breve espressione da tradurre".to_string());
        }
        if term.chars().count() > 80 || term.lines().count() > 1 {
            return Err("la traduzione accetta una parola o una breve espressione".to_string());
        }
        if let Some(translated) = translation_to_english_glossary(term) {
            return Ok(translated.to_string());
        }
        let config = self.ai_config();
        if !ai_engine::is_available(&config) {
            return Err(
                "La traduzione inglese richiede un modello neurale attivo (Impostazioni → \
                 Gestione modelli) oppure una parola del glossario CAA."
                    .to_string(),
            );
        }
        let prompt = format!(
            "Sei un traduttore per un vocabolario CAA. Traduci in inglese la parola o la breve \
             espressione italiana. Rispondi SOLO con la traduzione inglese, senza virgolette, \
             senza spiegazioni e senza punto finale. Se il testo è già inglese, ripetilo. \
             Se non è traducibile, ripeti il testo originale.\nTesto: {term}\nTraduzione:"
        );
        let cancel = self.ai_service.cancel_handle();
        // Priorità bassa: la traduzione serve alle alternative inglesi, non
        // deve precedere la simbolizzazione mentre si scrive.
        let raw = self.ai_service.complete(prompt, 24, 2, cancel)?;
        let first = raw.lines().next().unwrap_or("").trim();
        let cleaned = first
            .trim_matches(|ch| matches!(ch, '"' | '\'' | '«' | '»' | '“' | '”'))
            .trim()
            .trim_end_matches(['.', '!', '?', ';', ':'])
            .trim();
        let cleaned = cleaned
            .strip_prefix("Traduzione:")
            .or_else(|| cleaned.strip_prefix("traduzione:"))
            .or_else(|| cleaned.strip_prefix("Translation:"))
            .or_else(|| cleaned.strip_prefix("translation:"))
            .unwrap_or(cleaned)
            .trim();
        if cleaned.is_empty() || cleaned.chars().count() > 80 {
            return Err("il modello non ha prodotto una traduzione utilizzabile".to_string());
        }
        Ok(cleaned.to_string())
    }

    /// Chat: usa il provider online configurato se presente, altrimenti il
    /// modello locale. Nessun messaggio viene inviato se non c'è un modello:
    /// non si simula una risposta.
    pub fn chat(
        &self,
        messages: Vec<crate::ai_provider::ChatMessage>,
    ) -> Result<crate::ai_provider::ChatReply, String> {
        if messages.is_empty() {
            return Err("scrivi un messaggio".to_string());
        }
        let db = self.db()?;
        let provider = crate::ai_provider::load(&db);
        if provider.kind != "local" {
            return crate::ai_provider::chat(&db, &messages);
        }
        drop(db);
        let config = self.ai_config();
        if !ai_engine::is_available(&config) {
            return Err(
                "Per chattare serve un modello locale attivo (Impostazioni → Gestione modelli) \
                 oppure un provider online configurato."
                    .to_string(),
            );
        }
        let started = Instant::now();
        let mut prompt = String::from(
            "Sei un assistente per la Comunicazione Aumentativa e Alternativa. Rispondi in \
             italiano, in modo chiaro, concreto e breve. Non inventare dati personali.\n",
        );
        for message in messages.iter().rev().take(12).rev() {
            let role = if message.role == "assistant" { "Assistente" } else { "Utente" };
            prompt.push_str(&format!("{role}: {}\n", message.content.trim()));
        }
        prompt.push_str("Assistente:");
        let cancel = self.ai_service.cancel_handle();
        let reply = self.ai_service.complete(prompt, 384, 6, cancel)?;
        Ok(crate::ai_provider::ChatReply {
            reply: reply.trim().to_string(),
            backend: format!("locale · {}", self.active_model_label(&config)),
            elapsed_ms: started.elapsed().as_millis() as u64,
        })
    }

    /// Interpreta una richiesta di modifica del documento. Il parser
    /// deterministico è sempre disponibile; il modello locale interviene solo
    /// se il parser non riconosce nulla e l'utente lo consente.
    pub fn interpret_document_prompt(
        &self,
        prompt: &str,
        use_neural: bool,
    ) -> crate::document_edit::DocumentEditPlan {
        let mut plan = crate::document_edit::interpret(prompt);
        if plan.operations.is_empty() && use_neural {
            let config = self.ai_config();
            if config.enabled {
                let system = "Sei un interprete di richieste per un editor CAA. Rispondi SOLO con un oggetto JSON: {\"operations\":[...]}. Operazioni ammesse: {\"type\":\"set_uppercase\",\"value\":true|false}, {\"type\":\"scale_font_size\",\"factor\":0.5..2}, {\"type\":\"set_font_size\",\"value\":8..72}, {\"type\":\"scale_symbol_size\",\"factor\":0.5..2}, {\"type\":\"set_symbol_gap\",\"value\":0..64}, {\"type\":\"set_chip_border_color\",\"value\":\"#rrggbb\"}, {\"type\":\"set_alignment\",\"value\":\"left|center|right|justify\"}, {\"type\":\"insert_table\",\"rows\":1..12,\"columns\":1..12}, {\"type\":\"add_table_row\"}, {\"type\":\"add_table_column\"}, {\"type\":\"generate_board\",\"prompt\":\"...\"}. Non inventare altre operazioni. Se non capisci, restituisci operations vuoto.";
                let cancel = self.ai_service.cancel_handle();
                match self.ai_service.complete(
                    format!("{system}\n\nRichiesta: {prompt}"),
                    384,
                    2,
                    cancel,
                ) {
                    Ok(raw) => {
                        let neural = crate::document_edit::parse_model_plan(&raw);
                        if !neural.operations.is_empty() {
                            plan = neural;
                        } else {
                            plan.warnings.extend(neural.warnings);
                            plan.source = "neurale".to_string();
                        }
                    }
                    Err(error) => plan.warnings.push(format!(
                        "Modello non disponibile ({error}): resta valido il parser deterministico."
                    )),
                }
            } else {
                plan.warnings.push(
                    "Nessun modello neurale attivo: la richiesta è stata interpretata dalle regole locali."
                        .to_string(),
                );
            }
        }
        plan
    }

    pub fn ai_config(&self) -> AiConfig {
        self.ai_config.lock().map(|c| c.clone()).unwrap_or_default()
    }

    /// Applica operazioni validate al layout e restituisce le pagine
    /// ricalcolate. Gli elementi bloccati non vengono toccati.
    pub fn edit_layout(
        &self,
        mut spec: crate::layout::LayoutSpecification,
        prompt: Option<String>,
        operations: Option<Vec<crate::layout_edit::LayoutOperation>>,
    ) -> Result<crate::layout_edit::LayoutEditResult, String> {
        crate::layout::validate(&spec)?;
        let mut notes = Vec::new();
        let operations = match operations {
            Some(operations) => operations,
            None => {
                let prompt = prompt.unwrap_or_default();
                if prompt.trim().is_empty() {
                    return Err("serve una richiesta o un elenco di operazioni".to_string());
                }
                let (operations, parser_notes) =
                    crate::layout_edit::operations_from_prompt(&prompt, &spec);
                notes.extend(parser_notes);
                operations
            }
        };
        let applied = crate::layout_edit::apply(&mut spec, &operations)?;
        let document = self.layout_document(spec)?;
        notes.extend(document.notes);
        Ok(crate::layout_edit::LayoutEditResult {
            spec: document.spec,
            pages: document.pages,
            notes,
            operations: applied,
        })
    }

    /// Costruisce un documento impaginato: valida la specifica, risolve i
    /// pittogrammi mancanti con lo stesso motore semantico della scrittura e
    /// calcola la geometria in millimetri. Il modello non decide misure.
    pub fn layout_document(
        &self,
        mut spec: crate::layout::LayoutSpecification,
    ) -> Result<crate::layout::LayoutDocument, String> {
        crate::layout::validate(&spec)?;
        let mut notes = Vec::new();
        for element in &mut spec.elements {
            if element.hidden || element.svg.is_some() || element.label.trim().is_empty() {
                continue;
            }
            let opts = SymbolizeOptions {
                paragraph_id: format!("layout:{}", element.id),
                intensity: Intensity::Minima,
                mode: CaaMode::Sopra,
                ..Default::default()
            };
            match self.symbolize(&element.label, &opts) {
                Ok(result) => {
                    let token = result.tokens.iter().find(|t| {
                        !t.is_punct && (t.symbol_id.is_some() || !t.composition.is_empty())
                    });
                    match token {
                        Some(token) => {
                            element.concept_id = token.concept_id.clone();
                            element.symbol_id = token.symbol_id.clone();
                            element.svg = token.svg.clone();
                            element.image = token.image.clone();
                            if token.composition.len() > 1 {
                                notes.push(format!(
                                    "«{}» è un'espressione di {} pittogrammi: nel layout compare l'unità principale.",
                                    element.label,
                                    token.composition.len()
                                ));
                            }
                        }
                        None => notes.push(format!(
                            "«{}» non ha un pittogramma adeguato: resta il testo.",
                            element.label
                        )),
                    }
                }
                Err(error) => notes.push(format!("«{}»: {error}", element.label)),
            }
        }
        let pages = crate::layout::paginate(&spec)?;
        // Un collegamento deve puntare a una pagina che esiste davvero.
        for element in &spec.elements {
            if let Some(action) = &element.action {
                if action.action_type == "navigate" {
                    let target = action.target_page.unwrap_or(0) as usize;
                    if target >= pages.len() {
                        return Err(format!(
                            "{}: la pagina di destinazione {} non esiste (pagine: {})",
                            element.id,
                            target + 1,
                            pages.len()
                        ));
                    }
                }
            }
        }
        Ok(crate::layout::LayoutDocument {
            spec,
            pages,
            notes,
        })
    }

    /// Vocabolario di base per l'editor Utilizzatore: parole frequenti della
    /// comunicazione quotidiana risolte nel repertorio reale. Un vocabolo
    /// senza pittogramma viene omesso, non inventato.
    pub fn common_symbols(&self, limit: usize) -> Result<Vec<Alternative>, String> {
        const WORDS: &[&str] = &[
            "io", "tu", "noi", "mangiare", "bere", "giocare", "dormire", "leggere",
            "scrivere", "parlare", "ascoltare", "andare", "venire", "fare", "volere",
            "potere", "aiuto", "bene", "male", "felice", "triste", "arrabbiato",
            "paura", "stanco", "casa", "scuola", "mamma", "papà", "nonna", "nonno",
            "amico", "bambino", "cane", "gatto", "acqua", "pane", "mela", "pasta",
            "gelato", "bagno", "cucina", "parco", "libro", "palla", "telefono",
            "sole", "pioggia", "notte", "giorno", "grande", "piccolo", "caldo",
            "freddo", "sì", "no", "grazie", "ciao", "più", "finire", "aprire",
            "chiudere", "fuori", "dentro", "sopra", "sotto",
        ];
        let db = self.db.lock().map_err(|_| "database non disponibile".to_string())?;
        let filters = SymbolFilters {
            pack_id: None,
            commercial_only: false,
            collection_id: None,
        };
        let mut out: Vec<Alternative> = Vec::new();
        let mut seen = HashSet::new();
        for word in WORDS {
            if out.len() >= limit {
                break;
            }
            let Ok(page) = db.search_symbols(word, 6, 0, &filters) else {
                continue;
            };
            let wanted = crate::linguistics::normalize::normalize_word(word);
            if let Some(item) = page.items.into_iter().find(|item| {
                crate::linguistics::normalize::normalize_word(&item.label) == wanted
            }) {
                if seen.insert(item.symbol_id.clone()) {
                    out.push(item);
                }
            }
        }
        Ok(out)
    }

    pub fn clear_caches(&self) {
        if let Ok(mut cache) = self.cache.lock() {
            cache.clear();
        }
        if let Ok(mut svg) = self.svg_cache.lock() {
            svg.clear();
        }
    }

    pub fn db(&self) -> Result<std::sync::MutexGuard<'_, Database>, String> {
        self.db.lock().map_err(|_| "database non disponibile".to_string())
    }
}

fn confidence_from_score(score: f32) -> f32 {
    // Non è una probabilità calibrata: è un indice interno di preferenza.
    let normalized = 1.0 / (1.0 + (-(score - 1.0)).exp());
    normalized.clamp(0.05, 0.99)
}

/// Modificatori deducibili dal testo. I tempi verbali non sono ancora esposti
/// dall'analizzatore morfologico: past e future restano manuali e lo dichiara
/// l'interfaccia. `manual` è falso: sono una proposta, non una scelta.
fn auto_modifiers(token: &TokenAnalysis) -> UnitModifiers {
    UnitModifiers {
        plural: matches!(token.number.as_deref(), Some("pl")),
        negation: token.negated,
        past: false,
        future: false,
        manual: false,
    }
}

/// Unisce i modificatori automatici con quelli già presenti sull'unità: la
/// configurazione manuale non viene mai spenta dal ricalcolo automatico.
fn merge_modifiers(auto: UnitModifiers, stored: &UnitModifiers) -> UnitModifiers {
    UnitModifiers {
        plural: auto.plural || stored.plural,
        negation: auto.negation || stored.negation,
        past: auto.past || stored.past,
        future: auto.future || stored.future,
        manual: false,
    }
}

/// Classificazione chiusa delle parole grammaticali italiane: serve a
/// simbolizzarle solo se l'utente ha attivato la categoria corrispondente.
fn grammatical_kind(word: &str) -> Option<&'static str> {
    const ARTICLES: &[&str] = &[
        "il", "lo", "la", "l", "i", "gli", "le", "un", "uno", "una", "un'", "gl'",
        "dell'", "all'", "dall'", "nell'", "sull'",
    ];
    const PREPOSITIONS: &[&str] = &[
        "di", "a", "da", "in", "con", "su", "per", "tra", "fra", "senza", "sotto", "sopra",
        "del", "dello", "della", "dei", "degli", "delle", "al", "allo", "alla", "ai", "agli",
        "alle", "dal", "dallo", "dalla", "dai", "dagli", "dalle", "nel", "nello", "nella",
        "nei", "negli", "nelle", "sul", "sullo", "sulla", "sui", "sugli", "sulle", "vers",
    ];
    const CONJUNCTIONS: &[&str] = &[
        "e", "ed", "o", "od", "ma", "però", "perche", "perché", "che", "quando", "mentre",
        "se", "come", "anche", "oppure", "né", "ne", "quindi", "perciò",
    ];
    if ARTICLES.contains(&word) {
        Some("article")
    } else if PREPOSITIONS.contains(&word) {
        Some("preposition")
    } else if CONJUNCTIONS.contains(&word) {
        Some("conjunction")
    } else {
        None
    }
}

fn should_symbolize(token: &TokenAnalysis, opts: &SymbolizeOptions) -> bool {
    if token.is_punct {
        return false;
    }
    // Parole grammaticali: solo se l'utente ha attivato quella categoria.
    // Le unità multiword restano escluse da questo ramo: l'analizzatore
    // sostituisce le parti con un unico token, quindi la locuzione ha
    // precedenza sulla singola parola.
    if token.is_function {
        return match grammatical_kind(&token.normalized) {
            Some("article") => opts.symbolize_articles,
            Some("preposition") => opts.symbolize_prepositions,
            Some("conjunction") => opts.symbolize_conjunctions,
            _ => false,
        };
    }
    let intensity = if opts.mode == CaaMode::Semplificata {
        Intensity::Minima
    } else {
        opts.intensity
    };
    match intensity {
        Intensity::Minima => {
            token.key && matches!(token.pos.as_str(), "noun" | "verb" | "adj" | "mwe")
        }
        Intensity::Media => token.key && !token.is_function,
        Intensity::Completa => !token.is_function,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn function_token(surface: &str) -> TokenAnalysis {
        TokenAnalysis {
            surface: surface.to_string(),
            start: 0,
            end: surface.len(),
            normalized: surface.to_lowercase(),
            lemma: None,
            pos: "fun".to_string(),
            gender: None,
            number: None,
            concepts: Vec::new(),
            key: false,
            is_punct: false,
            is_function: true,
            negated: false,
            mwe_concept: None,
            mwe_parts: Vec::new(),
            span: 1,
        }
    }

    #[test]
    fn grammatical_toggles_are_independent() {
        let mut opts = SymbolizeOptions::default();
        opts.intensity = Intensity::Media;
        let mut token = function_token("con");
        // Senza interruttore la preposizione non viene simbolizzata.
        assert!(!should_symbolize(&token, &opts));
        opts.symbolize_prepositions = true;
        assert!(should_symbolize(&token, &opts));
        // Attivare le preposizioni non attiva anche gli articoli.
        token = function_token("il");
        assert!(!should_symbolize(&token, &opts));
        opts.symbolize_articles = true;
        assert!(should_symbolize(&token, &opts));
        // Le congiunzioni restano indipendenti.
        token = function_token("e");
        assert!(!should_symbolize(&token, &opts));
        opts.symbolize_conjunctions = true;
        assert!(should_symbolize(&token, &opts));
    }

    #[test]
    fn auto_modifiers_mark_plural_and_negation_without_forcing_manual() {
        let mut token = function_token("bambini");
        token.is_function = false;
        token.number = Some("pl".to_string());
        token.negated = true;
        let auto = auto_modifiers(&token);
        assert!(auto.plural && auto.negation && !auto.manual);
        // Un modificatore scelto dall'utente non viene spento dal ricalcolo.
        let stored = UnitModifiers {
            past: true,
            ..Default::default()
        };
        let merged = merge_modifiers(auto, &stored);
        assert!(merged.past && merged.plural && merged.negation);
        // Con configurazione manuale il motore usa quella, senza aggiungere altro.
        let manual = UnitModifiers {
            future: true,
            manual: true,
            ..Default::default()
        };
        assert!(manual.manual && manual.future && !manual.plural);
    }
    use crate::paths::AppPaths;

    fn test_resources() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("resources")
    }

    fn arasaac_pack_available() -> bool {
        test_resources()
            .join("symbol-packs")
            .join("arasaac")
            .exists()
    }

    fn test_engine() -> Engine {
        use std::sync::OnceLock;
        // Il database popolato (lessico, pacchetto core e intero catalogo
        // ARASAAC) viene costruito una volta per processo: importare migliaia
        // di pittogrammi a ogni test costerebbe minuti senza verificare nulla
        // di più. Ogni test riceve comunque una copia indipendente.
        static TEMPLATE: OnceLock<std::path::PathBuf> = OnceLock::new();
        let template = TEMPLATE.get_or_init(|| {
            let dir = tempfile::tempdir().unwrap();
            let dir_path = dir.path().to_path_buf();
            std::mem::forget(dir);
            let paths = AppPaths::new(dir_path.clone(), test_resources());
            let engine = Engine::new(paths).unwrap();
            let target = dir_path.join("template.sqlite3");
            {
                let db = engine.db().unwrap();
                db.conn()
                    .execute(
                        "VACUUM INTO ?1",
                        rusqlite::params![target.to_string_lossy().to_string()],
                    )
                    .unwrap();
            }
            drop(engine);
            target
        });

        let dir = tempfile::tempdir().unwrap();
        let paths = AppPaths::new(dir.path().to_path_buf(), test_resources());
        std::fs::copy(template, &paths.db_path).unwrap();
        // Il TempDir deve restare vivo: lo dimentichiamo volutamente nel test.
        std::mem::forget(dir);
        Engine::new(paths).unwrap()
    }

    fn symbolize(engine: &Engine, text: &str) -> ParagraphResult {
        symbolize_in_pack(engine, text, None)
    }

    fn symbolize_in_pack(engine: &Engine, text: &str, pack: Option<&str>) -> ParagraphResult {
        let opts = SymbolizeOptions {
            paragraph_id: "p1".to_string(),
            pack_id: pack.map(|id| id.to_string()),
            ..Default::default()
        };
        engine.symbolize(text, &opts).unwrap()
    }

    fn concept_of<'a>(result: &'a ParagraphResult, surface: &str) -> Option<&'a str> {
        result
            .tokens
            .iter()
            .find(|t| t.surface.eq_ignore_ascii_case(surface))
            .and_then(|t| t.concept_id.as_deref())
    }

    /// Caso di riferimento del capitolato: «da sole» non è il Sole.
    #[test]
    fn off_lexicon_word_uses_exact_library_symbol_without_hanging() {
        if !arasaac_pack_available() {
            eprintln!("catalogo ARASAAC assente: test saltato (scripts/fetch_arasaac.py)");
            return;
        }
        let engine = test_engine();
        let options = SymbolizeOptions {
            intensity: Intensity::Completa,
            paragraph_id: "p1".to_string(),
            ..Default::default()
        };
        let result = engine.symbolize("astronauta", &options).unwrap();
        let token = result
            .tokens
            .iter()
            .find(|token| token.surface == "astronauta")
            .unwrap();
        assert_eq!(token.symbol_id.as_deref(), Some("arasaac.picto.4569"));
        assert!(token.svg.is_some(), "il pittogramma deve essere leggibile");
        assert_eq!(result.symbolized, 1);
    }

    #[test]
    fn reference_case_da_sole_is_not_the_sun() {
        let engine = test_engine();
        let opts = SymbolizeOptions {
            intensity: Intensity::Media,
            mode: CaaMode::Sopra,
            document_key: "/tmp/da-sole.caadoc".to_string(),
            paragraph_id: "p1".to_string(),
            ..Default::default()
        };
        let result = engine
            .symbolize("Le ragazze vanno da sole.", &opts)
            .unwrap();
        let sole = result
            .tokens
            .iter()
            .find(|token| token.surface.eq_ignore_ascii_case("sole"));
        if let Some(token) = sole {
            assert_ne!(
                token.concept_id.as_deref(),
                Some("nature.sun"),
                "«sole» in «da sole» non deve essere la stella: concetto {:?}, strategia {}",
                token.concept_id,
                token.strategy
            );
        }
    }

    /// Bloccare un pittogramma e poi cambiare il testo: il simbolo resta.
    #[test]
    fn unknown_verb_is_reduced_to_its_real_lemma() {
        let engine = test_engine();
        let result = engine
            .symbolize(
                "Io vorrei sapere chi governa il mondo.",
                &SymbolizeOptions {
                    intensity: Intensity::Completa,
                    mode: CaaMode::Sopra,
                    paragraph_id: "p1".to_string(),
                    ..Default::default()
                },
            )
            .unwrap();
        let token = result.tokens.iter().find(|token| token.surface == "governa").unwrap();
        // «governa» viene ricondotto a «governare» e al concetto action.govern;
        // non viene accettato un simbolo solo perché condivide le lettere.
        assert_eq!(token.lemma.as_deref(), Some("governare"));
        assert_eq!(token.concept_id.as_deref(), Some("action.govern"));
        if let Some(symbol) = token.symbol_label.as_deref() {
            assert!(
                symbol.to_lowercase().contains("govern"),
                "simbolo non pertinente: {symbol}"
            );
        }
        assert_eq!(token.caption, "governa");
    }

    #[test]
    fn mandatory_semantic_regression_cases() {
        if !arasaac_pack_available() {
            eprintln!("catalogo ARASAAC assente: test saltato (scripts/fetch_arasaac.py)");
            return;
        }
        let engine = test_engine();
        let cases: &[(&str, &str, &[&str])] = &[
            ("Mi sento bravo.", "sento", &["state.feel"]),
            ("Mi sento bravo.", "bravo", &["state.capable", "ability.can"]),
            ("Sono bravo.", "bravo", &["state.capable", "ability.can"]),
            ("Non voglio.", "non", &[]),
            ("Non voglio.", "voglio", &["action.want"]),
            ("Parlo più lingue.", "lingue", &["communication.language"]),
            ("Uso un computer portatile.", "computer portatile", &["object.laptop"]),
            ("La sintesi vocale legge.", "sintesi vocale", &["device.speech_synthesis"]),
            ("Le principesse giocano.", "principesse", &["person.princess"]),
            ("Le principesse giocano.", "giocano", &["action.play"]),
            ("Assaggerei.", "assaggerei", &["action.taste"]),
            ("Mi sento triste.", "sento", &["state.feel"]),
            ("Mi sento triste.", "triste", &["emotion.sad"]),
            ("Non sono qui per dimostrare nulla.", "nulla", &["quantity.nothing"]),
        ];
        for (text, surface, concepts) in cases {
            let result = engine
                .symbolize(
                    text,
                    &SymbolizeOptions {
                        intensity: Intensity::Completa,
                        mode: CaaMode::Sopra,
                        paragraph_id: format!("reg-{surface}"),
                        ..Default::default()
                    },
                )
                .unwrap();
            let token = result
                .tokens
                .iter()
                .find(|token| token.surface.eq_ignore_ascii_case(surface))
                .unwrap_or_else(|| panic!("token «{surface}» assente in «{text}»"));
            if concepts.is_empty() {
                assert!(
                    token.symbol_id.is_none() && token.composition.is_empty(),
                    "«{surface}» non deve avere un pittogramma, ha {:?}",
                    token.symbol_label
                );
                continue;
            }
            let actual = token.concept_id.as_deref().unwrap_or("");
            assert!(
                concepts.contains(&actual),
                "«{surface}» in «{text}»: atteso {:?}, ottenuto «{actual}» ({})",
                concepts,
                token.strategy
            );
        }
    }

    #[test]
    fn orthographic_neighbours_are_never_used_as_symbols() {
        let engine = test_engine();
        for (text, surface, forbidden) in [
            ("Mi sento bravo.", "sento", "sentiero"),
            ("Sono bravo.", "bravo", "bravas"),
            ("Non voglio.", "non", "nonna"),
            ("Non sono qui per dimostrare nulla.", "nulla", "nonna"),
        ] {
            let result = engine
                .symbolize(
                    text,
                    &SymbolizeOptions {
                        intensity: Intensity::Completa,
                        mode: CaaMode::Sopra,
                        paragraph_id: format!("p-{surface}"),
                        ..Default::default()
                    },
                )
                .unwrap();
            if let Some(token) = result
                .tokens
                .iter()
                .find(|token| token.surface.eq_ignore_ascii_case(surface))
            {
                let label = token
                    .symbol_label
                    .as_deref()
                    .unwrap_or("")
                    .to_lowercase();
                assert!(
                    !label.contains(forbidden),
                    "«{surface}» ha ricevuto il simbolo fuorviante «{label}»"
                );
            }
        }
    }

    #[test]
    fn document_override_applies_to_a_single_word() {
        if !arasaac_pack_available() {
            eprintln!("catalogo ARASAAC assente: test saltato (scripts/fetch_arasaac.py)");
            return;
        }
        let engine = test_engine();
        let doc = "/tmp/override-mondo.caadoc";
        engine
            .db()
            .unwrap()
            .set_document_override(doc, "p1:0", Some("arasaac.picto.27504"), None)
            .unwrap();
        engine.clear_caches();
        let result = engine
            .symbolize(
                "mondo",
                &SymbolizeOptions {
                    intensity: Intensity::Completa,
                    mode: CaaMode::Sopra,
                    document_key: doc.to_string(),
                    paragraph_id: "p1".to_string(),
                    ..Default::default()
                },
            )
            .unwrap();
        let token = result.tokens.iter().find(|token| token.surface == "mondo").unwrap();
        assert_eq!(token.symbol_id.as_deref(), Some("arasaac.picto.27504"));
    }

    #[test]
    fn manual_override_applies_to_word_the_engine_would_skip() {
        if !arasaac_pack_available() {
            eprintln!("catalogo ARASAAC assente: test saltato (scripts/fetch_arasaac.py)");
            return;
        }
        // Parola fuori lessico (pos sconosciuta, key=false): l'intensità Media
        // la salterebbe, ma la scelta fatta a mano nel pannello deve restare.
        let engine = test_engine();
        let doc = "/tmp/override-parola-ignota.caadoc";
        engine
            .db()
            .unwrap()
            .set_document_override(doc, "p1:0", Some("arasaac.picto.27504"), None)
            .unwrap();
        engine.clear_caches();
        let result = engine
            .symbolize(
                "blorbare",
                &SymbolizeOptions {
                    intensity: Intensity::Media,
                    mode: CaaMode::Sopra,
                    document_key: doc.to_string(),
                    paragraph_id: "p1".to_string(),
                    ..Default::default()
                },
            )
            .unwrap();
        let token = result
            .tokens
            .iter()
            .find(|token| token.surface == "blorbare")
            .unwrap();
        assert_eq!(token.symbol_id.as_deref(), Some("arasaac.picto.27504"));
        assert!(token.locked);
    }

    #[test]
    fn locked_symbol_survives_text_change() {
        let engine = test_engine();
        let doc = "/tmp/blocco.caadoc";
        let mut opts = SymbolizeOptions {
            intensity: Intensity::Media,
            mode: CaaMode::Sopra,
            document_key: doc.to_string(),
            paragraph_id: "p1".to_string(),
            ..Default::default()
        };
        let first = engine.symbolize("Il bambino corre.", &opts).unwrap();
        let token = first
            .tokens
            .iter()
            .find(|token| token.surface == "bambino")
            .expect("token bambino");
        let chosen = token
            .alternatives
            .iter()
            .find(|alternative| alternative.symbol_id != token.symbol_id.clone().unwrap_or_default())
            .or_else(|| token.alternatives.first());
        let Some(chosen) = chosen else {
            // Senza alternative non c'è nulla da bloccare: il test non inventa.
            return;
        };
        let unit = format!("p1:{}", token.index);
        engine
            .db()
            .unwrap()
            .set_document_override(doc, &unit, Some(&chosen.symbol_id), chosen.concept_id.as_deref())
            .unwrap();
        engine
            .db()
            .unwrap()
            .set_unit_state(doc, &unit, true, false, &UnitModifiers::default())
            .unwrap();
        engine.clear_caches();
        // Cambia il verbo: il testo sotto il pittogramma bloccato è diverso.
        let after = engine.symbolize("Il bambino cammina.", &opts).unwrap();
        let same = after
            .tokens
            .iter()
            .find(|token| token.surface == "bambino")
            .expect("token bambino dopo la modifica");
        assert_eq!(same.symbol_id.as_deref(), Some(chosen.symbol_id.as_str()));
        assert!(same.locked);
        // Il parlante può sbloccare: la scelta torna automatica.
        engine
            .db()
            .unwrap()
            .set_unit_state(doc, &unit, false, false, &UnitModifiers::default())
            .unwrap();
        engine.clear_caches();
        opts.intensity = Intensity::Media;
        let unlocked = engine.symbolize("Il bambino cammina.", &opts).unwrap();
        let same = unlocked
            .tokens
            .iter()
            .find(|token| token.surface == "bambino")
            .unwrap();
        assert!(!same.locked);
    }

    /// Il pittogramma nascosto resta un'unità dell'editor, con il testo intatto.
    #[test]
    fn hidden_unit_is_reported_without_losing_text() {
        let engine = test_engine();
        let doc = "/tmp/nascosto.caadoc";
        let opts = SymbolizeOptions {
            intensity: Intensity::Media,
            mode: CaaMode::Sopra,
            document_key: doc.to_string(),
            paragraph_id: "p1".to_string(),
            ..Default::default()
        };
        let first = engine.symbolize("Il bambino mangia una mela.", &opts).unwrap();
        let token = first
            .tokens
            .iter()
            .find(|token| token.surface == "mela")
            .expect("token mela");
        let unit = format!("p1:{}", token.index);
        engine
            .db()
            .unwrap()
            .set_unit_state(doc, &unit, token.locked, true, &UnitModifiers::default())
            .unwrap();
        engine.clear_caches();
        let after = engine.symbolize("Il bambino mangia una mela.", &opts).unwrap();
        let mela = after
            .tokens
            .iter()
            .find(|token| token.surface == "mela")
            .unwrap();
        assert!(mela.hidden);
        assert_eq!(mela.surface, "mela");
        assert!(mela.symbol_id.is_some(), "l'associazione resta memorizzata");
    }

    #[test]
    fn engine_seeds_and_counts() {
        if !arasaac_pack_available() {
            eprintln!("catalogo ARASAAC assente: test saltato (scripts/fetch_arasaac.py)");
            return;
        }
        let engine = test_engine();
        let stats = engine.db().unwrap().statistics().unwrap();
        assert!(stats.symbols >= 80, "simboli: {}", stats.symbols);
        assert!(stats.concepts >= 80, "concetti: {}", stats.concepts);
        // Il pacchetto ARASAAC viene importato accanto al pacchetto core.
        let packs = engine.db().unwrap().list_packs().unwrap();
        assert!(
            packs.iter().any(|pack| pack.id == "arasaac"),
            "pacchetto ARASAAC assente: {:?}",
            packs.iter().map(|p| p.id.clone()).collect::<Vec<_>>()
        );
        // L'intero catalogo italiano di ARASAAC (13.906 pittogrammi) più i
        // simboli dei concetti del lessico.
        assert!(stats.symbols >= 13_000, "simboli con ARASAAC: {}", stats.symbols);
        let library = engine
            .db()
            .unwrap()
            .search_symbols("cane", 50, 0, &SymbolFilters::default())
            .unwrap();
        assert!(
            library.total > 0,
            "la libreria ARASAAC non è ricercabile per parola"
        );
    }

    #[test]
    fn library_word_without_concept_gets_a_symbol() {
        if !arasaac_pack_available() {
            eprintln!("catalogo ARASAAC assente: test saltato (scripts/fetch_arasaac.py)");
            return;
        }
        // «vedere» non ha un concetto nel lessico, ma esiste nel catalogo
        // ARASAAC: senza l'immagine nel token l'editor non mostrava nulla,
        // anche se la ricerca manuale lo trovava.
        let engine = test_engine();
        let result = symbolize(&engine, "Il bambino vede una casa.");
        let token = result
            .tokens
            .iter()
            .find(|t| t.surface.eq_ignore_ascii_case("vede"))
            .expect("token vede");
        assert_eq!(token.lemma.as_deref(), Some("vedere"));
        assert!(token.symbol_id.is_some(), "simbolo mancante per «vedere»");
        assert!(token.svg.is_some(), "SVG mancante per «vedere»");
    }

    #[test]
    fn bundled_neural_engine_is_configured() {
        // Runtime e modello sono nel pacchetto `resources/ai`: il motore deve
        // risultare pronto senza che l'utente configuri percorsi.
        let engine = test_engine();
        let status = engine.ai_status();
        assert!(status.binary_present, "binario incluso assente");
        assert!(status.model_present, "modello incluso assente");
        assert!(status.enabled, "motore incluso non attivato");
    }

    #[test]
    fn peach_is_a_fruit_when_eaten() {
        let engine = test_engine();
        let result = symbolize(&engine, "Luca mangia una pesca.");
        assert_eq!(concept_of(&result, "pesca"), Some("fruit.peach"));
        let token = result.tokens.iter().find(|t| t.surface == "pesca").unwrap();
        assert!(token.svg.is_some());
        assert!(!token.approximate);
    }

    #[test]
    fn peach_is_fishing_when_grandfather_goes() {
        let engine = test_engine();
        let result = symbolize(&engine, "Il nonno va a pesca.");
        assert_eq!(concept_of(&result, "pesca"), Some("activity.fishing"));
    }

    #[test]
    fn pen_is_tool_and_feather() {
        let engine = test_engine();
        let tool = symbolize(&engine, "La bambina scrive con la penna.");
        assert_eq!(concept_of(&tool, "penna"), Some("obj.pen"));
        let feather = symbolize(&engine, "La penna dell'uccello è colorata.");
        assert_eq!(concept_of(&feather, "penna"), Some("animal.feather"));
    }

    #[test]
    fn bank_vs_school_desk() {
        let engine = test_engine();
        let desk = symbolize(&engine, "Il bambino si siede al banco.");
        assert_eq!(concept_of(&desk, "banco"), Some("furniture.school_desk"));
        let bank = symbolize(&engine, "Ho versato i soldi in banca.");
        assert_eq!(concept_of(&bank, "banca"), Some("finance.bank"));
    }

    #[test]
    fn missing_symbol_uses_broader_concept_and_keeps_word() {
        let engine = test_engine();
        let result = symbolize(&engine, "Il bassotto corre nel parco.");
        let token = result
            .tokens
            .iter()
            .find(|t| t.surface.eq_ignore_ascii_case("bassotto"))
            .unwrap();
        assert_eq!(token.surface, "bassotto", "la parola scritta resta");
        assert_eq!(token.concept_id.as_deref(), Some("animal.dog"));
        assert_eq!(token.strategy, "broader");
        assert!(token.approximate);
        assert!(token.svg.is_some());
    }

    #[test]
    fn composition_for_missing_compound() {
        let engine = test_engine();
        // Solo pacchetto core: qui il concetto non ha un pittogramma dedicato e
        // deve scattare la composizione. Con ARASAAC il simbolo esiste, quindi
        // il filtro esplicito mantiene il senso della prova.
        let result = symbolize_in_pack(&engine, "Il monopattino elettrico è veloce.", Some("core"));
        let token = result
            .tokens
            .iter()
            .find(|t| t.span >= 2)
            .expect("espressione multiword");
        assert_eq!(token.strategy, "composition");
        assert_eq!(token.composition.len(), 2);
        assert!(token.composition.iter().all(|p| p.svg.is_some()));
    }

    #[test]
    fn unknown_term_gets_no_symbol() {
        let engine = test_engine();
        let result = symbolize(&engine, "Il fiordo è profondo.");
        let token = result
            .tokens
            .iter()
            .find(|t| t.surface.eq_ignore_ascii_case("fiordo"))
            .unwrap();
        assert!(token.symbol_id.is_none());
        assert_eq!(token.strategy, "none");
    }

    #[test]
    fn multiword_expression_uses_compound_symbol() {
        let engine = test_engine();
        let result = symbolize(&engine, "Il bambino lava i denti.");
        let token = result
            .tokens
            .iter()
            .find(|t| t.span >= 2)
            .expect("espressione");
        assert_eq!(token.concept_id.as_deref(), Some("hygiene.brush_teeth"));
        assert!(token.svg.is_some());
    }

    #[test]
    fn labels_with_more_words_match_the_single_word() {
        // Le etichette ARASAAC sono spesso frasi: «mani» deve trovare
        // «lavarsi le mani», mentre «mano» non deve corrispondere a «mani».
        assert!(label_matches_word("lavarsi le mani", "mani"));
        assert!(label_matches_word("Mani", "mani"));
        assert!(!label_matches_word("mano", "mani"));
        // Sotto le tre lettere nessuna corrispondenza parziale.
        assert!(!label_matches_word("cane a scuola", "a"));
        assert!(label_matches_word("a", "a"));
    }

    #[test]
    fn text_mode_returns_no_symbols() {
        let engine = test_engine();
        let opts = SymbolizeOptions {
            paragraph_id: "p1".to_string(),
            mode: CaaMode::Testo,
            ..Default::default()
        };
        let result = engine.symbolize("Luca mangia una pesca.", &opts).unwrap();
        assert!(result.tokens.iter().all(|t| t.symbol_id.is_none()));
    }

    #[test]
    fn document_override_wins() {
        let engine = test_engine();
        let opts = SymbolizeOptions {
            paragraph_id: "p1".to_string(),
            document_key: "/tmp/test.caadoc".to_string(),
            ..Default::default()
        };
        let first = engine.symbolize("Luca mangia una pesca.", &opts).unwrap();
        let token = first.tokens.iter().find(|t| t.surface == "pesca").unwrap();
        let unit_key = format!("p1:{}", token.index);
        let alternative = token
            .alternatives
            .iter()
            .find(|a| a.concept_id.as_deref() == Some("activity.fishing"))
            .expect("alternativa pesca-attività");
        engine
            .db()
            .unwrap()
            .set_document_override(
                "/tmp/test.caadoc",
                &unit_key,
                Some(&alternative.symbol_id),
                Some("activity.fishing"),
            )
            .unwrap();
        // La cache dei paragrafi non conosce le preferenze: va svuotata.
        engine.clear_caches();
        let second = engine.symbolize("Luca mangia una pesca.", &opts).unwrap();
        let token2 = second
            .tokens
            .iter()
            .find(|t| t.surface == "pesca")
            .unwrap();
        assert_eq!(token2.symbol_id.as_deref(), Some(alternative.symbol_id.as_str()));
        assert!(token2.locked);
    }

    #[test]
    fn incremental_symbolize_is_cached() {
        let engine = test_engine();
        let opts = SymbolizeOptions {
            paragraph_id: "p1".to_string(),
            ..Default::default()
        };
        let a = engine.symbolize("Il cane corre.", &opts).unwrap();
        let b = engine.symbolize("Il cane corre.", &opts).unwrap();
        assert_eq!(a.tokens.len(), b.tokens.len());
        assert_eq!(b.elapsed_ms, a.elapsed_ms);
    }

    fn affinity(engine: &Engine, word: &str) -> WordAffinity {
        engine.word_affinities(word, None, false).unwrap()
    }

    fn group<'a>(affinity: &'a WordAffinity, level: AffinityLevel) -> Option<&'a AffinityGroup> {
        affinity.groups.iter().find(|group| group.level == level)
    }

    #[test]
    fn affinity_orders_groups_and_never_duplicates_a_symbol() {
        let engine = test_engine();
        let found = affinity(&engine, "pesca");
        assert!(found.known);

        let ranks: Vec<u8> = found.groups.iter().map(|group| group.level.rank()).collect();
        assert!(
            ranks.windows(2).all(|pair| pair[0] < pair[1]),
            "i gruppi devono seguire l'ordine di forza: {ranks:?}"
        );

        let exact = group(&found, AffinityLevel::Exact).expect("gruppo esatto per «pesca»");
        assert!(exact
            .candidates
            .iter()
            .any(|candidate| candidate.symbol.symbol_id == "core.fruit.peach"));
        assert!(exact
            .candidates
            .iter()
            .any(|candidate| candidate.symbol.symbol_id == "core.activity.fishing"));
        assert!(exact
            .candidates
            .iter()
            .all(|candidate| candidate.score == 100 && !candidate.approximate));

        let general = group(&found, AffinityLevel::General).expect("concetto più generale");
        assert!(
            general
                .candidates
                .iter()
                .any(|candidate| candidate.symbol.label == "frutta"),
            "«pesca» deve risalire a «frutta», candidati: {:?}",
            general
                .candidates
                .iter()
                .map(|candidate| candidate.symbol.label.as_str())
                .collect::<Vec<_>>()
        );
        assert!(general.candidates.iter().all(|candidate| candidate.approximate));

        let mut seen = HashSet::new();
        for group in &found.groups {
            for candidate in &group.candidates {
                assert!(
                    seen.insert(candidate.symbol.symbol_id.clone()),
                    "simbolo duplicato tra i gruppi: {}",
                    candidate.symbol.symbol_id
                );
                assert!(!candidate.reason.trim().is_empty());
                assert!(candidate.score <= 100);
                assert_eq!(candidate.approximate, candidate.symbol.approximate);
            }
        }
    }

    #[test]
    fn affinity_uses_the_lemma_for_inflected_words() {
        let engine = test_engine();
        let found = affinity(&engine, "mangiano");
        let lemma = group(&found, AffinityLevel::Lemma).expect("forma base di «mangiano»");
        assert!(
            lemma.candidates.iter().any(|candidate| {
                candidate.symbol.symbol_id == "core.action.eat"
                    && candidate.symbol.label == "mangiare"
                    && candidate.score == 90
            }),
            "candidati: {:?}",
            lemma
                .candidates
                .iter()
                .map(|candidate| candidate.symbol.label.as_str())
                .collect::<Vec<_>>()
        );
        assert!(found
            .explanation
            .contains("forma base «mangiare»"));
    }

    #[test]
    fn affinity_reports_declared_synonyms() {
        let engine = test_engine();
        let found = affinity(&engine, "auto");
        let synonyms = group(&found, AffinityLevel::Synonym).expect("sinonimi di «auto»");
        let car = synonyms
            .candidates
            .iter()
            .find(|candidate| candidate.symbol.symbol_id == "core.vehicle.car")
            .expect("«macchina» come sinonimo dichiarato di «auto»");
        assert_eq!(car.symbol.label, "macchina");
        assert_eq!(car.score, 70);
        assert!(!car.approximate);
        assert!(found.explanation.contains("sinonimo"));
    }

    #[test]
    fn affinity_reduces_principesse_to_the_real_lemma() {
        if !arasaac_pack_available() {
            eprintln!("catalogo ARASAAC assente: test saltato (scripts/fetch_arasaac.py)");
            return;
        }
        let engine = test_engine();
        let found = affinity(&engine, "principesse");
        assert!(found.known);
        // La forma è ora nel lessico: viene ricondotta al lemma reale, non a
        // una variante ortografica ipotizzata. Il simbolo deve esistere.
        let matched_group = group(&found, AffinityLevel::Lemma)
            .or_else(|| group(&found, AffinityLevel::Variant))
            .expect("lemma o variante di «principesse»");
        let princess = matched_group
            .candidates
            .iter()
            .find(|candidate| candidate.symbol.label == "principessa")
            .expect("«principessa» esiste nel repertorio");
        assert_eq!(princess.symbol.label, "principessa");

        // Nessun identificativo inventato: ogni candidato esiste nel database.
        let db = engine.db().unwrap();
        for group in &found.groups {
            for candidate in &group.candidates {
                assert!(
                    db.symbol_by_id(&candidate.symbol.symbol_id).unwrap().is_some(),
                    "simbolo inesistente: {}",
                    candidate.symbol.symbol_id
                );
            }
        }
    }

    #[test]
    fn affinity_unknown_word_returns_no_candidates() {
        let engine = test_engine();
        let found = affinity(&engine, "zzzqqq");
        assert!(found.groups.is_empty(), "nessun gruppo per una parola inesistente");
        assert!(!found.known);
        assert!(engine.word_affinities("   ", None, false).is_err());
    }

    #[test]
    fn affinity_respects_the_pack_filter() {
        let engine = test_engine();
        let found = engine
            .word_affinities("pesca", Some("core".to_string()), false)
            .unwrap();
        assert!(!found.groups.is_empty());
        for group in &found.groups {
            for candidate in &group.candidates {
                assert_eq!(candidate.symbol.pack_id, "core");
            }
        }
    }

    #[test]
    fn gelati_is_recognised_as_the_plural_of_gelato() {
        let engine = test_engine();
        let found = affinity(&engine, "gelati");
        assert_eq!(found.lemma.as_deref(), Some("gelato"));
        let lemma = group(&found, AffinityLevel::Lemma).expect("forma base di «gelati»");
        assert!(
            lemma
                .candidates
                .iter()
                .any(|candidate| candidate.symbol.concept_id.as_deref() == Some("food.ice_cream")),
            "candidati: {:?}",
            lemma
                .candidates
                .iter()
                .map(|candidate| candidate.symbol.label.as_str())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn capire_finds_comprendere_in_the_affinity_groups() {
        if !arasaac_pack_available() {
            eprintln!("catalogo ARASAAC assente: test saltato (scripts/fetch_arasaac.py)");
            return;
        }
        let engine = test_engine();
        let found = affinity(&engine, "capire");
        let synonyms = group(&found, AffinityLevel::Synonym).expect("sinonimi di «capire»");
        assert!(
            synonyms
                .candidates
                .iter()
                .any(|candidate| candidate.symbol.label == "comprendere"),
            "candidati: {:?}",
            synonyms
                .candidates
                .iter()
                .map(|candidate| candidate.symbol.label.as_str())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn word_without_symbol_gets_library_alternatives_from_its_concept_label() {
        if !arasaac_pack_available() {
            eprintln!("catalogo ARASAAC assente: test saltato (scripts/fetch_arasaac.py)");
            return;
        }
        let engine = test_engine();
        let opts = SymbolizeOptions {
            paragraph_id: "p1".to_string(),
            ..Default::default()
        };
        let result = engine.symbolize("Io vorrei capire.", &opts).unwrap();
        let token = result
            .tokens
            .iter()
            .find(|token| token.surface.eq_ignore_ascii_case("capire"))
            .expect("token capire");
        assert_eq!(token.concept_label.as_deref(), Some("comprendere"));
        assert!(
            token
                .alternatives
                .iter()
                .any(|alternative| alternative.label == "comprendere"),
            "alternative: {:?}",
            token
                .alternatives
                .iter()
                .map(|alternative| alternative.label.as_str())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn assaggerei_uses_the_conditional_and_does_not_inherit_negation() {
        let engine = test_engine();
        let opts = SymbolizeOptions {
            paragraph_id: "p1".to_string(),
            ..Default::default()
        };
        let result = engine
            .symbolize("Non so se assaggerei tanto facilmente.", &opts)
            .unwrap();
        let taste = result
            .tokens
            .iter()
            .find(|token| token.surface.eq_ignore_ascii_case("assaggerei"))
            .expect("token assaggerei");
        assert_eq!(taste.lemma.as_deref(), Some("assaggiare"));
        assert!(
            !taste.negated,
            "la negazione di «non so» non deve passare ad «assaggerei»"
        );
        assert_eq!(taste.concept_label.as_deref(), Some("assaggiare"));

        if let Some(uncertain) = result
            .tokens
            .iter()
            .find(|token| token.surface.eq_ignore_ascii_case("non so"))
        {
            assert_eq!(uncertain.concept_id.as_deref(), Some("state.uncertain"));
            assert!(!uncertain.negated);
        }
    }

    #[test]
    fn non_so_is_an_expression_not_a_negated_word() {
        if !arasaac_pack_available() {
            eprintln!("catalogo ARASAAC assente: test saltato (scripts/fetch_arasaac.py)");
            return;
        }
        let engine = test_engine();
        let found = affinity(&engine, "non so");
        assert!(found.known);
        assert!(found.explanation.contains("espressione"));
        let synonyms = group(&found, AffinityLevel::Synonym).expect("sinonimi di incertezza");
        assert!(
            synonyms.candidates.iter().any(|candidate| {
                candidate.symbol.label == "io non so" || candidate.symbol.label == "dubbio"
            }),
            "candidati: {:?}",
            synonyms
                .candidates
                .iter()
                .map(|candidate| candidate.symbol.label.as_str())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn multiword_expressions_are_single_units_with_one_pictogram() {
        if !arasaac_pack_available() {
            eprintln!("catalogo ARASAAC assente: test saltato (scripts/fetch_arasaac.py)");
            return;
        }
        let engine = test_engine();
        let opts = SymbolizeOptions {
            paragraph_id: "p1".to_string(),
            ..Default::default()
        };

        // (testo, espressione attesa, concetto atteso, simbolo atteso)
        // Il concetto effettivo può diventare il concetto più generale quando
        // il repertorio non ha un pittogramma dedicato: «scuola primaria» usa
        // il simbolo di «scuola». «non so» resta senza pittogramma dedicato.
        let cases = [
            ("La sintesi vocale legge il testo.", "sintesi vocale", Some("device.speech_synthesis"), true),
            ("Uso un computer portatile.", "computer portatile", Some("object.laptop"), true),
            ("Uso il telefono cellulare.", "telefono cellulare", Some("device.mobile_phone"), true),
            ("Pago con la carta di credito.", "carta di credito", Some("finance.credit_card"), true),
            ("I mezzi di trasporto sono fermi.", "mezzi di trasporto", Some("vehicle.transport"), true),
            ("La sala da pranzo è grande.", "sala da pranzo", Some("place.dining_room"), true),
            ("Ho una macchina fotografica.", "macchina fotografica", Some("object.camera"), true),
            ("Vado alla scuola primaria.", "scuola primaria", None, true),
            ("Faccio colazione alle otto.", "faccio colazione", Some("food.breakfast"), true),
            ("Mi dai una mano?", "dai una mano", Some("action.help"), true),
            ("Vado a dormire.", "vado a dormire", Some("action.sleep"), true),
            ("Siamo andate da sole.", "da sole", Some("state.alone"), true),
            ("Non so cosa fare.", "non so", None, false),
            ("Sono in grado di leggere.", "sono in grado di", Some("ability.can"), true),
            ("Ho bisogno di aiuto.", "ho bisogno di", Some("action.need"), true),
            ("Non prendere in giro nessuno.", "prendere in giro", Some("action.tease"), true),
        ];

        for (text, surface, concept, has_symbol) in cases {
            let result = engine.symbolize(text, &opts).unwrap();
            let token = result
                .tokens
                .iter()
                .find(|token| token.surface.eq_ignore_ascii_case(surface))
                .unwrap_or_else(|| panic!("unità «{surface}» non trovata in «{text}»"));
            assert_eq!(token.span, surface.split_whitespace().count(), "span di «{surface}»");
            if let Some(concept) = concept {
                assert_eq!(token.concept_id.as_deref(), Some(concept), "concetto di «{surface}»");
            }
            assert_eq!(token.caption, token.surface, "didascalia di «{surface}»");
            assert_eq!(token.symbol_id.is_some(), has_symbol, "pittogramma di «{surface}»");
            assert!(
                result
                    .tokens
                    .iter()
                    .filter(|other| other.span > 1 && other.surface == token.surface)
                    .count()
                    == 1,
                "unità duplicata per «{surface}»"
            );
        }
    }

    #[test]
    fn manual_units_merge_and_split_ranges_separate() {
        let engine = test_engine();
        let text = "Uso il computer portatile.";

        // Unione manuale di un'espressione che il lessico non conosce.
        let start = text.find("computer portatile").unwrap();
        let end = start + "computer portatile".len();
        let merged = engine
            .symbolize(
                text,
                &SymbolizeOptions {
                    paragraph_id: "p1".to_string(),
                    manual_units: vec![ManualRange { start, end }],
                    ..Default::default()
                },
            )
            .unwrap();
        let unit = merged
            .tokens
            .iter()
            .find(|token| token.surface == "computer portatile")
            .expect("unità unita a mano");
        assert_eq!(unit.span, 2);
        assert_eq!(unit.caption, "computer portatile");

        // Separazione manuale della stessa espressione automatica.
        let split = engine
            .symbolize(
                "La sintesi vocale legge il testo.",
                &SymbolizeOptions {
                    paragraph_id: "p2".to_string(),
                    split_ranges: vec![ManualRange {
                        start: "La ".len(),
                        end: "La sintesi vocale".len(),
                    }],
                    ..Default::default()
                },
            )
            .unwrap();
        assert!(
            split
                .tokens
                .iter()
                .all(|token| token.surface != "sintesi vocale"),
            "l'unità separata non deve restare fusa"
        );
        assert!(split.tokens.iter().any(|token| token.surface == "sintesi"));
        assert!(split.tokens.iter().any(|token| token.surface == "vocale"));
    }

    #[test]
    fn imported_personal_pictogram_wins_for_the_same_word() {
        let engine = test_engine();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cane.png");
        // PNG valido: il contenuto visivo non conta, conta la pipeline.
        image::RgbImage::from_pixel(8, 8, image::Rgb([180, 90, 40]))
            .save(&path)
            .unwrap();
        let report = crate::pictogram_import::import_batch(
            &engine.db().unwrap(),
            &engine.paths.assets_dir,
            &[crate::pictogram_import::ImportItem {
                path: path.to_string_lossy().to_string(),
                lemma: "cane".to_string(),
                collection: Some("Animali".to_string()),
                category: Some("animali".to_string()),
                synonyms: vec!["cagnolino".to_string()],
                relative_path: Some("cane.png".to_string()),
                original_name: Some("cane.png".to_string()),
            }],
            &crate::pictogram_import::ImportLicense::default(),
            "Animali",
        );
        assert_eq!(report.imported, 1, "avvisi: {:?}", report.warnings);
        engine.clear_caches();

        let opts = SymbolizeOptions {
            paragraph_id: "p1".to_string(),
            ..Default::default()
        };
        let result = engine.symbolize("Il cane corre.", &opts).unwrap();
        let token = result
            .tokens
            .iter()
            .find(|token| token.surface.eq_ignore_ascii_case("cane"))
            .expect("token cane");
        assert_eq!(token.pack_id.as_deref(), Some("personal"));
        assert_eq!(token.caption, "cane");
    }

    #[test]
    fn concept_specific_preference_applies_only_to_that_concept() {
        if !arasaac_pack_available() {
            eprintln!("catalogo ARASAAC assente: test saltato (scripts/fetch_arasaac.py)");
            return;
        }
        let engine = test_engine();
        {
            let db = engine.db().unwrap();
            db.upsert_preference(&Preference {
                scope: "global".to_string(),
                lemma: "capire".to_string(),
                context_key: Some("concept:action.understand".to_string()),
                symbol_id: "arasaac.picto.16659".to_string(),
                concept_id: Some("action.understand".to_string()),
                locked: false,
                created_at: now_iso(),
            })
            .unwrap();
            db.upsert_preference(&Preference {
                scope: "global".to_string(),
                lemma: "capire".to_string(),
                context_key: Some("concept:other.meaning".to_string()),
                symbol_id: "core.action.eat".to_string(),
                concept_id: Some("other.meaning".to_string()),
                locked: false,
                created_at: now_iso(),
            })
            .unwrap();
        }
        let opts = SymbolizeOptions {
            paragraph_id: "p1".to_string(),
            ..Default::default()
        };
        let result = engine.symbolize("Vorrei capire.", &opts).unwrap();
        let token = result
            .tokens
            .iter()
            .find(|token| token.surface.eq_ignore_ascii_case("capire"))
            .expect("token capire");
        assert_eq!(token.symbol_id.as_deref(), Some("arasaac.picto.16659"));
        assert_ne!(token.symbol_id.as_deref(), Some("core.action.eat"));
    }

    #[test]
    fn affinity_never_uses_orthographic_similarity() {
        let engine = test_engine();
        for (word, forbidden) in [
            ("sento", "sentiero"),
            ("bravo", "bravas"),
            ("non", "nonna"),
        ] {
            let found = affinity(&engine, word);
            for group in &found.groups {
                for candidate in &group.candidates {
                    assert!(
                        !candidate.symbol.label.to_lowercase().contains(forbidden),
                        "«{word}» ha proposto «{}» per sola somiglianza ortografica",
                        candidate.symbol.label
                    );
                }
            }
        }
    }

    #[test]
    fn communicator_navigation_must_point_to_an_existing_page() {
        let engine = test_engine();
        let mut spec = crate::layout::LayoutSpecification::default();
        spec.title = "Prova".to_string();
        spec.elements.push(crate::layout::ElementSpec {
            id: "vai".to_string(),
            label: "Vai".to_string(),
            action: Some(crate::layout::ElementAction {
                action_type: "navigate".to_string(),
                target_page: Some(3),
            }),
            ..Default::default()
        });
        let error = engine.layout_document(spec.clone()).unwrap_err();
        assert!(error.contains("pagina di destinazione"));

        spec.elements[0].action = Some(crate::layout::ElementAction {
            action_type: "navigate".to_string(),
            target_page: Some(0),
        });
        let built = engine.layout_document(spec).unwrap();
        assert_eq!(built.pages.len(), 1);
    }

    #[test]
    fn common_vocabulary_returns_only_real_symbols() {
        let engine = test_engine();
        let items = engine.common_symbols(24).unwrap();
        assert!(items.len() >= 12, "vocabolario troppo piccolo: {}", items.len());
        for item in &items {
            assert!(!item.symbol_id.is_empty());
            assert!(item.svg.is_some(), "simbolo senza SVG: {}", item.symbol_id);
        }
    }

    #[test]
    fn cancelled_refinement_is_discarded_before_it_starts() {
        let engine = test_engine();
        engine.cancel_request("doc:p1:7");
        let result = engine.refine_cancellable(
            "Io vorrei sapere chi governa il mondo.",
            &SymbolizeOptions {
                paragraph_id: "p1".to_string(),
                document_key: "doc".to_string(),
                ..Default::default()
            },
            Some("doc:p1:7"),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("superata"));
        assert!(engine.metrics_snapshot().stale_results >= 1);
    }

    #[test]
    fn profile_preference_does_not_leak_to_other_profiles() {
        let engine = test_engine();
        {
            let db = engine.db().unwrap();
            db.upsert_preference(&Preference {
                scope: "profile".to_string(),
                lemma: "capire".to_string(),
                context_key: Some("profile:p1:concept:action.understand".to_string()),
                symbol_id: "core.fruit.peach".to_string(),
                concept_id: Some("action.understand".to_string()),
                locked: false,
                created_at: now_iso(),
            })
            .unwrap();
        }
        let with_profile = engine
            .symbolize(
                "Vorrei capire.",
                &SymbolizeOptions {
                    paragraph_id: "p1".to_string(),
                    profile_id: Some("p1".to_string()),
                    ..Default::default()
                },
            )
            .unwrap();
        let token = with_profile
            .tokens
            .iter()
            .find(|token| token.surface.eq_ignore_ascii_case("capire"))
            .unwrap();
        assert_eq!(token.symbol_id.as_deref(), Some("core.fruit.peach"));

        let other_profile = engine
            .symbolize(
                "Vorrei capire.",
                &SymbolizeOptions {
                    paragraph_id: "p1".to_string(),
                    profile_id: Some("p2".to_string()),
                    ..Default::default()
                },
            )
            .unwrap();
        let token = other_profile
            .tokens
            .iter()
            .find(|token| token.surface.eq_ignore_ascii_case("capire"))
            .unwrap();
        assert_ne!(token.symbol_id.as_deref(), Some("core.fruit.peach"));
    }

    #[test]
    fn caption_is_the_written_word_and_symbol_label_is_metadata() {
        let engine = test_engine();
        let opts = SymbolizeOptions {
            paragraph_id: "p1".to_string(),
            ..Default::default()
        };
        let result = engine.symbolize("I bambini corrono.", &opts).unwrap();
        let token = result
            .tokens
            .iter()
            .find(|token| token.surface.eq_ignore_ascii_case("bambini"))
            .expect("token bambini");
        assert_eq!(token.caption, "bambini");
        assert!(token.symbol_label.is_some());
        assert_ne!(
            token.symbol_label.as_deref(),
            Some(token.caption.as_str()),
            "l'etichetta del repertorio resta separata dalla parola scritta"
        );
    }
}
