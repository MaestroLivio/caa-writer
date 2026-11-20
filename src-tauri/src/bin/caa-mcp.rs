//! Server MCP (Model Context Protocol) di CAA Writer.
//!
//! Trasporto stdio con messaggi JSON-RPC delimitati da riga (standard MCP).
//! Accetta anche il vecchio framing `Content-Length` in ingresso.
//!
//! Il server non esegue shell e non espone percorsi arbitrari tramite i tool
//! `project.*`: ogni operazione passa dal dispatch condiviso con la CLI, quindi
//! politica di accesso, conferme e dry-run sono identici.

use std::io::{BufRead, Write};

use clap::Parser;
use serde_json::{json, Value};

use caa_writer_lib::automation::{self, AccessMode, DispatchOptions, API_VERSION};

const PROTOCOL_VERSION: &str = "2024-11-05";

#[derive(Parser)]
#[command(
    name = "caa-mcp",
    version,
    about = "Server MCP stdio per CAA Writer (stesse operazioni della CLI)"
)]
struct Cli {
    /// Cartella dati (default: quella dell'app, oppure CAA_WRITER_DATA_DIR).
    #[arg(long)]
    data_dir: Option<std::path::PathBuf>,
    /// Cartella risorse con dictionaries/, symbol-packs/, templates/.
    #[arg(long)]
    resources: Option<std::path::PathBuf>,
    /// Modalità di accesso: read, write o admin (default: write).
    #[arg(long, default_value = "write")]
    access: String,
    /// Trasporto: stdio o http.
    #[arg(long, default_value = "stdio")]
    transport: String,
    /// Indirizzo di ascolto per il trasporto HTTP (solo loopback consigliato).
    #[arg(long, default_value = "127.0.0.1:8765")]
    bind: String,
    /// Token opzionale per il trasporto HTTP. Se assente su loopback il server
    /// resta aperto solo localmente; su un indirizzo non loopback è obbligatorio.
    #[arg(long)]
    token: Option<String>,
    /// Non inoltrare le chiamate al bridge della GUI, anche se aperta.
    #[arg(long)]
    no_bridge: bool,
}

struct Server {
    engine: caa_writer_lib::engine::Engine,
    options: caa_writer_lib::automation::DispatchOptions,
    bridge: Option<caa_writer_lib::automation::bridge::BridgeClient>,
    session_id: String,
    http_token: Option<String>,
}

fn main() {
    caa_writer_lib::dev_library::configure_from_process();
    let cli = Cli::parse();
    let access = match AccessMode::parse(&cli.access) {
        Ok(access) => access,
        Err(error) => {
            eprintln!("{}", json!({"ok": false, "error": error}));
            std::process::exit(2);
        }
    };
    if cli.transport != "stdio" && cli.transport != "http" {
        eprintln!(
            "{}",
            json!({"ok": false, "error": format!("trasporto non supportato: {} (usa stdio o http)", cli.transport)})
        );
        std::process::exit(2);
    }
    if cli.transport == "http" {
        let bind = cli.bind.trim();
        let loopback = bind.starts_with("127.")
            || bind.starts_with("localhost")
            || bind.starts_with("[::1]")
            || bind.starts_with("::1");
        if !loopback && cli.token.is_none() {
            eprintln!(
                "{}",
                json!({"ok": false, "error": "per esporre caa-mcp su un indirizzo non loopback serve --token"})
            );
            std::process::exit(2);
        }
    }
    let engine = match automation::bootstrap::build_engine(cli.data_dir.clone(), cli.resources.clone()) {
        Ok(engine) => engine,
        Err(error) => {
            eprintln!("{}", json!({"ok": false, "error": error}));
            std::process::exit(1);
        }
    };
    let bridge = if cli.no_bridge {
        None
    } else {
        automation::bridge::BridgeClient::discover(&engine.paths.data_dir, cli.resources.as_deref())
    };
    let session_id = format!(
        "caa-mcp-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|value| value.as_millis())
            .unwrap_or(0)
    );
    let server = std::sync::Arc::new(Server {
        engine,
        options: DispatchOptions {
            access,
            dry_run: false,
        },
        bridge,
        session_id,
        http_token: cli.token.clone(),
    });
    let result = if cli.transport == "http" {
        let http_server = server.clone();
        http_server.run_http(&cli.bind)
    } else {
        server.run_stdio()
    };
    if let Err(error) = result {
        eprintln!("{}", json!({"ok": false, "error": error}));
        std::process::exit(1);
    }
    server.engine.shutdown();
}

