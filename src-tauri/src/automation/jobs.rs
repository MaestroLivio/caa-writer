//! Job in background.
//!
//! I job sono reali: il processo mantiene lo stato in memoria e il lavoro gira
//! in un thread che apre una propria istanza del motore sugli stessi percorsi.
//! Per questo hanno senso solo in un processo persistente (`caa-cli rpc` o il
//! server MCP): una singola invocazione CLI termina e perde il registro.
//! L'annullamento è cooperativo e viene controllato prima dell'esecuzione:
//! un'operazione già avviata (per esempio una richiesta al modello) non viene
//! interrotta a forza.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;

use serde_json::{json, Value};

use super::policy::{AccessMode, DispatchOptions};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobState {
    Queued,
    Running,
    Done,
    Failed,
    Cancelled,
}

impl JobState {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Done => "done",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Job {
    pub id: String,
    pub method: String,
    pub params: Value,
    pub state: JobState,
    pub created_at: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub elapsed_ms: Option<u128>,
    pub result: Option<Value>,
    pub error: Option<String>,
    pub cancel_requested: bool,
    pub access: String,
}

impl Job {
    pub fn to_json(&self, include_result: bool) -> Value {
        let mut value = json!({
            "id": self.id,
            "method": self.method,
            "state": self.state.as_str(),
            "created_at": self.created_at,
            "started_at": self.started_at,
            "finished_at": self.finished_at,
            "elapsed_ms": self.elapsed_ms,
            "cancel_requested": self.cancel_requested,
            "access": self.access,
        });
        if include_result {
            value["result"] = self.result.clone().unwrap_or(Value::Null);
            value["error"] = self.error.clone().map(Value::String).unwrap_or(Value::Null);
        } else {
            value["has_result"] = json!(self.result.is_some());
            value["error"] = self.error.clone().map(Value::String).unwrap_or(Value::Null);
        }
        value
    }
}

struct Entry {
    job: Job,
    cancel: Arc<AtomicBool>,
}

static STORE: OnceLock<Mutex<HashMap<String, Entry>>> = OnceLock::new();
static COUNTER: AtomicU64 = AtomicU64::new(1);
static NOTIFIER: OnceLock<Mutex<Option<Notifier>>> = OnceLock::new();

/// Callback opzionale usato dal bridge della GUI per avvisare il frontend
/// quando un job lanciato tramite il bridge ha modificato un documento.
pub type Notifier = Arc<dyn Fn(&str, &Value, &Value) + Send + Sync>;

pub fn set_notifier(notifier: Notifier) {
    let slot = NOTIFIER.get_or_init(|| Mutex::new(None));
    if let Ok(mut guard) = slot.lock() {
        *guard = Some(notifier);
    }
}

fn notify(method: &str, params: &Value, result: &Value) {
    let Some(slot) = NOTIFIER.get() else {
        return;
    };
    let handler = slot.lock().ok().and_then(|guard| guard.clone());
    if let Some(handler) = handler {
        handler(method, params, result);
    }
}

fn store() -> &'static Mutex<HashMap<String, Entry>> {
    STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn next_id() -> String {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("job-{millis}-{counter}")
}

/// Avvia un metodo in background. Il lavoro usa gli stessi percorsi dati e
/// risorse del processo chiamante.
pub fn start(
    method: &str,
    params: &Value,
    data_dir: PathBuf,
    resources_dir: PathBuf,
    access: AccessMode,
) -> Result<Value, String> {
    let operation = super::registry::find(method)
        .ok_or_else(|| format!("metodo sconosciuto: {method}"))?;
    if method.starts_with("job.") {
        return Err("un job non può avviare un altro job".to_string());
    }
    if method == "methods" {
        return Err("methods è un alias sincrono e non può essere eseguito come job".to_string());
    }
    if operation.mutating && access == AccessMode::Read {
        return Err(format!("operazione negata: {method} modifica dati in modalità read"));
    }
    if operation.destructive {
        let confirmed = params
            .get("confirm")
            .and_then(|value| value.as_bool())
            .unwrap_or(false);
        if access != AccessMode::Admin || !confirmed {
            return Err(format!(
                "operazione distruttiva non confermata o senza modalità admin: {method}"
            ));
        }
    }

    let id = next_id();
    let cancel = Arc::new(AtomicBool::new(false));
    let job = Job {
        id: id.clone(),
        method: method.to_string(),
        params: params.clone(),
        state: JobState::Queued,
        created_at: crate::util::now_iso(),
        started_at: None,
        finished_at: None,
        elapsed_ms: None,
        result: None,
        error: None,
        cancel_requested: false,
        access: access.as_str().to_string(),
    };
    let response = job.to_json(false);
    if let Ok(mut guard) = store().lock() {
        guard.insert(
            id.clone(),
            Entry {
                job,
                cancel: cancel.clone(),
            },
        );
    }

    let worker_id = id.clone();
    let method = method.to_string();
    let params = params.clone();
    std::thread::Builder::new()
        .name(format!("caa-job-{worker_id}"))
        .spawn(move || {
            if cancel.load(Ordering::SeqCst) {
                finish(&worker_id, JobState::Cancelled, None, None, None);
                return;
            }
            let started = Instant::now();
            if let Ok(mut guard) = store().lock() {
                if let Some(entry) = guard.get_mut(&worker_id) {
                    entry.job.state = JobState::Running;
                    entry.job.started_at = Some(crate::util::now_iso());
                }
            }
            let paths = crate::paths::AppPaths::new(data_dir, resources_dir);
            let outcome = match crate::engine::Engine::new(paths) {
                Ok(engine) => {
                    if cancel.load(Ordering::SeqCst) {
                        finish(&worker_id, JobState::Cancelled, None, None, Some(started));
                        return;
                    }
                    super::dispatch_with_options(
                        &engine,
                        &method,
                        &params,
                        DispatchOptions {
                            access,
                            dry_run: false,
                        },
                    )
                }
                Err(error) => Err(format!("avvio motore del job: {error}")),
            };
            match outcome {
                Ok(result) => {
                    finish(
                        &worker_id,
                        JobState::Done,
                        Some(result.clone()),
                        None,
                        Some(started),
                    );
                    notify(&method, &params, &result);
                }
                Err(error) => finish(&worker_id, JobState::Failed, None, Some(error), Some(started)),
            }
        })
        .map_err(|error| {
            if let Ok(mut guard) = store().lock() {
                guard.remove(&id);
            }
            format!("avvio del job non riuscito: {error}")
        })?;

    Ok(response)
}

fn finish(
    id: &str,
    state: JobState,
    result: Option<Value>,
    error: Option<String>,
    started: Option<Instant>,
) {
    if let Ok(mut guard) = store().lock() {
        if let Some(entry) = guard.get_mut(id) {
            entry.job.state = state;
            entry.job.finished_at = Some(crate::util::now_iso());
            entry.job.elapsed_ms = started.map(|instant| instant.elapsed().as_millis());
            entry.job.result = result;
            entry.job.error = error;
            entry.job.cancel_requested = entry.cancel.load(Ordering::SeqCst);
        }
    }
}

pub fn status(id: &str) -> Result<Value, String> {
    let guard = store().lock().map_err(|_| "registro job non disponibile".to_string())?;
    let entry = guard.get(id).ok_or_else(|| format!("job non trovato: {id}"))?;
    Ok(entry.job.to_json(false))
}

pub fn result(id: &str) -> Result<Value, String> {
    let guard = store().lock().map_err(|_| "registro job non disponibile".to_string())?;
    let entry = guard.get(id).ok_or_else(|| format!("job non trovato: {id}"))?;
    Ok(entry.job.to_json(true))
}

pub fn cancel(id: &str) -> Result<Value, String> {
    let mut guard = store().lock().map_err(|_| "registro job non disponibile".to_string())?;
    let entry = guard.get_mut(id).ok_or_else(|| format!("job non trovato: {id}"))?;
    entry.cancel.store(true, Ordering::SeqCst);
    entry.job.cancel_requested = true;
    if entry.job.state == JobState::Queued {
        entry.job.state = JobState::Cancelled;
        entry.job.finished_at = Some(crate::util::now_iso());
    }
    Ok(entry.job.to_json(false))
}

pub fn list() -> Value {
    let guard = store().lock().ok();
    let mut jobs: Vec<Value> = guard
        .as_ref()
        .map(|guard| guard.values().map(|entry| entry.job.to_json(false)).collect())
        .unwrap_or_default();
    jobs.sort_by(|a, b| {
        a["created_at"]
            .as_str()
            .unwrap_or_default()
            .cmp(b["created_at"].as_str().unwrap_or_default())
    });
    json!({ "jobs": jobs, "count": jobs.len() })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_state_names_are_stable() {
        assert_eq!(JobState::Queued.as_str(), "queued");
        assert_eq!(JobState::Done.as_str(), "done");
        assert_eq!(JobState::Cancelled.as_str(), "cancelled");
    }
}
