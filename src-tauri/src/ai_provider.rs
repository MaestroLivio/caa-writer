//! Provider IA opzionali.
//!
//! Regole:
//! - l'app funziona offline e il modello locale resta il default;
//! - nessuna richiesta online senza configurazione esplicita e consenso;
//! - la chiave API non viene mai salvata in `.caadoc`, log, errori o database:
//!   va nel portachiavi del sistema operativo;
//! - il provider online è compatibile con l'interfaccia OpenAI chat
//!   (`/chat/completions`) e non viene usato per inviare automaticamente
//!   documenti o dati personali.

use serde::{Deserialize, Serialize};

use crate::database::Database;

const SERVICE: &str = "caa-writer-ai";
const SETTING: &str = "ai.provider";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// `local` | `openai-compatible`
    #[serde(default = "default_kind")]
    pub kind: String,
    #[serde(default)]
    pub endpoint: String,
    #[serde(default)]
    pub model: String,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
    #[serde(default = "default_tokens")]
    pub max_tokens: u32,
    /// Consenso esplicito all'invio online.
    #[serde(default)]
    pub allow_online: bool,
    /// Ultima conferma privacy mostrata all'utente.
    #[serde(default)]
    pub privacy_ack: bool,
}

fn default_kind() -> String {
    "local".to_string()
}
fn default_timeout() -> u64 {
    20_000
}
fn default_tokens() -> u32 {
    512
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            kind: "local".to_string(),
            endpoint: String::new(),
            model: String::new(),
            timeout_ms: 20_000,
            max_tokens: 512,
            allow_online: false,
            privacy_ack: false,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderStatus {
    pub config: ProviderConfig,
    pub has_key: bool,
    pub key_storage: String,
    pub usable: bool,
    pub note: String,
}

fn entry() -> Result<keyring::Entry, String> {
    keyring::Entry::new(SERVICE, "provider").map_err(|error| format!("portachiavi: {error}"))
}

pub fn load(db: &Database) -> ProviderConfig {
    db.setting(SETTING)
        .and_then(|raw| serde_json::from_str::<ProviderConfig>(&raw).ok())
        .unwrap_or_default()
}

pub fn save(db: &Database, config: &ProviderConfig) -> Result<(), String> {
    if config.kind != "local" && config.kind != "openai-compatible" {
        return Err("provider non supportato".to_string());
    }
    let endpoint = config.endpoint.trim();
    if config.kind == "openai-compatible" {
        if endpoint.is_empty() {
            return Err("endpoint del provider mancante".to_string());
        }
        if !(endpoint.starts_with("https://")
            || endpoint.starts_with("http://127.0.0.1")
            || endpoint.starts_with("http://localhost"))
        {
            return Err("l'endpoint online deve usare HTTPS (o localhost)".to_string());
        }
        if !config.allow_online || !config.privacy_ack {
            return Err(
                "per usare un provider online servono consenso e conferma privacy".to_string(),
            );
        }
    }
    db.set_setting(
        SETTING,
        &serde_json::to_string(config).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}

pub fn status(db: &Database) -> ProviderStatus {
    let config = load(db);
    let has_key = entry()
        .and_then(|entry| entry.get_password().map_err(|e| e.to_string()))
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false);
    let usable = config.kind == "local"
        || (config.kind == "openai-compatible"
            && has_key
            && config.allow_online
            && config.privacy_ack
            && !config.endpoint.trim().is_empty());
    ProviderStatus {
        note: if config.kind == "local" {
            "Provider locale: nessun dato lascia il computer.".to_string()
        } else if usable {
            "Provider online configurato. Le richieste partono solo per azioni esplicite."
                .to_string()
        } else {
            "Provider online non utilizzabile: manca chiave, consenso o endpoint.".to_string()
        },
        key_storage: "Portachiavi del sistema operativo".to_string(),
        has_key,
        usable,
        config,
    }
}

pub fn set_key(key: &str) -> Result<(), String> {
    let key = key.trim();
    if key.is_empty() {
        return Err("chiave vuota".to_string());
    }
    entry()?
        .set_password(key)
        .map_err(|error| format!("salvataggio nel portachiavi non riuscito: {error}"))
}

pub fn clear_key() -> Result<(), String> {
    match entry()?.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(error) => Err(format!("rimozione dal portachiavi: {error}")),
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderTest {
    pub ok: bool,
    pub status: u16,
    pub elapsed_ms: u64,
    pub reply: String,
    pub note: String,
}

/// Test esplicito della connessione. Non viene mai chiamato automaticamente.
pub fn test(db: &Database) -> Result<ProviderTest, String> {
    let config = load(db);
    if config.kind == "local" {
        return Ok(ProviderTest {
            ok: true,
            status: 0,
            elapsed_ms: 0,
            reply: String::new(),
            note: "Provider locale attivo; nessuna connessione aperta.".to_string(),
        });
    }
    if !config.allow_online || !config.privacy_ack {
        return Err("consenso online o conferma privacy mancanti".to_string());
    }
    let key = entry()?
        .get_password()
        .map_err(|error| format!("chiave non leggibile dal portachiavi: {error}"))?;
    let endpoint = endpoint_url(&config.endpoint);
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_millis(config.timeout_ms.clamp(1000, 120_000)))
        .build()
        .map_err(|e| format!("client HTTP: {e}"))?;
    let started = std::time::Instant::now();
    let response = client
        .post(&endpoint)
        .bearer_auth(key)
        .json(&serde_json::json!({
            "model": config.model,
            "messages": [
                {"role": "system", "content": "Rispondi solo con la parola OK."},
                {"role": "user", "content": "Test di connessione."}
            ],
            "max_tokens": 8,
            "temperature": 0.0
        }))
        .send()
        .map_err(|error| format!("connessione al provider non riuscita: {error}"))?;
    let status = response.status().as_u16();
    let text = response.text().unwrap_or_default();
    let reply = serde_json::from_str::<serde_json::Value>(&text)
        .ok()
        .and_then(|value| {
            value
                .get("choices")?
                .get(0)?
                .get("message")?
                .get("content")?
                .as_str()
                .map(str::to_string)
        })
        .unwrap_or_default();
    Ok(ProviderTest {
        ok: (200..300).contains(&status),
        status,
        elapsed_ms: started.elapsed().as_millis() as u64,
        reply,
        note: "Test esplicito; la chiave non è stata registrata nei log.".to_string(),
    })
}

fn endpoint_url(endpoint: &str) -> String {
    let trimmed = endpoint.trim().trim_end_matches('/');
    if trimmed.ends_with("/chat/completions") {
        trimmed.to_string()
    } else {
        format!("{trimmed}/chat/completions")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_gets_chat_path_once() {
        assert_eq!(
            endpoint_url("https://example.test/v1"),
            "https://example.test/v1/chat/completions"
        );
        assert_eq!(
            endpoint_url("https://example.test/v1/chat/completions"),
            "https://example.test/v1/chat/completions"
        );
    }

    #[test]
    fn config_defaults_to_local_offline() {
        let config = ProviderConfig::default();
        assert_eq!(config.kind, "local");
        assert!(!config.allow_online);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChatReply {
    pub reply: String,
    pub backend: String,
    pub elapsed_ms: u64,
}

/// Chat con il provider online compatibile OpenAI. Non viene chiamata
/// automaticamente: serve un consenso esplicito e una chiave nel portachiavi.
pub fn chat(db: &Database, messages: &[ChatMessage]) -> Result<ChatReply, String> {
    let config = load(db);
    if config.kind == "local" {
        return Err("provider online non configurato".to_string());
    }
    if !config.allow_online || !config.privacy_ack {
        return Err("consenso online o conferma privacy mancanti".to_string());
    }
    if messages.is_empty() {
        return Err("nessun messaggio da inviare".to_string());
    }
    let key = entry()?
        .get_password()
        .map_err(|error| format!("chiave non leggibile dal portachiavi: {error}"))?;
    let endpoint = endpoint_url(&config.endpoint);
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_millis(
            config.timeout_ms.clamp(1000, 120_000),
        ))
        .build()
        .map_err(|e| format!("client HTTP: {e}"))?;
    let started = std::time::Instant::now();
    let response = client
        .post(&endpoint)
        .bearer_auth(key)
        .json(&serde_json::json!({
            "model": config.model,
            "messages": messages,
            "max_tokens": config.max_tokens,
            "temperature": 0.4
        }))
        .send()
        .map_err(|error| format!("provider non raggiungibile: {error}"))?;
    let status = response.status();
    let text = response.text().unwrap_or_default();
    let value = serde_json::from_str::<serde_json::Value>(&text).unwrap_or_default();
    if !status.is_success() {
        return Err(value["error"]["message"]
            .as_str()
            .map(str::to_string)
            .unwrap_or_else(|| format!("provider: stato {status}")));
    }
    let reply = value["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .trim()
        .to_string();
    if reply.is_empty() {
        return Err("il provider non ha restituito testo".to_string());
    }
    Ok(ChatReply {
        reply,
        backend: format!("online · {}", config.model),
        elapsed_ms: started.elapsed().as_millis() as u64,
    })
}