impl Server {
    fn run_stdio(&self) -> Result<(), String> {
        let stdin = std::io::stdin();
        let mut reader = stdin.lock();
        let mut stdout = std::io::stdout();
        loop {
            let Some(message) = read_message(&mut reader)? else {
                break;
            };
            let value: Value = match serde_json::from_str(&message) {
                Ok(value) => value,
                Err(error) => {
                    let response = error_response(Value::Null, -32700, &format!("JSON non valido: {error}"));
                    write_message(&mut stdout, &response)?;
                    continue;
                }
            };
            let id = value.get("id").cloned();
            let method = value.get("method").and_then(|value| value.as_str()).unwrap_or_default();
            let params = value.get("params").cloned().unwrap_or_else(|| json!({}));
            // Le notifiche non hanno id e non ricevono risposta.
            if id.is_none() {
                if method == "notifications/initialized" || method == "initialized" {
                    continue;
                }
                continue;
            }
            let id = id.unwrap_or(Value::Null);
            let response = match self.handle(method, &params) {
                Ok(result) => json!({"jsonrpc": "2.0", "id": id, "result": result}),
                Err(RpcError::Method(message)) => error_response(id, -32601, &message),
                Err(RpcError::Params(message)) => error_response(id, -32602, &message),
                Err(RpcError::Internal(message)) => error_response(id, -32603, &message),
            };
            write_message(&mut stdout, &response)?;
        }
        Ok(())
    }

