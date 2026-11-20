//! Orchestrazione del motore decisionale.
//!
//! Un solo `Core` dietro un unico worker: tutte le richieste (CLI, MCP, GUI)
//! passano da qui. Le richieste obsolete vengono annullate tramite
//! `cancel_key`; l'inferenza non blocca il chiamante oltre il timeout.

use std::collections::{BinaryHeap, HashMap};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use serde_json::json;

use crate::decision::audit::{AuditEntry, DecisionAudit};
use crate::decision::cache::{cache_key, DecisionCache};
use crate::decision::hypothesis::{build_pair, HYPOTHESIS_VERSION};
use crate::decision::model;
use crate::decision::normalize::{normalize, tokens};
use crate::decision::policy::ConfidencePolicy;
use crate::decision::rules;
use crate::decision::scores::{rank, ScoredCandidate};
use crate::decision::types::{
    Candidate, DecisionAction, DecisionMetadata, DecisionMode, DecisionOutcome, DecisionRequest,
    DecisionResponse, DecisionTask, Priority, RankedCandidate, RequestStatus,
};

pub const DEFAULT_TIMEOUT_MS: u64 = 8_000;
pub const DEFAULT_TAXONOMY_VERSION: &str = "it-core-1";

/// Interfaccia verso un valutatore di ipotesi.
pub trait HorizonScorer: Send + Sync {
    /// Probabilità di entailment per ogni coppia (premessa, ipotesi).
    fn score(&self, pairs: &[(String, String)]) -> Result<Vec<f32>, String>;
    /// `neural` oppure `rules`.
    fn method(&self) -> &'static str;
    fn model_id(&self) -> &str;
    fn model_revision(&self) -> &str;
    fn is_neural(&self) -> bool;
}

/// Valutatore lessicale deterministico.
///
/// Non è un classificatore neurale e non viene mai presentato come tale.
/// Serve a mantenere un comportamento esplicito quando il modello non è
/// installato: i risultati sono confronti di parole, non semantica.
pub struct RuleScorer;

const STOPWORDS: &[&str] = &[
    "il", "lo", "la", "i", "gli", "le", "un", "uno", "una", "di", "a", "da", "in", "con", "su",
    "per", "tra", "fra", "e", "o", "che", "del", "dello", "della", "dei", "degli", "delle", "al",
    "allo", "alla", "ai", "agli", "alle", "dal", "questo", "questa", "questi", "queste", "è",
    "sono", "essere", "ha", "hanno", "come", "nel", "nella", "nello", "nelle", "negli",
];

impl HorizonScorer for RuleScorer {
    fn score(&self, pairs: &[(String, String)]) -> Result<Vec<f32>, String> {
        Ok(pairs
            .iter()
            .map(|(premise, hypothesis)| {
                let premise_tokens = tokens(premise);
                let hypothesis_tokens = tokens(hypothesis);
                let content: Vec<&String> = hypothesis_tokens
                    .iter()
                    .filter(|token| !STOPWORDS.contains(&token.as_str()))
                    .collect();
                if content.is_empty() {
                    return 0.1;
                }
                let hits = content
                    .iter()
                    .filter(|token| {
                        premise_tokens
                            .iter()
                            .any(|word| word.as_str() == token.as_str())
                    })
                    .count();
                let ratio = hits as f32 / content.len() as f32;
                0.25 + 0.55 * ratio
            })
            .collect())
    }

    fn method(&self) -> &'static str {
        "rules"
    }

    fn model_id(&self) -> &str {
        "none"
    }

    fn model_revision(&self) -> &str {
        "none"
    }

    fn is_neural(&self) -> bool {
        false
    }
}

struct Core {
    scorer: Arc<dyn HorizonScorer>,
    policy: ConfidencePolicy,
    cache: DecisionCache,
    audit: DecisionAudit,
}

