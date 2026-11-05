//! Strumenti IA tipizzati.
//!
//! Ogni strumento ha: prompt specifico, limite di lunghezza, validazione
//! dell'output e strategia di ripiego deterministica. Il modello non esegue
//! comandi: propone testo o dati che vengono validati prima dell'uso.
//!
//! Regola: quando una risposta deterministica è equivalente, si usa quella e
//! il modello non viene interrogato.

use std::sync::Arc;

use serde_json::{json, Value};

use super::service::Service;

/// Elenco delle funzioni realmente presenti nell'applicazione. Serve
/// all'assistente per non inventare pulsanti o comandi.
const FEATURES: &[(&[&str], &str)] = &[
    (
        &["simbolo", "pittogramma", "aggiungere simbolo"],
        "Per aggiungere un pittogramma: seleziona la parola nell'editor e scegli un'alternativa nel pannello «Alternative», oppure usa la scheda «Cerca» per cercare nella libreria. Puoi anche importare un SVG dalla scheda Impostazioni.",
    ),
    (
        &["tabella", "griglia", "comunicazione"],
        "Le tabelle si creano dalla schermata iniziale («Tabelle di comunicazione»): puoi partire da un modello, descrivere la tabella oppure comporla a mano. Il Visual Resource Studio è nella scheda «Studio» del pannello laterale.",
    ),
    (
        &["stampa", "pdf", "esporta"],
        "Per stampare o salvare in PDF usa il pulsante «PDF» nella barra in alto. Sono disponibili anche DOCX e HTML.",
    ),
    (
        &["semplificare", "semplifica", "frase semplice"],
        "Puoi chiedere una semplificazione nell'area «Assistente IA» del pannello laterale: il testo originale resta invariato finché non accetti la proposta.",
    ),
    (
        &["impostazioni", "configurazione"],
        "Il pannello Impostazioni si apre dalla barra in alto o dalla scheda «Impostazioni» del pannello laterale: contiene modalità CAA, accessibilità, repertori, gestione dei modelli e informazioni.",
    ),
    (
        &["modello", "ia", "intelligenza artificiale", "neurale"],
        "La sezione «Gestione modelli» nelle Impostazioni mostra il modello attivo, l'hardware rilevato e le metriche. Il modello è incluso nell'applicazione e funziona senza rete.",
    ),
];

#[derive(Debug, Clone, serde::Serialize)]
pub struct ToolOutcome {
    pub tool: String,
    pub result: Value,
    pub source: String,
    pub notes: Vec<String>,
}

fn clean(text: &str) -> String {
    text.trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim()
        .to_string()
}

fn extract_json_object(text: &str) -> Option<Value> {
    let start = text.find('{')?;
    let end = text.rfind('}')?;
    if end <= start {
        return None;
    }
    serde_json::from_str(&text[start..=end]).ok()
}

fn words(text: &str) -> Vec<&str> {
    text.split_whitespace().filter(|w| !w.is_empty()).collect()
}

