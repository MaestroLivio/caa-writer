//! Servizio neurale condiviso: una sola istanza del modello per tutta l'app.
//!
//! Perché un server locale e non un processo per richiesta:
//! - avviare `llama-cli` ricarica il modello (centinaia di MB) a ogni chiamata;
//! - con un server il modello resta in memoria e la latenza scende;
//! - tutte le funzionalità (scrittura, ricerca, studio, accessibilità) usano
//!   lo stesso servizio, con coda, priorità, cache e metriche.
//!
//! Se `llama-server` non è disponibile si ricade sul ponte `llama-cli`, più
//! lento ma funzionante: il servizio non dichiara capacità che non ha.

use std::collections::{BinaryHeap, HashMap};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

use crate::model::AiConfig;

use super::hardware::HardwareProfile;

/// Esecutore di un prompt: il server locale, oppure il ponte `llama-cli`.
pub trait Executor: Send + Sync {
    fn execute(&self, prompt: &str, max_tokens: u32) -> Result<String, String>;
    fn describe(&self) -> String;
    /// Ferma le risorse esterne (per esempio il processo `llama-server`).
    /// L'app la chiama all'uscita: senza, il processo figlio può sopravvivere
    /// alla chiusura della finestra e occupare memoria.
    fn shutdown(&self) {}
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct MetricsSnapshot {
    pub requests: u64,
    pub completed: u64,
    pub cache_hits: u64,
    pub cancelled: u64,
    pub failures: u64,
    pub total_ms: u64,
    pub last_ms: u64,
    pub pending: u64,
    pub queue_limit: u64,
    pub executor: String,
}

#[derive(Debug, Default)]
struct Counters {
    requests: AtomicU64,
    completed: AtomicU64,
    cache_hits: AtomicU64,
    cancelled: AtomicU64,
    failures: AtomicU64,
    total_ms: AtomicU64,
    last_ms: AtomicU64,
}

// ---------------------------------------------------------------- HTTP minimo

fn dechunk(body: &str) -> String {
    let mut out = String::new();
    let mut rest = body;
    loop {
        let Some((size_line, tail)) = rest.split_once("\r\n") else {
            break;
        };
        let size = usize::from_str_radix(size_line.trim(), 16).unwrap_or(0);
        if size == 0 {
            break;
        }
        if tail.len() < size + 2 {
            out.push_str(&tail[..size.min(tail.len())]);
            break;
        }
        out.push_str(&tail[..size]);
        rest = &tail[size + 2..];
    }
    out
}

fn http_request(port: u16, method: &str, path: &str, body: &str, timeout: Duration) -> Result<String, String> {
    let address = format!("127.0.0.1:{port}");
    let target = address
        .parse()
        .map_err(|_| format!("indirizzo non valido: {address}"))?;
    let mut stream =
        TcpStream::connect_timeout(&target, timeout).map_err(|e| format!("connessione al modello: {e}"))?;
    let _ = stream.set_read_timeout(Some(timeout));
    let _ = stream.set_write_timeout(Some(timeout));
    let request = format!(
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\
         Content-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|e| format!("invio al modello: {e}"))?;
    let mut raw = Vec::new();
    stream
        .read_to_end(&mut raw)
        .map_err(|e| format!("lettura dal modello: {e}"))?;
    let text = String::from_utf8_lossy(&raw).to_string();
    let (head, payload) = text
        .split_once("\r\n\r\n")
        .ok_or_else(|| "risposta HTTP malformata".to_string())?;
    let status = head.lines().next().unwrap_or_default();
    if !status.contains(" 200") {
        return Err(format!("il modello ha risposto: {status}"));
    }
    if head.to_ascii_lowercase().contains("transfer-encoding: chunked") {
        return Ok(dechunk(payload));
    }
    Ok(payload.to_string())
}

// ------------------------------------------------------- richieste al modello

/// Istruzioni di sistema per il formato conversazionale: brevi e valide per
/// tutti i prompt dell'app (parole, scelte, riscritture), che restano nel
/// messaggio utente.
const CHAT_SYSTEM: &str = "Sei un assistente per la Comunicazione Aumentativa Alternativa (CAA). \
Segui esattamente le istruzioni dell'utente: rispondi solo con ciò che viene richiesto, senza \
introduzioni, commenti o spiegazioni.";

fn chat_payload(prompt: &str, max_tokens: u32) -> String {
    serde_json::json!({
        "messages": [
            { "role": "system", "content": CHAT_SYSTEM },
            { "role": "user", "content": prompt },
        ],
        "n_predict": max_tokens,
        "temperature": 0.0,
        "top_k": 1,
        "stream": false,
        "cache_prompt": true,
    })
    .to_string()
}

fn completion_payload(prompt: &str, max_tokens: u32) -> String {
    serde_json::json!({
        "prompt": prompt,
        "n_predict": max_tokens,
        "temperature": 0.0,
        "top_k": 1,
        "stream": false,
        "cache_prompt": true,
    })
    .to_string()
}

/// Contenuto del primo messaggio di `/v1/chat/completions`, se non vuoto.
fn chat_content(body: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(body).ok()?;
    let content = value
        .get("choices")?
        .get(0)?
        .get("message")?
        .get("content")?
        .as_str()?
        .trim();
    (!content.is_empty()).then(|| content.to_string())
}

/// Contenuto di `/completion`, se non vuoto.
fn completion_content(body: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(body).ok()?;
    let content = value.get("content")?.as_str()?.trim();
    (!content.is_empty()).then(|| content.to_string())
}

// ------------------------------------------------------------ istanza locale

struct ServerHandle {
    child: Child,
    port: u16,
    model: PathBuf,
}

impl Drop for ServerHandle {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

pub struct LocalModel {
    ai_dir: PathBuf,
    config: Mutex<AiConfig>,
    hardware: HardwareProfile,
    server: Mutex<Option<ServerHandle>>,
}

fn free_port() -> Result<u16, String> {
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    drop(listener);
    Ok(port)
}

impl LocalModel {
    pub fn new(ai_dir: PathBuf, config: AiConfig) -> Self {
        Self {
            ai_dir,
            config: Mutex::new(config),
            hardware: super::hardware::detect(),
            server: Mutex::new(None),
        }
    }

