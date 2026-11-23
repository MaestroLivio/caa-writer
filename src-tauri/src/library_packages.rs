//! Aggiornamento dei singoli pacchetti della libreria.
//!
//! La libreria materiali è inclusa nell'app (o nella cartella risorse) e
//! funziona offline. Ogni categoria è un **pacchetto** con una propria
//! versione, dichiarata nel manifest (`packages`) e confrontata con:
//!
//! - la versione **inclusa** nell'installazione (`resources/library`);
//! - la versione **installata** nella copia dell'utente
//!   (`<data-dir>/library`), che vince su quella inclusa;
//! - la versione **pubblicata nel repository** (controllo esplicito, richiede
//!   rete; senza rete il programma continua a funzionare e lo dichiara).
//!
//! L'aggiornamento di un pacchetto copia soltanto i documenti di quella
//! categoria nella copia dell'utente: gli altri pacchetti non vengono toccati,
//! e non si scarica nulla senza una richiesta esplicita.

use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::universal::{LibraryEntry, LibraryManifest, LibraryPackage};

/// Manifest pubblicato nel repository: serve solo al controllo esplicito
/// degli aggiornamenti.
const REMOTE_MANIFEST_URL: &str =
    "https://raw.githubusercontent.com/MaestroLivio/caa-writer/main/resources/library/manifest.json";
