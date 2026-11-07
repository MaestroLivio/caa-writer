//! Ricerca online federata di pittogrammi (provider intercambiabili).
//!
//! Regole di progetto:
//! - il testo completo della frase non lascia mai il dispositivo: si inviano
//!   solo lemmi/concetti già estratti localmente (massimo quattro termini);
//! - i risultati non vengono inseriti automaticamente nel documento: vanno
//!   prima salvati nella libreria personale, così il documento resta offline;
//! - ogni risultato conserva fonte, autore e licenza;
//! - provider e rete sono facoltativi e disattivati per default.

use std::path::Path;
use std::time::Duration;

use base64::Engine as _;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::database::Database;
use crate::util::now_iso;

pub const PROVIDER_WIKIMEDIA: &str = "wikimedia";
pub const PROVIDER_ARASAAC: &str = "arasaac";
const CACHE_TTL_HOURS: i64 = 24;
const MAX_DOWNLOAD_BYTES: usize = 10 * 1024 * 1024;
const MAX_TERMS: usize = 4;
/// Cambia quando cambia la semantica dei risultati: invalida la cache vecchia
/// senza cancellare il database (le righe precedenti restano inutilizzate).
const CACHE_VERSION: &str = "2";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnlineResult {
    pub provider: String,
    pub id: String,
    pub title: String,
    pub thumbnail_url: Option<String>,
    pub image_url: String,
    pub page_url: Option<String>,
    pub mime: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub author: Option<String>,
    pub license: String,
    pub license_url: Option<String>,
    pub source: String,
    pub commercial_ok: bool,
    /// `pittogramma` per i repertori CAA, `fotografia` per immagini generiche.
    #[serde(default)]
    pub kind: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct OnlineResponse {
    pub provider: String,
    pub query: String,
    pub results: Vec<OnlineResult>,
    pub cached: bool,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OnlineProviderInfo {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub note: String,
}

/// Provider aggiuntivo definito dall'utente. Deve esporre un JSON pubblico con
/// `{ "results": [ { id,title,image_url,thumbnail_url,... } ] }` oppure un
/// array diretto. La URL può contenere `{query}`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomProvider {
    pub id: String,
    pub name: String,
    pub search_url: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub note: String,
}

fn default_true() -> bool {
    true
}

pub fn custom_providers(db: &Database) -> Vec<CustomProvider> {
    db.setting("online.custom_providers")
        .and_then(|raw| serde_json::from_str::<Vec<CustomProvider>>(&raw).ok())
        .unwrap_or_default()
}

pub fn save_custom_provider(db: &Database, provider: CustomProvider) -> Result<(), String> {
    let id = provider.id.trim();
    if id.is_empty() || provider.name.trim().is_empty() || provider.search_url.trim().is_empty() {
        return Err("id, nome e URL del provider sono obbligatori".to_string());
    }
    if !provider.search_url.contains("{query}") {
        return Err("l'URL del provider deve contenere {query}".to_string());
    }
    if !provider.search_url.starts_with("https://")
        && !provider.search_url.starts_with("http://localhost")
        && !provider.search_url.starts_with("http://127.0.0.1")
    {
        return Err("il provider deve usare HTTPS (o localhost per test)".to_string());
    }
    let mut providers = custom_providers(db);
    if let Some(existing) = providers.iter_mut().find(|item| item.id == id) {
        *existing = provider;
    } else {
        providers.push(provider);
    }
    db.set_setting(
        "online.custom_providers",
        &serde_json::to_string(&providers).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}

pub fn delete_custom_provider(db: &Database, id: &str) -> Result<(), String> {
    let mut providers = custom_providers(db);
    providers.retain(|provider| provider.id != id);
    db.set_setting(
        "online.custom_providers",
        &serde_json::to_string(&providers).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}

pub fn providers_with_db(db: &Database) -> Vec<OnlineProviderInfo> {
    let mut out = providers();
    for provider in custom_providers(db) {
        out.push(OnlineProviderInfo {
            id: provider.id,
            name: provider.name,
            enabled: provider.enabled,
            note: if provider.note.is_empty() {
                format!("Provider personalizzato: {}", provider.search_url)
            } else {
                provider.note
            },
        });
    }
    out
}

fn fetch_custom(provider: &CustomProvider, query: &str) -> Result<OnlineResponse, String> {
    let url = provider
        .search_url
        .replace("{query}", &urlencoding::encode(query));
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(20))
        .user_agent("CAA-Writer/0.10 (offline-first)")
        .build()
        .map_err(|e| format!("client provider: {e}"))?;
    let response = client
        .get(&url)
        .send()
        .map_err(|e| format!("{} non raggiungibile: {e}", provider.name))?;
    if !response.status().is_success() {
        return Err(format!("{} ha risposto con stato {}", provider.name, response.status()));
    }
    let value: Value = response
        .json()
        .map_err(|e| format!("risposta non valida da {}: {e}", provider.name))?;
    let items = value
        .get("results")
        .and_then(|v| v.as_array())
        .or_else(|| value.as_array())
        .cloned()
        .unwrap_or_default();
    let mut results = Vec::new();
    for item in items.into_iter().take(24) {
        let image_url = item
            .get("image_url")
            .or_else(|| item.get("image"))
            .or_else(|| item.get("url"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if !image_url.starts_with("https://") && !image_url.starts_with("http://localhost") {
            continue;
        }
        results.push(OnlineResult {
            provider: provider.id.clone(),
            id: item
                .get("id")
                .map(|v| v.to_string().trim_matches('"').to_string())
                .unwrap_or_else(|| format!("{}", results.len() + 1)),
            title: item
                .get("title")
                .or_else(|| item.get("label"))
                .and_then(|v| v.as_str())
                .unwrap_or(query)
                .to_string(),
            thumbnail_url: item
                .get("thumbnail_url")
                .and_then(|v| v.as_str())
                .map(str::to_string),
            image_url: image_url.to_string(),
            page_url: item.get("page_url").and_then(|v| v.as_str()).map(str::to_string),
            mime: item
                .get("mime")
                .and_then(|v| v.as_str())
                .unwrap_or("image/png")
                .to_string(),
            width: item.get("width").and_then(|v| v.as_u64()).map(|v| v as u32),
            height: item.get("height").and_then(|v| v.as_u64()).map(|v| v as u32),
            author: item.get("author").and_then(|v| v.as_str()).map(str::to_string),
            license: item
                .get("license")
                .and_then(|v| v.as_str())
                .unwrap_or("licenza da verificare")
                .to_string(),
            license_url: item.get("license_url").and_then(|v| v.as_str()).map(str::to_string),
            source: provider.name.clone(),
            commercial_ok: item
                .get("commercial_ok")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            kind: item
                .get("kind")
                .and_then(|v| v.as_str())
                .unwrap_or("immagine")
                .to_string(),
        });
    }
    let empty = results.is_empty();
    Ok(OnlineResponse {
        provider: provider.id.clone(),
        query: query.to_string(),
        results,
        cached: false,
        warnings: if empty {
            vec![format!("Nessun risultato da {}.", provider.name)]
        } else {
            Vec::new()
        },
    })
}

pub fn providers() -> Vec<OnlineProviderInfo> {
    vec![
        OnlineProviderInfo {
            id: PROVIDER_ARASAAC.to_string(),
            name: "ARASAAC".to_string(),
            enabled: true,
            note: "API ufficiale ARASAAC; licenza CC BY-NC-SA 4.0, autore Sergio Palao. Uso non commerciale."
                .to_string(),
        },
        OnlineProviderInfo {
            id: PROVIDER_WIKIMEDIA.to_string(),
            name: "Wikimedia Commons".to_string(),
            enabled: true,
            note: "API ufficiale MediaWiki; licenze e autori sono letti dai metadati della file page."
                .to_string(),
        },
        OnlineProviderInfo {
            id: "openverse".to_string(),
            name: "Openverse".to_string(),
            enabled: false,
            note: "Non collegato in questa build: richiede verifica dei limiti e delle condizioni API."
                .to_string(),
        },
        OnlineProviderInfo {
            id: "global-symbols".to_string(),
            name: "Global Symbols".to_string(),
            enabled: false,
            note: "Non collegato: non è stata verificata un'API pubblica stabile.".to_string(),
        },
    ]
}

/// Riduce i termini a lemmi/concetti generici, senza email, URL, numeri o
/// frasi lunghe. È una barriera di privacy, non una semplice normalizzazione.
pub fn clean_terms(terms: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for raw in terms {
        let lower = raw.trim().to_lowercase();
        if lower.is_empty()
            || lower.contains('@')
            || lower.contains("http")
            || lower.chars().any(|c| c.is_ascii_digit())
        {
            continue;
        }
        let cleaned: String = lower
            .chars()
            .map(|c| {
                if c.is_alphabetic() || c == ' ' || c == '\'' || c == '-' {
                    c
                } else {
                    ' '
                }
            })
            .collect();
        let words: Vec<&str> = cleaned.split_whitespace().collect();
        if words.is_empty() || words.len() > 3 {
            continue;
        }
        let term = words.join(" ");
        if !out.contains(&term) {
            out.push(term);
        }
        if out.len() >= MAX_TERMS {
            break;
        }
    }
    out
}

pub fn build_query(terms: &[String]) -> String {
    clean_terms(terms).join(" ")
}

/// Filtra i termini con il lessico locale: si inviano solo parole/espressioni
/// che il motore conosce già, mai nomi propri o testo libero. È una seconda
/// barriera rispetto al chiamante.
pub fn known_terms(lexicon: &crate::linguistics::lexicon::Lexicon, terms: &[String]) -> Vec<String> {
    let normalized_labels: Vec<String> = lexicon
        .concepts()
        .map(|concept| crate::linguistics::normalize::normalize_word(&concept.label))
        .collect();
    clean_terms(terms)
        .into_iter()
        .filter(|term| {
            let normalized = crate::linguistics::normalize::normalize_word(term);
            !lexicon.lookup(term).is_empty()
                || lexicon.mwe.contains_key(&normalized)
                || normalized_labels.contains(&normalized)
        })
        .collect()
}

/// Termini da inviare per una ricerca esplicita. Prima si usano le parole
/// riconosciute dal lessico; se l'utente cerca una parola nuova, si invia solo
/// quella parola pulita. Non si invia mai la frase o il documento.
pub fn terms_for_search(
    lexicon: &crate::linguistics::lexicon::Lexicon,
    terms: &[String],
) -> Vec<String> {
    let requested = clean_terms(terms);
    let known = known_terms(lexicon, &requested);
    if !known.is_empty() {
        return known;
    }
    requested
        .into_iter()
        .take(1)
        .filter(|term| term.chars().count() <= 40)
        .collect()
}

fn cache_key(query: &str) -> String {
    format!("{CACHE_VERSION}:{query}")
}

fn cache_get(db: &Database, provider: &str, query: &str) -> Option<Vec<OnlineResult>> {
    let key = cache_key(query);
    let row: Option<(String, String)> = db
        .conn()
        .query_row(
            "SELECT payload, fetched_at FROM online_cache WHERE provider = ?1 AND query = ?2",
            rusqlite::params![provider, key],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .ok();
    let (payload, fetched_at) = row?;
    let fetched = time::OffsetDateTime::parse(
        &fetched_at,
        &time::format_description::well_known::Rfc3339,
    )
    .ok()?;
    let age = time::OffsetDateTime::now_utc() - fetched;
    if age.whole_hours() > CACHE_TTL_HOURS {
        return None;
    }
    serde_json::from_str(&payload).ok()
}

fn cache_put(db: &Database, provider: &str, query: &str, results: &[OnlineResult]) {
    let Ok(payload) = serde_json::to_string(results) else {
        return;
    };
    let key = cache_key(query);
    let _ = db.conn().execute(
        "INSERT INTO online_cache(provider, query, payload, fetched_at) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(provider, query) DO UPDATE SET payload=excluded.payload,
            fetched_at=excluded.fetched_at",
        rusqlite::params![provider, key, payload, now_iso()],
    );
}

fn strip_html(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut in_tag = false;
    for c in value.chars() {
        match c {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                out.push(' ');
            }
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn metadata_text(meta: &Value, key: &str) -> Option<String> {
    meta.get(key)
        .and_then(|value| value.get("value"))
        .and_then(|value| value.as_str())
        .map(strip_html)
        .filter(|value| !value.is_empty())
}

fn license_allows_commercial(license: &str) -> bool {
    let lower = license.to_lowercase();
    if lower.contains("nc") || lower.contains("non commercial") || lower.contains("non-commercial") {
        return false;
    }
    lower.contains("cc0")
        || lower.contains("public domain")
        || lower.contains("cc by")
        || lower.contains("pubblico dominio")
}

/// Interpreta la risposta MediaWiki. Funzione separata per poterla testare
/// con un frammento JSON senza rete.
pub fn parse_wikimedia(value: &Value, query: &str) -> OnlineResponse {
    let mut results = Vec::new();
    let mut warnings = Vec::new();
    let pages = value
        .get("query")
        .and_then(|value| value.get("pages"))
        .and_then(|value| value.as_object());
    if let Some(pages) = pages {
        for page in pages.values() {
            let Some(info) = page
                .get("imageinfo")
                .and_then(|value| value.as_array())
                .and_then(|value| value.first())
            else {
                continue;
            };
            let mime = info
                .get("mime")
                .and_then(|value| value.as_str())
                .unwrap_or("");
            if !matches!(
                mime,
                "image/png" | "image/jpeg" | "image/webp" | "image/gif" | "image/svg+xml"
            ) {
                continue;
            }
            let Some(image_url) = info.get("url").and_then(|value| value.as_str()) else {
                continue;
            };
            let width = info.get("width").and_then(|value| value.as_u64()).unwrap_or(0) as u32;
            let height = info.get("height").and_then(|value| value.as_u64()).unwrap_or(0) as u32;
            if width < 48 || height < 48 {
                continue;
            }
            let title = page
                .get("title")
                .and_then(|value| value.as_str())
                .unwrap_or("File")
                .trim_start_matches("File:")
                .to_string();
            let meta = info.get("extmetadata").cloned().unwrap_or(Value::Null);
            let license = metadata_text(&meta, "LicenseShortName")
                .unwrap_or_else(|| "Licenza da verificare".to_string());
            let author = metadata_text(&meta, "Artist")
                .or_else(|| metadata_text(&meta, "Credit"))
                .filter(|value| value.chars().count() < 160);
            let license_url = metadata_text(&meta, "LicenseUrl");
            let description = metadata_text(&meta, "ImageDescription")
                .map(|value| value.chars().take(220).collect::<String>());
            let description_ref = description.clone();
            let page_url = info
                .get("descriptionurl")
                .and_then(|value| value.as_str())
                .map(str::to_string);
            let thumbnail_url = info
                .get("thumburl")
                .and_then(|value| value.as_str())
                .map(str::to_string);
            results.push(OnlineResult {
                provider: PROVIDER_WIKIMEDIA.to_string(),
                id: page
                    .get("pageid")
                    .and_then(|value| value.as_i64())
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| image_url.to_string()),
                title: description.clone().unwrap_or_else(|| title.clone()),
                thumbnail_url,
                image_url: image_url.to_string(),
                page_url,
                mime: mime.to_string(),
                width: Some(width),
                height: Some(height),
                author,
                commercial_ok: license_allows_commercial(&license),
                license,
                license_url,
                source: "Wikimedia Commons".to_string(),
                kind: if result_is_pictogram(&title, description_ref.as_deref()) {
                    "pittogramma".to_string()
                } else {
                    "fotografia".to_string()
                },
            });
            if results.len() >= 24 {
                break;
            }
        }
    }
    if results.is_empty() {
        warnings.push(format!("Nessun risultato per «{query}»."));
    }
    OnlineResponse {
        provider: PROVIDER_WIKIMEDIA.to_string(),
        query: query.to_string(),
        results,
        cached: false,
        warnings,
    }
}

fn result_is_pictogram(title: &str, description: Option<&str>) -> bool {
    let haystack = format!("{} {}", title, description.unwrap_or("")).to_lowercase();
    ["pittogramm", "pictogram", "simbolo caa", "aac", "symbol"]
        .iter()
        .any(|needle| haystack.contains(needle))
}

fn parse_arasaac(value: &Value, query: &str) -> OnlineResponse {
    let mut results = Vec::new();
    let mut warnings = Vec::new();
    if let Some(items) = value.as_array() {
        for item in items {
            if item.get("violence").and_then(|value| value.as_bool()) == Some(true) {
                continue;
            }
            // `aac` non indica l'idoneità CAA: molti pittogrammi ARASAAC
            // validi hanno `aac: false`. Scartarli lasciava la ricerca vuota
            // per gran parte delle parole e faceva ripiegare sulle fotografie
            // di Wikimedia. Si filtra solo la violenza esplicita.
            let Some(id) = item.get("_id").and_then(|value| value.as_i64()) else {
                continue;
            };
            let keywords: Vec<String> = item
                .get("keywords")
                .and_then(|value| value.as_array())
                .map(|value| {
                    value
                        .iter()
                        .filter_map(|keyword| keyword.get("keyword").and_then(|value| value.as_str()))
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default();
            let title = keywords
                .iter()
                .find(|keyword| keyword.eq_ignore_ascii_case(query))
                .cloned()
                .or_else(|| keywords.first().cloned())
                .unwrap_or_else(|| query.to_string());
            let image_url = format!("https://static.arasaac.org/pictograms/{id}/{id}_300.png");
            results.push(OnlineResult {
                provider: PROVIDER_ARASAAC.to_string(),
                id: id.to_string(),
                title,
                // ARASAAC non espone più le miniature _100: il 300 è il
                // formato più piccolo realmente disponibile e va bene per
                // l'anteprima nel pannello.
                thumbnail_url: Some(format!(
                    "https://static.arasaac.org/pictograms/{id}/{id}_300.png"
                )),
                image_url,
                page_url: Some(format!("https://arasaac.org/pictograms/{id}")),
                mime: "image/png".to_string(),
                width: Some(300),
                height: Some(300),
                author: Some("Sergio Palao — ARASAAC".to_string()),
                license: "CC BY-NC-SA 4.0".to_string(),
                license_url: Some(
                    "https://creativecommons.org/licenses/by-nc-sa/4.0/".to_string(),
                ),
                source: "ARASAAC".to_string(),
                commercial_ok: false,
                kind: "pittogramma".to_string(),
            });
            if results.len() >= 24 {
                break;
            }
        }
    }
    if results.is_empty() {
        warnings.push(format!("Nessun pittogramma ARASAAC per «{query}»."));
    }
    OnlineResponse {
        provider: PROVIDER_ARASAAC.to_string(),
        query: query.to_string(),
        results,
        cached: false,
        warnings,
    }
}

fn fetch_arasaac(query: &str) -> Result<OnlineResponse, String> {
    // L'API ARASAAC cerca per singolo termine: si usa il primo lemma, non la
    // frase completa.
    let term = query.split_whitespace().next().unwrap_or(query);
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent("CAAWriter/0.5.1 (offline-first; ricerca su richiesta dell'utente)")
        .build()
        .map_err(|e| format!("client HTTP non inizializzabile: {e}"))?;
    let response = client
        .get(format!(
            "https://api.arasaac.org/api/pictograms/it/search/{}",
            urlencoding::encode(term)
        ))
        .send()
        .map_err(|e| format!("ARASAAC non raggiungibile: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("ARASAAC ha risposto con stato {}", response.status()));
    }
    let value: Value = response
        .json()
        .map_err(|e| format!("risposta ARASAAC non valida: {e}"))?;
    Ok(parse_arasaac(&value, query))
}

fn fetch_wikimedia(query: &str) -> Result<OnlineResponse, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent("CAAWriter/0.5.1 (offline-first; ricerca su richiesta dell'utente)")
        .build()
        .map_err(|e| format!("client HTTP non inizializzabile: {e}"))?;
    let response = client
        .get("https://commons.wikimedia.org/w/api.php")
        .query(&[
            ("action", "query".to_string()),
            ("format", "json".to_string()),
            ("generator", "search".to_string()),
            (
                "gsrsearch",
                format!("{query} filetype:bitmap|drawing"),
            ),
            ("gsrnamespace", "6".to_string()),
            ("gsrlimit", "24".to_string()),
            ("prop", "imageinfo".to_string()),
            (
                "iiprop",
                "url|mime|size|extmetadata".to_string(),
            ),
            ("iiurlwidth", "240".to_string()),
        ])
        .send()
        .map_err(|e| format!("rete non disponibile o provider irraggiungibile: {e}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "Wikimedia Commons ha risposto con stato {}",
            response.status()
        ));
    }
    let value: Value = response
        .json()
        .map_err(|e| format!("risposta non valida dal provider: {e}"))?;
    Ok(parse_wikimedia(&value, query))
}

pub fn search(
    db: &Database,
    terms: &[String],
    force_refresh: bool,
) -> Result<OnlineResponse, String> {
    let query = build_query(terms);
    if query.is_empty() {
        return Err("Nessun termine generico da cercare online.".to_string());
    }
    if !force_refresh {
        for provider in [PROVIDER_ARASAAC, PROVIDER_WIKIMEDIA] {
            if let Some(results) = cache_get(db, provider, &query) {
                if !results.is_empty() {
                    return Ok(OnlineResponse {
                        provider: provider.to_string(),
                        query,
                        results,
                        cached: true,
                        warnings: vec!["Risultati dalla cache locale (24 ore).".to_string()],
                    });
                }
            }
        }
    }
    // Prima i repertori CAA, poi le immagini generiche: un pittogramma è più
    // adatto di una fotografia, ma la fonte resta sempre dichiarata.
    match fetch_arasaac(&query) {
        Ok(response) if !response.results.is_empty() => {
            cache_put(db, PROVIDER_ARASAAC, &query, &response.results);
            return Ok(response);
        }
        Ok(_) => {}
        Err(error) => {
            // Se ARASAAC non risponde si tenta Wikimedia; l'errore resta nei
            // messaggi solo se anche il secondo provider fallisce.
            let _ = error;
        }
    }
    let response = fetch_wikimedia(&query)?;
    if !response.results.is_empty() {
        cache_put(db, PROVIDER_WIKIMEDIA, &query, &response.results);
        return Ok(response);
    }
    for provider in custom_providers(db).into_iter().filter(|provider| provider.enabled) {
        if let Ok(custom) = fetch_custom(&provider, &query) {
            if !custom.results.is_empty() {
                cache_put(db, &provider.id, &query, &custom.results);
                return Ok(custom);
            }
        }
    }
    cache_put(db, PROVIDER_WIKIMEDIA, &query, &response.results);
    Ok(response)
}

fn allowed_image_host(url: &str) -> bool {
    if url.starts_with("https://upload.wikimedia.org/")
        || url.starts_with("https://static.arasaac.org/pictograms/")
    {
        return true;
    }
    // Provider aggiuntivi espliciti: solo HTTPS pubblico, mai host locali/IP.
    if let Some(rest) = url.strip_prefix("https://") {
        let host = rest.split('/').next().unwrap_or("");
        return host.contains('.')
            && !host.starts_with("localhost")
            && !host.starts_with("127.")
            && !host.starts_with("10.")
            && !host.starts_with("192.168.")
            && !host.starts_with("172.16.");
    }
    false
}

/// Parola singola accettabile per l'inserimento automatico: niente frasi,
/// numeri, email, URL o spazi. La rete riceve solo questo termine.
fn automatic_word(raw: &str) -> Option<String> {
    let word = raw.trim().to_lowercase();
    if word.chars().count() < 2 || word.chars().count() > 40 {
        return None;
    }
    if word.contains('@') || word.contains("http") || word.chars().any(|c| c.is_ascii_digit()) {
        return None;
    }
    if !word
        .chars()
        .all(|c| c.is_alphabetic() || c == '\'' || c == '’' || c == '-')
    {
        return None;
    }
    Some(word)
}

/// Cerca su ARASAAC le parole indicate, salva i pittogrammi come copie offline
/// e restituisce quelle davvero importate. Non usa Wikimedia: durante la
/// scrittura servono pittogrammi CAA, non fotografie generiche. Le parole già
/// presenti nella libreria non vengono riscaricate.
pub fn ensure_arasaac_symbols(
    db: &Database,
    assets_dir: &Path,
    words: &[String],
    limit: usize,
) -> Result<Vec<String>, String> {
    let filters = crate::database::SymbolFilters::default();
    // Solo la copia personale evita di riscaricare un simbolo già importato
    // automaticamente. Un simbolo presente nel catalogo ARASAAC locale non
    // basta: il motore lo usa nel percorso lessicale, non per le parole fuori
    // lessico, quindi qui serve una copia personale collegabile.
    let personal = crate::database::SymbolFilters {
        pack_id: Some("personal".to_string()),
        ..Default::default()
    };
    let mut saved = Vec::new();
    for raw in words.iter().take(limit.clamp(1, 12)) {
        let Some(word) = automatic_word(raw) else {
            continue;
        };
        if db
            .symbol_by_label(&word, &personal)
            .map_err(|e| e.to_string())?
            .is_some()
        {
            // Già presente: segnalarlo permette alla rianalisi di collegarlo
            // anche se la cache precedente non lo aveva ancora visto.
            saved.push(word);
            continue;
        }
        let Ok(response) = fetch_arasaac(&word) else {
            continue;
        };
        // Solo un'etichetta esattamente uguale: un pittogramma sbagliato è
        // peggio di nessun pittogramma. Le corrispondenze vaghe restano nel
        // pannello, dove l'utente può sceglierle.
        let wanted = crate::linguistics::normalize::normalize_word(&word);
        let Some(result) = response.results.into_iter().find(|result| {
            result.kind == "pittogramma"
                && crate::linguistics::normalize::normalize_word(&result.title) == wanted
        }) else {
            continue;
        };
        if save_result(db, assets_dir, &result, &word, "ARASAAC automatico").is_ok() {
            saved.push(word);
        } else if db
            .symbol_by_label(&word, &filters)
            .ok()
            .flatten()
            .is_some()
        {
            // Il file era già nella libreria ma l'import ha segnalato un
            // duplicato: la parola è comunque collegabile.
            saved.push(word);
        }
    }
    Ok(saved)
}

pub fn download_bytes(url: &str) -> Result<Vec<u8>, String> {
    if !allowed_image_host(url) {
        return Err("URL immagine non autorizzato: serve HTTPS pubblico."
            .to_string());
    }
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(20))
        .user_agent("CAAWriter/0.5.1 (offline-first)")
        .build()
        .map_err(|e| format!("client HTTP non inizializzabile: {e}"))?;
    let mut response = client
        .get(url)
        .send()
        .map_err(|e| format!("download non riuscito: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("il provider ha risposto con stato {}", response.status()));
    }
    if let Some(length) = response.content_length() {
        if length as usize > MAX_DOWNLOAD_BYTES {
            return Err(format!(
                "immagine troppo grande: {:.1} MB (limite {} MB)",
                length as f64 / 1_048_576.0,
                MAX_DOWNLOAD_BYTES / 1_048_576
            ));
        }
    }
    let mut bytes: Vec<u8> = Vec::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        use std::io::Read;
        let read = response
            .read(&mut buffer)
            .map_err(|e| format!("lettura download: {e}"))?;
        if read == 0 {
            break;
        }
        bytes.extend_from_slice(&buffer[..read]);
        if bytes.len() > MAX_DOWNLOAD_BYTES {
            return Err("immagine oltre il limite di 10 MB".to_string());
        }
    }
    if bytes.is_empty() {
        return Err("il provider ha restituito un file vuoto".to_string());
    }
    Ok(bytes)
}

/// Scarica una miniatura e la restituisce come data URI: il WebView non deve
/// contattare direttamente il provider (CSP e privacy restano sotto controllo).
pub fn download_thumbnail(url: &str) -> Result<String, String> {
    if !allowed_image_host(url) {
        return Err("URL miniatura non autorizzato".to_string());
    }
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent("CAAWriter/0.5.1 (offline-first)")
        .build()
        .map_err(|e| e.to_string())?;
    let response = client
        .get(url)
        .send()
        .map_err(|e| format!("miniatura non scaricabile: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("miniatura: stato {}", response.status()));
    }
    let bytes = response.bytes().map_err(|e| e.to_string())?;
    if bytes.len() > 1024 * 1024 {
        return Err("miniatura oltre 1 MB".to_string());
    }
    let mime = match crate::pictogram_import::detect_format(&bytes) {
        crate::pictogram_import::DetectedFormat::Png => "image/png",
        crate::pictogram_import::DetectedFormat::Jpeg => "image/jpeg",
        crate::pictogram_import::DetectedFormat::Gif => "image/gif",
        crate::pictogram_import::DetectedFormat::Webp => "image/webp",
        crate::pictogram_import::DetectedFormat::Svg => "image/svg+xml",
        _ => return Err("formato miniatura non riconosciuto".to_string()),
    };
    let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Ok(format!("data:{mime};base64,{encoded}"))
}

fn extension_for(mime: &str) -> &'static str {
    match mime {
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/webp" => "webp",
        "image/gif" => "gif",
        "image/svg+xml" => "svg",
        _ => "img",
    }
}

/// Salva un risultato nella libreria personale (copia locale) e restituisce
/// l'identificativo del simbolo importato.
pub fn save_result(
    db: &Database,
    assets_dir: &Path,
    result: &OnlineResult,
    lemma: &str,
    collection: &str,
) -> Result<String, String> {
    let bytes = download_bytes(&result.image_url)?;
    let detected = crate::pictogram_import::detect_format(&bytes);
    if !detected.decoder_available() {
        return Err(format!(
            "formato scaricato non supportato: {}",
            detected.as_str()
        ));
    }
    let tmp_dir = assets_dir.join("tmp");
    std::fs::create_dir_all(&tmp_dir).map_err(|e| format!("cartella temporanea: {e}"))?;
    let tmp_path = tmp_dir.join(format!(
        "online-{}.{}",
        crate::util::now_iso().replace([':', '-', '.'], ""),
        extension_for(&result.mime)
    ));
    std::fs::write(&tmp_path, &bytes).map_err(|e| format!("salvataggio temporaneo: {e}"))?;
    let item = crate::pictogram_import::ImportItem {
        path: tmp_path.to_string_lossy().to_string(),
        lemma: lemma.to_string(),
        collection: Some(collection.to_string()),
        category: Some("online".to_string()),
        synonyms: Vec::new(),
        relative_path: Some(result.title.clone()),
        original_name: Some(result.title.clone()),
    };
    let license = crate::pictogram_import::ImportLicense {
        name: Some(result.license.clone()),
        url: result.license_url.clone(),
        author: result.author.clone(),
        source_url: Some(
            result
                .page_url
                .clone()
                .unwrap_or_else(|| result.image_url.clone()),
        ),
        commercial_ok: result.commercial_ok,
    };
    let report =
        crate::pictogram_import::import_batch(db, assets_dir, &[item], &license, collection);
    let _ = std::fs::remove_file(&tmp_path);
    if report.imported == 0 && report.skipped == 0 {
        return Err(report
            .warnings
            .first()
            .cloned()
            .unwrap_or_else(|| "importazione non riuscita".to_string()));
    }
    report
        .items
        .first()
        .and_then(|item| item.symbol_id.clone())
        .ok_or_else(|| "simbolo non creato".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn privacy_filter_rejects_sentences_personal_data_and_numbers() {
        let lexicon = crate::linguistics::lexicon::Lexicon::load(
            &std::path::Path::new("../resources/dictionaries/it-core.json"),
        )
        .unwrap();
        let terms = known_terms(&lexicon, &[
            "Marco".to_string(),
            "deve prendere la medicina alle otto".to_string(),
            "mario.rossi@example.com".to_string(),
            "https://example.com".to_string(),
            "cane".to_string(),
            "cane".to_string(),
            "computer portatile".to_string(),
        ]);
        assert_eq!(terms, vec!["cane", "computer portatile"]);
        assert_eq!(build_query(&terms), "cane computer portatile");
        assert!(clean_terms(&["12345".to_string()]).is_empty());
    }

    #[test]
    fn wikimedia_response_keeps_license_author_and_filters_non_images() {
        let value: Value = serde_json::from_str(
            r#"{
              "query": {
                "pages": {
                  "1": {
                    "pageid": 1,
                    "title": "File:Cane.png",
                    "imageinfo": [{
                      "url": "https://upload.wikimedia.org/cane.png",
                      "thumburl": "https://upload.wikimedia.org/thumb/cane.png",
                      "descriptionurl": "https://commons.wikimedia.org/wiki/File:Cane.png",
                      "mime": "image/png",
                      "width": 400,
                      "height": 300,
                      "extmetadata": {
                        "LicenseShortName": {"value": "CC BY-SA 4.0"},
                        "Artist": {"value": "<a href='#'>Mario</a>"},
                        "LicenseUrl": {"value": "https://creativecommons.org/licenses/by-sa/4.0/"},
                        "ImageDescription": {"value": "Un cane"}
                      }
                    }]
                  },
                  "2": {
                    "pageid": 2,
                    "title": "File:Video.webm",
                    "imageinfo": [{
                      "url": "https://upload.wikimedia.org/video.webm",
                      "mime": "video/webm",
                      "width": 640,
                      "height": 480
                    }]
                  }
                }
              }
            }"#,
        )
        .unwrap();
        let response = parse_wikimedia(&value, "cane");
        assert_eq!(response.results.len(), 1);
        let result = &response.results[0];
        assert_eq!(result.title, "Un cane");
        assert_eq!(result.author.as_deref(), Some("Mario"));
        assert_eq!(result.license, "CC BY-SA 4.0");
        assert!(result.commercial_ok);
        assert_eq!(result.kind, "fotografia");
    }

    #[test]
    fn arasaac_results_are_pictograms_with_license() {
        let value: Value = serde_json::from_str(
            r#"[{"_id":7202,"aac":true,"violence":false,
                 "keywords":[{"type":2,"keyword":"cane"}],
                 "categories":["mammal"]},
                {"_id":1,"aac":true,"violence":true,"keywords":[{"keyword":"cane"}]},
                {"_id":4569,"aac":false,"violence":false,
                 "keywords":[{"type":2,"keyword":"astronauta"}]}]"#,
        )
        .unwrap();
        let response = parse_arasaac(&value, "cane");
        assert_eq!(response.results.len(), 2);
        let result = &response.results[0];
        assert_eq!(response.results[1].id, "4569");
        assert_eq!(result.kind, "pittogramma");
        assert_eq!(result.license, "CC BY-NC-SA 4.0");
        assert!(!result.commercial_ok);
        assert!(result.image_url.contains("/7202/7202_300.png"));
        assert!(result
            .thumbnail_url
            .as_deref()
            .unwrap_or("")
            .ends_with("/7202_300.png"));
    }

    #[test]
    fn non_commercial_license_is_not_marked_commercial() {
        assert!(!license_allows_commercial("CC BY-NC-SA 4.0"));
        assert!(license_allows_commercial("CC0"));
        assert!(license_allows_commercial("Public domain"));
        assert!(!license_allows_commercial("Licenza da verificare"));
    }

    #[test]
    fn explicit_new_words_fall_back_to_the_cleaned_term() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources/dictionaries/it-core.json");
        let lexicon = crate::linguistics::lexicon::Lexicon::load(&path).unwrap();
        // «cane» è nel lessico: resta la scelta lessicale.
        let known = terms_for_search(&lexicon, &["cane".to_string()]);
        assert!(known.iter().any(|term| term == "cane"));
        // «astronauta» non è nel lessico, ma una ricerca esplicita deve
        // comunque inviare la parola pulita, non una lista già vuota.
        let fresh = terms_for_search(&lexicon, &["astronauta".to_string()]);
        assert_eq!(fresh, vec!["astronauta".to_string()]);
        // Frasi, email, URL e numeri restano esclusi.
        assert!(terms_for_search(
            &lexicon,
            &["scrivi a mario@example.com".to_string()]
        )
        .is_empty());
    }

    #[test]
    fn automatic_words_reject_sentences_and_personal_data() {
        assert_eq!(automatic_word(" Astronauta "), Some("astronauta".to_string()));
        assert_eq!(automatic_word("cane"), Some("cane".to_string()));
        assert_eq!(automatic_word("mario.rossi@example.com"), None);
        assert_eq!(automatic_word("https://example.com"), None);
        assert_eq!(automatic_word("12345"), None);
        assert_eq!(automatic_word("due parole"), None);
    }

    #[test]
    fn image_downloads_require_public_https() {
        assert!(allowed_image_host("https://upload.wikimedia.org/a.png"));
        assert!(allowed_image_host("https://provider.example.org/a.png"));
        assert!(!allowed_image_host("http://upload.wikimedia.org/a.png"));
        assert!(!allowed_image_host("https://localhost/a.png"));
        assert!(!allowed_image_host("https://127.0.0.1/a.png"));
        assert!(!allowed_image_host("https://192.168.1.2/a.png"));
    }
}
