//! Valutazione semantica sul corpus annotato a mano (`tests/semantic_corpus.json`).
//!
//! Il test non "passa" perché l'accuratezza è alta: verifica la struttura del
//! corpus, misura il motore e scrive un report verificabile in
//! `tests/last_report.json`. Le soglie minime sono dichiarate in fondo.
//!
//! Il confronto con il sistema ibrido (LLM) non è misurabile senza un modello
//! GGUF e un eseguibile llama.cpp configurati: in quel caso il report lo dice
//! esplicitamente invece di inventare numeri.

use std::collections::BTreeMap;
use std::path::PathBuf;

use caa_writer_lib::engine::Engine;
use caa_writer_lib::model::{CaaMode, Intensity, SymbolizeOptions};
use caa_writer_lib::paths::AppPaths;
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
struct Corpus {
    sentences: Vec<Sentence>,
}

#[derive(Deserialize)]
struct Sentence {
    id: String,
    category: String,
    text: String,
    targets: Vec<Target>,
}

#[derive(Deserialize)]
struct Target {
    surface: String,
    #[serde(default)]
    concepts: Vec<String>,
    #[serde(default)]
    strategy: Option<String>,
    #[serde(default)]
    no_symbol: bool,
    #[serde(default)]
    composition: Vec<String>,
}

#[derive(Default, Clone)]
struct Tally {
    targets: usize,
    hits: usize,
    top3: usize,
    approximate: usize,
    misleading: usize,
    missing_expected: usize,
    missing_detected: usize,
}

fn corpus_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tests/semantic_corpus.json")
}

fn resources_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../resources")
}

fn normalize(text: &str) -> String {
    text.to_lowercase()
        .replace(['’', '‘'], "'")
        .trim_matches(|c: char| !c.is_alphanumeric() && c != '\'')
        .to_string()
}

fn build_engine() -> Engine {
    let dir = tempfile::TempDir::new().expect("cartella temporanea");
    let paths = AppPaths::new(dir.path().to_path_buf(), resources_dir());
    let engine = Engine::new(paths).expect("motore inizializzabile");
    // Il database vive nella cartella temporanea: la si tiene viva per il test.
    std::mem::forget(dir);
    engine
}

