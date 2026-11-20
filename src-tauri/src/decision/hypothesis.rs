//! Costruzione centralizzata delle ipotesi in italiano.
//!
//! Versionare i template è obbligatorio: un cambio di formulazione invalida
//! la cache e va rivalidato sui dati italiani.

use crate::decision::normalize::normalize;
use crate::decision::types::{Candidate, DecisionInput, DecisionTask};

pub const HYPOTHESIS_VERSION: &str = "it-1";
pub const PREMISE_MAX_CHARS: usize = 1400;

/// Restituisce la coppia (premessa, ipotesi) da passare al modello come
/// classificazione a due sequenze. La premessa resta in italiano.
pub fn build_pair(
    task: DecisionTask,
    input: &DecisionInput,
    candidate: &Candidate,
) -> (String, String) {
    let premise = premise(task, input);
    let hypothesis = candidate
        .hypothesis
        .clone()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| {
            // Per la disambiguazione includere la parola indicata aiuta il
            // modello a collegare l'ipotesi al termine, non solo al tema.
            if matches!(task, DecisionTask::PictogramDisambiguation) {
                if let Some(target) = &input.target {
                    let word = normalize(&target.text);
                    if !word.is_empty() {
                        return format!(
                            "In questa frase, «{word}» indica {}.",
                            candidate.label.trim()
                        );
                    }
                }
            }
            hypothesis_for(task, candidate)
        });
    (premise, hypothesis)
}

fn premise(task: DecisionTask, input: &DecisionInput) -> String {
    let text = normalize(&input.text);
    // Per la disambiguazione serve il contesto della frase e la parola
    // indicata; per gli altri task basta la frase.
    if matches!(task, DecisionTask::PictogramDisambiguation) {
        if let Some(target) = &input.target {
            let word = normalize(&target.text);
            if !word.is_empty() {
                let mut premise = format!("Frase: {text} Parola indicata: {word}.");
                if premise.chars().count() > PREMISE_MAX_CHARS {
                    premise = premise.chars().take(PREMISE_MAX_CHARS).collect();
                }
                return premise;
            }
        }
    }
    if text.chars().count() > PREMISE_MAX_CHARS {
        text.chars().take(PREMISE_MAX_CHARS).collect()
    } else {
        text
    }
}

/// Template specifico per task. Le etichette sono brevi e descrittive:
/// il modello zero-shot collega meglio il lessico esplicito.
pub fn hypothesis_for(task: DecisionTask, candidate: &Candidate) -> String {
    let label = candidate.label.trim();
    match task {
        DecisionTask::PictogramDisambiguation => {
            format!("Nel contesto della frase, la parola indicata significa {label}.")
        }
        DecisionTask::CommunicationIntent => format!("Questa frase esprime {label}."),
        DecisionTask::SemanticCategory => {
            format!("Questo concetto appartiene alla categoria {label}.")
        }
        DecisionTask::BoardCategory => {
            format!("Questa frase serve in una tabella comunicativa di {label}.")
        }
        DecisionTask::SemanticSearchRanking => {
            format!("Il pittogramma «{label}» corrisponde alla descrizione richiesta.")
        }
        DecisionTask::DidacticTemplateSelection => {
            format!("Questo modello didattico è adatto all'obiettivo {label}.")
        }
        DecisionTask::LearningObjective => {
            format!("Questa attività ha come obiettivo didattico {label}.")
        }
        DecisionTask::ExerciseType => format!("Questo esercizio è del tipo {label}."),
        DecisionTask::SocialStoryTopic => format!("Questa storia sociale riguarda {label}."),
        DecisionTask::ExpressedEmotion => {
            format!("La frase esprime l'emozione {label}.")
        }
        DecisionTask::InteractiveAction => {
            format!("Il pulsante deve eseguire l'azione {label}.")
        }
        DecisionTask::LayoutTemplate => format!("Il layout adatto è {label}."),
        DecisionTask::CaaSemanticCoherence => {
            format!("Il simbolo «{label}» è coerente con la parola indicata nella frase.")
        }
        DecisionTask::ContentCataloguing => {
            format!("Questo materiale appartiene alla categoria {label}.")
        }
        DecisionTask::CommandRouting => {
            format!("La richiesta dell'utente corrisponde al comando {label}.")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision::types::{DecisionInput, TargetSpan};
    use serde_json::json;

    fn candidate(label: &str) -> Candidate {
        Candidate {
            id: "c1".to_string(),
            label: label.to_string(),
            hypothesis: None,
            locked: false,
            blocked: false,
            metadata: json!({}),
        }
    }

    #[test]
    fn pictogram_premise_contains_sentence_and_target() {
        let input = DecisionInput {
            text: "Ho mangiato una pesca.".to_string(),
            target: Some(TargetSpan {
                text: "pesca".to_string(),
                start: 16,
                end: 21,
            }),
        };
        let (premise, hypothesis) = build_pair(
            DecisionTask::PictogramDisambiguation,
            &input,
            &candidate("frutto"),
        );
        assert!(premise.contains("ho mangiato una pesca"));
        assert!(premise.contains("Parola indicata: pesca"));
        assert!(hypothesis.contains("frutto"));
    }

    #[test]
    fn custom_hypothesis_is_preserved() {
        let input = DecisionInput {
            text: "Vado a pesca.".to_string(),
            target: None,
        };
        let mut custom = candidate("attività");
        custom.hypothesis = Some("La parola indica l'attività di catturare pesci.".to_string());
        let (_, hypothesis) = build_pair(DecisionTask::PictogramDisambiguation, &input, &custom);
        assert_eq!(
            hypothesis,
            "La parola indica l'attività di catturare pesci."
        );
    }

    #[test]
    fn each_task_has_its_own_template() {
        let input = DecisionInput {
            text: "Sono preoccupato.".to_string(),
            target: None,
        };
        let emotion = build_pair(
            DecisionTask::ExpressedEmotion,
            &input,
            &candidate("preoccupazione"),
        );
        let intent = build_pair(
            DecisionTask::CommunicationIntent,
            &input,
            &candidate("preoccupazione"),
        );
        assert!(emotion.1.contains("emozione"));
        assert!(intent.1.contains("esprime"));
        assert_ne!(emotion.1, intent.1);
    }

    #[test]
    fn long_premise_is_truncated() {
        let input = DecisionInput {
            text: "parola ".repeat(2000),
            target: None,
        };
        let (premise, _) = build_pair(
            DecisionTask::CommunicationIntent,
            &input,
            &candidate("richiesta"),
        );
        assert!(premise.chars().count() <= PREMISE_MAX_CHARS);
    }
}
