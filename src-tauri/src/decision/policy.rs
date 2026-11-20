//! Politica di confidenza.
//!
//! I punteggi zero-shot non sono probabilità calibrate. Per i task a etichetta
//! singola la decisione usa il margine fra primo e secondo candidato, che è
//! confrontabile anche quando il numero di alternative cambia. Le soglie
//! predefinite sono state misurate sul corpus italiano
//! `tests/decision_corpus_500.json` (split di calibrazione); vedi
//! `docs/DECISION_ENGINE.md`.
//!
//! Alcuni task didattici restano sempre "suggerimento": il modello non può
//! vincolare una decisione pedagogica.

use crate::decision::scores::margin;
use crate::decision::types::{DecisionAction, DecisionMode, DecisionTask, RankedCandidate};

#[derive(Debug, Clone, Copy)]
pub struct ConfidencePolicy {
    /// Pavimento assoluto sotto il quale non si suggerisce nulla.
    pub suggest_threshold: f32,
    /// Pavimento assoluto per l'applicazione automatica.
    pub apply_threshold: f32,
    /// Margine minimo top1-top2 per suggerire (etichetta singola).
    pub min_margin: f32,
    /// Margine minimo top1-top2 per applicare automaticamente.
    pub apply_margin: f32,
}

/// Margini misurati per task sul corpus di calibrazione (precisione >= 0,80
/// per suggerire, >= 0,95 per applicare). I task assenti usano i valori
/// globali prudenti. `pictogram_disambiguation` ha un margine di applicazione
/// più alto del misurato per non modificare simboli su margini fragili.
const TASK_MARGINS: &[(&str, f32, f32)] = &[
    ("pictogram_disambiguation", 0.01, 0.20),
    ("communication_intent", 0.40, 0.49),
    ("semantic_category", 0.51, 0.54),
    ("board_category", 0.80, 0.85),
    ("expressed_emotion", 0.06, 0.13),
    ("exercise_type", 0.08, 0.08),
    ("social_story_topic", 0.08, 0.08),
    ("interactive_action", 0.01, 0.06),
    ("layout_template", 0.01, 0.02),
    ("learning_objective", 0.40, 0.50),
];

fn margins_for(task: DecisionTask, fallback: (f32, f32)) -> (f32, f32) {
    TASK_MARGINS
        .iter()
        .find(|(name, _, _)| *name == task.as_str())
        .map(|(_, suggest, apply)| (*suggest, *apply))
        .unwrap_or(fallback)
}