    pub fn hardware(&self) -> HardwareProfile {
        self.hardware.clone()
    }

    pub fn set_config(&self, config: AiConfig) {
        if let Ok(mut slot) = self.config.lock() {
            *slot = config;
        }
        self.stop_server();
    }

    pub fn config(&self) -> AiConfig {
        self.config.lock().map(|c| c.clone()).unwrap_or_default()
    }

    fn server_binary(&self) -> Option<PathBuf> {
        for name in ["llama-server", "llama-server.exe"] {
            for directory in [self.ai_dir.clone(), self.ai_dir.join("runtime")] {
                let candidate = directory.join(name);
                if candidate.exists() {
                    return Some(candidate);
                }
            }
        }
        None
    }

    pub fn is_running(&self) -> bool {
        self.server
            .lock()
            .map(|slot| slot.is_some())
            .unwrap_or(false)
    }

    /// PID dell'istanza condivisa, per le misure di memoria del pannello
    /// diagnostico. `None` quando il modello non è in esecuzione.
    pub fn server_pid(&self) -> Option<u32> {
        self.server
            .lock()
            .ok()
            .and_then(|slot| slot.as_ref().map(|handle| handle.child.id()))
    }

    pub fn stop_server(&self) {
        if let Ok(mut slot) = self.server.lock() {
            *slot = None;
        }
    }

