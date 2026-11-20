//! Bridge locale tra la GUI aperta e gli altri client di automazione.
//!
//! Quando l'applicazione Tauri è aperta avvia un piccolo server HTTP su
//! `127.0.0.1` con un token casuale. CLI e server MCP lo scoprono dal file
//! `<data-dir>/automation-bridge.json` e inoltrano lì le chiamate, così usano
//! lo stesso motore della GUI invece di aprire un secondo processo headless.
//! Se la GUI non è aperta il file non esiste e i client continuano in locale.
//!
//! Il bridge non è raggiungibile dalla rete: ascolta solo sull'indirizzo di
//! loopback, richiede il token e non espone alcun comando di shell.
//!
//! Un osservatore del filesystem notifica il frontend anche quando un file viene
//! modificato da un processo esterno (CLI headless, MCP, altro editor), non solo
//! quando la modifica passa dal bridge. Così i job lanciati da client headless
//! raggiungono la GUI aperta.

use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter, Manager};

use super::{dispatch_with_options, policy::AccessMode, DispatchOptions};
use crate::automation::http;

pub const DESCRIPTOR_FILE: &str = "automation-bridge.json";
pub const SCHEMA_VERSION: u32 = 1;
const MAX_CONCURRENT: usize = 32;
const WATCH_INTERVAL: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeDescriptor {
    pub schema: u32,
    pub pid: u32,
    pub port: u16,
    pub token: String,
    pub url: String,
    pub started_at: String,
    pub app_version: String,
    pub api_version: String,
    pub data_dir: String,
    pub resources_dir: String,
}

impl BridgeDescriptor {
    pub fn socket(&self) -> Result<SocketAddr, String> {
        format!("127.0.0.1:{}", self.port)
            .parse()
            .map_err(|error| format!("indirizzo bridge non valido: {error}"))
    }
}

static LOCAL_STATUS: OnceLock<Mutex<Option<Value>>> = OnceLock::new();

fn local_slot() -> &'static Mutex<Option<Value>> {
    LOCAL_STATUS.get_or_init(|| Mutex::new(None))
}

/// Stato del bridge attivo in questo processo, se c'è. La GUI lo include in
/// `app.status`; un processo headless restituisce `null`.
pub fn local_status() -> Value {
    local_slot()
        .lock()
        .ok()
        .and_then(|guard| guard.clone())
        .unwrap_or(Value::Null)
}

pub fn descriptor_path(data_dir: &Path) -> PathBuf {
    data_dir.join(DESCRIPTOR_FILE)
}

/// Impronta leggera del file usata per accorgersi di una modifica esterna.
pub fn file_stamp(path: &Path) -> Option<String> {
    let metadata = fs::metadata(path).ok()?;
    if !metadata.is_file() {
        return None;
    }
    let modified = metadata
        .modified()
        .ok()
        .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
        .map(|value| value.as_nanos())
        .unwrap_or(0);
    Some(format!("{modified}:{}", metadata.len()))
}

fn random_token() -> String {
    let mut bytes = [0u8; 32];
    #[cfg(unix)]
    {
        if let Ok(mut file) = fs::File::open("/dev/urandom") {
            if file.read_exact(&mut bytes).is_ok() {
                return hex(&bytes);
            }
        }
    }
    let seed = format!(
        "{}:{}:{:?}:{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|value| value.as_nanos())
            .unwrap_or(0),
        std::thread::current().id(),
        LOCAL_STATUS.get_or_init(|| Mutex::new(None)) as *const _ as usize,
    );
    hex(&Sha256::digest(seed.as_bytes()))
}

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

fn write_descriptor(path: &Path, descriptor: &BridgeDescriptor) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("cartella dati {}: {error}", parent.display()))?;
    }
    let text = serde_json::to_vec_pretty(descriptor).map_err(|error| error.to_string())?;
    let temporary = path.with_extension(format!("tmp-{}", std::process::id()));
    fs::write(&temporary, text)
        .map_err(|error| format!("scrittura {}: {error}", temporary.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&temporary, fs::Permissions::from_mode(0o600));
    }
    fs::rename(&temporary, path)
        .map_err(|error| format!("installazione {}: {error}", path.display()))?;
    Ok(())
}

