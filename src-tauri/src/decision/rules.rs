//! Regole deterministiche di contesto.
//!
//! Non sono un classificatore neurale: sono vincoli espliciti e verificabili
//! per gli omonimi italiani più frequenti nei materiali CAA. Il motore le usa
//! come prior e lascia a Horizon il compito sui casi non coperti.
//!
//! Ogni regola scatta solo su indizi lessicali specifici. In assenza di indizi
//! restituisce `None` e il modello decide da solo.

use crate::decision::normalize::{normalize, tokens};
use crate::decision::types::{Candidate, DecisionInput, DecisionTask};

struct Homonym {
    /// Sottostringhe che identificano il candidato (id o etichetta).
    candidate_markers: &'static [&'static str],
    /// Indizi nel testo che sostengono questo significato.
    cues: &'static [&'static str],
}

struct Rule {
    /// Parola indicata a cui si applica la regola.
    word: &'static str,
    senses: &'static [Homonym],
}

const RULES: &[Rule] = &[
    Rule {
        word: "pesca",
        senses: &[
            Homonym {
                candidate_markers: &["frutto", "alimento", "pesca_frutto"],
                cues: &[
                    "mangiato", "mangio", "mangiare", "frutto", "matura", "albero", "dolce",
                ],
            },
            Homonym {
                candidate_markers: &["attivit", "pescare", "pescatore", "pesca_attivita"],
                cues: &[
                    "vado",
                    "andato",
                    "andare",
                    "pescare",
                    "pescatore",
                    "mare",
                    "rete",
                    "canna",
                ],
            },
        ],
    },
    Rule {
        word: "amo",
        senses: &[
            Homonym {
                candidate_markers: &["amore", "sentimento", "amare"],
                cues: &["amore", "ti", "pizza", "persona", "famiglia"],
            },
            Homonym {
                candidate_markers: &["amo da pesca", "uncino", "oggetto per pescare", "amo_pesca"],
                cues: &[
                    "comprato", "comprare", "esca", "pesce", "pescare", "lenza", "uncino",
                ],
            },
        ],
    },
    Rule {
        word: "riso",
        senses: &[
            Homonym {
                candidate_markers: &["alimento", "risotto", "cibo"],
                cues: &[
                    "mangio", "mangiato", "piatto", "risotto", "cucina", "alimento",
                ],
            },
            Homonym {
                candidate_markers: &["risata", "ridere", "contagioso"],
                cues: &[
                    "ride",
                    "ridere",
                    "risata",
                    "contagioso",
                    "scherzo",
                    "divertente",
                ],
            },
        ],
    },
    Rule {
        word: "porta",
        senses: &[
            Homonym {
                candidate_markers: &["porta di", "di una stanza", "porta_casa", "uscio"],
                cues: &[
                    "chiudi", "chiudere", "apri", "aprire", "entrare", "stanza", "casa",
                ],
            },
            Homonym {
                candidate_markers: &["verbo", "portare", "porta ("],
                cues: &[
                    "bambina", "bambino", "libro", "zaino", "porta un", "porta il", "porta la",
                ],
            },
        ],
    },
    Rule {
        word: "piano",
        senses: &[
            Homonym {
                candidate_markers: &["pianoforte", "strumento"],
                cues: &["suono", "suonare", "strumento", "musica", "tasti"],
            },
            Homonym {
                candidate_markers: &["lentamente", "avverbio", "piano ("],
                cues: &[
                    "cammina",
                    "camminare",
                    "lentamente",
                    "piano piano",
                    "adagio",
                ],
            },
        ],
    },
    Rule {
        word: "penna",
        senses: &[
            Homonym {
                candidate_markers: &["per scrivere", "scrivere", "penna_scrivere", "inchiostro"],
                cues: &[
                    "scrivo",
                    "scrivere",
                    "compito",
                    "quaderno",
                    "inchiostro",
                    "matita",
                ],
            },
            Homonym {
                candidate_markers: &["piuma", "uccello", "penna_piuma"],
                cues: &["uccello", "piuma", "vola", "ali", "nido"],
            },
        ],
    },
];