    fn ensure_server(&self) -> Result<u16, String> {
        let config = self.config();
        if !config.enabled {
            return Err("motore neurale disattivato".to_string());
        }
        let model = PathBuf::from(&config.model_path);
        if !model.exists() {
            return Err("modello GGUF non trovato".to_string());
        }
        let Some(binary) = self.server_binary() else {
            return Err("llama-server non disponibile: uso il ponte llama-cli".to_string());
        };

        let mut slot = self.server.lock().map_err(|_| "istanza del modello non disponibile".to_string())?;
        if let Some(handle) = slot.as_mut() {
            if handle.model == model && handle.child.try_wait().ok().flatten().is_none() {
                return Ok(handle.port);
            }
        }
        *slot = None;

        let port = free_port()?;
        let context = config
            .context_size
            .clamp(512, self.hardware.max_context.max(512));
        let threads = config.threads.clamp(1, self.hardware.max_threads.max(1));
        let mut command = crate::process::command(&binary);
        command
            .arg("-m")
            .arg(&model)
            .arg("-c")
            .arg(context.to_string())
            .arg("-t")
            .arg(threads.to_string())
            .arg("--host")
            .arg("127.0.0.1")
            .arg("--port")
            .arg(port.to_string())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        match config.backend.as_str() {
            "metal" => {
                command.arg("-ngl").arg("999");
            }
            "cpu" => {
                command.arg("-ngl").arg("0");
            }
            _ => {
                if self.hardware.accelerated {
                    command.arg("-ngl").arg("999");
                }
            }
        }
        for extra in &config.extra_args {
            command.arg(extra);
        }
        let child = command
            .spawn()
            .map_err(|e| format!("avvio del modello non riuscito: {e}"))?;

        // Il server impiega tempo a caricare il modello: si attende /health.
        let deadline = Instant::now() + Duration::from_secs(45);
        while Instant::now() < deadline {
            if let Ok(body) = http_request(port, "GET", "/health", "", Duration::from_millis(800)) {
                if body.contains("ok") {
                    *slot = Some(ServerHandle {
                        child,
                        port,
                        model,
                    });
                    return Ok(port);
                }
            }
            std::thread::sleep(Duration::from_millis(200));
        }
        let mut child = child;
        let _ = child.kill();
        Err("il modello non è diventato pronto entro 45 s".to_string())
    }

    fn complete_with_server(&self, prompt: &str, max_tokens: u32) -> Result<String, String> {
        let port = self.ensure_server()?;
        let config = self.config();
        let timeout = Duration::from_millis(config.timeout_ms.clamp(1000, 60_000));

        // I modelli Instruct (Qwen, SmolLM2…) sono addestrati sul template di
        // chat: con il solo `/completion` tendono a ripetere la prima parola.
        // Si prova prima il formato conversazionale e, se il modello non ha un
        // template o la richiesta fallisce, si ricade sulla continuazione
        // semplice: nessuna capacità viene dichiarata se non funziona.
        if let Ok(body) = http_request(
            port,
            "POST",
            "/v1/chat/completions",
            &chat_payload(prompt, max_tokens),
            timeout,
        ) {
            if let Some(content) = chat_content(&body) {
                return Ok(content);
            }
        }

        let body = http_request(
            port,
            "POST",
            "/completion",
            &completion_payload(prompt, max_tokens),
            timeout,
        )?;
        completion_content(&body).ok_or_else(|| {
            "il modello non ha prodotto output".to_string()
        })
    }
}

impl Executor for LocalModel {
    fn execute(&self, prompt: &str, max_tokens: u32) -> Result<String, String> {
        match self.complete_with_server(prompt, max_tokens) {
            Ok(content) => Ok(content),
            Err(server_error) => {
                // Ripiego dichiarato: il ponte a processo singolo resta valido.
                let config = self.config();
                if !super::is_available(&config) {
                    return Err(server_error);
                }
                super::llama_cli::run_completion(&config, prompt).map_err(|cli_error| {
                    format!("{server_error}; ripiego llama-cli: {cli_error}")
                })
            }
        }
    }

    fn describe(&self) -> String {
        match self.server_binary() {
            Some(path) => format!("llama-server ({})", path.display()),
            None => "llama-cli (nessuna istanza condivisa disponibile)".to_string(),
        }
    }