const REMOTE_ROOT: &str =
    "https://raw.githubusercontent.com/MaestroLivio/caa-writer/main/resources/";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InstalledPackage {
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub updated_at: String,
    #[serde(default)]
    pub documents: usize,
    #[serde(default)]
    pub bytes: u64,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub installed_at: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InstalledIndex {
    #[serde(default)]
    pub packages: BTreeMap<String, InstalledPackage>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LibraryPackageState {
    pub id: String,
    pub name: String,
    pub documents: usize,
    /// Versione nella copia dell'utente; `None` = si usa quella inclusa.
    pub installed_version: Option<String>,
    pub bundled_version: String,
    pub remote_version: Option<String>,
    /// `aggiornato` | `aggiornabile`
    pub status: String,
    /// `nessuna` | `inclusa` | `repository`
    pub action: String,
    pub updated_at: Option<String>,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct LibraryPackagesStatus {
    pub packages: Vec<LibraryPackageState>,
    pub library_dir: String,
    pub outdated: usize,
    pub checked_remote: bool,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LibraryPackageUpdate {
    pub id: String,
    pub version: String,
    pub source: String,
    pub copied: usize,
    pub bytes: u64,
    pub updated: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LibraryPackagesUpdateReport {
    pub source: String,
    pub packages: Vec<LibraryPackageUpdate>,
    pub updated: usize,
    pub failed: usize,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PackageOutlineDocument {
    pub id: String,
    pub title: String,
    pub subcategory: String,
    pub pages: usize,
    pub elements: usize,
    pub audience: String,
    pub objective: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PackageOutline {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub documents: usize,
    pub pages: usize,
    pub elements: usize,
    /// Obiettivi dichiarati nei documenti, senza duplicati.
    pub objectives: Vec<String>,
    pub audiences: Vec<String>,
    pub subcategories: Vec<(String, usize)>,
    pub documents_detail: Vec<PackageOutlineDocument>,
}

pub fn library_root(data_dir: &Path) -> PathBuf {
    data_dir.join("library")
}

fn installed_path(data_dir: &Path) -> PathBuf {
    library_root(data_dir).join("installed.json")
}

fn overlay_manifest_path(data_dir: &Path) -> PathBuf {
    library_root(data_dir).join("manifest.json")
}

pub fn load_installed(data_dir: &Path) -> InstalledIndex {
    crate::text_io::read_lossy(&installed_path(data_dir))
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

fn save_installed(data_dir: &Path, index: &InstalledIndex) -> Result<(), String> {
    let root = library_root(data_dir);
    std::fs::create_dir_all(&root).map_err(|e| format!("cartella libreria: {e}"))?;
    let raw = serde_json::to_string_pretty(index).map_err(|e| e.to_string())?;
    std::fs::write(installed_path(data_dir), raw)
        .map_err(|e| format!("salvataggio stato pacchetti: {e}"))
}

/// Manifest della copia dell'utente: contiene solo i pacchetti aggiornati.
pub fn overlay_manifest(data_dir: &Path) -> Option<LibraryManifest> {
    let raw = crate::text_io::read_lossy(&overlay_manifest_path(data_dir)).ok()?;
    let mut manifest: LibraryManifest = serde_json::from_str(&raw).ok()?;
    for entry in &mut manifest.entries {
        crate::template_catalog::normalize_entry(entry);
    }
    Some(manifest)
}

/// Unisce la libreria dell'utente con quella inclusa: per ogni categoria
/// aggiornata le voci nuove sostituiscono le vecchie.
pub fn merge(bundled: &LibraryManifest, overlay: &LibraryManifest) -> LibraryManifest {
    let replaced: Vec<&str> = overlay
        .packages
        .iter()
        .map(|package| package.id.as_str())
        .chain(overlay.entries.iter().map(|entry| entry.category_id.as_str()))
        .collect();
    let mut merged = bundled.clone();
    merged.entries.retain(|entry| {
        !replaced.iter().any(|id| !id.is_empty() && *id == entry.category_id)
    });
    merged.entries.extend(overlay.entries.iter().cloned());
    for package in &overlay.packages {
        merged
            .packages
            .retain(|existing| existing.id != package.id);
        merged.packages.push(package.clone());
    }
    if overlay.generated_at > merged.generated_at {
        merged.generated_at = overlay.generated_at.clone();
    }
    merged
}

fn is_newer(candidate: &str, current: &str) -> bool {
    fn parts(value: &str) -> Vec<u64> {
        value
            .split(|c: char| !c.is_ascii_digit())
            .filter(|piece| !piece.is_empty())
            .map(|piece| piece.parse().unwrap_or(0))
            .collect()
    }
    let left = parts(candidate);
    let right = parts(current);
    if left.is_empty() {
        return false;
    }
    for index in 0..left.len().max(right.len()) {
        let a = left.get(index).copied().unwrap_or(0);
        let b = right.get(index).copied().unwrap_or(0);
        if a != b {
            return a > b;
        }
    }
    false
}

fn bundled_packages(manifest: &LibraryManifest) -> Vec<LibraryPackage> {
    if !manifest.packages.is_empty() {
        return manifest.packages.clone();
    }
    // Compatibilità: manifest senza versioni per pacchetto. La versione
    // coincide con la data di generazione del catalogo.
    let mut seen: BTreeMap<String, LibraryPackage> = BTreeMap::new();
    for entry in &manifest.entries {
        if entry.category_id.is_empty() {
            continue;
        }
        let package = seen
            .entry(entry.category_id.clone())
            .or_insert_with(|| LibraryPackage {
                id: entry.category_id.clone(),
                name: entry.category.clone(),
                version: manifest.generated_at.clone(),
                updated_at: manifest.generated_at.clone(),
                documents: 0,
            });
        package.documents += 1;
    }
    seen.into_values().collect()
}

/// Stato dei pacchetti. `remote` è il manifest pubblicato: si passa solo dopo
/// un controllo esplicito dell'utente, così l'uso normale resta offline.
pub fn status(
    bundled: &LibraryManifest,
    data_dir: &Path,
    remote: Option<&LibraryManifest>,
) -> LibraryPackagesStatus {
    let installed = load_installed(data_dir);
    let mut warnings = Vec::new();
    let mut packages = Vec::new();
    let mut indexes = bundled_packages(bundled);
    // Categorie presenti solo nella copia dell'utente (pacchetto importato o
    // rimosso dalla versione inclusa): restano visibili e aggiornabili.
    let installed_only: Vec<LibraryPackage> = installed
        .packages
        .iter()
        .filter(|(id, _)| !indexes.iter().any(|package| &package.id == *id))
        .map(|(id, record)| LibraryPackage {
            id: id.clone(),
            name: id.clone(),
            version: record.version.clone(),
            updated_at: record.updated_at.clone(),
            documents: record.documents,
        })
        .collect();
    indexes.extend(installed_only);
    for package in indexes {
        let remote_version = remote
            .and_then(|manifest| manifest.packages.iter().find(|item| item.id == package.id))
            .map(|item| item.version.clone())
            .filter(|version| !version.is_empty());
        let record = installed.packages.get(&package.id);
        let installed_version = record.map(|item| item.version.clone());
        let effective = installed_version.clone().unwrap_or_else(|| package.version.clone());
        let mut action = "nessuna";
        let mut target = effective.clone();
        if let Some(remote) = &remote_version {
            if is_newer(remote, &package.version) && is_newer(remote, &effective) {
                action = "repository";
                target = remote.clone();
            }
        }
        if action == "nessuna" && is_newer(&package.version, &effective) {
            action = "inclusa";
            target = package.version.clone();
        }
        let up_to_date = !is_newer(&target, &effective);
        if !up_to_date && action == "nessuna" {
            // Nessuna sorgente più recente della copia installata: dichiara
            // l'incongruenza invece di proporre un aggiornamento inutile.
            warnings.push(format!(
                "Il pacchetto «{}» è più recente della versione inclusa: nessun aggiornamento disponibile.",
                package.name
            ));
        }
        let documents = if package.documents == 0 {
            record.map(|item| item.documents).unwrap_or(0)
        } else {
            package.documents
        };
        packages.push(LibraryPackageState {
            id: package.id.clone(),
            name: package.name.clone(),
            documents,
            installed_version,
            bundled_version: package.version.clone(),
            remote_version,
            status: if up_to_date {
                "aggiornato".to_string()
            } else {
                "aggiornabile".to_string()
            },
            action: if up_to_date {
                "nessuna".to_string()
            } else {
                action.to_string()
            },
            updated_at: record
                .map(|item| item.updated_at.clone())
                .filter(|value| !value.is_empty()),
            bytes: record.map(|item| item.bytes).unwrap_or(0),
        });
    }
    packages.sort_by(|a, b| {
        b.documents
            .cmp(&a.documents)
            .then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    let outdated = packages
        .iter()
        .filter(|package| package.status == "aggiornabile")
        .count();
    LibraryPackagesStatus {
        packages,
        library_dir: library_root(data_dir).to_string_lossy().to_string(),
        outdated,
        checked_remote: remote.is_some(),
        warnings,
    }
}

/// Percorso di scrittura dentro `<data-dir>/library`, con protezione dal
/// path traversal: solo componenti normali, niente percorsi assoluti.
fn safe_target(data_dir: &Path, relative: &str) -> Result<PathBuf, String> {
    let relative = relative.strip_prefix("resources/").unwrap_or(relative);
    let candidate = Path::new(relative);
    if candidate.is_absolute() {
        return Err(format!("percorso assoluto rifiutato: {relative}"));
    }
    for component in candidate.components() {
        match component {
            Component::Normal(_) => {}
            Component::CurDir => {}
            _ => return Err(format!("percorso non consentito: {relative}")),
        }
    }
    if !candidate.starts_with("library") {
        return Err(format!("percorso fuori dalla libreria: {relative}"));
    }
    Ok(library_root(data_dir).join(
        candidate
            .strip_prefix("library")
            .unwrap_or(candidate)
            .to_path_buf(),
    ))
}

/// Installa un pacchetto con i file già in memoria (usato sia per la copia
/// inclusa sia per il download dal repository). Non tocca altre categorie.
pub fn install_entries(
    data_dir: &Path,
    package: &LibraryPackage,
    entries: &[LibraryEntry],
    files: &[(String, Vec<u8>)],
    previews: &[(String, Vec<u8>)],
    source: &str,
    replace_installed: bool,
) -> Result<LibraryPackageUpdate, String> {
    if entries.is_empty() {
        return Err(format!(
            "il pacchetto «{}» non contiene documenti da installare",
            package.id
        ));
    }
    let mut installed_index = load_installed(data_dir);
    let existing = installed_index.packages.get(&package.id);
    if let Some(record) = existing {
        if record.version != package.version && !replace_installed {
            return Err(format!(
                "il pacchetto «{}» è alla versione {} e verrebbe sostituito con la {}: serve conferma",
                package.name, record.version, package.version
            ));
        }
    }
    let mut bytes = 0u64;
    for (relative, data) in files {
        let target = safe_target(data_dir, relative)?;
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("cartella {}: {e}", parent.display()))?;
        }
        std::fs::write(&target, data).map_err(|e| format!("{}: {e}", target.display()))?;
        bytes += data.len() as u64;
    }
    for (relative, data) in previews {
        let target = safe_target(data_dir, relative)?;
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("cartella {}: {e}", parent.display()))?;
        }
        std::fs::write(&target, data).map_err(|e| format!("{}: {e}", target.display()))?;
    }

    let mut overlay = overlay_manifest(data_dir).unwrap_or_default();
    overlay.version = 1;
    overlay.entries.retain(|entry| entry.category_id != package.id);
    overlay
        .entries
        .extend(entries.iter().cloned());
    overlay.packages.retain(|item| item.id != package.id);
    overlay.packages.push(package.clone());
    overlay.generated_at = now_iso();
    std::fs::create_dir_all(library_root(data_dir))
        .map_err(|e| format!("cartella libreria: {e}"))?;
    let raw = serde_json::to_string_pretty(&overlay).map_err(|e| e.to_string())?;
    std::fs::write(overlay_manifest_path(data_dir), raw)
        .map_err(|e| format!("salvataggio manifest locale: {e}"))?;

    installed_index.packages.insert(
        package.id.clone(),
        InstalledPackage {
            version: package.version.clone(),
            updated_at: package.updated_at.clone(),
            documents: entries.len(),
            bytes,
            source: source.to_string(),
            installed_at: now_iso(),
        },
    );
    save_installed(data_dir, &installed_index)?;

    Ok(LibraryPackageUpdate {
        id: package.id.clone(),
        version: package.version.clone(),
        source: source.to_string(),
        copied: files.len(),
        bytes,
        updated: true,
        message: format!(
            "Pacchetto «{}» aggiornato alla versione {} ({} documenti).",
            package.name,
            package.version,
            entries.len()
        ),
    })
}

fn now_iso() -> String {
    // Timestamp leggibile senza dipendenze aggiuntive.
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|value| value.as_secs())
        .unwrap_or(0);
    format!("{seconds}")
}

fn read_bundled_files(
    resources: &Path,
    entries: &[LibraryEntry],
) -> Result<(Vec<(String, Vec<u8>)>, Vec<(String, Vec<u8>)>), String> {
    let mut files = Vec::new();
    let mut previews = Vec::new();
    for entry in entries {
        let source = resources.join(
            entry
                .path
                .strip_prefix("resources/")
                .unwrap_or(&entry.path),
        );
        let data = std::fs::read(&source)
            .map_err(|e| format!("documento incluso non leggibile ({}): {e}", source.display()))?;
        files.push((entry.path.clone(), data));
        if !entry.preview.is_empty() {
            let preview = resources.join(
                entry
                    .preview
                    .strip_prefix("resources/")
                    .unwrap_or(&entry.preview),
            );
            if let Ok(svg) = std::fs::read(&preview) {
                previews.push((entry.preview.clone(), svg));
            }
        }
    }
    Ok((files, previews))
}

fn remote_get(client: &reqwest::blocking::Client, url: &str) -> Result<Vec<u8>, String> {
    let response = client
        .get(url)
        .send()
        .map_err(|e| format!("rete non disponibile: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("HTTP {}", response.status()));
    }
    response
        .bytes()
        .map(|body| body.to_vec())
        .map_err(|e| format!("lettura risposta: {e}"))
}

/// Manifest pubblicato nel repository. Senza rete restituisce l'errore: il
/// chiamante decide se trasformarlo in un avviso non bloccante.
pub fn fetch_remote_manifest() -> Result<LibraryManifest, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent("CAA-Writer")
        .build()
        .map_err(|e| format!("client HTTP: {e}"))?;
    let raw = remote_get(&client, REMOTE_MANIFEST_URL)?;
    serde_json::from_slice(&raw).map_err(|e| format!("manifest remoto non valido: {e}"))
}

fn read_remote_files(
    resources: &Path,
    entries: &[LibraryEntry],
) -> Result<(Vec<(String, Vec<u8>)>, Vec<(String, Vec<u8>)>), String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("CAA-Writer")
        .build()
        .map_err(|e| format!("client HTTP: {e}"))?;
    let mut files = Vec::new();
    let mut previews = Vec::new();
    for entry in entries {
        let relative = entry
            .path
            .strip_prefix("resources/")
            .unwrap_or(&entry.path);
        let data = remote_get(&client, &format!("{REMOTE_ROOT}{relative}"))?;
        files.push((entry.path.clone(), data));
        if !entry.preview.is_empty() {
            let preview_relative = entry
                .preview
                .strip_prefix("resources/")
                .unwrap_or(&entry.preview);
            if let Ok(svg) = remote_get(&client, &format!("{REMOTE_ROOT}{preview_relative}")) {
                previews.push((entry.preview.clone(), svg));
            }
        }
        // Le anteprime locali restano valide se il documento non ne ha una
        // nuova: meglio non bloccare l'aggiornamento per un'anteprima.
        let _ = resources;
    }
    Ok((files, previews))
}

/// Aggiorna un singolo pacchetto. `source`:
/// - `inclusa`: copia la versione che sta nell'installazione (funziona offline);
/// - `repository`: scarica la versione pubblicata (richiede rete);
/// - `auto`: prova il repository, ripiega sulla copia inclusa.
pub fn update(
    resources: &Path,
    data_dir: &Path,
    package_id: &str,
    source: &str,
    confirm: bool,
) -> Result<LibraryPackageUpdate, String> {
    let id = package_id.trim_start_matches("library:");
    let bundled = crate::universal::load_library_manifest(resources);
    let package = bundled_packages(&bundled)
        .into_iter()
        .find(|package| package.id == id)
        .ok_or_else(|| format!("pacchetto «{id}» non presente nella libreria inclusa"))?;
    let source = match source {
        "inclusa" | "bundled" => "inclusa",
        "repository" | "remote" => "repository",
        _ => "auto",
    };
    if source != "inclusa" {
        let remote = match source {
            "repository" => Some(fetch_remote_manifest()?),
            _ => fetch_remote_manifest().ok(),
        };
        if let Some(remote) = remote {
            if let Some(remote_package) =
                remote.packages.iter().find(|item| item.id == id).cloned()
            {
                if is_newer(&remote_package.version, &package.version) || source == "repository" {
                    let entries: Vec<LibraryEntry> = remote
                        .entries
                        .into_iter()
                        .filter(|entry| entry.category_id == id)
                        .collect();
                    let (files, previews) = read_remote_files(resources, &entries)?;
                    return install_entries(
                        data_dir,
                        &remote_package,
                        &entries,
                        &files,
                        &previews,
                        "repository",
                        confirm,
                    );
                }
            }
        } else if source == "repository" {
            return Err("rete non disponibile: aggiornamento dal repository non eseguito".into());
        }
    }
    let entries: Vec<LibraryEntry> = bundled
        .entries
        .iter()
        .filter(|entry| entry.category_id == id)
        .cloned()
        .collect();
    let (files, previews) = read_bundled_files(resources, &entries)?;
    install_entries(
        data_dir,
        &package,
        &entries,
        &files,
        &previews,
        "inclusa",
        confirm,
    )
}

/// Aggiorna tutti i pacchetti che hanno una versione più recente.
pub fn update_all(
    resources: &Path,
    data_dir: &Path,
    source: &str,
    confirm: bool,
) -> LibraryPackagesUpdateReport {
    let bundled = crate::universal::load_library_manifest(resources);
    let remote = fetch_remote_manifest().ok();
    let state = status(&bundled, data_dir, remote.as_ref());
    let mut packages = Vec::new();
    let mut warnings = Vec::new();
    let mut failed = 0usize;
    for package in state.packages.iter().filter(|item| item.status == "aggiornabile") {
        match update(resources, data_dir, &package.id, source, confirm) {
            Ok(update) => packages.push(update),
            Err(error) => {
                failed += 1;
                warnings.push(format!("{}: {error}", package.name));
            }
        }
    }
    let updated = packages.len();
    LibraryPackagesUpdateReport {
        source: source.to_string(),
        packages,
        updated,
        failed,
        warnings,
    }
}

/// Scheda didattica di un pacchetto: obiettivi, pubblico, sottocategorie e
/// documenti. Solo dati presenti nel catalogo, nessun testo inventato.
pub fn outline(bundled: &LibraryManifest, package_id: &str) -> Result<PackageOutline, String> {
    let id = package_id.trim_start_matches("library:");
    let package = bundled_packages(bundled)
        .into_iter()
        .find(|package| package.id == id)
        .ok_or_else(|| format!("pacchetto «{id}» non presente nel catalogo"))?;
    let entries: Vec<&LibraryEntry> = bundled
        .entries
        .iter()
        .filter(|entry| entry.category_id == id)
        .collect();
    let mut objectives: Vec<String> = Vec::new();
    let mut audiences: Vec<String> = Vec::new();
    let mut subcategories: BTreeMap<String, usize> = BTreeMap::new();
    let mut documents = Vec::new();
    let mut pages = 0usize;
    let mut elements = 0usize;
    for entry in &entries {
        pages += entry.pages;
        elements += entry.elements;
        for (target, value) in [
            (&mut objectives, entry.objective.clone()),
            (&mut audiences, entry.audience.clone()),
        ] {
            let value = value.trim().to_string();
            if !value.is_empty() && !target.iter().any(|item| item == &value) {
                target.push(value);
            }
        }
        let subcategory = if entry.subcategory.is_empty() {
            entry.category.clone()
        } else {
            entry.subcategory.clone()
        };
        *subcategories.entry(subcategory.clone()).or_insert(0) += 1;
        documents.push(PackageOutlineDocument {
            id: entry.id.clone(),
            title: entry.title.clone(),
            subcategory,
            pages: entry.pages,
            elements: entry.elements,
            audience: entry.audience.clone(),
            objective: entry.objective.clone(),
            tags: entry.tags.iter().take(6).cloned().collect(),
        });
    }
    objectives.sort();
    audiences.sort();
    documents.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
    Ok(PackageOutline {
        id: package.id,
        name: package.name,
        description: String::new(),
        version: package.version,
        documents: entries.len(),
        pages,
        elements,
        objectives,
        audiences,
        subcategories: subcategories.into_iter().collect(),
        documents_detail: documents,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::universal::LibraryEntry;

    fn entry(id: &str, category: &str, path: &str) -> LibraryEntry {
        LibraryEntry {
            id: id.to_string(),
            title: id.to_string(),
            category: category.to_string(),
            category_id: category.to_string(),
            source_type: "template".to_string(),
            path: path.to_string(),
            preview: String::new(),
            objective: format!("Obiettivo {id}"),
            audience: "primaria".to_string(),
            ..LibraryEntry::default()
        }
    }

    fn manifest(version: &str) -> LibraryManifest {
        LibraryManifest {
            version: 1,
            generated_at: "2026-11-23T00:00:00Z".to_string(),
            packages: vec![LibraryPackage {
                id: "aac".to_string(),
                name: "Comunicazione".to_string(),
                version: version.to_string(),
                updated_at: version.to_string(),
                documents: 1,
            }],
            entries: vec![entry("a1", "aac", "library/templates/a1.caadoc")],
            report: serde_json::Value::Null,
        }
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "caa-lib-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|value| value.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn newer_versions_are_compared_numerically() {
        assert!(is_newer("2026.11", "2026.9"));
        assert!(!is_newer("2026.9", "2026.11"));
        assert!(!is_newer("2026.11", "2026.11"));
    }

    #[test]
    fn install_then_status_reports_up_to_date() {
        let data = temp_dir("status");
        let bundled = manifest("2026.11");
        let entries = bundled.entries.clone();
        let files = vec![(
            "library/templates/a1.caadoc".to_string(),
            b"contenuto".to_vec(),
        )];
        install_entries(
            &data,
            &bundled.packages[0],
            &entries,
            &files,
            &[],
            "inclusa",
            false,
        )
        .unwrap();
        let state = status(&bundled, &data, None);
        assert_eq!(state.packages[0].status, "aggiornato");
        assert_eq!(
            state.packages[0].installed_version.as_deref(),
            Some("2026.11")
        );
        assert!(data
            .join("library/templates/a1.caadoc")
            .exists());
    }

    #[test]
    fn stale_install_is_reported_as_updatable_and_needs_confirmation() {
        let data = temp_dir("stale");
        let bundled = manifest("2026.11");
        let entries = bundled.entries.clone();
        let files = vec![(
            "library/templates/a1.caadoc".to_string(),
            b"vecchio".to_vec(),
        )];
        let mut old = bundled.packages[0].clone();
        old.version = "2026.09".to_string();
        install_entries(&data, &old, &entries, &files, &[], "inclusa", false).unwrap();
        let state = status(&bundled, &data, None);
        assert_eq!(state.packages[0].status, "aggiornabile");
        assert_eq!(state.packages[0].action, "inclusa");

        let error = install_entries(
            &data,
            &bundled.packages[0],
            &entries,
            &files,
            &[],
            "inclusa",
            false,
        )
        .unwrap_err();
        assert!(error.contains("serve conferma"));
        install_entries(
            &data,
            &bundled.packages[0],
            &entries,
            &files,
            &[],
            "inclusa",
            true,
        )
        .unwrap();
        assert_eq!(status(&bundled, &data, None).packages[0].status, "aggiornato");
    }

    #[test]
    fn paths_outside_the_library_are_refused() {
        let data = temp_dir("paths");
        assert!(safe_target(&data, "library/templates/a.caadoc").is_ok());
        assert!(safe_target(&data, "../escape.caadoc").is_err());
        assert!(safe_target(&data, "library/../../escape.caadoc").is_err());
        assert!(safe_target(&data, "documents/a.caadoc").is_err());
        assert!(safe_target(&data, "/etc/passwd").is_err());
    }

    #[test]
    fn overlay_replaces_only_its_own_category() {
        let bundled = LibraryManifest {
            version: 1,
            generated_at: "2026-11-01T00:00:00Z".to_string(),
            packages: vec![
                LibraryPackage {
                    id: "aac".to_string(),
                    name: "Comunicazione".to_string(),
                    version: "2026.11".to_string(),
                    updated_at: "2026-11-23".to_string(),
                    documents: 1,
                },
                LibraryPackage {
                    id: "schede".to_string(),
                    name: "Schede".to_string(),
                    version: "2026.11".to_string(),
                    updated_at: "2026-11-23".to_string(),
                    documents: 1,
                },
            ],
            entries: vec![
                entry("a1", "aac", "library/templates/a1.caadoc"),
                entry("s1", "schede", "library/templates/s1.caadoc"),
            ],
            report: serde_json::Value::Null,
        };
        let overlay = LibraryManifest {
            version: 1,
            generated_at: "2026-11-23T00:00:00Z".to_string(),
            packages: vec![LibraryPackage {
                id: "aac".to_string(),
                name: "Comunicazione".to_string(),
                version: "2026.12".to_string(),
                updated_at: "2026-12-01".to_string(),
                documents: 2,
            }],
            entries: vec![
                entry("a1", "aac", "library/templates/a1.caadoc"),
                entry("a2", "aac", "library/templates/a2.caadoc"),
            ],
            report: serde_json::Value::Null,
        };
        let merged = merge(&bundled, &overlay);
        assert_eq!(merged.entries.len(), 3);
        assert!(merged.entries.iter().any(|item| item.id == "s1"));
        assert!(merged.entries.iter().any(|item| item.id == "a2"));
        assert_eq!(
            merged
                .packages
                .iter()
                .find(|package| package.id == "aac")
                .map(|package| package.version.as_str()),
            Some("2026.12")
        );
    }
}