fn remove_descriptor_if_ours(path: &Path, token: &str) {
    let Ok(text) = fs::read_to_string(path) else {
        return;
    };
    let Ok(value) = serde_json::from_str::<Value>(&text) else {
        return;
    };
    if value.get("token").and_then(Value::as_str) == Some(token) {
        let _ = fs::remove_file(path);
    }
}

/// Server vivo finché la GUI è aperta. Va tenuto nello stato Tauri e fermato
/// esplicitamente all'uscita.
pub struct BridgeServer {
    descriptor: BridgeDescriptor,
    stop: Arc<AtomicBool>,
    path: PathBuf,
}

impl BridgeServer {
    pub fn start(app: AppHandle, data_dir: PathBuf, resources_dir: PathBuf) -> Result<Self, String> {
        let listener = TcpListener::bind(("127.0.0.1", 0))
            .map_err(|error| format!("bridge GUI: impossibile ascoltare su localhost: {error}"))?;
        listener
            .set_nonblocking(true)
            .map_err(|error| format!("bridge GUI: {error}"))?;
        let address = listener
            .local_addr()
            .map_err(|error| format!("bridge GUI: {error}"))?;
        let descriptor = BridgeDescriptor {
            schema: SCHEMA_VERSION,
            pid: std::process::id(),
            port: address.port(),
            token: random_token(),
            url: format!("http://127.0.0.1:{}/call", address.port()),
            started_at: crate::util::now_iso(),
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            api_version: super::API_VERSION.to_string(),
            data_dir: data_dir.to_string_lossy().to_string(),
            resources_dir: resources_dir.to_string_lossy().to_string(),
        };
        let path = descriptor_path(&data_dir);
        write_descriptor(&path, &descriptor)?;

        if let Ok(mut slot) = local_slot().lock() {
            *slot = Some(json!({
                "available": true,
                "transport": "http-loopback",
                "url": format!("http://127.0.0.1:{}/call", address.port()),
                "health": format!("http://127.0.0.1:{}/health", address.port()),
                "pid": descriptor.pid,
                "app_version": descriptor.app_version,
                "api_version": descriptor.api_version,
                "descriptor": path,
                "started_at": descriptor.started_at,
            }));
        }

        let stop = Arc::new(AtomicBool::new(false));
        let shared = Arc::new(Shared {
            app: app.clone(),
            descriptor: descriptor.clone(),
            active: Arc::new(Mutex::new(0usize)),
            max: MAX_CONCURRENT,
        });

        // I job lanciati tramite il bridge scrivono con un proprio motore: al
        // termine il bridge manda l'evento di ricarica come per le chiamate
        // sincrone.
        let shared_for_notifier = shared.clone();
        super::jobs::set_notifier(Arc::new(move |method, params, result| {
            let shared = shared_for_notifier.clone();
            notify_changes(&shared.app, method, params, result);
        }));

        // Osservatore del filesystem: notifica anche le modifiche fatte da
        // processi esterni (CLI headless, MCP, altro editor), compresi i job
        // che girano fuori dal bridge.
        let watcher_stop = stop.clone();
        let watcher_app = app.clone();
        let watcher_data_dir = data_dir.clone();
        let watcher_resources_dir = resources_dir.clone();
        std::thread::Builder::new()
            .name("caa-bridge-watcher".to_string())
            .spawn(move || {
                watch_filesystem(
                    watcher_app,
                    watcher_data_dir,
                    watcher_resources_dir,
                    watcher_stop,
                );
            })
            .ok();

        let stop_loop = stop.clone();
        let shared_loop = shared.clone();
        std::thread::Builder::new()
            .name("caa-bridge-listener".to_string())
            .spawn(move || {
                while !stop_loop.load(Ordering::SeqCst) {
                    match listener.accept() {
                        Ok((stream, _)) => {
                            let shared = Arc::clone(&shared_loop);
                            let _ = stream.set_nonblocking(false);
                            let _ = stream.set_nodelay(true);
                            let _ = std::thread::Builder::new()
                                .name("caa-bridge-request".to_string())
                                .spawn(move || {
                                    if !shared.enter() {
                                        let mut stream = stream;
                                        let _ = http::respond_json(
                                            &mut stream,
                                            503,
                                            &json!({"ok": false, "error": "bridge occupato: troppe richieste contemporanee"}),
                                            &[],
                                        );
                                        return;
                                    }
                                    handle_connection(&shared, stream);
                                    shared.leave();
                                });
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            std::thread::sleep(Duration::from_millis(40));
                        }
                        Err(_) => break,
                    }
                }
            })
            .map_err(|error| format!("bridge GUI: thread non avviato: {error}"))?;

        Ok(Self {
            descriptor,
            stop,
            path,
        })
    }