    fn shutdown(&self) {
        self.stop_server();
    }
}

// ------------------------------------------------------------------ coda

struct Job {
    priority: u8,
    seq: u64,
    prompt: String,
    max_tokens: u32,
    epoch: u64,
    cancel: Arc<AtomicBool>,
    reply: mpsc::Sender<Result<String, String>>,
}

impl PartialEq for Job {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority && self.seq == other.seq
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
        // Priorità più alta vince; a parità, la richiesta più vecchia (seq più
        // bassa) viene servita prima.
        self.priority
            .cmp(&other.priority)
            .then_with(|| other.seq.cmp(&self.seq))
    }
}

pub struct Service {
    tx: mpsc::Sender<Job>,
    counters: Arc<Counters>,
    executor: Arc<dyn Executor>,
    seq: AtomicU64,
    /// Cambia quando cambia il modello: le voci di cache del modello
    /// precedente non devono essere restituite (risposte sbagliate ma
    /// apparentemente valide).
    cache_epoch: Arc<AtomicU64>,
    timeout: Duration,
    /// Richieste in attesa o in esecuzione: la coda non deve accumulare
    /// decine di inferenze obsolete su una macchina da 4 GB.
    pending: Arc<AtomicU64>,
    queue_limit: u64,
}

const CACHE_LIMIT: usize = 256;

impl Service {
    pub fn new(executor: Arc<dyn Executor>, request_timeout: Duration) -> Self {
        let (tx, rx) = mpsc::channel::<Job>();
        let counters = Arc::new(Counters::default());
        let worker_counters = counters.clone();
        let worker_executor = executor.clone();
        std::thread::Builder::new()
            .name("caa-ai-worker".to_string())
            .spawn(move || {
                let mut queue: BinaryHeap<Job> = BinaryHeap::new();
                let mut cache: HashMap<String, String> = HashMap::new();
                loop {
                    if queue.is_empty() {
                        match rx.recv() {
                            Ok(job) => queue.push(job),
                            Err(_) => return,
                        }
                    }
                    while let Ok(job) = rx.try_recv() {
                        queue.push(job);
                    }
                    let Some(job) = queue.pop() else { continue };
                    if job.cancel.load(Ordering::SeqCst) {
                        worker_counters.cancelled.fetch_add(1, Ordering::Relaxed);
                        let _ = job.reply.send(Err("richiesta annullata".to_string()));
                        continue;
                    }
                    let key = format!("{}|{}|{}", job.epoch, job.max_tokens, job.prompt);
                    if let Some(hit) = cache.get(&key) {
                        worker_counters.cache_hits.fetch_add(1, Ordering::Relaxed);
                        let _ = job.reply.send(Ok(hit.clone()));
                        continue;
                    }
                    let started = Instant::now();
                    let outcome = worker_executor.execute(&job.prompt, job.max_tokens);
                    let elapsed = started.elapsed().as_millis() as u64;
                    worker_counters.total_ms.fetch_add(elapsed, Ordering::Relaxed);
                    worker_counters.last_ms.store(elapsed, Ordering::Relaxed);
                    match outcome {
                        Ok(text) => {
                            worker_counters.completed.fetch_add(1, Ordering::Relaxed);
                            if cache.len() >= CACHE_LIMIT {
                                cache.clear();
                            }
                            cache.insert(key, text.clone());
                            let _ = job.reply.send(Ok(text));
                        }
                        Err(error) => {
                            worker_counters.failures.fetch_add(1, Ordering::Relaxed);
                            let _ = job.reply.send(Err(error));
                        }
                    }
                }
            })
            .expect("impossibile avviare il thread del motore neurale");

        Self {
            tx,
            counters,
            executor,
            seq: AtomicU64::new(0),
            cache_epoch: Arc::new(AtomicU64::new(0)),
            timeout: request_timeout,
            pending: Arc::new(AtomicU64::new(0)),
            queue_limit: 8,
        }
    }