    fn handle(&self, method: &str, params: &Value) -> Result<Value, RpcError> {
        match method {
            "initialize" => Ok(json!({
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": {
                    "tools": {"listChanged": false},
                    "resources": {"subscribe": false, "listChanged": false},
                    "prompts": {"listChanged": false}
                },
                "serverInfo": {
                    "name": "caa-writer",
                    "title": "CAA Writer",
                    "version": env!("CARGO_PKG_VERSION")
                },
                "instructions": instructions(self.options.access)
            })),
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({"tools": automation::mcp_tools()})),
            "tools/call" => self.call_tool(params),
            "resources/list" => Ok(json!({
                "resources": [
                    {
                        "uri": "caa://capabilities",
                        "name": "Catalogo delle capacità",
                        "description": "Operazioni, schemi e modalità di accesso generate dal registro reale.",
                        "mimeType": "application/json"
                    },
                    {
                        "uri": "caa://status",
                        "name": "Stato dell'applicazione",
                        "description": "Database, percorsi, modello locale, avvisi.",
                        "mimeType": "application/json"
                    },
                    {
                        "uri": "caa://instructions",
                        "name": "Istruzioni per l'agente",
                        "description": "Regole operative, sicurezza e limiti dichiarati.",
                        "mimeType": "text/markdown"
                    }
                ]
            })),
            "resources/read" => self.read_resource(params),
            "prompts/list" => Ok(json!({"prompts": prompts()})),
            "prompts/get" => self.get_prompt(params),
            other => Err(RpcError::Method(format!("metodo MCP non supportato: {other}"))),
        }
    }

    fn dispatch(&self, method: &str, params: &Value, options: DispatchOptions) -> Result<Value, String> {
        automation::bridge::execute(&self.engine, self.bridge.as_ref(), method, params, options)
    }

    fn run_http(self: std::sync::Arc<Self>, bind: &str) -> Result<(), String> {
        use std::net::TcpListener;
        let listener = TcpListener::bind(bind)
            .map_err(|error| format!("caa-mcp http: impossibile ascoltare su {bind}: {error}"))?;
        listener
            .set_nonblocking(true)
            .map_err(|error| format!("caa-mcp http: {error}"))?;
        loop {
            match listener.accept() {
                Ok((stream, _)) => {
                    let server = self.clone();
                    let token = self.http_token.clone();
                    let session = self.session_id.clone();
                    std::thread::Builder::new()
                        .name("caa-mcp-http-request".to_string())
                        .spawn(move || {
                            if let Err(error) = handle_http_connection(&server, token.as_deref(), &session, stream) {
                                eprintln!("{}", json!({"ok": false, "error": error}));
                            }
                        })
                        .ok();
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(std::time::Duration::from_millis(40));
                }
                Err(error) => return Err(format!("caa-mcp http: {error}")),
            }
        }
    }

    fn call_tool(&self, params: &Value) -> Result<Value, RpcError> {
        let name = params
            .get("name")
            .and_then(|value| value.as_str())
            .ok_or_else(|| RpcError::Params("serve il nome del tool".to_string()))?;
        let mut arguments = params
            .get("arguments")
            .cloned()
            .unwrap_or_else(|| json!({}));
        let mut options = self.options;
        if let Some(dry_run) = arguments.get("dry_run").and_then(|value| value.as_bool()) {
            options.dry_run = dry_run;
            if let Some(object) = arguments.as_object_mut() {
                object.remove("dry_run");
            }
        }
        let method = automation::mcp_method_for_tool(name)
            .ok_or_else(|| RpcError::Params(format!("tool sconosciuto: {name}")))?;
        let outcome = self.dispatch(
            method,
            &arguments,
            options,
        );
        match outcome {
            Ok(value) => Ok(json!({
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string_pretty(&value).unwrap_or_else(|_| value.to_string())
                }],
                "structuredContent": value,
                "isError": false
            })),
            Err(error) => Ok(json!({
                "content": [{"type": "text", "text": error}],
                "isError": true
            })),
        }
    }

    fn read_resource(&self, params: &Value) -> Result<Value, RpcError> {
        let uri = params
            .get("uri")
            .and_then(|value| value.as_str())
            .ok_or_else(|| RpcError::Params("serve uri".to_string()))?;
        match uri {
            "caa://capabilities" => {
                let value = self.dispatch(
                    "app.capabilities",
                    &json!({}),
                    self.options,
                )
                .map_err(RpcError::Internal)?;
                Ok(resource(uri, "application/json", &serde_json::to_string_pretty(&value).unwrap_or_default()))
            }
            "caa://status" => {
                let value = self.dispatch(
                    "app.status",
                    &json!({}),
                    self.options,
                )
                .map_err(RpcError::Internal)?;
                Ok(resource(uri, "application/json", &serde_json::to_string_pretty(&value).unwrap_or_default()))
            }
            "caa://instructions" => Ok(resource(uri, "text/markdown", &instructions(self.options.access))),
            other => Err(RpcError::Params(format!("risorsa sconosciuta: {other}"))),
        }
    }

    fn get_prompt(&self, params: &Value) -> Result<Value, RpcError> {
        let name = params
            .get("name")
            .and_then(|value| value.as_str())
            .ok_or_else(|| RpcError::Params("serve il nome del prompt".to_string()))?;
        let arguments = params.get("arguments").cloned().unwrap_or_else(|| json!({}));
        let text = prompt_text(name, &arguments)?;
        Ok(json!({
            "description": format!("Prompt CAA Writer: {name}"),
            "messages": [{
                "role": "user",
                "content": {"type": "text", "text": text}
            }]
        }))
    }
}

enum RpcError {
    Method(String),
    Params(String),
    Internal(String),
}

fn resource(uri: &str, mime: &str, text: &str) -> Value {
    json!({
        "contents": [{
            "uri": uri,
            "mimeType": mime,
            "text": text
        }]
    })
}