    pub fn descriptor(&self) -> &BridgeDescriptor {
        &self.descriptor
    }

    pub fn shutdown(&self) {
        self.stop.store(true, Ordering::SeqCst);
        remove_descriptor_if_ours(&self.path, &self.descriptor.token);
        if let Ok(mut slot) = local_slot().lock() {
            *slot = Some(json!({
                "available": false,
                "transport": "http-loopback",
                "stopped_at": crate::util::now_iso(),
            }));
        }
    }
}

impl Drop for BridgeServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        remove_descriptor_if_ours(&self.path, &self.descriptor.token);
    }
}

struct Shared {
    app: AppHandle,
    descriptor: BridgeDescriptor,
    active: Arc<Mutex<usize>>,
    max: usize,
}

impl Shared {
    fn enter(&self) -> bool {
        match self.active.lock() {
            Ok(mut guard) => {
                if *guard >= self.max {
                    false
                } else {
                    *guard += 1;
                    true
                }
            }
            Err(_) => false,
        }
    }

    fn leave(&self) {
        if let Ok(mut guard) = self.active.lock() {
            *guard = guard.saturating_sub(1);
        }
    }
}

fn handle_connection(shared: &Shared, mut stream: TcpStream) {
    let request = match http::read_request(&mut stream, http::MAX_BODY) {
        Ok(Some(request)) => request,
        Ok(None) => return,
        Err(error) => {
            let _ = http::respond_json(
                &mut stream,
                400,
                &json!({"ok": false, "error": error}),
                &[],
            );
            return;
        }
    };

    let provided = request
        .header("authorization")
        .and_then(|value| value.strip_prefix("Bearer "))
        .or_else(|| request.header("x-caa-token"));
    if !http::token_matches(&shared.descriptor.token, provided) {
        let _ = http::respond_json(
            &mut stream,
            401,
            &json!({"ok": false, "error": "token del bridge non valido"}),
            &[],
        );
        return;
    }

    let result = match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/health") => Ok((
            200,
            json!({
                "ok": true,
                "service": "caa-writer-bridge",
                "schema": SCHEMA_VERSION,
                "pid": shared.descriptor.pid,
                "app_version": shared.descriptor.app_version,
                "api_version": shared.descriptor.api_version,
                "started_at": shared.descriptor.started_at,
            }),
        )),
        ("POST", "/call") => call(shared, &request),
        ("GET", "/status") => call_method(shared, "app.status", &json!({}), AccessMode::Read, false),
        (_, "/call") | (_, "/health") | (_, "/status") => Err((
            405,
            json!({"ok": false, "error": "metodo HTTP non supportato"}),
        )),
        _ => Err((404, json!({"ok": false, "error": "percorso non trovato"}))),
    };

    match result {
        Ok((status, value)) => {
            let _ = http::respond_json(&mut stream, status, &value, &[("X-CAA-Bridge", "1")]);
        }
        Err((status, value)) => {
            let _ = http::respond_json(&mut stream, status, &value, &[("X-CAA-Bridge", "1")]);
        }
    }
}

fn call(shared: &Shared, request: &http::HttpRequest) -> Result<(u16, Value), (u16, Value)> {
    let body = request.json().map_err(|error| (400, json!({"ok": false, "error": error})))?;
    let method = body
        .get("method")
        .and_then(Value::as_str)
        .ok_or_else(|| (400, json!({"ok": false, "error": "serve il campo method"})))?;
    let params = body.get("params").cloned().unwrap_or_else(|| json!({}));
    let access = match body.get("access").and_then(Value::as_str) {
        Some(value) => AccessMode::parse(value)
            .map_err(|error| (400, json!({"ok": false, "error": error})))?,
        // Senza modalità esplicita il bridge resta in sola lettura: un client
        // non deve poter modificare dati per omissione.
        None => AccessMode::Read,
    };
    let dry_run = body
        .get("dry_run")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    call_method(shared, method, &params, access, dry_run)
}

