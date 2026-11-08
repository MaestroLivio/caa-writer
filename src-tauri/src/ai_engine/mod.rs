//! Motore IA locale facoltativo.
//!
//! Scelte di progetto:
//! - L'inferenza avviene con llama.cpp (binario locale) su un modello GGUF
//!   quantizzato Q4: nessuna GPU obbligatoria, nessuna API cloud.
//! - Il modello NON genera identificativi: riceve solo concetti già presenti
//!   nel database e può solo scegliere tra quelli.
//! - Qualsiasi risposta non conforme allo schema viene scartata e si usa la
//!   selezione deterministica.
//! - La confidenza restituita NON è una probabilità calibrata: è il valore
//!   dichiarato dal modello, mostrato come tale.

pub mod catalog;
pub mod hardware;
pub mod installer;
pub mod llama_cli;
pub mod service;
pub mod tools;

use serde::{Deserialize, Serialize};

use crate::model::AiConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candidate {
    pub concept_id: String,
    pub label: String,
    pub meaning: String,
    pub category: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    pub concept_id: String,
    pub confidence: f32,
    pub raw: String,
}

/// Il motore è disponibile solo se binario e modello esistono su disco.
pub fn is_available(config: &AiConfig) -> bool {
    config.enabled
        && !config.binary_path.trim().is_empty()
        && !config.model_path.trim().is_empty()
        && std::path::Path::new(&config.binary_path).exists()
        && std::path::Path::new(&config.model_path).exists()
}

/// Costruisce il prompt: poche righe, risposta con il **numero** dell'opzione.
///
/// Un modello da 0,5 B sbaglia spesso il formato JSON; chiedere un numero è
/// molto più affidabile e non richiede identificativi inventati dal modello.
pub fn build_prompt(sentence: &str, target: &str, candidates: &[Candidate]) -> String {
    let mut list = String::new();
    for (i, candidate) in candidates.iter().enumerate() {
        list.push_str(&format!(
            "{}. {} — {}\n",
            i + 1,
            candidate.label,
            candidate.meaning
        ));
    }
    format!(
        "Scegli il significato della parola «{target}» in questa frase: {sentence}\n\
         Opzioni:\n{list}\
         Rispondi SOLO con il numero dell'opzione corretta (1-{}), senza altro testo.\n\
         Risposta:",
        candidates.len()
    )
}

/// Chiede al modello parole italiane di significato vicino, per la ricerca di
/// pittogrammi quando il lessico non ha un concetto per la parola scritta.
/// Il contesto (la frase in cui la parola compare) guida la
/// concettualizzazione: «banco» in un'aula non è «banco» di pesce.
pub const RELATED_WORDS_LIMIT: usize = 6;

pub fn related_words_prompt(word: &str, context: Option<&str>) -> String {
    let context = context
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| format!("Frase: «{value}»\n"))
        .unwrap_or_default();
    format!(
        "{context}Elenca fino a {RELATED_WORDS_LIMIT} parole italiane di significato vicino a \
         «{word}» in questa frase.\n\
         Una parola per riga, senza numeri, senza spiegazioni.\n\
         Parole:\n"
    )
}

/// Campo semantico della frase: una sola richiesta per paragrafo, usata per
/// arricchire di simboli affini tutte le parole chiave già simbolizzate.
/// Costa poco (una inferenza) e non sostituisce mai la scelta deterministica.
pub fn semantic_field_prompt(sentence: &str) -> String {
    format!(
        "Frase: «{}»\n\
         Elenca fino a 8 parole italiane concrete della frase (persone, cose, azioni, luoghi) \
         utili per cercare pittogrammi CAA.\n\
         Una parola per riga, senza numeri, senza spiegazioni.\n\
         Parole:\n",
        sentence.trim()
    )
}

/// Parole grammaticali che non producono un pittogramma utile: se il modello
/// le propone, vengono scartate invece di generare suggerimenti fuorvianti.
fn is_function_word(word: &str) -> bool {
    const STOP: &[&str] = &[
        "il", "lo", "la", "i", "gli", "le", "un", "uno", "una", "di", "a", "da", "in", "con",
        "su", "per", "tra", "fra", "e", "o", "ma", "che", "chi", "cui", "non", "più", "anche",
        "come", "dove", "quando", "questo", "questa", "quello", "quella", "suo", "sua", "sono",
        "essere", "avere", "ha", "hanno", "è", "era",
    ];
    STOP.contains(&word)
}

/// Estrae le parole proposte dal modello: una per riga oppure separate da
/// virgole; niente numeri, elenchi o frasi lunghe. `limit` fissa quante
/// parole accettare al massimo.
pub fn parse_related_words(raw: &str, limit: usize) -> Vec<String> {
    parse_words(raw, limit, 2)
}

