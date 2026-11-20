//! Contratti tipizzati del motore decisionale semantico.
//!
//! Regola di progetto: il modello neurale non modifica mai il documento e non
//! esegue comandi. Restituisce soltanto classificazioni strutturate che i
//! servizi dell'applicazione validano e applicano secondo i propri permessi.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Tipi di decisione supportati. L'elenco è stabile e versionato: aggiungere
/// una variante richiede un template di ipotesi e test dedicati.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionTask {
    PictogramDisambiguation,
    CommunicationIntent,
    SemanticCategory,
    BoardCategory,
    SemanticSearchRanking,
    DidacticTemplateSelection,
    LearningObjective,
    ExerciseType,
    SocialStoryTopic,
    ExpressedEmotion,
    InteractiveAction,
    LayoutTemplate,
    CaaSemanticCoherence,
    ContentCataloguing,
    CommandRouting,
}

impl DecisionTask {
    pub const ALL: [DecisionTask; 15] = [
        DecisionTask::PictogramDisambiguation,
        DecisionTask::CommunicationIntent,
        DecisionTask::SemanticCategory,
        DecisionTask::BoardCategory,
        DecisionTask::SemanticSearchRanking,
        DecisionTask::DidacticTemplateSelection,
        DecisionTask::LearningObjective,
        DecisionTask::ExerciseType,
        DecisionTask::SocialStoryTopic,
        DecisionTask::ExpressedEmotion,
        DecisionTask::InteractiveAction,
        DecisionTask::LayoutTemplate,
        DecisionTask::CaaSemanticCoherence,
        DecisionTask::ContentCataloguing,
        DecisionTask::CommandRouting,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            DecisionTask::PictogramDisambiguation => "pictogram_disambiguation",
            DecisionTask::CommunicationIntent => "communication_intent",
            DecisionTask::SemanticCategory => "semantic_category",
            DecisionTask::BoardCategory => "board_category",
            DecisionTask::SemanticSearchRanking => "semantic_search_ranking",
            DecisionTask::DidacticTemplateSelection => "didactic_template_selection",
            DecisionTask::LearningObjective => "learning_objective",
            DecisionTask::ExerciseType => "exercise_type",
            DecisionTask::SocialStoryTopic => "social_story_topic",
            DecisionTask::ExpressedEmotion => "expressed_emotion",
            DecisionTask::InteractiveAction => "interactive_action",
            DecisionTask::LayoutTemplate => "layout_template",
            DecisionTask::CaaSemanticCoherence => "caa_semantic_coherence",
            DecisionTask::ContentCataloguing => "content_cataloguing",
            DecisionTask::CommandRouting => "command_routing",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        let normalized = value.trim().to_ascii_lowercase().replace('-', "_");
        Self::ALL
            .into_iter()
            .find(|task| task.as_str() == normalized)
    }