fn call_method(
    shared: &Shared,
    method: &str,
    params: &Value,
    access: AccessMode,
    dry_run: bool,
) -> Result<(u16, Value), (u16, Value)> {
    let engine = shared.app.state::<crate::engine::Engine>();
    let options = DispatchOptions { access, dry_run };
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        dispatch_with_options(&engine, method, params, options)
    }))
    .map_err(|_| {
        (
            500,
            json!({"ok": false, "error": format!("operazione {method} interrotta da un panic")}),
        )
    })?;
    match outcome {
        Ok(result) => {
            if !dry_run && !method.starts_with("job.") {
                notify_changes(&shared.app, method, params, &result);
            }
            Ok((200, json!({"ok": true, "result": result})))
        }
        Err(error) => Ok((200, json!({"ok": false, "error": error}))),
    }
}

#[derive(Debug, Clone, Serialize)]
struct DocumentChange {
    path: String,
    method: String,
    source: &'static str,
    removed: bool,
    exists: bool,
    stamp: Option<String>,
    at: String,
}

fn notify_changes(app: &AppHandle, method: &str, params: &Value, result: &Value) {
    for (path, removed) in changed_paths(method, params, result) {
        let exists = !removed && path.is_file();
        let payload = DocumentChange {
            stamp: if exists { file_stamp(&path) } else { None },
            path: path.to_string_lossy().to_string(),
            method: method.to_string(),
            source: "bridge",
            removed,
            exists,
            at: crate::util::now_iso(),
        };
        let _ = app.emit("automation-document-changed", payload);
    }
}

fn changed_paths(method: &str, params: &Value, result: &Value) -> Vec<(PathBuf, bool)> {
    let as_path = |value: Option<&Value>| {
        value
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
    };
    let result_path = as_path(result.get("path"));
    let result_removed = as_path(result.get("removed"));
    let source = as_path(result.get("source"));
    let mut out = Vec::new();
    match method {
        "project.delete" => {
            if let Some(path) = result_removed.or_else(|| as_path(params.get("path"))) {
                out.push((path, true));
            }
        }
        "project.rename" => {
            if let Some(path) = source {
                out.push((path, true));
            }
            if let Some(path) = result_path {
                out.push((path, false));
            }
        }
        "project.create" | "project.save" | "project.duplicate" => {
            if let Some(path) = result_path {
                out.push((path, false));
            }
        }
        "document.create" | "document.update" | "element.add" | "element.update"
        | "element.delete" | "element.move" | "element.duplicate" => {
            if let Some(path) = result_path {
                out.push((path, false));
            }
        }
        "content.generate" | "content.transform" => {
            if let Some(path) = result_path {
                out.push((path, false));
            }
        }
        _ => {}
    }
    out
}

