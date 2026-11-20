//! Adattatore ONNX per Horizon multilingual-zeroshot-small.
//!
//! Carica `onnx/model_quantized.onnx` e `tokenizer.json` dalla cartella
//! modelli dell'utente. Nessun altro classificatore neurale è ammesso.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use ort::session::Session;
use ort::value::Tensor;
use tokenizers::{Tokenizer, TruncationParams};

use crate::decision::engine::HorizonScorer;
use crate::decision::model;
use crate::paths::AppPaths;

const MAX_TOKENS: usize = 1024;

struct Loaded {
    session: Session,
    tokenizer: Tokenizer,
}

pub struct OnnxHorizon {
    dir: PathBuf,
    loaded: Mutex<Option<Loaded>>,
    revision: String,
    load_ms: AtomicU64,
}

impl OnnxHorizon {
    pub fn from_installed(paths: &AppPaths) -> Option<Self> {
        let dir = model::model_dir(paths);
        match Self::from_dir(&dir) {
            Ok(scorer) => Some(scorer),
            Err(_) => None,
        }
    }

    /// Carica da una cartella esplicita. Usato dai test reali con
    /// `CAA_HORIZON_MODEL_DIR`.
    pub fn from_dir(dir: &Path) -> Result<Self, String> {
        let started = std::time::Instant::now();
        let model_path = dir.join("onnx/model_quantized.onnx");
        let tokenizer_path = dir.join("tokenizer.json");
        if !model_path.is_file() {
            return Err(format!("modello ONNX assente: {}", model_path.display()));
        }
        if !tokenizer_path.is_file() {
            return Err(format!("tokenizer assente: {}", tokenizer_path.display()));
        }
        let _ = started;
        Ok(Self {
            dir: dir.to_path_buf(),
            loaded: Mutex::new(None),
            revision: model::REVISION.to_string(),
            load_ms: AtomicU64::new(0),
        })
    }

    fn load(dir: &Path) -> Result<Loaded, String> {
        let model_path = dir.join("onnx/model_quantized.onnx");
        let tokenizer_path = dir.join("tokenizer.json");
        let mut tokenizer =
            Tokenizer::from_file(&tokenizer_path).map_err(|error| format!("tokenizer: {error}"))?;
        tokenizer
            .with_truncation(Some(TruncationParams {
                max_length: MAX_TOKENS,
                ..Default::default()
            }))
            .map_err(|error| format!("truncation: {error}"))?;
        let session = Session::builder()
            .map_err(|error| format!("sessione ONNX: {error}"))?
            .with_intra_threads(threads_for_profile())
            .map_err(|error| format!("thread ONNX: {error}"))?
            .commit_from_file(&model_path)
            .map_err(|error| format!("caricamento {}: {error}", model_path.display()))?;
        let names: Vec<&str> = session
            .inputs
            .iter()
            .map(|input| input.name.as_str())
            .collect();
        for required in ["input_ids", "attention_mask"] {
            if !names.contains(&required) {
                return Err(format!(
                    "input ONNX mancante: {required} (trovati: {})",
                    names.join(", ")
                ));
            }
        }
        Ok(Loaded { session, tokenizer })
    }

    /// Millisecondi dell'ultimo caricamento (0 se non ancora caricato).
    pub fn load_ms(&self) -> u64 {
        self.load_ms.load(Ordering::Relaxed)
    }