fn sentences(text: &str) -> Vec<&str> {
    text.split(['.', '!', '?', ';'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect()
}

/// Semplificazione deterministica: divide le frasi lunghe alle congiunzioni.
/// Non è una riscrittura: è una riduzione di complessità verificabile.
fn simplify_deterministic(text: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    for sentence in sentences(text) {
        let parts: Vec<&str> = sentence
            .split(", oppure ")
            .flat_map(|chunk| chunk.split(" e "))
            .flat_map(|chunk| chunk.split(" ma "))
            .collect();
        if parts.len() > 1 && words(sentence).len() > 12 {
            for part in parts {
                let part = part.trim();
                if !part.is_empty() {
                    out.push(part.to_string());
                }
            }
        } else {
            out.push(sentence.to_string());
        }
    }
    out.join(". ") + "."
}

pub fn analyse_clarity(text: &str) -> ToolOutcome {
    let sentences = sentences(text);
    let total_words = words(text).len();
    let average = if sentences.is_empty() {
        0.0
    } else {
        total_words as f32 / sentences.len() as f32
    };
    let long_words = words(text)
        .iter()
        .filter(|word| word.chars().filter(|c| c.is_alphabetic()).count() > 12)
        .count();
    let subordinate = text.to_lowercase().matches(" che ").count()
        + text.to_lowercase().matches(" perché ").count()
        + text.to_lowercase().matches(" il quale ").count();

    let mut issues: Vec<String> = Vec::new();
    if average > 18.0 {
        issues.push(format!(
            "Frasi lunghe: in media {:.0} parole per frase.",
            average
        ));
    }
    if long_words > 0 {
        issues.push(format!("{long_words} parole molto lunghe."));
    }
    if subordinate > 2 {
        issues.push(format!("{subordinate} subordinate: la struttura è complessa."));
    }
    if sentences.len() > 6 && total_words > 120 {
        issues.push("Testo denso: valuta di dividerlo in più paragrafi.".to_string());
    }

    let mut score = 100.0f32;
    score -= (average - 12.0).max(0.0) * 2.0;
    score -= long_words as f32 * 2.0;
    score -= subordinate as f32 * 3.0;
    let score = score.clamp(0.0, 100.0).round() as u32;

    ToolOutcome {
        tool: "analyse_text_clarity".to_string(),
        result: json!({
            "score": score,
            "average_words_per_sentence": (average * 10.0).round() / 10.0,
            "long_words": long_words,
            "subordinates": subordinate,
            "issues": issues,
        }),
        source: "deterministico".to_string(),
        notes: vec!["Misura calcolata sul testo, senza modello: è un indicatore, non un giudizio.".to_string()],
    }
}

pub fn explain_feature(question: &str) -> ToolOutcome {
    let lower = question.to_lowercase();
    let mut best: Option<(&str, usize)> = None;
    for (keywords, answer) in FEATURES {
        let hits = keywords.iter().filter(|keyword| lower.contains(**keyword)).count();
        if hits > 0 && best.map(|(_, score)| hits > score).unwrap_or(true) {
            best = Some((answer, hits));
        }
    }
    match best {
        Some((answer, _)) => ToolOutcome {
            tool: "explain_application_feature".to_string(),
            result: json!({ "answer": answer }),
            source: "deterministico".to_string(),
            notes: vec!["Risposta presa dall'elenco delle funzioni realmente presenti.".to_string()],
        },
        None => ToolOutcome {
            tool: "explain_application_feature".to_string(),
            result: json!({
                "answer": "Non ho una risposta verificata per questa domanda. Le funzioni documentate sono: simbolizzazione, ricerca nella libreria, tabelle e materiali nello Studio, stampa/PDF/DOCX/HTML, impostazioni e gestione dei modelli.",
            }),
            source: "deterministico".to_string(),
            notes: vec!["Nessuna funzione inventata: la domanda non corrisponde a una funzione registrata.".to_string()],
        },
    }
}

pub fn prepare_svg_description(concept: &str, label: &str, meaning: &str) -> ToolOutcome {
    let brief = format!(
        "Pittogramma CAA per «{}» (concetto: {}). Deve mostrare: {}. Stile: contorno netto, \
         pochi elementi, sfondo trasparente, leggibile a 48 px, senza testo. Colori: massimo \
         tre, con contrasto sufficiente anche in bianco e nero.",
        label, concept, meaning
    );
    ToolOutcome {
        tool: "prepare_svg_description".to_string(),
        result: json!({ "description": brief }),
        source: "deterministico".to_string(),
        notes: vec![
            "Descrizione pronta per un generatore esterno; nessun SVG viene inventato dal modello testuale."
                .to_string(),
        ],
    }
}

fn simplify_prompt(text: &str) -> String {
    format!(
        "Riscrivi in italiano semplice la frase seguente, per una persona che usa la CAA.\n\
         Regole: massimo 12 parole, una sola frase, niente figure retoriche, mantieni il senso.\n\
         Rispondi solo con la frase riscritta.\n\n\
         Frase: {text}\n\
         Risposta:"
    )
}

fn sequence_prompt(task: &str, steps: u32) -> String {
    format!(
        "Scompóni in passaggi osservabili l'attività: «{task}».\n\
         Rispondi SOLO con un oggetto JSON: {{\"steps\": [\"...\", \"...\"]}}\n\
         Massimo {steps} passaggi, ognuno di massimo 10 parole, in ordine cronologico.\n\
         Risposta:"
    )
}

pub fn run(service: Arc<Service>, tool: &str, input: &Value) -> Result<ToolOutcome, String> {
    let text = input
        .get("text")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .trim()
        .to_string();

    match tool {
        "analyse_text_clarity" => {
            if text.is_empty() {
                return Err("serve un testo da analizzare".to_string());
            }
            Ok(analyse_clarity(&text))
        }
        "explain_application_feature" => {
            let question = input
                .get("question")
                .and_then(|v| v.as_str())
                .unwrap_or(&text)
                .to_string();
            if question.is_empty() {
                return Err("serve una domanda".to_string());
            }
            Ok(explain_feature(&question))
        }
        "prepare_svg_description" => {
            let label = input
                .get("label")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            if label.is_empty() {
                return Err("serve un'etichetta".to_string());
            }
            Ok(prepare_svg_description(
                input.get("concept").and_then(|v| v.as_str()).unwrap_or(""),
                label,
                input.get("meaning").and_then(|v| v.as_str()).unwrap_or(""),
            ))
        }
        "simplify_sentence" => {
            if text.is_empty() {
                return Err("serve una frase".to_string());
            }
            let fallback = simplify_deterministic(&text);
            let cancel = service.cancel_handle();
            let answer = service.complete(simplify_prompt(&text), 64, 7, cancel);
            let mut notes = Vec::new();
            let original_words = words(&text).len().max(1);
            let candidate = match answer {
                Ok(raw) => {
                    let cleaned = clean(&raw);
                    let cleaned = cleaned.lines().next().unwrap_or_default().trim().to_string();
                    if cleaned.is_empty() {
                        None
                    } else if words(&cleaned).len() > (original_words * 3 / 2).max(12) {
                        notes.push("La proposta del modello era più lunga dell'originale: scartata.".to_string());
                        None
                    } else {
                        Some(cleaned)
                    }
                }
                Err(error) => {
                    notes.push(format!("Modello non disponibile ({error}): semplificazione deterministica."));
                    None
                }
            };
            match candidate {
                Some(simplified) => Ok(ToolOutcome {
                    tool: tool.to_string(),
                    result: json!({ "simplified": simplified, "original": text }),
                    source: "neurale".to_string(),
                    notes,
                }),
                None => Ok(ToolOutcome {
                    tool: tool.to_string(),
                    result: json!({ "simplified": fallback, "original": text }),
                    source: "deterministico".to_string(),
                    notes,
                }),
            }
        }
        "generate_task_sequence" => {
            let task = input
                .get("task")
                .and_then(|v| v.as_str())
                .unwrap_or(&text)
                .to_string();
            if task.is_empty() {
                return Err("serve un'attività da scomporre".to_string());
            }
            let cancel = service.cancel_handle();
            let answer = service.complete(sequence_prompt(&task, 8), 160, 4, cancel);
            let mut notes = Vec::new();
            let parsed = match answer {
                Ok(raw) => extract_json_object(&clean(&raw)).and_then(|value| {
                    value
                        .get("steps")
                        .and_then(|steps| steps.as_array())
                        .map(|steps| {
                            steps
                                .iter()
                                .filter_map(|step| step.as_str().map(str::trim))
                                .filter(|step| !step.is_empty() && words(step).len() <= 14)
                                .take(10)
                                .map(str::to_string)
                                .collect::<Vec<_>>()
                        })
                        .filter(|steps| (2..=10).contains(&steps.len()))
                }),
                Err(error) => {
                    notes.push(format!("Modello non disponibile ({error}): sequenza deterministica."));
                    None
                }
            };
            match parsed {
                Some(steps) => Ok(ToolOutcome {
                    tool: tool.to_string(),
                    result: json!({ "task": task, "steps": steps }),
                    source: "neurale".to_string(),
                    notes,
                }),
                None => Ok(ToolOutcome {
                    tool: tool.to_string(),
                    result: json!({
                        "task": task,
                        "steps": [
                            "Preparare il materiale necessario",
                            "Eseguire il primo passaggio",
                            "Completare il passaggio successivo",
                            "Controllare il risultato",
                        ],
                    }),
                    source: "deterministico".to_string(),
                    notes: vec![
                        "Sequenza generica: il modello non ha prodotto passaggi validi.".to_string(),
                        "Va adattata manualmente all'attività.".to_string(),
                    ],
                }),
            }
        }
        other => Err(format!("strumento non riconosciuto: {other}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicU32;
    use std::sync::Mutex;
    use std::time::Duration;

    use crate::ai_engine::service::Executor;

    struct Fixed {
        answer: Mutex<String>,
        calls: AtomicU32,
    }

    impl Executor for Fixed {
        fn execute(&self, _prompt: &str, _max_tokens: u32) -> Result<String, String> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(self.answer.lock().unwrap().clone())
        }
        fn describe(&self) -> String {
            "finto".to_string()
        }
    }

    use std::sync::atomic::Ordering;

    fn service(answer: &str) -> Arc<Service> {
        Arc::new(Service::new(
            Arc::new(Fixed {
                answer: Mutex::new(answer.to_string()),
                calls: AtomicU32::new(0),
            }),
            Duration::from_secs(5),
        ))
    }

    #[test]
    fn clarity_is_deterministic_and_flags_long_sentences() {
        let outcome = run(
            service("x"),
            "analyse_text_clarity",
            &json!({"text": "Il bambino che ha mangiato la mela rossa e grande corre veloce verso la scuola perché è tardi e la maestra lo aspetta con impazienza"}),
        )
        .unwrap();
        assert_eq!(outcome.source, "deterministico");
        assert!(outcome.result["issues"].as_array().unwrap().len() >= 1);
    }

    #[test]
    fn feature_answer_comes_from_registry() {
        let outcome = run(
            service("x"),
            "explain_application_feature",
            &json!({"question": "Come stampo questa pagina?"}),
        )
        .unwrap();
        assert!(outcome.result["answer"].as_str().unwrap().contains("PDF"));
    }

    #[test]
    fn unknown_feature_is_admitted() {
        let outcome = run(
            service("x"),
            "explain_application_feature",
            &json!({"question": "Come attivo il pilota automatico?"}),
        )
        .unwrap();
        assert!(outcome.result["answer"].as_str().unwrap().contains("Non ho una risposta verificata"));
    }

    #[test]
    fn sequence_falls_back_when_model_returns_garbage() {
        let outcome = run(
            service("non json"),
            "generate_task_sequence",
            &json!({"task": "preparare lo zaino"}),
        )
        .unwrap();
        assert_eq!(outcome.source, "deterministico");
        assert!(outcome.result["steps"].as_array().unwrap().len() >= 2);
    }

    #[test]
    fn sequence_accepts_valid_json() {
        let outcome = run(
            service("{\"steps\":[\"Aprire lo zaino\",\"Mettere i libri\",\"Chiudere lo zaino\"]}"),
            "generate_task_sequence",
            &json!({"task": "preparare lo zaino"}),
        )
        .unwrap();
        assert_eq!(outcome.source, "neurale");
        assert_eq!(outcome.result["steps"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn simplification_rejects_overlong_model_output() {
        let long = "parola ".repeat(40);
        let outcome = run(
            service(&long),
            "simplify_sentence",
            &json!({"text": "Il bambino corre."}),
        )
        .unwrap();
        assert_eq!(outcome.source, "deterministico");
    }
}