    /// Invalida la cache delle risposte: da chiamare quando cambia il modello
    /// attivo, perché la stessa richiesta avrebbe un esito diverso.
    pub fn invalidate_cache(&self) {
        self.cache_epoch.fetch_add(1, Ordering::SeqCst);
    }

    /// Invia una richiesta e attende il risultato. `priority`: 10 interazione
    /// esplicita, 7 scrittura, 4 anteprime, 1 background.
    pub fn complete(
        &self,
        prompt: String,
        max_tokens: u32,
        priority: u8,
        cancel: Arc<AtomicBool>,
    ) -> Result<String, String> {
        let pending = self.pending.fetch_add(1, Ordering::Relaxed) + 1;
        if pending > self.queue_limit {
            self.pending.fetch_sub(1, Ordering::Relaxed);
            self.counters.failures.fetch_add(1, Ordering::Relaxed);
            return Err(
                "coda del motore neurale piena: la richiesta è stata rinviata per non bloccare \
                 la scrittura"
                    .to_string(),
            );
        }
        self.counters.requests.fetch_add(1, Ordering::Relaxed);
        let seq = self.seq.fetch_add(1, Ordering::Relaxed);
        let (reply, rx) = mpsc::channel();
        self.tx
            .send(Job {
                priority,
                seq,
                prompt,
                max_tokens,
                epoch: self.cache_epoch.load(Ordering::SeqCst),
                cancel,
                reply,
            })
            .map_err(|_| "motore neurale non disponibile".to_string())?;
        let result = rx.recv_timeout(self.timeout);
        self.pending.fetch_sub(1, Ordering::Relaxed);
        result.map_err(|_| "il modello non ha risposto in tempo".to_string())?
    }

    pub fn cancel_handle(&self) -> Arc<AtomicBool> {
        Arc::new(AtomicBool::new(false))
    }

    /// Ferma le risorse esterne dell'esecutore (processo del modello).
    pub fn shutdown(&self) {
        self.executor.shutdown();
    }

    pub fn metrics(&self) -> MetricsSnapshot {
        let requests = self.counters.requests.load(Ordering::Relaxed);
        let completed = self.counters.completed.load(Ordering::Relaxed);
        let total = self.counters.total_ms.load(Ordering::Relaxed);
        MetricsSnapshot {
            requests,
            completed,
            cache_hits: self.counters.cache_hits.load(Ordering::Relaxed),
            cancelled: self.counters.cancelled.load(Ordering::Relaxed),
            failures: self.counters.failures.load(Ordering::Relaxed),
            total_ms: total,
            last_ms: self.counters.last_ms.load(Ordering::Relaxed),
            pending: self.pending.load(Ordering::Relaxed),
            queue_limit: self.queue_limit,
            executor: self.executor.describe(),
        }
    }
}

/// Comodità: percorso del modello incluso, usato come default.
pub fn bundled_model(ai_dir: &Path) -> Option<PathBuf> {
    let candidate = ai_dir.join("model.gguf");
    candidate.exists().then_some(candidate)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicU32;

    struct Counting {
        calls: AtomicU32,
    }

    impl Executor for Counting {
        fn execute(&self, prompt: &str, _max_tokens: u32) -> Result<String, String> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(format!("risposta a {prompt}"))
        }
        fn describe(&self) -> String {
            "finto".to_string()
        }
    }

    #[test]
    fn second_identical_request_hits_cache() {
        let executor = Arc::new(Counting {
            calls: AtomicU32::new(0),
        });
        let service = Service::new(executor.clone(), Duration::from_secs(5));
        let cancel = service.cancel_handle();
        let first = service
            .complete("ciao".into(), 8, 5, cancel.clone())
            .unwrap();
        let second = service
            .complete("ciao".into(), 8, 5, cancel)
            .unwrap();
        assert_eq!(first, second);
        assert_eq!(executor.calls.load(Ordering::SeqCst), 1);
        assert!(service.metrics().cache_hits >= 1);
    }