/// Osserva la cartella progetti e notifica il frontend quando un file .caadoc
/// viene modificato da un processo esterno. Serve a coprire i job lanciati da
/// client headless (che girano fuori dal bridge) e qualsiasi altro strumento
/// che tocchi i file.
fn watch_filesystem(
    app: AppHandle,
    data_dir: PathBuf,
    resources_dir: PathBuf,
    stop: Arc<AtomicBool>,
) {
    let projects_root = data_dir.join("projects");
    let library_root = resources_dir.join("library");
    let mut known: HashMap<PathBuf, SystemTime> = HashMap::new();
    // Prima scansione: registra lo stato senza emettere eventi.
    scan_projects(&projects_root, &mut known);
    let mut known_library: HashMap<PathBuf, SystemTime> = HashMap::new();
    if crate::dev_library::enabled() {
        scan_library(&library_root, &mut known_library);
    }
    while !stop.load(Ordering::SeqCst) {
        std::thread::sleep(WATCH_INTERVAL);
        if stop.load(Ordering::SeqCst) {
            break;
        }
        let mut current: HashMap<PathBuf, SystemTime> = HashMap::new();
        scan_projects(&projects_root, &mut current);
        // File modificati o nuovi.
        for (path, modified) in &current {
            match known.get(path) {
                Some(previous) if previous != modified => {
                    let payload = DocumentChange {
                        path: path.to_string_lossy().to_string(),
                        method: "filesystem.watch".to_string(),
                        source: "watcher",
                        removed: false,
                        exists: true,
                        stamp: file_stamp(path),
                        at: crate::util::now_iso(),
                    };
                    let _ = app.emit("automation-document-changed", payload);
                }
                None => {
                    // File nuovo: notifica solo se è un documento .caadoc
                    // già presente nella sessione precedente? No: un file nuovo
                    // non è aperto, ma la notifica è innocua perché il frontend
                    // filtra per percorso corrente.
                    let payload = DocumentChange {
                        path: path.to_string_lossy().to_string(),
                        method: "filesystem.watch".to_string(),
                        source: "watcher",
                        removed: false,
                        exists: true,
                        stamp: file_stamp(path),
                        at: crate::util::now_iso(),
                    };
                    let _ = app.emit("automation-document-changed", payload);
                }
                _ => {}
            }
        }
        // File rimossi.
        for path in known.keys() {
            if !current.contains_key(path) {
                let payload = DocumentChange {
                    path: path.to_string_lossy().to_string(),
                    method: "filesystem.watch".to_string(),
                    source: "watcher",
                    removed: true,
                    exists: false,
                    stamp: None,
                    at: crate::util::now_iso(),
                };
                let _ = app.emit("automation-document-changed", payload);
            }
        }
        known = current;

        // Modalità sviluppatore: osserva i pacchetti della libreria e avvisa
        // il browser dev quando un coding agent aggiunge, modifica o elimina
        // un documento o il catalogo.
        if crate::dev_library::enabled() {
            let mut current_library: HashMap<PathBuf, SystemTime> = HashMap::new();
            scan_library(&library_root, &mut current_library);
            for (path, modified) in &current_library {
                let change = match known_library.get(path) {
                    Some(previous) if previous != modified => Some("modified"),
                    None => Some("created"),
                    _ => None,
                };
                if let Some(change) = change {
                    let _ = app.emit(
                        "developer-library-changed",
                        json!({
                            "path": path.to_string_lossy().to_string(),
                            "change": change,
                            "stamp": file_stamp(path),
                            "at": crate::util::now_iso(),
                        }),
                    );
                }
            }
            for path in known_library.keys() {
                if !current_library.contains_key(path) {
                    let _ = app.emit(
                        "developer-library-changed",
                        json!({
                            "path": path.to_string_lossy().to_string(),
                            "change": "removed",
                            "stamp": Value::Null,
                            "at": crate::util::now_iso(),
                        }),
                    );
                }
            }
            known_library = current_library;
        }
    }
}

fn scan_library(root: &Path, out: &mut HashMap<PathBuf, SystemTime>) {
    scan_projects(root, out);
    let manifest = root.join("manifest.json");
    if let Ok(metadata) = fs::metadata(&manifest) {
        if let Ok(modified) = metadata.modified() {
            out.insert(manifest, modified);
        }
    }
}

fn scan_projects(root: &Path, out: &mut HashMap<PathBuf, SystemTime>) {
    if !root.is_dir() {
        return;
    }
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_projects(&path, out);
        } else if path
            .extension()
            .and_then(|value| value.to_str())
            .map(|value| value.eq_ignore_ascii_case("caadoc"))
            .unwrap_or(false)
        {
            if let Ok(metadata) = fs::metadata(&path) {
                if let Ok(modified) = metadata.modified() {
                    out.insert(path, modified);
                }
            }
        }
    }
}

// ------------------------------------------------------------------ client

#[derive(Debug, Clone)]
pub enum BridgeError {
    /// GUI chiusa, descriptor assente o server non raggiungibile: si può
    /// continuare in locale senza aver modificato nulla.
    Unavailable(String),
    /// Il server ha risposto ma il token non è valido.
    Unauthorized(String),
    /// La richiesta è arrivata e il server ha restituito un errore reale.
    Remote(String),
}

#[derive(Debug, Clone)]
pub struct BridgeClient {
    descriptor: BridgeDescriptor,
}

impl BridgeClient {
    /// Scopre un bridge vivo per la stessa cartella dati. Se `resources_dir`
    /// è indicata esplicitamente deve combaciare, altrimenti il client non
    /// userebbe le risorse attese.
    pub fn discover(data_dir: &Path, resources_dir: Option<&Path>) -> Option<Self> {
        if std::env::var("CAA_WRITER_NO_BRIDGE")
            .map(|value| !value.is_empty() && value != "0")
            .unwrap_or(false)
        {
            return None;
        }
        let text = fs::read_to_string(descriptor_path(data_dir)).ok()?;
        let descriptor: BridgeDescriptor = serde_json::from_str(&text).ok()?;
        if descriptor.schema != SCHEMA_VERSION || descriptor.token.is_empty() {
            return None;
        }
        if let Some(expected) = resources_dir {
            let actual = Path::new(&descriptor.resources_dir);
            let same = match (fs::canonicalize(expected), fs::canonicalize(actual)) {
                (Ok(left), Ok(right)) => left == right,
                _ => expected == actual,
            };
            if !same {
                return None;
            }
        }
        let client = Self { descriptor };
        if client.health().is_ok() {
            Some(client)
        } else {
            None
        }
    }