/// Sinonimi e brevi espressioni («gettare l'ancora», «mettere in sicurezza»):
/// si accettano fino a quattro parole, perché il repertorio ARASAAC contiene
/// anche etichette composte.
pub fn parse_synonyms(raw: &str, limit: usize) -> Vec<String> {
    parse_words(raw, limit, 4)
}

fn parse_words(raw: &str, limit: usize, max_words: usize) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let limit = limit.max(1);
    for line in answer_text(raw).lines() {
        // I modelli conversazionali a volte mettono più parole sulla stessa
        // riga separate da punti o da barre: si accettano come elenco.
        for piece in line.split([',', ';', '.', '!', '?', '|', '\u{2022}', '\u{00b7}']) {
            let cleaned = piece
                .trim()
                .trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c == ')' || c == '-' || c == '*')
                .trim()
                .trim_matches('"')
                .to_string();
            if cleaned.is_empty() || cleaned.split_whitespace().count() > max_words {
                continue;
            }
            if !cleaned.chars().any(|c| c.is_alphabetic()) {
                continue;
            }
            let normalized = crate::linguistics::normalize::normalize_word(&cleaned);
            if normalized.chars().count() < 3 || is_function_word(&normalized) {
                continue;
            }
            if out.contains(&normalized) {
                continue;
            }
            out.push(normalized);
            if out.len() >= limit {
                return out;
            }
        }
    }
    out
}

/// Parole del campo semantico della frase (massimo otto).
pub fn parse_semantic_field(raw: &str) -> Vec<String> {
    parse_related_words(raw, 8)
}

/// Estrae il primo oggetto JSON dalla risposta del modello.
pub fn extract_json(raw: &str) -> Option<&str> {
    let start = raw.find('{')?;
    let end = raw.rfind('}')?;
    if end <= start {
        return None;
    }
    Some(&raw[start..=end])
}