    #[test]
    fn cancelled_request_does_not_reach_executor() {
        let executor = Arc::new(Counting {
            calls: AtomicU32::new(0),
        });
        let service = Service::new(executor.clone(), Duration::from_secs(5));
        let cancel = service.cancel_handle();
        cancel.store(true, Ordering::SeqCst);
        assert!(service.complete("annulla".into(), 4, 9, cancel).is_err());
        assert_eq!(executor.calls.load(Ordering::SeqCst), 0);
        assert_eq!(service.metrics().cancelled, 1);
    }

    #[test]
    fn invalidate_cache_forces_a_new_inference() {
        struct Incrementing {
            calls: AtomicU32,
        }
        impl Executor for Incrementing {
            fn execute(&self, _prompt: &str, _max_tokens: u32) -> Result<String, String> {
                let n = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
                Ok(format!("risposta {n}"))
            }
            fn describe(&self) -> String {
                "incrementale".to_string()
            }
        }
        let executor = Arc::new(Incrementing {
            calls: AtomicU32::new(0),
        });
        let service = Service::new(executor.clone(), Duration::from_secs(5));
        let first = service
            .complete("stesso prompt".into(), 8, 5, service.cancel_handle())
            .unwrap();
        let cached = service
            .complete("stesso prompt".into(), 8, 5, service.cancel_handle())
            .unwrap();
        assert_eq!(first, cached);
        service.invalidate_cache();
        let fresh = service
            .complete("stesso prompt".into(), 8, 5, service.cancel_handle())
            .unwrap();
        assert_ne!(fresh, first);
        assert_eq!(executor.calls.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn shutdown_reaches_the_executor() {
        struct Stoppable {
            stopped: Arc<AtomicBool>,
        }
        impl Executor for Stoppable {
            fn execute(&self, _prompt: &str, _max_tokens: u32) -> Result<String, String> {
                Ok(String::new())
            }
            fn describe(&self) -> String {
                "fermabile".to_string()
            }
            fn shutdown(&self) {
                self.stopped.store(true, Ordering::SeqCst);
            }
        }
        let stopped = Arc::new(AtomicBool::new(false));
        let service = Service::new(
            Arc::new(Stoppable {
                stopped: stopped.clone(),
            }),
            Duration::from_secs(5),
        );
        service.shutdown();
        assert!(stopped.load(Ordering::SeqCst));
    }

    #[test]
    fn dechunk_decodes_chunked_body() {
        // Due blocchi: 4 byte ({"a") e 3 byte (:1}), poi il blocco finale.
        let body = "4\r\n{\"a\"\r\n3\r\n:1}\r\n0\r\n\r\n";
        assert_eq!(dechunk(body), "{\"a\":1}");
    }

    #[test]
    fn chat_payload_carries_system_and_user_messages() {
        let payload = chat_payload("Elenca parole.\nParole:", 24);
        let value: serde_json::Value = serde_json::from_str(&payload).unwrap();
        let messages = value["messages"].as_array().unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0]["role"], "system");
        assert!(messages[0]["content"].as_str().unwrap().contains("CAA"));
        assert_eq!(messages[1]["content"], "Elenca parole.\nParole:");
        assert_eq!(value["n_predict"], 24);
        assert_eq!(value["temperature"], 0.0);
    }

    #[test]
    fn chat_content_reads_reply_and_rejects_empty() {
        let body = r#"{"choices":[{"message":{"role":"assistant","content":"  2  "}}]}"#;
        assert_eq!(chat_content(body).as_deref(), Some("2"));
        let empty = r#"{"choices":[{"message":{"content":"   "}}]}"#;
        assert_eq!(chat_content(empty), None);
        assert_eq!(chat_content("non json"), None);
        assert_eq!(chat_content(r#"{"error":"no template"}"#), None);
    }

    #[test]
    fn completion_content_reads_plain_reply() {
        let body = r#"{"content":" mare \n barca "}"#;
        assert_eq!(
            completion_content(body).as_deref(),
            Some("mare \n barca")
        );
        assert_eq!(completion_content(r#"{"content":""}"#), None);
    }
}