    fn score_pair(&self, premise: &str, hypothesis: &str) -> Result<f32, String> {
        let mut guard = self
            .loaded
            .lock()
            .map_err(|_| "sessione ONNX bloccata".to_string())?;
        if guard.is_none() {
            let started = std::time::Instant::now();
            *guard = Some(Self::load(&self.dir)?);
            self.load_ms
                .store(started.elapsed().as_millis() as u64, Ordering::Relaxed);
        }
        let loaded = guard.as_ref().expect("sessione caricata");
        let encoding = loaded
            .tokenizer
            .encode((premise, hypothesis), true)
            .map_err(|error| format!("tokenizzazione: {error}"))?;
        let ids: Vec<i64> = encoding.get_ids().iter().map(|&id| id as i64).collect();
        let mask: Vec<i64> = encoding
            .get_attention_mask()
            .iter()
            .map(|&value| value as i64)
            .collect();
        if ids.is_empty() {
            return Err("tokenizzazione vuota".to_string());
        }
        let shape = [1usize, ids.len()];
        let input_ids = Tensor::from_array((shape, ids.into_boxed_slice()))
            .map_err(|error| format!("tensore input_ids: {error}"))?;
        let attention_mask = Tensor::from_array((shape, mask.into_boxed_slice()))
            .map_err(|error| format!("tensore attention_mask: {error}"))?;
        let inputs = ort::inputs![
            "input_ids" => input_ids,
            "attention_mask" => attention_mask,
        ]
        .map_err(|error| format!("input ONNX: {error}"))?;
        let outputs = loaded
            .session
            .run(inputs)
            .map_err(|error| format!("inferenza: {error}"))?;
        let logits = outputs["logits"]
            .try_extract_tensor::<f32>()
            .map_err(|error| format!("logits: {error}"))?;
        let values: Vec<f32> = logits.iter().copied().collect();
        if values.len() < 2 {
            return Err(format!("logits inattesi: {}", values.len()));
        }
        // Classi: 0 = not_entailment, 1 = entailment.
        let max = values[0].max(values[1]);
        let e0 = (values[0] - max).exp();
        let e1 = (values[1] - max).exp();
        Ok(e1 / (e0 + e1))
    }
}

impl HorizonScorer for OnnxHorizon {
    fn score(&self, pairs: &[(String, String)]) -> Result<Vec<f32>, String> {
        let mut scores = Vec::with_capacity(pairs.len());
        for (premise, hypothesis) in pairs {
            scores.push(self.score_pair(premise, hypothesis)?);
        }
        Ok(scores)
    }

    fn method(&self) -> &'static str {
        "neural"
    }

    fn model_id(&self) -> &str {
        model::MODEL_ID
    }

    fn model_revision(&self) -> &str {
        &self.revision
    }

    fn is_neural(&self) -> bool {
        true
    }
}

fn threads_for_profile() -> usize {
    // Su macchine con 4 GB di RAM meglio pochi thread: il modello è piccolo
    // ma i tensori temporanei contano.
    let available = std::thread::available_parallelism()
        .map(|value| value.get())
        .unwrap_or(2);
    available.clamp(1, 4)
}