/// Rimuove l'eco del prompt e le righe di servizio di llama.cpp.
///
/// `llama-cli` scrive `> prompt` e i tempi (`[ Prompt: … ]`) nello stesso
/// flusso dell'output: senza questa pulizia il parsing leggeva il prompt
/// invece della risposta.
fn answer_text(raw: &str) -> String {
    // Se l'output ripete il prompt, conta solo ciò che segue l'ultima
    // occorrenza di «Risposta:»: altrimenti i numeri delle opzioni elencate
    // verrebbero scambiati per la risposta.
    let body = match raw.rfind("Risposta:") {
        Some(index) => &raw[index + "Risposta:".len()..],
        None => raw,
    };
    body.lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.starts_with('>') && !trimmed.starts_with("[ Prompt") && !trimmed.starts_with("Exiting")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Ultimo numero intero dell'output dentro l'intervallo delle opzioni.
fn option_number(raw: &str, count: usize) -> Option<usize> {
    let digits: String = raw.chars().map(|c| if c.is_ascii_digit() { c } else { ' ' }).collect();
    for chunk in digits.split_whitespace() {
        if let Ok(value) = chunk.parse::<usize>() {
            if (1..=count).contains(&value) {
                return Some(value);
            }
        }
    }
    None
}

#[derive(Debug, Deserialize)]
struct RawAnswer {
    concept_id: String,
    #[serde(default)]
    confidence: Option<f64>,
}

/// Valida la risposta contro i candidati ammessi.
pub fn validate_answer(
    raw: &str,
    candidates: &[Candidate],
    fallback: &str,
) -> Result<Choice, String> {
    if candidates.is_empty() {
        return Err("nessun candidato".to_string());
    }
    let text = answer_text(raw);

    // Formato storico (JSON): accettato se il concept_id esiste davvero.
    if let Some(json) = extract_json(&text) {
        if let Ok(answer) = serde_json::from_str::<RawAnswer>(json) {
            if candidates.iter().any(|c| c.concept_id == answer.concept_id) {
                let confidence = answer
                    .confidence
                    .map(|c| c.clamp(0.0, 1.0) as f32)
                    .unwrap_or(0.5);
                return Ok(Choice {
                    concept_id: answer.concept_id,
                    confidence,
                    raw: raw.to_string(),
                });
            }
        }
    }

    // Formato richiesto: numero dell'opzione.
    let number = option_number(&text, candidates.len())
        .ok_or_else(|| "la risposta non contiene un numero di opzione valido".to_string())?;
    let _ = fallback;
    Ok(Choice {
        concept_id: candidates[number - 1].concept_id.clone(),
        confidence: 0.6,
        raw: raw.to_string(),
    })
}

pub fn choose_concept(
    config: &AiConfig,
    sentence: &str,
    target: &str,
    candidates: &[Candidate],
) -> Result<Choice, String> {
    if candidates.is_empty() {
        return Err("nessun candidato".to_string());
    }
    let prompt = build_prompt(sentence, target, candidates);
    let raw = llama_cli::run_completion(config, &prompt)?;
    validate_answer(&raw, candidates, &candidates[0].concept_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidates() -> Vec<Candidate> {
        vec![
            Candidate {
                concept_id: "fruit.peach".into(),
                label: "pesca".into(),
                meaning: "frutto".into(),
                category: "alimenti".into(),
            },
            Candidate {
                concept_id: "activity.fishing".into(),
                label: "pesca".into(),
                meaning: "attività".into(),
                category: "attivita".into(),
            },
        ]
    }

    #[test]
    fn prompt_contains_only_real_candidates() {
        let prompt = build_prompt("Luca mangia una pesca", "pesca", &candidates());
        assert!(prompt.contains("1. pesca"));
        assert!(prompt.contains("2. pesca"));
        assert!(prompt.contains("SOLO con il numero"));
        assert!(!prompt.contains("fruit.peach"), "niente identificativi interni");
    }

    #[test]
    fn numeric_answer_maps_to_candidate() {
        let raw = "\n> Scegli il significato...\n2\n\n[ Prompt: 100 t/s | Generation: 50 t/s ]\n";
        let choice = validate_answer(raw, &candidates(), "fruit.peach").unwrap();
        assert_eq!(choice.concept_id, "activity.fishing");
    }

    #[test]
    fn number_out_of_range_is_rejected() {
        assert!(validate_answer("7", &candidates(), "fruit.peach").is_err());
    }

    #[test]
    fn validation_accepts_real_concept() {
        let raw = "Ecco: {\"concept_id\": \"fruit.peach\", \"confidence\": 0.94}";
        let choice = validate_answer(raw, &candidates(), "fruit.peach").unwrap();
        assert_eq!(choice.concept_id, "fruit.peach");
        assert!((choice.confidence - 0.94).abs() < 0.001);
    }

    #[test]
    fn validation_rejects_invented_id() {
        let raw = "{\"concept_id\": \"symbol_281\", \"confidence\": 0.9}";
        assert!(validate_answer(raw, &candidates(), "fruit.peach").is_err());
    }

    #[test]
    fn validation_rejects_non_json() {
        assert!(validate_answer("La pesca è un frutto", &candidates(), "fruit.peach").is_err());
    }

    #[test]
    fn related_words_ignore_prompt_echo_and_numbering() {
        let raw = "> Elenca 3 parole italiane con significato vicino a «zaino»\n1. sacca\n2) cartella\n- valigia\n[ Prompt: 100 t/s ]\n";
        assert_eq!(
            parse_related_words(raw, 6),
            vec!["sacca", "cartella", "valigia"]
        );
    }

    #[test]
    fn related_words_skip_sentences_and_duplicates() {
        let raw = "Non ho capito la richiesta, ma posso dire: cane, Cane, gatto";
        assert_eq!(parse_related_words(raw, 6), vec!["cane", "gatto"]);
    }

    #[test]
    fn related_words_respect_limit_and_drop_function_words() {
        let raw = "cane, il, di, gatto, topo, volpe, lupo, orso, tigre";
        assert_eq!(
            parse_related_words(raw, 3),
            vec!["cane", "gatto", "topo"]
        );
    }

    #[test]
    fn related_words_accept_one_line_chat_replies() {
        // Un modello conversazionale può rispondere con le parole separate
        // da punti invece che da a capo: non vanno perse.
        let raw = "Nonno. Va. Pesca. Barca. Con.";
        assert_eq!(parse_related_words(raw, 8), vec!["nonno", "pesca", "barca"]);
    }

    #[test]
    fn related_words_prompt_carries_the_sentence_context() {
        let prompt = related_words_prompt("banco", Some("Il bambino si siede al banco."));
        assert!(prompt.contains("Il bambino si siede al banco."));
        assert!(prompt.contains("«banco»"));
        assert!(prompt.contains(&RELATED_WORDS_LIMIT.to_string()));
    }

    #[test]
    fn semantic_field_asks_for_concrete_words() {
        let prompt = semantic_field_prompt("Il nonno va a pesca con la barca.");
        assert!(prompt.contains("pesca con la barca"));
        assert!(prompt.contains("pittogrammi CAA"));
        let parsed = parse_semantic_field("nonno\nbarca\npesca\nmare");
        assert_eq!(parsed, vec!["nonno", "barca", "pesca", "mare"]);
    }

    #[test]
    fn availability_requires_files() {
        let mut config = AiConfig::default();
        assert!(!is_available(&config));
        config.enabled = true;
        config.binary_path = "/nonexistent/llama-cli".into();
        config.model_path = "/nonexistent/model.gguf".into();
        assert!(!is_available(&config));
    }
}