    pub fn url(&self) -> &str {
        &self.descriptor.url
    }

    pub fn pid(&self) -> u32 {
        self.descriptor.pid
    }

    pub fn health(&self) -> Result<Value, BridgeError> {
        let (status, value) = self.request("GET", "/health", None)?;
        if status == 401 {
            return Err(BridgeError::Unauthorized(
                "token del bridge non valido".to_string(),
            ));
        }
        if status != 200 {
            return Err(BridgeError::Unavailable(format!(
                "bridge ha risposto con HTTP {status}"
            )));
        }
        Ok(value)
    }

    pub fn call(
        &self,
        method: &str,
        params: &Value,
        options: DispatchOptions,
    ) -> Result<Value, BridgeError> {
        let body = json!({
            "method": method,
            "params": params,
            "access": options.access.as_str(),
            "dry_run": options.dry_run,
        });
        let (status, value) = self.request("POST", "/call", Some(&body))?;
        if status == 401 {
            return Err(BridgeError::Unauthorized(
                value
                    .get("error")
                    .and_then(Value::as_str)
                    .unwrap_or("token del bridge non valido")
                    .to_string(),
            ));
        }
        if status != 200 {
            return Err(BridgeError::Remote(format!(
                "bridge ha risposto con HTTP {status}: {}",
                value
                    .get("error")
                    .and_then(Value::as_str)
                    .unwrap_or("errore sconosciuto")
            )));
        }
        match value.get("ok").and_then(Value::as_bool) {
            Some(true) => Ok(value.get("result").cloned().unwrap_or(Value::Null)),
            Some(false) => Err(BridgeError::Remote(
                value
                    .get("error")
                    .and_then(Value::as_str)
                    .unwrap_or("errore del bridge")
                    .to_string(),
            )),
            None => Err(BridgeError::Remote("risposta del bridge non valida".to_string())),
        }
    }

    fn request(
        &self,
        method: &str,
        path: &str,
        body: Option<&Value>,
    ) -> Result<(u16, Value), BridgeError> {
        let address = self
            .descriptor
            .socket()
            .map_err(BridgeError::Unavailable)?;
        let mut stream = TcpStream::connect_timeout(&address, Duration::from_millis(800))
            .map_err(|error| BridgeError::Unavailable(format!("bridge non raggiungibile: {error}")))?;
        stream
            .set_read_timeout(Some(Duration::from_secs(1800)))
            .map_err(|error| BridgeError::Unavailable(format!("bridge: {error}")))?;
        stream
            .set_write_timeout(Some(Duration::from_secs(30)))
            .map_err(|error| BridgeError::Unavailable(format!("bridge: {error}")))?;
        let body_bytes = body
            .map(|value| serde_json::to_vec(value))
            .transpose()
            .map_err(|error| BridgeError::Remote(format!("richiesta bridge: {error}")))?
            .unwrap_or_default();
        let request = format!(
            "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nAuthorization: Bearer {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            self.descriptor.token,
            body_bytes.len()
        );
        stream
            .write_all(request.as_bytes())
            .and_then(|_| stream.write_all(&body_bytes))
            .and_then(|_| stream.flush())
            .map_err(|error| BridgeError::Unavailable(format!("invio al bridge: {error}")))?;

        let mut response = Vec::new();
        stream
            .read_to_end(&mut response)
            .map_err(|error| BridgeError::Unavailable(format!("risposta del bridge: {error}")))?;
        let split = response
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .ok_or_else(|| BridgeError::Remote("risposta HTTP del bridge incompleta".to_string()))?;
        let head = String::from_utf8_lossy(&response[..split]);
        let status = head
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .and_then(|value| value.parse::<u16>().ok())
            .ok_or_else(|| BridgeError::Remote("stato HTTP del bridge non valido".to_string()))?;
        let bytes = &response[split + 4..];
        let value = if bytes.is_empty() {
            json!({})
        } else {
            serde_json::from_slice(bytes)
                .map_err(|error| BridgeError::Remote(format!("JSON del bridge: {error}")))?
        };
        Ok((status, value))
    }
}