    /// I task multilabel non normalizzano i punteggi fra loro.
    pub fn default_multilabel(self) -> bool {
        matches!(
            self,
            DecisionTask::CommunicationIntent
                | DecisionTask::SemanticCategory
                | DecisionTask::BoardCategory
                | DecisionTask::LearningObjective
                | DecisionTask::CaaSemanticCoherence
                | DecisionTask::ContentCataloguing
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestStatus {
    Queued,
    Running,
    Completed,
    Cancelled,
    TimedOut,
    Invalid,
    Unavailable,
    Failed,
}

impl RequestStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            RequestStatus::Queued => "queued",
            RequestStatus::Running => "running",
            RequestStatus::Completed => "completed",
            RequestStatus::Cancelled => "cancelled",
            RequestStatus::TimedOut => "timed_out",
            RequestStatus::Invalid => "invalid",
            RequestStatus::Unavailable => "unavailable",
            RequestStatus::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionAction {
    Apply,
    Suggest,
    Abstain,
}

impl DecisionAction {
    pub fn as_str(self) -> &'static str {
        match self {
            DecisionAction::Apply => "apply",
            DecisionAction::Suggest => "suggest",
            DecisionAction::Abstain => "abstain",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionMode {
    /// Mostra solo suggerimenti; nessuna modifica automatica.
    Suggest,
    /// Applica solo le decisioni che superano la soglia di applicazione.
    Automatic,
}

impl DecisionMode {
    pub fn as_str(self) -> &'static str {
        match self {
            DecisionMode::Suggest => "suggest",
            DecisionMode::Automatic => "automatic",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    High,
    Medium,
    Low,
}

impl Priority {
    pub fn weight(self) -> u8 {
        match self {
            Priority::High => 2,
            Priority::Medium => 1,
            Priority::Low => 0,
        }
    }
}

/// Intervallo di caratteri (non byte) sul testo originale.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TargetSpan {
    pub text: String,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DecisionInput {
    pub text: String,
    #[serde(default)]
    pub target: Option<TargetSpan>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Candidate {
    pub id: String,
    pub label: String,
    /// Ipotesi già formulata dal chiamante. Se assente la costruisce
    /// `HypothesisBuilder` con il template del task.
    #[serde(default)]
    pub hypothesis: Option<String>,
    /// Scelta manuale dell'utente o simbolo bloccato: prevale su ogni punteggio.
    #[serde(default)]
    pub locked: bool,
    /// Candidato vietato dalle regole (es. licenza, permessi, filtro).
    #[serde(default)]
    pub blocked: bool,
    #[serde(default)]
    pub metadata: Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DecisionContext {
    #[serde(default)]
    pub document_revision: Option<u64>,
    #[serde(default)]
    pub element_id: Option<String>,
    /// Chiave facoltativa usata per invalidare richieste obsolete.
    #[serde(default)]
    pub cancel_key: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DecisionOptions {
    #[serde(default)]
    pub mode: Option<DecisionMode>,
    #[serde(default)]
    pub multi_label: Option<bool>,
    #[serde(default)]
    pub apply_threshold: Option<f32>,
    #[serde(default)]
    pub suggest_threshold: Option<f32>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    #[serde(default)]
    pub priority: Option<Priority>,
    #[serde(default)]
    pub taxonomy_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionRequest {
    #[serde(default)]
    pub request_id: Option<String>,
    pub task: DecisionTask,
    #[serde(default = "default_language")]
    pub language: String,
    pub input: DecisionInput,
    #[serde(default)]
    pub candidates: Vec<Candidate>,
    #[serde(default)]
    pub context: DecisionContext,
    #[serde(default)]
    pub options: DecisionOptions,
}

fn default_language() -> String {
    "it".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedCandidate {
    pub id: String,
    pub label: String,
    pub score: f32,
    pub rank: usize,
    /// `entailment` (modello), `rule` (deterministico) o `locked`.
    pub score_kind: String,
    #[serde(default)]
    pub locked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionOutcome {
    pub action: DecisionAction,
    pub requires_confirmation: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DecisionMetadata {
    pub model: String,
    pub model_revision: String,
    pub method: String,
    pub cached: bool,
    pub latency_ms: u64,
    pub hypothesis_version: String,
    pub taxonomy_version: String,
    #[serde(default)]
    pub truncated: bool,
    #[serde(default)]
    pub candidates_considered: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionResponse {
    pub request_id: String,
    pub status: RequestStatus,
    #[serde(default)]
    pub selected_candidate_id: Option<String>,
    #[serde(default)]
    pub ranked_candidates: Vec<RankedCandidate>,
    pub decision: DecisionOutcome,
    pub metadata: DecisionMetadata,
    #[serde(default)]
    pub message: Option<String>,
}

impl DecisionResponse {
    pub fn failure(
        request_id: impl Into<String>,
        status: RequestStatus,
        message: impl Into<String>,
        metadata: DecisionMetadata,
    ) -> Self {
        Self {
            request_id: request_id.into(),
            status,
            selected_candidate_id: None,
            ranked_candidates: Vec::new(),
            decision: DecisionOutcome {
                action: DecisionAction::Abstain,
                requires_confirmation: false,
                reason: status.as_str().to_string(),
            },
            metadata,
            message: Some(message.into()),
        }
    }
}

impl DecisionRequest {
    /// Validazione strutturale preliminare, senza toccare il modello.
    pub fn validate(&self) -> Result<(), String> {
        if self.input.text.trim().is_empty() {
            return Err("testo vuoto".to_string());
        }
        if self.input.text.chars().count() > 4000 {
            return Err("testo troppo lungo (massimo 4000 caratteri)".to_string());
        }
        if self.candidates.is_empty() {
            return Err("servono almeno due candidati".to_string());
        }
        if self.candidates.len() > 64 {
            return Err("troppi candidati (massimo 64)".to_string());
        }
        let mut ids = std::collections::HashSet::new();
        for candidate in &self.candidates {
            if candidate.id.trim().is_empty() {
                return Err("candidato senza id".to_string());
            }
            if !ids.insert(candidate.id.as_str()) {
                return Err(format!("id candidato duplicato: {}", candidate.id));
            }
        }
        if let Some(target) = &self.input.target {
            let len = self.input.text.chars().count();
            if target.start > target.end || target.end > len {
                return Err(format!(
                    "intervallo target non valido: {}..{} su {} caratteri",
                    target.start, target.end, len
                ));
            }
        }
        if !matches!(self.language.as_str(), "it" | "en" | "en-GB") {
            return Err(format!("lingua non supportata: {}", self.language));
        }
        Ok(())
    }

    pub fn request_id(&self) -> String {
        self.request_id
            .clone()
            .unwrap_or_else(|| format!("dec-{}", crate::util::now_iso()))
    }

    pub fn cancel_key(&self) -> Option<String> {
        if let Some(key) = &self.context.cancel_key {
            return Some(key.clone());
        }
        match (&self.context.element_id, self.context.document_revision) {
            (Some(element), Some(revision)) => Some(format!("{element}@{revision}")),
            (Some(element), None) => Some(element.clone()),
            _ => None,
        }
    }
}