impl Default for ConfidencePolicy {
    fn default() -> Self {
        // Valori misurati sul corpus italiano (split di calibrazione).
        // Il pavimento assoluto resta basso perché la softmax fra molti
        // candidati comprime i punteggi; decide il margine.
        Self {
            suggest_threshold: 0.05,
            apply_threshold: 0.15,
            min_margin: 0.37,
            apply_margin: 0.55,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PolicyResult {
    pub action: DecisionAction,
    pub selected_candidate_id: Option<String>,
    pub requires_confirmation: bool,
    pub reason: String,
}

impl ConfidencePolicy {
    pub fn evaluate(
        &self,
        task: DecisionTask,
        mode: DecisionMode,
        _multilabel: bool,
        ranked: &[RankedCandidate],
    ) -> PolicyResult {
        let Some(top) = ranked.first() else {
            return PolicyResult {
                action: DecisionAction::Abstain,
                selected_candidate_id: None,
                requires_confirmation: false,
                reason: "nessun candidato valutabile".to_string(),
            };
        };

        // Le scelte manuali/bloccate sono già state decise dal RuleEngine.
        if top.locked {
            return PolicyResult {
                action: DecisionAction::Apply,
                selected_candidate_id: Some(top.id.clone()),
                requires_confirmation: false,
                reason: "scelta manuale o simbolo bloccato".to_string(),
            };
        }

        let gap = margin(ranked);
        let (suggest_margin, apply_margin) = margins_for(task, (self.min_margin, self.apply_margin));

        // La risposta seleziona comunque un solo candidato principale: il
        // margine top1-top2 è quindi richiesto anche nei task multilabel.
        if top.score < self.suggest_threshold || gap < suggest_margin {
            return PolicyResult {
                action: DecisionAction::Abstain,
                selected_candidate_id: None,
                requires_confirmation: false,
                reason: format!(
                    "confidenza insufficiente (punteggio {:.2}, margine {:.2})",
                    top.score, gap
                ),
            };
        }

        // Task consultivi: mai applicazione automatica.
        let advisory = matches!(
            task,
            DecisionTask::DidacticTemplateSelection
                | DecisionTask::LearningObjective
                | DecisionTask::ExerciseType
                | DecisionTask::SocialStoryTopic
                | DecisionTask::ExpressedEmotion
                | DecisionTask::ContentCataloguing
                | DecisionTask::CaaSemanticCoherence
                | DecisionTask::CommandRouting
        );

        let confident = top.score >= self.apply_threshold && gap >= apply_margin;

        if mode == DecisionMode::Automatic && confident && !advisory {
            return PolicyResult {
                action: DecisionAction::Apply,
                selected_candidate_id: Some(top.id.clone()),
                requires_confirmation: false,
                reason: format!("punteggio {:.2}, margine {:.2}", top.score, gap),
            };
        }

        PolicyResult {
            action: DecisionAction::Suggest,
            selected_candidate_id: Some(top.id.clone()),
            requires_confirmation: false,
            reason: if advisory {
                "task consultivo: richiede conferma umana".to_string()
            } else if gap < apply_margin {
                format!("margine sotto la soglia di applicazione ({:.2})", gap)
            } else {
                format!("suggerimento con margine {:.2}", gap)
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ranked(score: f32, second: f32) -> Vec<RankedCandidate> {
        vec![
            RankedCandidate {
                id: "a".to_string(),
                label: "a".to_string(),
                score,
                rank: 1,
                score_kind: "entailment".to_string(),
                locked: false,
            },
            RankedCandidate {
                id: "b".to_string(),
                label: "b".to_string(),
                score: second,
                rank: 2,
                score_kind: "entailment".to_string(),
                locked: false,
            },
        ]
    }

    #[test]
    fn abstains_with_narrow_margin() {
        let result = ConfidencePolicy::default().evaluate(
            DecisionTask::CommandRouting,
            DecisionMode::Suggest,
            false,
            &ranked(0.50, 0.48),
        );
        assert_eq!(result.action, DecisionAction::Abstain);
    }

    #[test]
    fn suggests_with_clear_margin() {
        let result = ConfidencePolicy::default().evaluate(
            DecisionTask::PictogramDisambiguation,
            DecisionMode::Suggest,
            false,
            &ranked(0.60, 0.10),
        );
        assert_eq!(result.action, DecisionAction::Suggest);
        assert_eq!(result.selected_candidate_id.as_deref(), Some("a"));
    }

    #[test]
    fn locked_choice_wins() {
        let mut candidates = ranked(0.10, 0.05);
        candidates[0].locked = true;
        let result = ConfidencePolicy::default().evaluate(
            DecisionTask::PictogramDisambiguation,
            DecisionMode::Automatic,
            false,
            &candidates,
        );
        assert_eq!(result.action, DecisionAction::Apply);
    }

    #[test]
    fn didactic_tasks_stay_advisory() {
        let result = ConfidencePolicy::default().evaluate(
            DecisionTask::LearningObjective,
            DecisionMode::Automatic,
            true,
            &ranked(0.99, 0.0),
        );
        assert_eq!(result.action, DecisionAction::Suggest);
    }

    #[test]
    fn policy_matches_calibration_report() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("radice repository")
            .join("resources/decision/calibration.json");
        let text = std::fs::read_to_string(&path).expect("report di calibrazione");
        let value: serde_json::Value = serde_json::from_str(&text).expect("JSON valido");
        let policy = &value["policy"];
        let defaults = ConfidencePolicy::default();
        let read = |key: &str| policy[key].as_f64().expect("numero") as f32;
        assert!((read("suggest_threshold") - defaults.suggest_threshold).abs() < 1e-5);
        assert!((read("apply_threshold") - defaults.apply_threshold).abs() < 1e-5);
        assert!((read("min_margin") - defaults.min_margin).abs() < 1e-5);
        assert!((read("apply_margin") - defaults.apply_margin).abs() < 1e-5);
    }

    #[test]
    fn task_margins_match_report() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("radice repository")
            .join("resources/decision/calibration.json");
        let text = std::fs::read_to_string(&path).expect("report di calibrazione");
        let value: serde_json::Value = serde_json::from_str(&text).expect("JSON valido");
        let per_task = value["policy"]["per_task"].as_object().expect("per_task");
        assert_eq!(per_task.len(), TASK_MARGINS.len());
        for (name, suggest, apply) in TASK_MARGINS {
            let row = &per_task[*name];
            let report_suggest = row["suggest_margin"].as_f64().expect("suggest") as f32;
            let report_apply = row["apply_margin"].as_f64().expect("apply") as f32;
            assert!((report_suggest - suggest).abs() < 1e-5, "{name}");
            assert!((report_apply - apply).abs() < 1e-5, "{name}");
        }
    }

    #[test]
    fn task_margins_are_applied() {
        assert_eq!(margins_for(DecisionTask::CommunicationIntent, (0.37, 0.55)), (0.40, 0.49));
        assert_eq!(margins_for(DecisionTask::CommandRouting, (0.37, 0.55)), (0.37, 0.55));
    }

    #[test]
    fn multilabel_uses_absolute_score() {
        let result = ConfidencePolicy::default().evaluate(
            DecisionTask::SemanticCategory,
            DecisionMode::Suggest,
            true,
            &ranked(0.70, 0.10),
        );
        assert_eq!(result.action, DecisionAction::Suggest);
    }
}