/// Descrizione del bridge per `app.status` lato client. Non apre connessioni
/// se il descriptor non esiste.
pub fn client_status(data_dir: &Path, resources_dir: Option<&Path>) -> Value {
    let Some(client) = BridgeClient::discover(data_dir, resources_dir) else {
        return json!({
            "available": false,
            "transport": "http-loopback",
            "descriptor": descriptor_path(data_dir),
        });
    };
    json!({
        "available": true,
        "transport": "http-loopback",
        "pid": client.pid(),
        "url": client.url(),
        "descriptor": descriptor_path(data_dir),
    })
}

/// Esegue un metodo preferendo il bridge quando la GUI è aperta. In caso di
/// GUI chiusa o non raggiungibile usa il motore locale. Un errore restituito
/// dal bridge non viene mai ritentato in locale: la richiesta potrebbe aver
/// già modificato file e un secondo tentativo duplicherebbe l'effetto.
pub fn execute(
    engine: &crate::engine::Engine,
    bridge: Option<&BridgeClient>,
    method: &str,
    params: &Value,
    options: DispatchOptions,
) -> Result<Value, String> {
    if let Some(bridge) = bridge {
        match bridge.call(method, params, options) {
            Ok(value) => return Ok(value),
            Err(BridgeError::Unavailable(_)) | Err(BridgeError::Unauthorized(_)) => {}
            Err(BridgeError::Remote(error)) => return Err(error),
        }
    }
    dispatch_with_options(engine, method, params, options)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptor_path_is_inside_data_dir() {
        let path = descriptor_path(Path::new("/tmp/caa"));
        assert_eq!(path, Path::new("/tmp/caa/automation-bridge.json"));
    }

    #[test]
    fn token_is_random_and_hex() {
        let first = random_token();
        let second = random_token();
        assert_eq!(first.len(), 64);
        assert!(first.chars().all(|value| value.is_ascii_hexdigit()));
        assert_ne!(first, second);
    }

    #[test]
    fn stamp_changes_with_content_length() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("doc.caadoc");
        std::fs::write(&path, b"one").unwrap();
        let first = file_stamp(&path).unwrap();
        std::fs::write(&path, b"a longer body").unwrap();
        let second = file_stamp(&path).unwrap();
        assert_ne!(first, second);
        std::fs::remove_file(&path).unwrap();
        assert!(file_stamp(&path).is_none());
    }

    #[test]
    fn library_scan_finds_manifest_and_documents() {
        let dir = tempfile::tempdir().unwrap();
        let library = dir.path().join("library");
        std::fs::create_dir_all(library.join("templates")).unwrap();
        std::fs::write(library.join("manifest.json"), b"{}").unwrap();
        std::fs::write(library.join("templates/doc.caadoc"), b"zip").unwrap();
        let mut seen = HashMap::new();
        scan_library(&library, &mut seen);
        assert!(seen.contains_key(&library.join("manifest.json")));
        assert!(seen.contains_key(&library.join("templates/doc.caadoc")));
    }

    #[test]
    fn changed_paths_reports_rename_source_and_target() {
        let changes = changed_paths(
            "project.rename",
            &json!({"path": "a.caadoc", "target": "b.caadoc"}),
            &json!({"source": "/tmp/a.caadoc", "path": "/tmp/b.caadoc"}),
        );
        assert_eq!(changes.len(), 2);
        assert!(changes[0].1);
        assert!(!changes[1].1);
    }

    #[test]
    fn remote_errors_are_not_retried_locally() {
        assert!(matches!(
            BridgeError::Remote("x".to_string()),
            BridgeError::Remote(_)
        ));
    }

    #[test]
    fn scan_projects_finds_caadoc_recursively() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("projects");
        std::fs::create_dir_all(root.join("sub")).unwrap();
        std::fs::write(root.join("a.caadoc"), b"x").unwrap();
        std::fs::write(root.join("sub").join("b.caadoc"), b"y").unwrap();
        std::fs::write(root.join("ignore.txt"), b"z").unwrap();
        let mut known = HashMap::new();
        scan_projects(&root, &mut known);
        assert_eq!(known.len(), 2);
    }
}