#[test]
fn semantic_corpus_accuracy() {
    let raw = std::fs::read_to_string(corpus_path()).expect("corpus leggibile");
    let corpus: Corpus = serde_json::from_str(&raw).expect("corpus JSON valido");

    assert!(
        corpus.sentences.len() >= 300,
        "il corpus deve contenere almeno 300 frasi, trovate {}",
        corpus.sentences.len()
    );

    let engine = build_engine();
    let mut overall = Tally::default();
    let mut by_category: BTreeMap<String, Tally> = BTreeMap::new();
    let mut failures: Vec<serde_json::Value> = Vec::new();
    let mut errors: Vec<String> = Vec::new();
    let mut total_ms: u128 = 0;
    let mut worst_ms: u128 = 0;

    for sentence in &corpus.sentences {
        let options = SymbolizeOptions {
            intensity: Intensity::Media,
            mode: CaaMode::Sopra,
            document_key: format!("corpus-{}", sentence.id),
            commercial_only: false,
            pack_id: None,
            paragraph_id: format!("c{}", sentence.id),
            symbolize_articles: false,
            symbolize_prepositions: false,
            symbolize_conjunctions: false,
            manual_units: Vec::new(),
            split_ranges: Vec::new(),
            profile_id: None,
        };
        let result = match engine.symbolize(&sentence.text, &options) {
            Ok(result) => result,
            Err(error) => {
                errors.push(format!("{}: {error}", sentence.id));
                continue;
            }
        };
        total_ms += result.elapsed_ms as u128;
        worst_ms = worst_ms.max(result.elapsed_ms as u128);

        for target in &sentence.targets {
            let tally = by_category.entry(sentence.category.clone()).or_default();
            tally.targets += 1;
            overall.targets += 1;

            // Accoppiamento prudente: prima l'uguaglianza esatta, poi un token
            // che *contiene* la parola cercata (le espressioni multiword hanno
            // surface più lunga, es. "lavarsi i denti" per "denti").
            // Mai il contrario: altrimenti "a" combacerebbe con "pesca".
            let wanted = normalize(&target.surface);
            let token = result
                .tokens
                .iter()
                .find(|candidate| {
                    !candidate.is_punct && normalize(&candidate.surface) == wanted
                })
                .or_else(|| {
                    if wanted.chars().count() < 4 {
                        return None;
                    }
                    result.tokens.iter().find(|candidate| {
                        !candidate.is_punct
                            && normalize(&candidate.surface).contains(&wanted)
                    })
                });

            let token = match token {
                Some(token) => token,
                None => {
                    failures.push(json!({
                        "id": sentence.id,
                        "surface": target.surface,
                        "reason": "nessun token corrispondente",
                        "text": sentence.text,
                    }));
                    continue;
                }
            };

            let has_symbol = token.symbol_id.is_some() || !token.composition.is_empty();
            if token.approximate {
                tally.approximate += 1;
                overall.approximate += 1;
            }

            if target.no_symbol {
                tally.missing_expected += 1;
                overall.missing_expected += 1;
                if !has_symbol {
                    tally.missing_detected += 1;
                    overall.missing_detected += 1;
                    tally.hits += 1;
                    overall.hits += 1;
                } else {
                    failures.push(json!({
                        "id": sentence.id,
                        "surface": target.surface,
                        "reason": "simbolo assegnato a termine senza corrispondenza",
                        "concept": token.concept_id,
                        "strategy": token.strategy,
                    }));
                }
                continue;
            }

            let mut matched = false;
            if !target.composition.is_empty() {
                let parts: Vec<String> = token
                    .composition
                    .iter()
                    .map(|part| part.concept_id.clone())
                    .collect();
                matched = target.composition.iter().all(|expected| parts.contains(expected));
                if matched {
                    tally.hits += 1;
                    overall.hits += 1;
                }
            } else if !target.concepts.is_empty() {
                let concept = token.concept_id.clone().unwrap_or_default();
                matched = target.concepts.iter().any(|expected| expected == &concept);
                if matched {
                    tally.hits += 1;
                    overall.hits += 1;
                }
            }

            let alternative_concepts: Vec<String> = token
                .alternatives
                .iter()
                .filter_map(|alternative| alternative.concept_id.clone())
                .collect();
            if target.concepts.iter().any(|expected| alternative_concepts.contains(expected))
            {
                tally.top3 += 1;
                overall.top3 += 1;
            }

            if !matched && token.approximate && token.composition.is_empty() {
                tally.misleading += 1;
                overall.misleading += 1;
            }

            if !matched {
                let way = match (&target.strategy, token.strategy.as_str()) {
                    (Some(expected), actual) => format!("atteso {expected}, ottenuto {actual}"),
                    (None, actual) => format!("ottenuto {actual}"),
                };
                failures.push(json!({
                    "id": sentence.id,
                    "surface": target.surface,
                    "expected": if target.composition.is_empty() { target.concepts.clone() } else { target.composition.clone() },
                    "actual": token.concept_id,
                    "caption": token.caption,
                    "detail": way,
                    "text": sentence.text,
                }));
            }
        }
    }

    let rate = |part: usize, total: usize| -> f64 {
        if total == 0 {
            0.0
        } else {
            (part as f64 * 100.0) / total as f64
        }
    };

    println!("\n=== Accuratezza semantica (corpus annotato) ===");
    println!("frasi: {} · bersagli: {}", corpus.sentences.len(), overall.targets);
    println!(
        "prima scelta corretta: {:.1}% ({}/{})",
        rate(overall.hits, overall.targets),
        overall.hits,
        overall.targets
    );
    println!(
        "concetto corretto tra le alternative: {:.1}% ({}/{})",
        rate(overall.top3, overall.targets),
        overall.top3,
        overall.targets
    );
    println!(
        "approssimazioni dichiarate: {:.1}% ({}) · approssimazioni fuorvianti: {}",
        rate(overall.approximate, overall.targets),
        overall.approximate,
        overall.misleading
    );
    println!(
        "assenze rilevate correttamente: {}/{}",
        overall.missing_detected, overall.missing_expected
    );
    println!(
        "tempo medio per paragrafo: {:.2} ms · peggiore: {} ms",
        if corpus.sentences.is_empty() {
            0.0
        } else {
            total_ms as f64 / corpus.sentences.len() as f64
        },
        worst_ms
    );

    for (category, tally) in &by_category {
        println!(
            "  {:<20} prima scelta {:>5.1}%  ({}/{})  top-alt {:>5.1}%  fuorvianti {}",
            category,
            rate(tally.hits, tally.targets),
            tally.hits,
            tally.targets,
            rate(tally.top3, tally.targets),
            tally.misleading
        );
    }

    if !errors.is_empty() {
        println!("errori di simbolizzazione: {:?}", errors);
    }

    let report = json!({
        "generated_at": time::OffsetDateTime::now_utc().to_string(),
        "platform": std::env::consts::OS,
        "sentences": corpus.sentences.len(),
        "targets": overall.targets,
        "first_choice_pct": rate(overall.hits, overall.targets),
        "top_alternative_pct": rate(overall.top3, overall.targets),
        "approximate_pct": rate(overall.approximate, overall.targets),
        "misleading_approximations": overall.misleading,
        "missing_expected": overall.missing_expected,
        "missing_detected": overall.missing_detected,
        "avg_ms_per_paragraph": if corpus.sentences.is_empty() { 0.0 } else { total_ms as f64 / corpus.sentences.len() as f64 },
        "worst_ms": worst_ms,
        "hybrid_llm": "non misurato: richiede llama.cpp e un modello GGUF configurati",
        "categories": by_category.iter().map(|(name, tally)| {
            (
                name.clone(),
                json!({
                    "targets": tally.targets,
                    "first_choice_pct": rate(tally.hits, tally.targets),
                    "top_alternative_pct": rate(tally.top3, tally.targets),
                    "approximate": tally.approximate,
                    "misleading": tally.misleading,
                }),
            )
        }).collect::<BTreeMap<String, serde_json::Value>>(),
        "failures": failures,
        "errors": errors,
    });

    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tests/last_report.json");
    std::fs::write(&out, serde_json::to_string_pretty(&report).expect("report serializzabile"))
        .expect("report scrivibile");
    println!("report scritto in {}", out.display());

    // Soglie di regressione: impediscono peggioramenti, senza pretendere
    // un'accuratezza che il corpus non ha ancora dimostrato.
    // Soglie fissate dopo la prima misurazione reale (97,6% prima scelta,
    // 2 approssimazioni fuorvianti): servono a intercettare regressioni, non
    // a dichiarare un'accuratezza che il corpus non ha ancora dimostrato su
    // altri domini.
    assert!(
        rate(overall.hits, overall.targets) >= 90.0,
        "prima scelta sotto la soglia di regressione del 90%"
    );
    assert!(
        overall.misleading <= 5,
        "troppe approssimazioni fuorvianti: {}",
        overall.misleading
    );
    assert!(
        overall.missing_detected == overall.missing_expected,
        "assenze non rilevate: {}/{} rilevate",
        overall.missing_detected,
        overall.missing_expected
    );
    assert!(
        errors.is_empty(),
        "la simbolizzazione ha prodotto errori: {errors:?}"
    );
}