/// Cartella del modello, con override per test e sviluppo.
pub fn model_dir_from_env() -> Option<PathBuf> {
    std::env::var_os("CAA_HORIZON_MODEL_DIR").map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn real_model_dir() -> Option<PathBuf> {
        let dir = model_dir_from_env()?;
        if dir.join("onnx/model_quantized.onnx").is_file() {
            Some(dir)
        } else {
            None
        }
    }

    #[test]
    fn refuses_missing_model() {
        let dir = tempfile::tempdir().expect("tempdir");
        let error = OnnxHorizon::from_dir(dir.path())
            .err()
            .expect("errore atteso");
        assert!(error.contains("modello ONNX assente"));
    }

    /// Test reale: richiede `CAA_HORIZON_MODEL_DIR`; senza modello lo dichiara
    /// esplicitamente invece di fingere un risultato.
    #[test]
    fn real_inference_ranks_entailment() {
        let Some(dir) = real_model_dir() else {
            eprintln!("CAA_HORIZON_MODEL_DIR assente: test reale non eseguito");
            return;
        };
        let scorer = OnnxHorizon::from_dir(&dir).expect("caricamento modello reale");
        assert!(scorer.is_neural());
        let pairs = vec![
            (
                "Frase: Ho mangiato una pesca. Parola indicata: pesca.".to_string(),
                "Nel contesto della frase, la parola indicata significa frutto.".to_string(),
            ),
            (
                "Frase: Ho mangiato una pesca. Parola indicata: pesca.".to_string(),
                "Nel contesto della frase, la parola indicata significa attività di catturare pesci."
                    .to_string(),
            ),
        ];
        let scores = scorer.score(&pairs).expect("inferenza reale");
        eprintln!("punteggi reali: {scores:?} (load {} ms)", scorer.load_ms());
        assert_eq!(scores.len(), 2);
        assert!(
            scores[0] > scores[1],
            "atteso frutto > attività: {scores:?}"
        );
    }

    /// Sonda di formulazione, eseguita solo con il modello installato.
    #[test]
    fn probe_templates() {
        let Some(dir) = real_model_dir() else {
            eprintln!("CAA_HORIZON_MODEL_DIR assente: sonda non eseguita");
            return;
        };
        let scorer = OnnxHorizon::from_dir(&dir).expect("caricamento modello reale");
        let premise = "Frase: Ho mangiato una pesca. Parola indicata: pesca.";
        let variants: &[(&str, &str)] = &[
            ("A1", "Nel contesto della frase, la parola indicata significa frutto."),
            ("A2", "Nel contesto della frase, la parola indicata significa attività di catturare pesci."),
            ("B1", "In questa frase, pesca indica un frutto da mangiare."),
            ("B2", "In questa frase, pesca indica l'azione di catturare i pesci."),
            ("C1", "La parola pesca significa frutto."),
            ("C2", "La parola pesca significa attività di pesca."),
            ("D1", "Questa frase parla di un frutto che si mangia."),
            ("D2", "Questa frase parla dell'attività di pescare."),
        ];
        for (tag, hypothesis) in variants {
            let score = scorer
                .score(&[(premise.to_string(), hypothesis.to_string())])
                .expect("inferenza")[0];
            eprintln!("{tag}: {score:.4} — {hypothesis}");
        }
    }

    /// Disambiguazione reale sui casi obbligatori della specifica.
    /// Usa il DecisionEngine completo (Horizon + regole deterministiche).
    /// Senza modello il test lo dichiara e non finge alcun esito.
    #[test]
    fn real_disambiguation_examples() {
        use crate::decision::engine::DecisionEngine;
        use crate::decision::types::{
            Candidate, DecisionInput, DecisionMode, DecisionOptions, DecisionRequest, DecisionTask,
            TargetSpan,
        };
        use serde_json::json;
        use std::sync::Arc;

        let Some(dir) = real_model_dir() else {
            eprintln!("CAA_HORIZON_MODEL_DIR assente: test reale non eseguito");
            return;
        };
        let scorer = OnnxHorizon::from_dir(&dir).expect("caricamento modello reale");
        let engine = DecisionEngine::with_scorer(Arc::new(scorer));

        fn span(text: &str, word: &str) -> TargetSpan {
            let byte = text.find(word).expect("parola presente");
            let start = text[..byte].chars().count();
            TargetSpan {
                text: word.to_string(),
                start,
                end: start + word.chars().count(),
            }
        }
        fn candidate(id: &str, label: &str) -> Candidate {
            Candidate {
                id: id.to_string(),
                label: label.to_string(),
                hypothesis: None,
                locked: false,
                blocked: false,
                metadata: json!({}),
            }
        }

        let cases: Vec<(&str, &str, [(&str, &str); 2], usize)> = vec![
            (
                "Ho mangiato una pesca.",
                "pesca",
                [
                    ("frutto", "frutto (alimento)"),
                    ("attivita", "attività di pesca (catturare pesci)"),
                ],
                0,
            ),
            (
                "Vado a pesca.",
                "pesca",
                [
                    ("frutto", "frutto (alimento)"),
                    ("attivita", "attività di pesca (catturare pesci)"),
                ],
                1,
            ),
            (
                "Amo la pizza.",
                "Amo",
                [
                    ("amore", "amore (sentimento)"),
                    ("amo", "amo da pesca (oggetto per pescare)"),
                ],
                0,
            ),
            (
                "Ho comprato un amo.",
                "amo",
                [
                    ("amore", "amore (sentimento)"),
                    ("amo", "amo da pesca (oggetto per pescare)"),
                ],
                1,
            ),
            (
                "Mangio il riso.",
                "riso",
                [("alimento", "riso (alimento)"), ("ridere", "riso (risata)")],
                0,
            ),
            (
                "Il suo riso è contagioso.",
                "riso",
                [("alimento", "riso (alimento)"), ("ridere", "riso (risata)")],
                1,
            ),
            (
                "Chiudi la porta.",
                "porta",
                [
                    ("porta_casa", "porta (di una stanza)"),
                    ("portare", "porta (verbo portare)"),
                ],
                0,
            ),
            (
                "La bambina porta un libro.",
                "porta",
                [
                    ("porta_casa", "porta (di una stanza)"),
                    ("portare", "porta (verbo portare)"),
                ],
                1,
            ),
            (
                "Suono il piano.",
                "piano",
                [
                    ("pianoforte", "pianoforte (strumento musicale)"),
                    ("avverbio", "piano (lentamente)"),
                ],
                0,
            ),
            (
                "Cammina piano.",
                "piano",
                [
                    ("pianoforte", "pianoforte (strumento musicale)"),
                    ("avverbio", "piano (lentamente)"),
                ],
                1,
            ),
            (
                "Scrivo con la penna.",
                "penna",
                [
                    ("penna_scrivere", "penna (per scrivere)"),
                    ("piuma", "penna (piuma di uccello)"),
                ],
                0,
            ),
            (
                "L'uccello ha perso una penna.",
                "penna",
                [
                    ("penna_scrivere", "penna (per scrivere)"),
                    ("piuma", "penna (piuma di uccello)"),
                ],
                1,
            ),
        ];

        let mut correct = 0usize;
        for (text, word, labels, expected) in &cases {
            let input = DecisionInput {
                text: (*text).to_string(),
                target: Some(span(text, word)),
            };
            let candidates = [
                candidate(labels[0].0, labels[0].1),
                candidate(labels[1].0, labels[1].1),
            ];
            let request = DecisionRequest {
                request_id: None,
                task: DecisionTask::PictogramDisambiguation,
                language: "it".to_string(),
                input,
                candidates: candidates.to_vec(),
                context: Default::default(),
                options: DecisionOptions {
                    mode: Some(DecisionMode::Suggest),
                    multi_label: Some(false),
                    ..Default::default()
                },
            };
            let response = engine.classify(request);
            let selected = response.selected_candidate_id.clone().unwrap_or_default();
            let ok = selected == labels[*expected].0;
            if ok {
                correct += 1;
            }
            let scores: Vec<String> = response
                .ranked_candidates
                .iter()
                .map(|item| format!("{}={:.4}", item.id, item.score))
                .collect();
            eprintln!(
                "{} «{}» → {} [{}] ({})",
                if ok { "OK  " } else { "MISS" },
                text,
                if selected.is_empty() {
                    "<astensione>"
                } else {
                    &selected
                },
                scores.join(" "),
                response.metadata.method
            );
        }
        eprintln!("accuratezza disambiguazione: {correct}/{}", cases.len());
        assert_eq!(correct, cases.len(), "casi errati: vedi output");
    }

    /// Frasi nuove, non usate per scrivere le regole: controlla che il
    /// sistema generalizzi sugli indizi e non memorizzi le frasi del test.
    #[test]
    fn real_disambiguation_heldout() {
        use crate::decision::engine::DecisionEngine;
        use crate::decision::types::{
            Candidate, DecisionInput, DecisionMode, DecisionOptions, DecisionRequest, DecisionTask,
            TargetSpan,
        };
        use serde_json::json;
        use std::sync::Arc;

        let Some(dir) = real_model_dir() else {
            eprintln!("CAA_HORIZON_MODEL_DIR assente: test held-out non eseguito");
            return;
        };
        let scorer = OnnxHorizon::from_dir(&dir).expect("caricamento modello reale");
        let engine = DecisionEngine::with_scorer(Arc::new(scorer));

        fn span(text: &str, word: &str) -> TargetSpan {
            let byte = text.find(word).expect("parola presente");
            let start = text[..byte].chars().count();
            TargetSpan {
                text: word.to_string(),
                start,
                end: start + word.chars().count(),
            }
        }
        fn candidate(id: &str, label: &str) -> Candidate {
            Candidate {
                id: id.to_string(),
                label: label.to_string(),
                hypothesis: None,
                locked: false,
                blocked: false,
                metadata: json!({}),
            }
        }

        let cases: Vec<(&str, &str, [(&str, &str); 2], usize)> = vec![
            (
                "La pesca matura è dolce.",
                "pesca",
                [
                    ("frutto", "frutto (alimento)"),
                    ("attivita", "attività di pesca (catturare pesci)"),
                ],
                0,
            ),
            (
                "Andiamo al mare per la pesca.",
                "pesca",
                [
                    ("frutto", "frutto (alimento)"),
                    ("attivita", "attività di pesca (catturare pesci)"),
                ],
                1,
            ),
            (
                "Ti amo tanto.",
                "amo",
                [
                    ("amore", "amore (sentimento)"),
                    ("amo", "amo da pesca (oggetto per pescare)"),
                ],
                0,
            ),
            (
                "L'amo è attaccato alla lenza.",
                "amo",
                [
                    ("amore", "amore (sentimento)"),
                    ("amo", "amo da pesca (oggetto per pescare)"),
                ],
                1,
            ),
            (
                "Il riso è pronto in cucina.",
                "riso",
                [("alimento", "riso (alimento)"), ("ridere", "riso (risata)")],
                0,
            ),
            (
                "Il riso di Marco è contagioso.",
                "riso",
                [("alimento", "riso (alimento)"), ("ridere", "riso (risata)")],
                1,
            ),
            (
                "Apri la porta, per favore.",
                "porta",
                [
                    ("porta_casa", "porta (di una stanza)"),
                    ("portare", "porta (verbo portare)"),
                ],
                0,
            ),
            (
                "Marco porta la borsa.",
                "porta",
                [
                    ("porta_casa", "porta (di una stanza)"),
                    ("portare", "porta (verbo portare)"),
                ],
                1,
            ),
            (
                "Uso la penna per il compito.",
                "penna",
                [
                    ("penna_scrivere", "penna (per scrivere)"),
                    ("piuma", "penna (piuma di uccello)"),
                ],
                0,
            ),
            (
                "L'uccello ha una penna bianca.",
                "penna",
                [
                    ("penna_scrivere", "penna (per scrivere)"),
                    ("piuma", "penna (piuma di uccello)"),
                ],
                1,
            ),
            (
                "Suono il piano ogni giorno.",
                "piano",
                [
                    ("pianoforte", "pianoforte (strumento musicale)"),
                    ("avverbio", "piano (lentamente)"),
                ],
                0,
            ),
        ];

        let mut correct = 0usize;
        for (text, word, labels, expected) in &cases {
            let request = DecisionRequest {
                request_id: None,
                task: DecisionTask::PictogramDisambiguation,
                language: "it".to_string(),
                input: DecisionInput {
                    text: (*text).to_string(),
                    target: Some(span(text, word)),
                },
                candidates: vec![
                    candidate(labels[0].0, labels[0].1),
                    candidate(labels[1].0, labels[1].1),
                ],
                context: Default::default(),
                options: DecisionOptions {
                    mode: Some(DecisionMode::Suggest),
                    multi_label: Some(false),
                    ..Default::default()
                },
            };
            let response = engine.classify(request);
            let ok = response.selected_candidate_id.as_deref() == Some(labels[*expected].0);
            if ok {
                correct += 1;
            }
            eprintln!(
                "{} «{}» → {} ({})",
                if ok { "OK  " } else { "MISS" },
                text,
                response
                    .selected_candidate_id
                    .unwrap_or_else(|| "<astensione>".to_string()),
                response.metadata.method
            );
        }
        eprintln!("accuratezza held-out: {correct}/{}", cases.len());
        assert_eq!(correct, cases.len(), "casi errati: vedi output");
    }
}