/// Punteggio deterministico per un candidato, se la regola riconosce sia la
/// parola sia il candidato. `None` = nessun vincolo.
pub fn homonym_score(
    task: DecisionTask,
    input: &DecisionInput,
    candidate: &Candidate,
) -> Option<f32> {
    if !matches!(task, DecisionTask::PictogramDisambiguation) {
        return None;
    }
    let target = input
        .target
        .as_ref()
        .map(|span| normalize(&span.text))
        .filter(|text| !text.is_empty())?;
    let rule = RULES.iter().find(|rule| rule.word == target)?;
    let words = tokens(&input.text);
    let normalized = normalize(&input.text);
    let marker = format!(
        "{} {}",
        candidate.id.to_lowercase(),
        candidate.label.to_lowercase()
    );

    let cue_hits: Vec<bool> = rule
        .senses
        .iter()
        .map(|sense| {
            sense.cues.iter().any(|cue| {
                let cue = cue.to_lowercase();
                if cue.contains(' ') {
                    normalized.contains(&cue)
                } else {
                    words
                        .iter()
                        .any(|word| word == &cue || word.rsplit('\'').next() == Some(cue.as_str()))
                }
            })
        })
        .collect();
    if !cue_hits.iter().any(|hit| *hit) {
        return None;
    }

    for (index, sense) in rule.senses.iter().enumerate() {
        let belongs = sense
            .candidate_markers
            .iter()
            .any(|mark| marker.contains(&mark.to_lowercase()));
        if belongs {
            // Un indizio esplicito a favore di un senso esclude l'altro.
            return Some(if cue_hits[index] { 1.0 } else { 0.0 });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision::types::TargetSpan;
    use serde_json::json;

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

    fn input(text: &str, word: &str) -> DecisionInput {
        let byte = text.find(word).unwrap();
        let start = text[..byte].chars().count();
        DecisionInput {
            text: text.to_string(),
            target: Some(TargetSpan {
                text: word.to_string(),
                start,
                end: start + word.chars().count(),
            }),
        }
    }

    #[test]
    fn pesca_as_food() {
        let input = input("Ho mangiato una pesca.", "pesca");
        let fruit = homonym_score(
            DecisionTask::PictogramDisambiguation,
            &input,
            &candidate("frutto", "frutto (alimento)"),
        );
        let activity = homonym_score(
            DecisionTask::PictogramDisambiguation,
            &input,
            &candidate("attivita", "attività di pesca"),
        );
        assert_eq!(fruit, Some(1.0));
        assert_eq!(activity, Some(0.0));
    }

    #[test]
    fn pesca_as_activity() {
        let input = input("Vado a pesca.", "pesca");
        let activity = homonym_score(
            DecisionTask::PictogramDisambiguation,
            &input,
            &candidate("attivita", "attività di pesca"),
        );
        assert_eq!(activity, Some(1.0));
    }

    #[test]
    fn penna_feather_cues() {
        let input = input("L'uccello ha perso una penna.", "penna");
        let feather = homonym_score(
            DecisionTask::PictogramDisambiguation,
            &input,
            &candidate("piuma", "penna (piuma di uccello)"),
        );
        eprintln!("feather rule: {feather:?}");
        assert_eq!(feather, Some(1.0));
    }

    #[test]
    fn amo_hook_cues_exclude_love() {
        let input = input("Ho comprato un amo.", "amo");
        let hook = homonym_score(
            DecisionTask::PictogramDisambiguation,
            &input,
            &candidate("amo", "amo da pesca (oggetto per pescare)"),
        );
        let love = homonym_score(
            DecisionTask::PictogramDisambiguation,
            &input,
            &candidate("amore", "amore (sentimento)"),
        );
        assert_eq!(hook, Some(1.0));
        assert_eq!(love, Some(0.0));
    }

    #[test]
    fn unknown_word_has_no_rule() {
        let input = input("Mangio la mela.", "mela");
        assert_eq!(
            homonym_score(
                DecisionTask::PictogramDisambiguation,
                &input,
                &candidate("frutto", "frutto")
            ),
            None
        );
    }

    #[test]
    fn other_tasks_have_no_homonym_rules() {
        let input = input("Ho mangiato una pesca.", "pesca");
        assert_eq!(
            homonym_score(
                DecisionTask::CommunicationIntent,
                &input,
                &candidate("frutto", "frutto")
            ),
            None
        );
    }
}