fn instructions(access: AccessMode) -> String {
    format!(
        "# CAA Writer MCP\n\n\
         Versione API {API_VERSION}. Modalità di accesso corrente: **{}**.\n\n\
         1. Chiama `app_get_capabilities` prima di pianificare un workflow: schemi e limiti sono generati dal registro reale.\n\
         2. Preferisci i tool `project_*` per i file: accettano solo percorsi relativi alla radice progetti e rifiutano `..` e link che escono dalla radice.\n\
         3. Le operazioni distruttive richiedono `confirm: true` e modalità `admin`; senza conferma non toccano nulla.\n\
         4. Per vedere cosa farebbe una scrittura senza eseguirla, aggiungi `dry_run: true` agli argomenti del tool; per evitare del tutto le scritture avvia il server con `--access read`.\n\
         5. Le operazioni di rete e quelle pesanti dichiarano l'errore reale: non inventare risultati. Per lavori lunghi usa `job_start` e interroga `job_get_status`.\n\
         6. Il testo originale e le scelte manuali dell'utente non vanno mai sovrascritti.\n\
         7. Non esiste alcun tool di esecuzione shell.\n",
        access.as_str()
    )
}

fn prompts() -> Value {
    json!([
        {
            "name": "create_material",
            "title": "Crea un materiale CAA",
            "description": "Prepara un piano per un materiale (tabella, agenda, token board) e salvalo come progetto.",
            "arguments": [
                {"name": "request", "description": "Richiesta in italiano", "required": true},
                {"name": "template_id", "description": "Modello facoltativo", "required": false}
            ]
        },
        {
            "name": "edit_document",
            "title": "Modifica un documento",
            "description": "Interpreta una richiesta di modifica e applica comandi universali a un progetto.",
            "arguments": [
                {"name": "path", "description": "Percorso relativo del progetto", "required": true},
                {"name": "request", "description": "Modifica richiesta", "required": true}
            ]
        },
        {
            "name": "find_symbol",
            "title": "Trova un simbolo",
            "description": "Cerca simboli e affinità senza inventare pittogrammi mancanti.",
            "arguments": [
                {"name": "word", "description": "Parola italiana", "required": true}
            ]
        },
        {
            "name": "analyze_text",
            "title": "Analizza un testo",
            "description": "Simbolizza un testo e riporta i pittogrammi mancanti.",
            "arguments": [
                {"name": "text", "description": "Testo da analizzare", "required": true}
            ]
        }
    ])
}

fn argument<'a>(arguments: &'a Value, key: &str) -> Result<&'a str, RpcError> {
    arguments
        .get(key)
        .and_then(|value| value.as_str())
        .ok_or_else(|| RpcError::Params(format!("serve l'argomento {key}")))
}

fn prompt_text(name: &str, arguments: &Value) -> Result<String, RpcError> {
    match name {
        "create_material" => {
            let request = argument(arguments, "request")?;
            let template = arguments.get("template_id").and_then(|value| value.as_str());
            Ok(format!(
                "Usa i tool MCP di CAA Writer per creare un materiale CAA.\n\
                 Richiesta: {request}\n{}\n\
                 Passi: chiama content_generate con kind=\"material\" e prompt, salva in un percorso relativo con project_create/project_save, poi valida con project_validate. Riporta i pittogrammi mancanti senza inventarli.",
                template.map(|value| format!("Modello: {value}")).unwrap_or_default()
            ))
        }
        "edit_document" => {
            let path = argument(arguments, "path")?;
            let request = argument(arguments, "request")?;
            Ok(format!(
                "Modifica il progetto `{path}` in CAA Writer.\nRichiesta: {request}\n\
                 Passi: project_open, poi content_transform con document_path e comandi tipizzati, quindi project_validate. Se un comando non è supportato, dichiaralo invece di simulare la modifica."
            ))
        }
        "find_symbol" => {
            let word = argument(arguments, "word")?;
            Ok(format!(
                "Cerca un simbolo CAA per «{word}».\n\
                 Passi: search_symbols e word_affinities; valuta licenza e repertorio. Se manca un pittogramma adeguato, dichiaralo e proponi alternative senza inventare simboli."
            ))
        }
        "analyze_text" => {
            let text = argument(arguments, "text")?;
            Ok(format!(
                "Analizza questo testo con CAA Writer:\n\n{text}\n\n\
                 Usa analyze_text e content_validate. Elenca i pittogrammi mancanti e non modificare il testo originale."
            ))
        }
        other => Err(RpcError::Params(format!("prompt sconosciuto: {other}"))),
    }
}

fn error_response(id: Value, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {"code": code, "message": message}
    })
}

