//! Pacchetti della libreria e controllo aggiornamenti.
//!
//! Due famiglie distinte:
//! - **libreria materiali**: i modelli sono inclusi nell'app e funzionano
//!   offline; ogni categoria è un pacchetto che si può sospendere o
//!   riattivare senza cancellare file;
//! - **pittogrammi**: ARASAAC e Global Symbols si scaricano su richiesta dai
//!   rispettivi server e restano nella copia locale dell'utente, con autore,
//!   licenza e fonte. Nessun pittogramma di terzi è incluso negli installer.
//!
//! Il catalogo dei pacchetti e le versioni della libreria vengono letti dal
//! repository pubblico; senza rete il programma continua a funzionare con i
//! dati locali e dichiara che il controllo non è disponibile.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::model::PackInfo;
use crate::engine::Engine;
use crate::template_catalog;

const RELEASES_API: &str = "https://api.github.com/repos/MaestroLivio/caa-writer/releases/latest";
const LIBRARY_MANIFEST_URL: &str =
    "https://raw.githubusercontent.com/MaestroLivio/caa-writer/main/resources/library/manifest.json";

#[derive(Debug, Clone, Serialize)]
pub struct PackageInfo {
    pub id: String,
    /// `library` oppure `symbols`.
    pub kind: String,
    pub name: String,
    pub description: String,
    pub count: usize,
    pub installed: bool,
    /// Incluso nell'app: si può sospendere, non cancellare dal disco.
    pub bundled: bool,
    pub removable: bool,
    pub license: Option<String>,
    pub source_url: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PackageCatalog {
    pub library: Vec<PackageInfo>,
    pub symbols: Vec<PackageInfo>,
    pub library_version: String,
    pub library_generated_at: String,
    /// Avvisi non bloccanti (rete assente, catalogo remoto non letto).
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PackageReport {
    pub id: String,
    pub downloaded: usize,
    pub failed: usize,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: Option<String>,
    pub software_available: bool,
    pub release_url: Option<String>,
    pub published_at: Option<String>,
    pub release_notes: Option<String>,
    pub library_current: String,
    pub library_latest: Option<String>,
    pub library_available: bool,
    pub warnings: Vec<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct LocalPackages {
    #[serde(default)]
    disabled_library: Vec<String>,
}

fn settings_path(data_dir: &Path) -> PathBuf {
    data_dir.join("packages.json")
}

fn load_local(data_dir: &Path) -> LocalPackages {
    std::fs::read_to_string(settings_path(data_dir))
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

fn save_local(data_dir: &Path, value: &LocalPackages) -> Result<(), String> {
    let raw = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    std::fs::write(settings_path(data_dir), raw).map_err(|e| format!("salvataggio pacchetti: {e}"))
}

/// Categorie sospese: la libreria le nasconde senza cancellare i file.
pub fn disabled_library(data_dir: &Path) -> HashSet<String> {
    load_local(data_dir).disabled_library.into_iter().collect()
}

pub fn set_library_enabled(data_dir: &Path, id: &str, enabled: bool) -> Result<(), String> {
    let mut local = load_local(data_dir);
    local.disabled_library.retain(|entry| entry != id);
    if !enabled {
        local.disabled_library.push(id.to_string());
    }
    save_local(data_dir, &local)
}

/// Manifest della libreria senza le categorie sospese.
pub fn filtered_manifest(
    resources_dir: &Path,
    data_dir: &Path,
) -> crate::universal::LibraryManifest {
    let mut manifest = crate::universal::load_library_manifest(resources_dir);
    let disabled = disabled_library(data_dir);
    if !disabled.is_empty() {
        manifest
            .entries
            .retain(|entry| !disabled.contains(&entry.category_id));
    }
    manifest
}

pub fn catalog(engine: &Engine) -> Result<PackageCatalog, String> {
    let manifest = crate::universal::load_library_manifest(&engine.paths.resources_dir);
    let disabled = disabled_library(&engine.paths.data_dir);
    let overview = template_catalog::overview(&manifest);
    let mut library: Vec<PackageInfo> = overview
        .categories
        .iter()
        .filter(|category| category.id != "personali")
        .map(|category| PackageInfo {
            id: format!("library:{}", category.id),
            kind: "library".to_string(),
            name: category.name.clone(),
            description: category.description.clone(),
            count: category.count,
            installed: !disabled.contains(&category.id),
            bundled: true,
            removable: false,
            license: Some("CC BY-NC-SA 4.0".to_string()),
            source_url: Some("https://github.com/MaestroLivio/caa-writer".to_string()),
            version: Some(manifest.generated_at.clone()),
        })
        .collect();
    library.sort_by(|a, b| b.count.cmp(&a.count).then(a.name.cmp(&b.name)));

    // Installazione snella: la libreria materiali non è più inclusa nel
    // pacchetto. Non è un errore: l'app funziona con i repertori di base e
    // propone l'installazione dalla pagina della release.
    let mut warnings: Vec<String> = Vec::new();
    if library.is_empty() {
        library.push(PackageInfo {
            id: "library:materiali".to_string(),
            kind: "library".to_string(),
            name: "Libreria materiali".to_string(),
            description: "Modelli e tabelle CAA pronti da aprire. Non è inclusa in questa installazione: si scarica dalla pagina della release e si copia nella cartella delle risorse."
                .to_string(),
            count: 0,
            installed: false,
            bundled: false,
            removable: false,
            license: Some("CC BY-NC-SA 4.0".to_string()),
            source_url: Some(
                "https://github.com/MaestroLivio/caa-writer/releases/latest".to_string(),
            ),
            version: None,
        });
        warnings.push(
            "La libreria materiali non è installata: l'app si apre normalmente, ma la Libreria resta vuota finché non la scarichi."
                .to_string(),
        );
    }

    let packs = engine.db()?.list_packs().map_err(|e| e.to_string())?;
    let symbols = merge_symbol_packages(&packs);

    Ok(PackageCatalog {
        library,
        symbols,
        library_version: manifest.version.to_string(),
        library_generated_at: manifest.generated_at,
        warnings,
    })
}

fn merge_symbol_packages(packs: &[PackInfo]) -> Vec<PackageInfo> {
    // Catalogo di riferimento indipendente dalle installazioni: un pacchetto
    // rimosso non deve sparire dall'elenco. Il database fornisce solo conteggi
    // e metadati aggiornati; le voci note esistono sempre.
    const KNOWN_PACKS: &[(&str, &str, &str, &str, bool)] = &[
        (
            "arasaac",
            "ARASAAC — pittogrammi",
            "CC BY-NC-SA 4.0",
            "https://arasaac.org/",
            true,
        ),
        (
            "globalsymbols",
            "Global Symbols — vocabolario di base",
            "Licenze dei symbolset Global Symbols",
            "https://globalsymbols.com/",
            true,
        ),
        (
            "core",
            "CAA Writer — pittogrammi originali",
            "CC BY-NC-SA 4.0",
            "https://github.com/MaestroLivio/caa-writer",
            false,
        ),
    ];
    let by_id: std::collections::HashMap<&str, &PackInfo> =
        packs.iter().map(|pack| (pack.id.as_str(), pack)).collect();
    let mut symbols: Vec<PackageInfo> = Vec::new();
    for (id, fallback_name, fallback_license, fallback_source, removable) in KNOWN_PACKS {
        let pack = by_id.get(id).copied();
        let description = match *id {
            "arasaac" => {
                "Catalogo ARASAAC per l'italiano. Le immagini si scaricano dai server ARASAAC e restano nella copia locale dell'utente."
            }
            "globalsymbols" => {
                "Vocabolario di base Global Symbols in inglese, salvato localmente con autore, licenza e fonte."
            }
            _ => "Pittogrammi vettoriali originali di CAA Writer, inclusi nell'app.",
        };
        let count = pack.map(|pack| pack.symbol_count.max(0) as usize).unwrap_or(0);
        symbols.push(PackageInfo {
            id: format!("symbols:{id}"),
            kind: "symbols".to_string(),
            name: pack.map(|pack| pack.name.clone()).unwrap_or_else(|| fallback_name.to_string()),
            description: description.to_string(),
            count,
            installed: count > 0 || *id == "core",
            bundled: *id == "core",
            removable: *removable,
            license: Some(
                pack.map(|pack| pack.license.clone())
                    .unwrap_or_else(|| fallback_license.to_string()),
            ),
            source_url: pack
                .and_then(|pack| pack.source_url.clone())
                .or_else(|| Some(fallback_source.to_string())),
            version: pack.map(|pack| pack.version.clone()),
        });
    }
    // Collezioni personali o repertori importati: restano quelle del database.
    for pack in packs {
        if KNOWN_PACKS.iter().any(|(id, ..)| *id == pack.id) {
            continue;
        }
        symbols.push(PackageInfo {
            id: format!("symbols:{}", pack.id),
            kind: "symbols".to_string(),
            name: pack.name.clone(),
            description: "Collezioni personali importate dall'utente.".to_string(),
            count: pack.symbol_count.max(0) as usize,
            installed: pack.symbol_count > 0,
            bundled: false,
            removable: false,
            license: Some(pack.license.clone()),
            source_url: pack.source_url.clone(),
            version: Some(pack.version.clone()),
        });
    }
    symbols
}

/// Scarica le immagini ARASAAC per le parole del lessico, in blocchi.
/// Se il catalogo è stato rimosso, prima lo ricostruisce dai metadati
/// ufficiali (6,4 MB): così «Rimuovi» non è definitivo e non serve Python.
/// I metadati restano nel database; i PNG finiscono in `assets/arasaac` e il
/// percorso viene riallineato nella tabella simboli.
pub fn download_arasaac(engine: &Engine, limit: usize) -> Result<PackageReport, String> {
    let db = engine.db()?;
    let count: i64 = db
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM symbols WHERE pack_id = 'arasaac' AND hide = 0",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);
    let restored = if count == 0 {
        restore_arasaac_catalog(&db)?
    } else {
        0
    };
    let mut stmt = db
        .conn()
        .prepare("SELECT DISTINCT text FROM lemmas WHERE text <> '' ORDER BY text LIMIT ?1")
        .map_err(|e| e.to_string())?;
    let words: Vec<String> = stmt
        .query_map([limit.clamp(1, 1200) as i64], |row| row.get(0))
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect();
    let filters = crate::database::SymbolFilters {
        pack_id: Some("arasaac".to_string()),
        ..Default::default()
    };
    let mut ids: Vec<String> = Vec::new();
    let mut seen = HashSet::new();
    for word in &words {
        if let Some(record) = db.symbol_by_label(word, &filters).map_err(|e| e.to_string())? {
            if seen.insert(record.id.clone()) {
                ids.push(record.id);
            }
        }
    }
    let mut downloaded = 0usize;
    let mut failed = 0usize;
    for chunk in ids.chunks(40) {
        match crate::online_search::hydrate_arasaac_symbols(
            &db,
            &engine.paths.assets_dir,
            chunk,
        ) {
            Ok(map) => {
                downloaded += map.len();
                failed += chunk.len().saturating_sub(map.len());
            }
            Err(_) => failed += chunk.len(),
        }
    }
    engine.clear_caches();
    Ok(PackageReport {
        id: "symbols:arasaac".to_string(),
        downloaded,
        failed,
        message: format!(
            "ARASAAC: {downloaded} immagini salvate in locale su {} parole del lessico{}.",
            words.len(),
            if restored > 0 {
                format!(" (catalogo ricostruito: {restored} pittogrammi)")
            } else {
                String::new()
            }
        ),
    })
}

/// Scarica un simbolo Global Symbols per le parole di base, salvando le copie
/// locali sotto il pacchetto `globalsymbols`. Se una parola non ha risultati
/// viene contata come non trovata, non inventata.
pub fn download_globalsymbols(engine: &Engine, limit: usize) -> Result<PackageReport, String> {
    let db = engine.db()?;
    ensure_globalsymbols_pack(&db)?;
    let mut stmt = db
        .conn()
        .prepare("SELECT DISTINCT text FROM lemmas WHERE text <> '' ORDER BY text LIMIT ?1")
        .map_err(|e| e.to_string())?;
    let words: Vec<String> = stmt
        .query_map([limit.clamp(1, 300) as i64], |row| row.get(0))
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect();
    let mut downloaded = 0usize;
    let mut failed = 0usize;
    for word in &words {
        let term = engine
            .translate_to_english(word)
            .unwrap_or_else(|_| word.clone());
        let response = match crate::online_search::search_global_symbols(&db, &term) {
            Ok(value) => value,
            Err(_) => {
                failed += 1;
                continue;
            }
        };
        let Some(result) = response
            .results
            .into_iter()
            .find(|item| {
                item.kind == "pittogramma"
                    && !item.source.to_ascii_lowercase().contains("arasaac")
            })
        else {
            failed += 1;
            continue;
        };
        let label = if result.title.trim().is_empty() {
            word.clone()
        } else {
            result.title.clone()
        };
        match crate::online_search::save_result(
            &db,
            &engine.paths.assets_dir,
            &result,
            &label,
            "Global Symbols",
        ) {
            Ok(symbol_id) => {
                let _ = db.conn().execute(
                    "UPDATE symbols SET pack_id = 'globalsymbols' WHERE id = ?1",
                    rusqlite::params![symbol_id],
                );
                downloaded += 1;
            }
            Err(_) => failed += 1,
        }
    }
    engine.clear_caches();
    Ok(PackageReport {
        id: "symbols:globalsymbols".to_string(),
        downloaded,
        failed,
        message: format!(
            "Global Symbols: {downloaded} simboli salvati in locale su {} parole di base.",
            words.len()
        ),
    })
}

fn ensure_globalsymbols_pack(db: &crate::database::Database) -> Result<(), String> {
    db.conn()
        .execute(
            "INSERT INTO symbol_packs(id, name, version, description, author, rights_holder, license,
                license_url, source_url, constraints, commercial_ok, share_alike, no_derivatives,
                builtin, installed_at)
             VALUES ('globalsymbols','Global Symbols — vocabolario di base','1.0',
                'Copie locali dei pittogrammi Global Symbols usati dal lessico',
                'Autori dei symbolset','Global Symbols','Licenze dei symbolset Global Symbols',
                'https://globalsymbols.com/','https://globalsymbols.com/',
                'Ogni simbolo conserva autore, licenza e URL della fonte',
                0,0,0,0,?1)
             ON CONFLICT(id) DO NOTHING",
            rusqlite::params![crate::util::now_iso()],
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Ricostruisce i metadati del catalogo ARASAAC italiano dal servizio
/// ufficiale. Le immagini non vengono scaricate qui: restano on demand e
/// vengono salvate da `download_arasaac` o dall'uso normale del programma.
fn restore_arasaac_catalog(db: &crate::database::Database) -> Result<usize, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .user_agent("CAA-Writer")
        .build()
        .map_err(|e| format!("client HTTP: {e}"))?;
    let response = client
        .get("https://api.arasaac.org/api/pictograms/all/it")
        .send()
        .map_err(|_| "ARASAAC non raggiungibile: il catalogo non è stato ricostruito.".to_string())?;
    if !response.status().is_success() {
        return Err(format!(
            "ARASAAC ha risposto con stato {}: catalogo non ricostruito.",
            response.status()
        ));
    }
    let items: Vec<serde_json::Value> = response
        .json()
        .map_err(|e| format!("catalogo ARASAAC non valido: {e}"))?;
    db.conn()
        .execute(
            "INSERT INTO symbol_packs(id, name, version, description, author, rights_holder, license,
                license_url, source_url, constraints, commercial_ok, share_alike, no_derivatives,
                builtin, installed_at)
             VALUES ('arasaac','ARASAAC — pittogrammi','2.0.1',
                'Catalogo ARASAAC completo ricostruito dai metadati ufficiali; le immagini si scaricano su richiesta',
                'Sergio Palao','Governo di Aragona (Spagna)','CC BY-NC-SA 4.0',
                'https://creativecommons.org/licenses/by-nc-sa/4.0/','https://arasaac.org/',
                'Attribuzione ad ARASAAC e a Sergio Palao; uso non commerciale; condividi allo stesso modo',
                0,1,0,0,?1)
             ON CONFLICT(id) DO NOTHING",
            rusqlite::params![crate::util::now_iso()],
        )
        .map_err(|e| e.to_string())?;
    let tx = db.conn().unchecked_transaction().map_err(|e| e.to_string())?;
    let now = crate::util::now_iso();
    let mut inserted = 0usize;
    for item in &items {
        let Some(id) = item.get("_id").and_then(|value| value.as_i64()) else {
            continue;
        };
        let keywords: Vec<String> = item
            .get("keywords")
            .and_then(|value| value.as_array())
            .map(|list| {
                list.iter()
                    .filter_map(|entry| entry.get("keyword").and_then(|v| v.as_str()))
                    .filter(|word| !word.trim().is_empty())
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();
        let Some(label) = keywords.first() else {
            continue;
        };
        let symbol_id = format!("arasaac.picto.{id}");
        let changed = tx
            .execute(
                "INSERT INTO symbols(id, concept_id, pack_id, label, meaning, author, rights_holder,
                    license, license_url, source_url, constraints, modifications, format, storage,
                    data_path, width, height, status, hide, checksum, created_at)
                 VALUES (?1, NULL, 'arasaac', ?2, NULL, 'Sergio Palao', 'Governo di Aragona (Spagna)',
                    'CC BY-NC-SA 4.0', 'https://creativecommons.org/licenses/by-nc-sa/4.0/',
                    ?3, 'Attribuzione ad ARASAAC e a Sergio Palao', 'Catalogo ufficiale',
                    'png', 'db', NULL, 500, 500, 'approved', 0, NULL, ?4)
                 ON CONFLICT(id) DO NOTHING",
                rusqlite::params![
                    symbol_id,
                    label,
                    format!("https://arasaac.org/pictograms/{id}"),
                    now
                ],
            )
            .map_err(|e| e.to_string())?;
        if changed == 0 {
            continue;
        }
        inserted += 1;
        for keyword in keywords.iter().take(12) {
            let _ = tx.execute(
                "INSERT OR IGNORE INTO symbol_keywords(symbol_id, term) VALUES (?1, ?2)",
                rusqlite::params![symbol_id, keyword.to_lowercase()],
            );
        }
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(inserted)
}

/// Rimuove un pacchetto di pittogrammi dal database e gli asset locali.
/// `core` e `personal` sono protetti: contengono i simboli base e le scelte
/// dell'utente, non un pacchetto scaricabile.
pub fn remove_symbol_pack(engine: &Engine, pack_id: &str) -> Result<(), String> {
    if pack_id == "core" || pack_id == "personal" {
        return Err("Questo pacchetto non si può rimuovere: contiene simboli di base o importati dall'utente.".to_string());
    }
    let db = engine.db()?;
    let mut stmt = db
        .conn()
        .prepare(
            "SELECT COALESCE(a.path, s.data_path) FROM symbols s
             LEFT JOIN symbol_assets a ON a.symbol_id = s.id
             WHERE s.pack_id = ?1",
        )
        .map_err(|e| e.to_string())?;
    let paths: Vec<String> = stmt
        .query_map([pack_id], |row| row.get::<_, Option<String>>(0))
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .flatten()
        .collect();
    for path in paths {
        let candidate = Path::new(&path);
        let absolute = if candidate.is_absolute() {
            candidate.to_path_buf()
        } else {
            engine.paths.assets_dir.join(candidate)
        };
        if absolute.starts_with(&engine.paths.assets_dir) {
            let _ = std::fs::remove_file(absolute);
        }
    }
    db.conn()
        .execute(
            "DELETE FROM symbol_preferences WHERE symbol_id IN (SELECT id FROM symbols WHERE pack_id = ?1)",
            [pack_id],
        )
        .map_err(|e| e.to_string())?;
    db.conn()
        .execute(
            "UPDATE document_overrides SET symbol_id = NULL, locked = 0 WHERE symbol_id IN (SELECT id FROM symbols WHERE pack_id = ?1)",
            [pack_id],
        )
        .map_err(|e| e.to_string())?;
    db.conn()
        .execute("DELETE FROM symbols WHERE pack_id = ?1", [pack_id])
        .map_err(|e| e.to_string())?;
    // La riga di `symbol_packs` resta: la voce continua a esistere nel
    // catalogo con zero simboli e può essere reinstallata. Il registro delle
    // installazioni non è la fonte dell'elenco dei pacchetti disponibili.
    engine.clear_caches();
    Ok(())
}

fn parse_version(value: &str) -> Vec<u64> {
    value
        .trim_start_matches('v')
        .split(['.', '-', '+'])
        .map(|part| part.parse::<u64>().unwrap_or(0))
        .collect()
}

fn is_newer(latest: &str, current: &str) -> bool {
    let mut left = parse_version(latest).into_iter();
    let mut right = parse_version(current).into_iter();
    loop {
        match (left.next(), right.next()) {
            (None, None) => return false,
            (Some(a), Some(b)) if a != b => return a > b,
            (Some(_), Some(_)) => {}
            (Some(a), None) => return a > 0,
            (None, Some(_)) => return false,
        }
    }
}

/// Confronta la versione dell'app e la libreria con il repository pubblico.
/// Senza rete restituisce gli avvisi, non un errore: il controllo è
/// un'informazione, non un requisito per usare il programma.
pub fn check_updates(engine: &Engine) -> UpdateInfo {
    let mut warnings = Vec::new();
    let current = env!("CARGO_PKG_VERSION").to_string();
    let local = crate::universal::load_library_manifest(&engine.paths.resources_dir);
    let mut latest_version = None;
    let mut release_url = None;
    let mut published_at = None;
    let mut release_notes = None;

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .user_agent("CAA-Writer")
        .build();
    if let Ok(client) = client {
        match client.get(RELEASES_API).send() {
            Ok(response) if response.status().is_success() => {
                if let Ok(value) = response.json::<serde_json::Value>() {
                    latest_version = value
                        .get("tag_name")
                        .and_then(|v| v.as_str())
                        .map(|v| v.trim_start_matches('v').to_string());
                    release_url = value
                        .get("html_url")
                        .and_then(|v| v.as_str())
                        .map(str::to_string);
                    published_at = value
                        .get("published_at")
                        .and_then(|v| v.as_str())
                        .map(str::to_string);
                    release_notes = value
                        .get("body")
                        .and_then(|v| v.as_str())
                        .map(|body| body.chars().take(1200).collect());
                }
            }
            Ok(response) => warnings.push(format!(
                "Controllo aggiornamenti non disponibile (HTTP {}).",
                response.status()
            )),
            Err(_) => warnings.push(
                "Nessuna connessione: controllo aggiornamenti non eseguito. L'app e la libreria locali restano utilizzabili."
                    .to_string(),
            ),
        }

        match client.get(LIBRARY_MANIFEST_URL).send() {
            Ok(response) if response.status().is_success() => {
                if let Ok(value) = response.json::<serde_json::Value>() {
                    let remote = value
                        .get("generated_at")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    if !remote.is_empty() {
                        return finish_update(
                            current,
                            latest_version,
                            release_url,
                            published_at,
                            release_notes,
                            local,
                            Some(remote),
                            warnings,
                        );
                    }
                }
                warnings.push("La libreria remota non contiene una data di aggiornamento.".to_string());
            }
            Ok(response) => warnings.push(format!(
                "Libreria remota non disponibile (HTTP {}).",
                response.status()
            )),
            Err(_) => warnings.push("Libreria remota non raggiungibile.".to_string()),
        }
    } else if let Err(error) = client {
        warnings.push(format!("Client HTTP non disponibile: {error}"));
    }

    finish_update(
        current,
        latest_version,
        release_url,
        published_at,
        release_notes,
        local,
        None,
        warnings,
    )
}

#[allow(clippy::too_many_arguments)]
fn finish_update(
    current: String,
    latest_version: Option<String>,
    release_url: Option<String>,
    published_at: Option<String>,
    release_notes: Option<String>,
    local: crate::universal::LibraryManifest,
    library_latest: Option<String>,
    warnings: Vec<String>,
) -> UpdateInfo {
    let software_available = latest_version
        .as_deref()
        .map(|latest| is_newer(latest, &current))
        .unwrap_or(false);
    let library_available = library_latest
        .as_deref()
        .map(|remote| remote > local.generated_at.as_str())
        .unwrap_or(false);
    UpdateInfo {
        current_version: current,
        latest_version,
        software_available,
        release_url,
        published_at,
        release_notes,
        library_current: local.generated_at,
        library_latest,
        library_available,
        warnings,
    }
}

/// Apre un link del repository pubblico nel browser di sistema.
pub fn open_external(url: &str) -> Result<(), String> {
    let allowed = url.starts_with("https://github.com/MaestroLivio/caa-writer");
    if !allowed {
        return Err("Link non consentito: si aprono solo pagine del repository CAA Writer.".to_string());
    }
    #[cfg(target_os = "macos")]
    let command = std::process::Command::new("open").arg(url).spawn();
    #[cfg(target_os = "windows")]
    let command = std::process::Command::new("cmd")
        .args(["/C", "start", "", url])
        .spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let command = std::process::Command::new("xdg-open").arg(url).spawn();
    command
        .map(|_| ())
        .map_err(|e| format!("impossibile aprire il browser: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_compare_numerically() {
        assert!(is_newer("0.15.14", "0.15.13"));
        assert!(is_newer("0.16.0", "0.15.99"));
        assert!(!is_newer("0.15.13", "0.15.13"));
        assert!(!is_newer("v0.15.12", "0.15.13"));
        assert!(is_newer("v0.15.10", "0.15.9"));
    }

    #[test]
    fn external_links_are_limited_to_the_repository() {
        assert!(open_external("https://example.com").is_err());
        assert!(open_external("file:///etc/passwd").is_err());
    }

    /// Un pacchetto disinstallato (nessuna riga nel database o zero simboli)
    /// deve restare nel catalogo, con `installed = false` e la possibilità di
    /// reinstallarlo. Il registro delle installazioni non è l'elenco.
    #[test]
    fn uninstalled_packs_remain_in_the_catalog() {
        let symbols = merge_symbol_packages(&[]);
        let arasaac = symbols
            .iter()
            .find(|item| item.id == "symbols:arasaac")
            .expect("ARASAAC resta nel catalogo");
        assert!(!arasaac.installed);
        assert!(arasaac.removable);
        assert!(symbols.iter().any(|item| item.id == "symbols:globalsymbols"));
        let core = symbols
            .iter()
            .find(|item| item.id == "symbols:core")
            .expect("il repertorio core resta incluso");
        assert!(core.installed && core.bundled);
    }
}
