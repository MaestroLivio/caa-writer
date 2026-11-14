//! Minimo server HTTP condiviso dal bridge della GUI e dal trasporto HTTP di
//! `caa-mcp`.
//!
//! Non è un framework: gestisce una richiesta per connessione, con
//! `Connection: close`. Basta per un servizio locale su `127.0.0.1`, senza
//! aggiungere dipendenze e senza esporre un server generico.

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use serde_json::Value;

/// Corpo massimo accettato: i documenti `.caadoc` possono contenere asset
/// incorporati, ma un client locale non deve poter esaurire la memoria.
pub const MAX_BODY: usize = 64 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: String,
    pub path: String,
    pub query: BTreeMap<String, String>,
    pub headers: BTreeMap<String, String>,
    pub body: Vec<u8>,
}

impl HttpRequest {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(&name.to_ascii_lowercase()).map(String::as_str)
    }

    pub fn json(&self) -> Result<Value, String> {
        if self.body.is_empty() {
            return Ok(Value::Object(Default::default()));
        }
        serde_json::from_slice(&self.body).map_err(|error| format!("JSON non valido: {error}"))
    }
}

/// Legge una richiesta HTTP/1.1 da un socket. Restituisce `None` se il client
/// chiude prima di inviare qualcosa.
pub fn read_request(stream: &mut TcpStream, max_body: usize) -> Result<Option<HttpRequest>, String> {
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .map_err(|error| format!("timeout di lettura: {error}"))?;
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 8192];
    let header_end = loop {
        if let Some(position) = find_header_end(&buffer) {
            break position;
        }
        if buffer.len() > 128 * 1024 {
            return Err("intestazioni HTTP troppo grandi".to_string());
        }
        let read = stream
            .read(&mut chunk)
            .map_err(|error| format!("lettura HTTP: {error}"))?;
        if read == 0 {
            if buffer.is_empty() {
                return Ok(None);
            }
            return Err("richiesta HTTP incompleta".to_string());
        }
        buffer.extend_from_slice(&chunk[..read]);
    };

    let head = String::from_utf8_lossy(&buffer[..header_end]).to_string();
    let mut lines = head.split("\r\n");
    let request_line = lines.next().unwrap_or_default();
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_ascii_uppercase();
    let target = parts.next().unwrap_or_default().to_string();
    if method.is_empty() || target.is_empty() {
        return Err("riga di richiesta HTTP non valida".to_string());
    }
    let (path, query) = split_target(&target);
    let mut headers = BTreeMap::new();
    for line in lines {
        if line.is_empty() {
            continue;
        }
        if let Some((name, value)) = line.split_once(':') {
            headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_string());
        }
    }
    let length = headers
        .get("content-length")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    if length > max_body {
        return Err(format!(
            "corpo HTTP troppo grande: {length} byte (limite {max_body})"
        ));
    }
    let body_start = header_end + 4;
    let mut body = if buffer.len() > body_start {
        buffer[body_start..].to_vec()
    } else {
        Vec::new()
    };
    if body.len() > length {
        body.truncate(length);
    }
    while body.len() < length {
        let mut rest = vec![0u8; length - body.len()];
        let read = stream
            .read(&mut rest)
            .map_err(|error| format!("lettura corpo HTTP: {error}"))?;
        if read == 0 {
            return Err("corpo HTTP incompleto".to_string());
        }
        body.extend_from_slice(&rest[..read]);
    }
    Ok(Some(HttpRequest {
        method,
        path,
        query,
        headers,
        body,
    }))
}

fn find_header_end(buffer: &[u8]) -> Option<usize> {
    buffer
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
}

fn split_target(target: &str) -> (String, BTreeMap<String, String>) {
    let (path, query) = match target.split_once('?') {
        Some((path, query)) => (path, query),
        None => (target, ""),
    };
    let mut values = BTreeMap::new();
    for pair in query.split('&').filter(|pair| !pair.is_empty()) {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        values.insert(percent_decode(key), percent_decode(value));
    }
    (path.to_string(), values)
}

fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'%' if index + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[index + 1..index + 3]).unwrap_or("");
                if let Ok(byte) = u8::from_str_radix(hex, 16) {
                    out.push(byte);
                    index += 3;
                    continue;
                }
                out.push(bytes[index]);
                index += 1;
            }
            b'+' => {
                out.push(b' ');
                index += 1;
            }
            byte => {
                out.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).to_string()
}

/// Scrive una risposta JSON e chiude la connessione.
pub fn respond_json(
    stream: &mut TcpStream,
    status: u16,
    value: &Value,
    extra_headers: &[(&str, &str)],
) -> Result<(), String> {
    let body = serde_json::to_vec(value).map_err(|error| error.to_string())?;
    respond(stream, status, "application/json; charset=utf-8", &body, extra_headers)
}

/// Scrive una risposta testuale e chiude la connessione.
pub fn respond_text(
    stream: &mut TcpStream,
    status: u16,
    content_type: &str,
    body: &[u8],
    extra_headers: &[(&str, &str)],
) -> Result<(), String> {
    respond(stream, status, content_type, body, extra_headers)
}

fn respond(
    stream: &mut TcpStream,
    status: u16,
    content_type: &str,
    body: &[u8],
    extra_headers: &[(&str, &str)],
) -> Result<(), String> {
    let reason = match status {
        200 => "OK",
        202 => "Accepted",
        204 => "No Content",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        413 => "Payload Too Large",
        500 => "Internal Server Error",
        _ => "Response",
    };
    let mut head = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\n",
        body.len()
    );
    for (name, value) in extra_headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str("\r\n");
    stream
        .write_all(head.as_bytes())
        .and_then(|_| stream.write_all(body))
        .and_then(|_| stream.flush())
        .map_err(|error| format!("risposta HTTP: {error}"))
}

/// Confronto a tempo costante per i token locali.
pub fn token_matches(expected: &str, provided: Option<&str>) -> bool {
    let Some(provided) = provided else {
        return false;
    };
    let expected = expected.as_bytes();
    let provided = provided.as_bytes();
    if expected.len() != provided.len() {
        return false;
    }
    let mut difference = 0u8;
    for (left, right) in expected.iter().zip(provided.iter()) {
        difference |= left ^ right;
    }
    difference == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::thread;

    #[test]
    fn decodes_query_and_percent_escapes() {
        let (path, query) = split_target("/call?name=CAA%20Writer&x=1+2");
        assert_eq!(path, "/call");
        assert_eq!(query.get("name").map(String::as_str), Some("CAA Writer"));
        assert_eq!(query.get("x").map(String::as_str), Some("1 2"));
    }

    #[test]
    fn compares_tokens_without_prefix_shortcut() {
        assert!(token_matches("abc", Some("abc")));
        assert!(!token_matches("abc", Some("abd")));
        assert!(!token_matches("abc", None));
    }

    #[test]
    fn reads_a_real_http_request() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        thread::spawn(move || {
            let mut client = std::net::TcpStream::connect(address).unwrap();
            client
                .write_all(b"POST /call?x=1 HTTP/1.1\r\nHost: localhost\r\nContent-Length: 12\r\n\r\n{\"method\":1}")
                .unwrap();
        });
        let (mut stream, _) = listener.accept().unwrap();
        let request = read_request(&mut stream, MAX_BODY).unwrap().unwrap();
        assert_eq!(request.method, "POST");
        assert_eq!(request.path, "/call");
        assert_eq!(request.query.get("x").map(String::as_str), Some("1"));
        assert_eq!(request.header("host"), Some("localhost"));
        assert_eq!(request.json().unwrap()["method"], 1);
    }
}