fn write_message(stdout: &mut std::io::Stdout, value: &Value) -> Result<(), String> {
    let line = serde_json::to_string(value).map_err(|error| error.to_string())?;
    writeln!(stdout, "{line}").map_err(|error| format!("stdout: {error}"))?;
    stdout.flush().map_err(|error| format!("stdout: {error}"))?;
    Ok(())
}

/// Legge un messaggio. Supporta righe JSON e il framing `Content-Length`.
fn read_message<R: BufRead>(reader: &mut R) -> Result<Option<String>, String> {
    loop {
        let mut line = String::new();
        let read = reader
            .read_line(&mut line)
            .map_err(|error| format!("stdin: {error}"))?;
        if read == 0 {
            return Ok(None);
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.to_ascii_lowercase().starts_with("content-length:") {
            let length: usize = trimmed
                .split_once(':')
                .and_then(|(_, value)| value.trim().parse().ok())
                .ok_or_else(|| "Content-Length non valido".to_string())?;
            // Consuma le altre intestazioni fino alla riga vuota.
            loop {
                let mut header = String::new();
                let read = reader
                    .read_line(&mut header)
                    .map_err(|error| format!("stdin: {error}"))?;
                if read == 0 || header.trim().is_empty() {
                    break;
                }
            }
            let mut buffer = vec![0u8; length];
            reader
                .read_exact(&mut buffer)
                .map_err(|error| format!("stdin: {error}"))?;
            return String::from_utf8(buffer)
                .map(Some)
                .map_err(|error| format!("UTF-8 non valido: {error}"));
        }
        return Ok(Some(trimmed.to_string()));
    }
}

fn handle_http_connection(
    server: &Server,
    expected_token: Option<&str>,
    session_id: &str,
    mut stream: std::net::TcpStream,
) -> Result<(), String> {
    let request = match caa_writer_lib::automation::http::read_request(&mut stream, caa_writer_lib::automation::http::MAX_BODY) {
        Ok(Some(request)) => request,
        Ok(None) => return Ok(()),
        Err(error) => {
            return caa_writer_lib::automation::http::respond_json(
                &mut stream,
                400,
                &serde_json::json!({"jsonrpc":"2.0","id":null,"error":{"code":-32700,"message":error}}),
                &[("Mcp-Session-Id", session_id)],
            );
        }
    };

    // Token opzionale: se configurato protegge l'endpoint HTTP.
    if let Some(expected) = expected_token {
        let provided = request
            .header("authorization")
            .and_then(|value| value.strip_prefix("Bearer "))
            .or_else(|| request.header("x-caa-token"));
        if !caa_writer_lib::automation::http::token_matches(expected, provided) {
            return caa_writer_lib::automation::http::respond_json(
                &mut stream,
                401,
                &serde_json::json!({"jsonrpc":"2.0","id":null,"error":{"code":-32001,"message":"token non valido"}}),
                &[("Mcp-Session-Id", session_id)],
            );
        }
    }

    // Preflight CORS per client browser/locali.
    if request.method == "OPTIONS" {
        return caa_writer_lib::automation::http::respond_text(
            &mut stream,
            204,
            "text/plain",
            b"",
            &[
                ("Access-Control-Allow-Origin", "*"),
                ("Access-Control-Allow-Methods", "GET, POST, DELETE, OPTIONS"),
                ("Access-Control-Allow-Headers", "Content-Type, Authorization, Mcp-Session-Id, Mcp-Protocol-Version"),
                ("Mcp-Session-Id", session_id),
            ],
        );
    }

    let path = request.path.as_str();
    match (request.method.as_str(), path) {
        ("GET", "/health") => {
            caa_writer_lib::automation::http::respond_json(
                &mut stream,
                200,
                &serde_json::json!({"ok": true, "service": "caa-writer-mcp", "transport": "http"}),
                &[("Mcp-Session-Id", session_id)],
            )
        }
        ("GET", "/mcp") => {
            // Nessun canale server->client SSE in questa versione: i client
            // usano POST. Risposta esplicita invece di lasciare la connessione
            // appesa.
            caa_writer_lib::automation::http::respond_json(
                &mut stream,
                405,
                &serde_json::json!({"jsonrpc":"2.0","id":null,"error":{"code":-32000,"message":"GET non supportato: usa POST /mcp"}}),
                &[("Mcp-Session-Id", session_id), ("Allow", "POST, DELETE, OPTIONS")],
            )
        }
        ("DELETE", "/mcp") => {
            caa_writer_lib::automation::http::respond_text(
                &mut stream,
                204,
                "text/plain",
                b"",
                &[("Mcp-Session-Id", session_id)],
            )
        }
        ("POST", "/mcp") | ("POST", "/") => {
            let value: serde_json::Value = match request.json() {
                Ok(value) => value,
                Err(error) => {
                    return caa_writer_lib::automation::http::respond_json(
                        &mut stream,
                        400,
                        &serde_json::json!({"jsonrpc":"2.0","id":null,"error":{"code":-32700,"message":error}}),
                        &[("Mcp-Session-Id", session_id)],
                    );
                }
            };
            let is_batch = value.is_array();
            let responses = if is_batch {
                let mut out = Vec::new();
                if let Some(items) = value.as_array() {
                    for item in items {
                        if let Some(response) = handle_one_jsonrpc(server, item) {
                            out.push(response);
                        }
                    }
                }
                serde_json::Value::Array(out)
            } else if let Some(response) = handle_one_jsonrpc(server, &value) {
                response
            } else {
                // Notifica senza id: nessuna risposta.
                return Ok(());
            };
            caa_writer_lib::automation::http::respond_json(
                &mut stream,
                200,
                &responses,
                &[
                    ("Mcp-Session-Id", session_id),
                    ("Content-Type", "application/json"),
                ],
            )
        }
        _ => {
            caa_writer_lib::automation::http::respond_json(
                &mut stream,
                404,
                &serde_json::json!({"jsonrpc":"2.0","id":null,"error":{"code":-32000,"message":"percorso non trovato"}}),
                &[("Mcp-Session-Id", session_id)],
            )
        }
    }
}

fn handle_one_jsonrpc(server: &Server, value: &serde_json::Value) -> Option<serde_json::Value> {
    let id = value.get("id").cloned();
    let method = value.get("method").and_then(|value| value.as_str()).unwrap_or_default();
    let params = value.get("params").cloned().unwrap_or_else(|| serde_json::json!({}));
    if id.is_none() {
        // Notifica: nessuna risposta.
        if method == "notifications/initialized" || method == "initialized" {
            return None;
        }
        return None;
    }
    let id = id.unwrap_or(serde_json::Value::Null);
    let response = match server.handle(method, &params) {
        Ok(result) => serde_json::json!({"jsonrpc":"2.0","id":id,"result":result}),
        Err(RpcError::Method(message)) => serde_json::json!({"jsonrpc":"2.0","id":id,"error":{"code":-32601,"message":message}}),
        Err(RpcError::Params(message)) => serde_json::json!({"jsonrpc":"2.0","id":id,"error":{"code":-32602,"message":message}}),
        Err(RpcError::Internal(message)) => serde_json::json!({"jsonrpc":"2.0","id":id,"error":{"code":-32603,"message":message}}),
    };
    Some(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn reads_newline_delimited_messages() {
        let mut cursor = Cursor::new(b"{\"id\":1}\n{\"id\":2}\n");
        assert_eq!(read_message(&mut cursor).unwrap().as_deref(), Some("{\"id\":1}"));
        assert_eq!(read_message(&mut cursor).unwrap().as_deref(), Some("{\"id\":2}"));
        assert!(read_message(&mut cursor).unwrap().is_none());
    }

    #[test]
    fn reads_content_length_frames() {
        let body = "{\"id\":7}";
        let frame = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
        let mut cursor = Cursor::new(frame.into_bytes());
        assert_eq!(read_message(&mut cursor).unwrap().as_deref(), Some(body));
    }

    #[test]
    fn prompt_requires_arguments() {
        assert!(prompt_text("find_symbol", &json!({})).is_err());
        assert!(prompt_text("find_symbol", &json!({"word": "cane"})).is_ok());
    }
}