impl Core {
    fn evaluate(&mut self, request: &DecisionRequest) -> DecisionResponse {
        let started = Instant::now();
        let request_id = request.request_id();
        let taxonomy_version = request
            .options
            .taxonomy_version
            .clone()
            .unwrap_or_else(|| DEFAULT_TAXONOMY_VERSION.to_string());
        let multi_label = request
            .options
            .multi_label
            .unwrap_or_else(|| request.task.default_multilabel());
        let mode = request.options.mode.unwrap_or(DecisionMode::Suggest);

        let metadata = DecisionMetadata {
            model: self.scorer.model_id().to_string(),
            model_revision: self.scorer.model_revision().to_string(),
            method: self.scorer.method().to_string(),
            cached: false,
            latency_ms: 0,
            hypothesis_version: HYPOTHESIS_VERSION.to_string(),
            taxonomy_version: taxonomy_version.clone(),
            truncated: false,
            candidates_considered: request.candidates.len(),
        };

        if let Err(error) = request.validate() {
            return self.finish(
                DecisionResponse::failure(request_id, RequestStatus::Invalid, error, metadata),
                request,
                started,
            );
        }

        let key = cache_key(
            request,
            self.scorer.model_id(),
            self.scorer.model_revision(),
            &taxonomy_version,
        );
        if let Some(mut cached) = self.cache.get(&key) {
            cached.request_id = request_id;
            cached.metadata.latency_ms = started.elapsed().as_millis() as u64;
            self.audit.record(AuditEntry {
                request_id: cached.request_id.clone(),
                task: request.task.as_str().to_string(),
                status: cached.status.as_str().to_string(),
                method: cached.metadata.method.clone(),
                latency_ms: cached.metadata.latency_ms,
                candidates: cached.metadata.candidates_considered,
                cached: true,
            });
            return cached;
        }

        let allowed: Vec<&Candidate> = request
            .candidates
            .iter()
            .filter(|candidate| !candidate.blocked)
            .collect();
        if allowed.is_empty() {
            return self.finish(
                DecisionResponse::failure(
                    request_id,
                    RequestStatus::Invalid,
                    "tutti i candidati sono bloccati",
                    metadata,
                ),
                request,
                started,
            );
        }

        let locked: Vec<&Candidate> = allowed
            .iter()
            .copied()
            .filter(|candidate| candidate.locked)
            .collect();
        if locked.len() > 1 {
            return self.finish(
                DecisionResponse::failure(
                    request_id,
                    RequestStatus::Invalid,
                    "più candidati bloccati: la scelta manuale è ambigua",
                    metadata,
                ),
                request,
                started,
            );
        }

        // Scelta manuale: prevale su qualsiasi punteggio neurale.
        if let Some(locked_candidate) = locked.first() {
            let mut ranked: Vec<ScoredCandidate> = allowed
                .iter()
                .map(|candidate| ScoredCandidate {
                    id: candidate.id.clone(),
                    label: candidate.label.clone(),
                    score: if candidate.id == locked_candidate.id {
                        1.0
                    } else {
                        0.0
                    },
                    kind: if candidate.id == locked_candidate.id {
                        "locked"
                    } else {
                        "rule"
                    },
                    locked: candidate.id == locked_candidate.id,
                })
                .collect();
            ranked.sort_by(|a, b| {
                b.score
                    .partial_cmp(&a.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            let ranked = rank(ranked, true);
            let response = DecisionResponse {
                request_id: request_id.clone(),
                status: RequestStatus::Completed,
                selected_candidate_id: Some(locked_candidate.id.clone()),
                ranked_candidates: ranked,
                decision: DecisionOutcome {
                    action: DecisionAction::Apply,
                    requires_confirmation: false,
                    reason: "scelta manuale o simbolo bloccato".to_string(),
                },
                metadata: DecisionMetadata {
                    method: "rules".to_string(),
                    ..metadata
                },
                message: None,
            };
            return self.finish(response, request, started);
        }

        let pairs: Vec<(String, String)> = allowed
            .iter()
            .map(|candidate| build_pair(request.task, &request.input, candidate))
            .collect();

        let (mut scores, mut method_used) = match self.scorer.score(&pairs) {
            Ok(scores) if scores.len() == allowed.len() => {
                (scores, self.scorer.method().to_string())
            }
            Ok(_) => {
                let scores = RuleScorer.score(&pairs).unwrap_or_default();
                (scores, "rules_fallback".to_string())
            }
            Err(_) => {
                let scores = RuleScorer.score(&pairs).unwrap_or_default();
                (scores, "rules_fallback".to_string())
            }
        };

        // Vincoli deterministici: per gli omonimi noti le regole entrano nel
        // punteggio. Non sostituiscono il modello sugli altri casi.
        let rule_scores: Vec<Option<f32>> = allowed
            .iter()
            .map(|candidate| rules::homonym_score(request.task, &request.input, candidate))
            .collect();
        let any_rule = rule_scores.iter().any(|score| score.is_some());
        const RULE_WEIGHT: f32 = 0.7;
        if any_rule && scores.len() == allowed.len() {
            scores = scores
                .iter()
                .zip(rule_scores.iter())
                .map(|(neural, rule)| match rule {
                    Some(rule) => RULE_WEIGHT * rule + (1.0 - RULE_WEIGHT) * neural,
                    None => *neural,
                })
                .collect();
            if method_used == "neural" {
                method_used = "neural+rules".to_string();
            }
        }
        let hybrid = method_used == "neural+rules";
        let neural_ok = method_used == "neural" || hybrid;

        let truncated =
            request.input.text.chars().count() > crate::decision::hypothesis::PREMISE_MAX_CHARS;
        let scored: Vec<ScoredCandidate> = allowed
            .iter()
            .zip(scores)
            .map(|(candidate, score)| ScoredCandidate {
                id: candidate.id.clone(),
                label: candidate.label.clone(),
                score,
                kind: if hybrid {
                    "hybrid"
                } else if method_used == "neural" {
                    "entailment"
                } else {
                    "rule"
                },
                locked: false,
            })
            .collect();
        let ranked = rank(scored, multi_label || hybrid);
        let policy_result = self
            .policy
            .evaluate(request.task, mode, multi_label, &ranked);

        let message = if method_used == "neural" {
            None
        } else if hybrid {
            Some(
                "punteggio combinato: modello Horizon + regole lessicali deterministiche"
                    .to_string(),
            )
        } else {
            Some(
                "modello neurale non disponibile: risultato lessicale deterministico, non semantico"
                    .to_string(),
            )
        };
        let response = DecisionResponse {
            request_id: request_id.clone(),
            status: if neural_ok {
                RequestStatus::Completed
            } else {
                RequestStatus::Unavailable
            },
            selected_candidate_id: policy_result.selected_candidate_id,
            ranked_candidates: ranked,
            decision: DecisionOutcome {
                action: policy_result.action,
                requires_confirmation: policy_result.requires_confirmation,
                reason: policy_result.reason,
            },
            metadata: DecisionMetadata {
                method: method_used,
                truncated,
                ..metadata
            },
            message,
        };
        self.finish(response, request, started)
    }

    fn finish(
        &mut self,
        mut response: DecisionResponse,
        request: &DecisionRequest,
        started: Instant,
    ) -> DecisionResponse {
        response.metadata.latency_ms = started.elapsed().as_millis() as u64;
        if response.status == RequestStatus::Completed {
            let taxonomy = request
                .options
                .taxonomy_version
                .clone()
                .unwrap_or_else(|| DEFAULT_TAXONOMY_VERSION.to_string());
            let key = cache_key(
                request,
                self.scorer.model_id(),
                self.scorer.model_revision(),
                &taxonomy,
            );
            self.cache.put(key, response.clone());
        }
        self.audit.record(AuditEntry {
            request_id: response.request_id.clone(),
            task: request.task.as_str().to_string(),
            status: response.status.as_str().to_string(),
            method: response.metadata.method.clone(),
            latency_ms: response.metadata.latency_ms,
            candidates: response.metadata.candidates_considered,
            cached: false,
        });
        response
    }
}

// --------------------------------------------------------------- scheduler

struct Job {
    priority: u8,
    sequence: u64,
    request: DecisionRequest,
    cancel: Arc<AtomicBool>,
    reply: Sender<DecisionResponse>,
}

impl PartialEq for Job {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority && self.sequence == other.sequence
    }
}
impl Eq for Job {}
impl PartialOrd for Job {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Job {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.priority
            .cmp(&other.priority)
            .then_with(|| other.sequence.cmp(&self.sequence))
    }
}

struct Scheduler {
    tx: Option<Sender<Job>>,
    worker: Option<JoinHandle<()>>,
    cancel: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
}

impl Scheduler {
    fn new(core: Arc<Mutex<Core>>) -> Self {
        let (tx, rx) = mpsc::channel::<Job>();
        let cancel: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let worker_cancel = Arc::clone(&cancel);
        let worker = std::thread::Builder::new()
            .name("decision-worker".to_string())
            .spawn(move || {
                let receiver: Receiver<Job> = rx;
                while let Ok(first) = receiver.recv() {
                    let mut heap = BinaryHeap::new();
                    heap.push(first);
                    while let Ok(job) = receiver.try_recv() {
                        heap.push(job);
                    }
                    while let Some(job) = heap.pop() {
                        if job.cancel.load(Ordering::SeqCst) {
                            let _ = job.reply.send(DecisionResponse::failure(
                                job.request.request_id(),
                                RequestStatus::Cancelled,
                                "richiesta annullata: documento o contesto cambiato",
                                DecisionMetadata::default(),
                            ));
                            continue;
                        }
                        let response = match core.lock() {
                            Ok(mut core) => core.evaluate(&job.request),
                            Err(_) => DecisionResponse::failure(
                                job.request.request_id(),
                                RequestStatus::Failed,
                                "motore decisionale non disponibile",
                                DecisionMetadata::default(),
                            ),
                        };
                        if job.cancel.load(Ordering::SeqCst) {
                            let _ = job.reply.send(DecisionResponse::failure(
                                job.request.request_id(),
                                RequestStatus::Cancelled,
                                "risultato scartato: richiesta obsoleta",
                                response.metadata,
                            ));
                        } else {
                            let _ = job.reply.send(response);
                        }
                    }
                }
                let _ = worker_cancel;
            })
            .expect("avvio worker decisionale");
        Self {
            tx: Some(tx),
            worker: Some(worker),
            cancel,
        }
    }

    fn submit(
        &self,
        request: DecisionRequest,
        priority: Priority,
        timeout: Duration,
    ) -> DecisionResponse {
        let request_id = request.request_id();
        let flag = Arc::new(AtomicBool::new(false));
        if let Some(key) = request.cancel_key() {
            if let Ok(mut map) = self.cancel.lock() {
                if let Some(previous) = map.insert(key, Arc::clone(&flag)) {
                    previous.store(true, Ordering::SeqCst);
                }
            }
        }
        let (reply, receiver) = mpsc::channel();
        let job = Job {
            priority: priority.weight(),
            sequence: next_sequence(),
            request,
            cancel: Arc::clone(&flag),
            reply,
        };
        let Some(tx) = &self.tx else {
            return DecisionResponse::failure(
                request_id,
                RequestStatus::Failed,
                "motore decisionale chiuso",
                DecisionMetadata::default(),
            );
        };
        if tx.send(job).is_err() {
            return DecisionResponse::failure(
                request_id,
                RequestStatus::Failed,
                "worker decisionale non disponibile",
                DecisionMetadata::default(),
            );
        }
        match receiver.recv_timeout(timeout) {
            Ok(response) => response,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                flag.store(true, Ordering::SeqCst);
                DecisionResponse::failure(
                    request_id,
                    RequestStatus::TimedOut,
                    format!("inferenza oltre {} ms", timeout.as_millis()),
                    DecisionMetadata::default(),
                )
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => DecisionResponse::failure(
                request_id,
                RequestStatus::Failed,
                "worker decisionale terminato",
                DecisionMetadata::default(),
            ),
        }
    }
}

impl Drop for Scheduler {
    fn drop(&mut self) {
        self.tx.take();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn next_sequence() -> u64 {
    use std::sync::atomic::AtomicU64;
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

// ------------------------------------------------------------------ facade

pub struct DecisionEngine {
    scheduler: Scheduler,
    core: Arc<Mutex<Core>>,
}

impl DecisionEngine {
    /// Costruisce il motore con il valutatore richiesto (usato dai test).
    pub fn with_scorer(scorer: Arc<dyn HorizonScorer>) -> Self {
        let core = Arc::new(Mutex::new(Core {
            scorer,
            policy: ConfidencePolicy::default(),
            cache: DecisionCache::default(),
            audit: DecisionAudit::default(),
        }));
        let scheduler = Scheduler::new(Arc::clone(&core));
        Self { scheduler, core }
    }

    /// Motore di produzione: usa Horizon se installato, altrimenti il
    /// valutatore lessicale deterministico.
    pub fn from_paths(paths: &crate::paths::AppPaths) -> Self {
        #[cfg(feature = "onnx")]
        {
            if let Some(scorer) = crate::decision::onnx::OnnxHorizon::from_installed(paths) {
                return Self::with_scorer(Arc::new(scorer));
            }
        }
        let _ = paths;
        Self::with_scorer(Arc::new(RuleScorer))
    }

    pub fn classify(&self, request: DecisionRequest) -> DecisionResponse {
        let timeout =
            Duration::from_millis(request.options.timeout_ms.unwrap_or(DEFAULT_TIMEOUT_MS));
        let priority = request.options.priority.unwrap_or(Priority::High);
        self.scheduler.submit(request, priority, timeout)
    }

    pub fn status(&self, model_status: serde_json::Value) -> serde_json::Value {
        let (hits, misses) = self
            .core
            .lock()
            .map(|core| core.cache.stats())
            .unwrap_or((0, 0));
        let audit_len = self.core.lock().map(|core| core.audit.len()).unwrap_or(0);
        let (method, is_neural) = self
            .core
            .lock()
            .map(|core| (core.scorer.method(), core.scorer.is_neural()))
            .unwrap_or(("rules", false));
        json!({
            "engine": "DecisionEngine",
            "method": method,
            "neural": is_neural,
            "hypothesis_version": HYPOTHESIS_VERSION,
            "taxonomy_version": DEFAULT_TAXONOMY_VERSION,
            "cache": {"hits": hits, "misses": misses},
            "audit_entries": audit_len,
            "model": model_status,
        })
    }

    pub fn audit_snapshot(&self) -> Vec<serde_json::Value> {
        self.core
            .lock()
            .map(|core| {
                core.audit
                    .snapshot()
                    .into_iter()
                    .map(|entry| {
                        json!({
                            "request_id": entry.request_id,
                            "task": entry.task,
                            "status": entry.status,
                            "method": entry.method,
                            "latency_ms": entry.latency_ms,
                            "candidates": entry.candidates,
                            "cached": entry.cached,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn clear_cache(&self) {
        if let Ok(mut core) = self.core.lock() {
            core.cache.clear();
        }
    }
}

/// Motore condiviso per cartella dati.
///
/// Evita di ricaricare i pesi a ogni chiamata CLI/MCP/GUI: una sola sessione
/// ONNX per processo. Dopo installazione o rimozione del modello la cache va
/// invalidata (le funzioni di installazione lo fanno).
pub fn cached_engine(paths: &crate::paths::AppPaths) -> Arc<DecisionEngine> {
    let map = ENGINE_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let key = paths.data_dir.display().to_string();
    if let Ok(mut map) = map.lock() {
        if let Some(engine) = map.get(&key) {
            return Arc::clone(engine);
        }
        let engine = Arc::new(DecisionEngine::from_paths(paths));
        map.insert(key, Arc::clone(&engine));
        return engine;
    }
    Arc::new(DecisionEngine::from_paths(paths))
}

/// Da chiamare dopo installazione o rimozione dei pesi.
pub fn invalidate_cached_engines() {
    if let Some(map) = ENGINE_CACHE.get() {
        if let Ok(mut map) = map.lock() {
            map.clear();
        }
    }
}

static ENGINE_CACHE: OnceLock<Mutex<HashMap<String, Arc<DecisionEngine>>>> = OnceLock::new();

/// Stato del modello Horizon e del motore, senza inizializzare la sessione.
pub fn status_json(paths: &crate::paths::AppPaths) -> serde_json::Value {
    let status = model::status(paths);
    json!({
        "model": status,
        "runtime_feature": cfg!(feature = "onnx"),
    })
}

/// Normalizza un testo per i confronti esterni (stessa funzione dell'engine).
pub fn normalized_text(text: &str) -> String {
    normalize(text)
}

/// Controlla che un insieme di candidati contenga la parola indicata.
pub fn target_matches(task: DecisionTask, text: &str, target: &str) -> bool {
    if !matches!(task, DecisionTask::PictogramDisambiguation) {
        return true;
    }
    let target_tokens = tokens(target);
    let text_tokens = tokens(text);
    target_tokens
        .iter()
        .all(|token| text_tokens.iter().any(|word| word == token))
}

/// Costruisce un rank di comodo per un elenco di etichette, utile ai chiamanti
/// che non hanno ancora un motore (es. anteprima della tassonomia).
pub fn labels_to_candidates(labels: &[&str]) -> Vec<Candidate> {
    labels
        .iter()
        .map(|label| Candidate {
            id: label.to_lowercase().replace(' ', "_"),
            label: (*label).to_string(),
            hypothesis: None,
            locked: false,
            blocked: false,
            metadata: json!({}),
        })
        .collect()
}

/// Margine fra i primi due risultati, per diagnostica.
pub fn top_margin(ranked: &[RankedCandidate]) -> f32 {
    crate::decision::scores::margin(ranked)
}

/// Serve solo a evitare import inutilizzati nelle build senza ONNX.
#[allow(dead_code)]
fn _assert_send_sync<T: Send + Sync>() {}

#[allow(dead_code)]
fn _assert_traits() {
    _assert_send_sync::<DecisionEngine>();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision::types::{Candidate, DecisionInput, DecisionOptions, DecisionTask};
    use serde_json::json;

    struct SlowScorer {
        delay: Duration,
    }

    impl HorizonScorer for SlowScorer {
        fn score(&self, pairs: &[(String, String)]) -> Result<Vec<f32>, String> {
            std::thread::sleep(self.delay);
            Ok(pairs.iter().map(|_| 0.9).collect())
        }

        fn method(&self) -> &'static str {
            "neural"
        }

        fn model_id(&self) -> &str {
            "slow-test"
        }

        fn model_revision(&self) -> &str {
            "test"
        }

        fn is_neural(&self) -> bool {
            true
        }
    }

    fn request(cancel_key: &str) -> DecisionRequest {
        DecisionRequest {
            request_id: Some(format!("req-{cancel_key}")),
            task: DecisionTask::CommunicationIntent,
            language: "it".to_string(),
            input: DecisionInput {
                text: "Vorrei bere dell'acqua.".to_string(),
                target: None,
            },
            candidates: vec![
                Candidate {
                    id: "richiesta".to_string(),
                    label: "una richiesta".to_string(),
                    hypothesis: None,
                    locked: false,
                    blocked: false,
                    metadata: json!({}),
                },
                Candidate {
                    id: "saluto".to_string(),
                    label: "un saluto".to_string(),
                    hypothesis: None,
                    locked: false,
                    blocked: false,
                    metadata: json!({}),
                },
            ],
            context: crate::decision::types::DecisionContext {
                cancel_key: Some(cancel_key.to_string()),
                ..Default::default()
            },
            options: DecisionOptions::default(),
        }
    }

    #[test]
    fn stale_result_is_cancelled() {
        let engine = std::sync::Arc::new(DecisionEngine::with_scorer(std::sync::Arc::new(
            SlowScorer {
                delay: Duration::from_millis(200),
            },
        )));
        let first = {
            let engine = std::sync::Arc::clone(&engine);
            std::thread::spawn(move || engine.classify(request("elemento-1")))
        };
        std::thread::sleep(Duration::from_millis(30));
        // La seconda richiesta con la stessa chiave invalida la prima.
        let second = engine.classify(request("elemento-1"));
        let first = first.join().expect("prima richiesta");
        assert_eq!(second.status, RequestStatus::Completed);
        assert_eq!(first.status, RequestStatus::Cancelled);
    }

    #[test]
    fn a_blocked_candidate_is_never_selected() {
        let engine = DecisionEngine::with_scorer(std::sync::Arc::new(RuleScorer));
        let mut req = request("elemento-2");
        req.candidates[0].locked = true;
        req.candidates[0].blocked = true;
        let response = engine.classify(req);
        assert_ne!(
            response.selected_candidate_id.as_deref(),
            Some("richiesta"),
            "un candidato bloccato non deve essere scelto"
        );
    }
}
