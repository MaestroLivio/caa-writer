//! Motore decisionale semantico condiviso.
//!
//! Un unico classificatore neurale — Horizon multilingual-zeroshot-small —
//! dietro un unico orchestrabile. Il motore non modifica documenti, non
//! esegue comandi e non prende decisioni pedagogiche vincolanti.

pub mod audit;
pub mod cache;
pub mod engine;
pub mod hypothesis;
pub mod model;
pub mod normalize;
pub mod policy;
pub mod registry;
pub mod rules;
pub mod scores;
pub mod types;

#[cfg(feature = "onnx")]
pub mod onnx;

pub use engine::{DecisionEngine, HorizonScorer, RuleScorer};
pub use types::{
    Candidate, DecisionAction, DecisionContext, DecisionInput, DecisionMetadata, DecisionMode,
    DecisionOptions, DecisionOutcome, DecisionRequest, DecisionResponse, DecisionTask, Priority,
    RankedCandidate, RequestStatus, TargetSpan,
};
