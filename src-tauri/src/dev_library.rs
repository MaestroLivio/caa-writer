//! Modalità sviluppatore per i pacchetti della libreria.
//!
//! Attiva solo nelle build di sviluppo (debug) e solo se il processo è stato
//! avviato con `CAA_WRITER_DEV_LIBRARY=1` oppure con l'argomento
//! `--dev-library`. Consente di elencare, aprire, anteprimare, salvare,
//! sostituire, aggiungere ed eliminare documenti dentro `resources/library`.
//!
//! Le operazioni sono volutamente separate dal normale dispatch: in una build
//! di release `enabled()` restituisce sempre `false`, quindi nessun comando
//! può modificare la libreria installata.

use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use serde::Serialize;

use crate::engine::Engine;
use crate::template_catalog;
use crate::universal::{LibraryEntry, LibraryManifest, UniversalDocument};
use crate::universal_build;

static ENABLED: AtomicBool = AtomicBool::new(false);

pub fn set_enabled(value: bool) {
    ENABLED.store(value, Ordering::Relaxed);
}

/// Legge `CAA_WRITER_DEV_LIBRARY=1` o l'argomento `--dev-library`.
/// Nelle build di release non attiva nulla.
pub fn configure_from_process() {
    let requested = std::env::var("CAA_WRITER_DEV_LIBRARY")
        .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
        || std::env::args().any(|argument| argument == "--dev-library");
    if requested && cfg!(debug_assertions) {
        set_enabled(true);
    }
}

/// La modalità sviluppatore è disponibile solo nelle build di debug.
pub fn enabled() -> bool {
    cfg!(debug_assertions) && ENABLED.load(Ordering::Relaxed)
}

fn ensure_enabled() -> Result<(), String> {
    if enabled() {
        Ok(())
    } else {
        Err(
            "Modalità sviluppatore non attiva. Avvia la versione dev con \
             `npm run dev:library` oppure con l'argomento `--dev-library`."
                .to_string(),
        )
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DevStatus {
    pub enabled: bool,
    pub resources_dir: String,
    pub manifest_path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DevDocument {
    pub id: String,
    pub title: String,
    pub path: String,
    pub source_type: String,
    pub kind: String,
    pub pages: usize,
    pub elements: usize,
    pub missing_symbols: usize,
    pub category_id: String,
    pub subcategory_id: String,
    pub subcategory: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DevPackage {
    pub id: String,
    pub name: String,
    pub description: String,
    pub count: usize,
    pub documents: Vec<DevDocument>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DevSaveResult {
    pub path: String,
    pub entry: LibraryEntry,
}

#[derive(Debug, Clone, Serialize)]
pub struct DevDeleteResult {
    pub removed: bool,
    pub path: String,
}

pub fn status(resources: &Path) -> DevStatus {
    DevStatus {
        enabled: enabled(),
        resources_dir: resources.to_string_lossy().to_string(),
        manifest_path: resources
            .join("library/manifest.json")
            .to_string_lossy()
            .to_string(),
    }
}

fn manifest_path(resources: &Path) -> PathBuf {
    resources.join("library/manifest.json")
}

fn library_root(resources: &Path) -> PathBuf {
    resources.join("library")
}

/// Vero se il percorso (esistente o di destinazione) è dentro `library`.
pub fn is_inside_library(resources: &Path, path: &Path) -> bool {
    let root = library_root(resources);
    let canonical_root = match root.canonicalize() {
        Ok(value) => value,
        Err(_) => return false,
    };
    let candidate = if path.exists() {
        match path.canonicalize() {
            Ok(value) => value,
            Err(_) => return false,
        }
    } else {
        match path.parent().and_then(|parent| parent.canonicalize().ok()) {
            Some(parent) => parent.join(path.file_name().unwrap_or_default()),
            None => return false,
        }
    };
    candidate.starts_with(&canonical_root)
}

fn read_manifest(resources: &Path) -> Result<LibraryManifest, String> {
    let path = manifest_path(resources);
    let raw = std::fs::read_to_string(&path)
        .map_err(|e| format!("lettura catalogo libreria {}: {e}", path.display()))?;
    serde_json::from_str(&raw).map_err(|e| format!("catalogo libreria non valido: {e}"))
}

fn write_manifest(resources: &Path, manifest: &LibraryManifest) -> Result<(), String> {
    let path = manifest_path(resources);
    let raw = serde_json::to_string_pretty(manifest)
        .map_err(|e| format!("serializzazione catalogo libreria: {e}"))?;
    std::fs::write(&path, raw).map_err(|e| format!("scrittura catalogo libreria: {e}"))
}

/// Converte un percorso in un percorso relativo a `resources`, con separatori
/// `/` come nel catalogo. Restituisce `None` se il file non è nella libreria.
fn relative_to_resources(resources: &Path, path: &Path) -> Option<String> {
    let root = library_root(resources);
    let canonical_root = root.canonicalize().ok()?;
    let canonical = path.canonicalize().ok()?;
    let relative = canonical.strip_prefix(&canonical_root).ok()?;
    let rel = Path::new("library").join(relative);
    Some(rel.to_string_lossy().replace('\\', "/"))
}

fn normalize_relative(relative: &str) -> Result<PathBuf, String> {
    let trimmed = relative.trim();
    if trimmed.is_empty() {
        return Err("Percorso vuoto".to_string());
    }
    let mut path = PathBuf::from(trimmed);
    if path.is_absolute() {
        return Err("Il percorso deve essere relativo alla cartella della libreria".to_string());
    }
    if path.starts_with("library") {
        path = path.strip_prefix("library").map_err(|_| "Percorso non valido".to_string())?.to_path_buf();
    }
    for component in path.components() {
        match component {
            Component::Normal(_) => {}
            _ => return Err("Il percorso contiene componenti non consentiti".to_string()),
        }
    }
    Ok(path)
}

/// Risolve un percorso esistente dentro `resources/library`.
pub fn resolve_existing(resources: &Path, relative: &str) -> Result<PathBuf, String> {
    let rel = normalize_relative(relative)?;
    let root = library_root(resources);
    let candidate = root.join(&rel);
    let canonical_root = root
        .canonicalize()
        .map_err(|e| format!("cartella libreria non accessibile: {e}"))?;
    let canonical = candidate
        .canonicalize()
        .map_err(|e| format!("documento non trovato ({}): {e}", candidate.display()))?;
    if !canonical.starts_with(&canonical_root) {
        return Err("Il percorso esce dalla cartella della libreria".to_string());
    }
    if canonical.extension().and_then(|e| e.to_str()) != Some("caadoc") {
        return Err("Il documento deve avere estensione .caadoc".to_string());
    }
    Ok(canonical)
}

/// Risolve un percorso di destinazione, eventualmente non ancora esistente.
pub fn resolve_target(resources: &Path, relative: &str) -> Result<PathBuf, String> {
    let rel = normalize_relative(relative)?;
    let root = library_root(resources);
    if rel.components().next().is_none() {
        return Err("Percorso di destinazione vuoto".to_string());
    }
    let target = root.join(&rel);
    if target.extension().and_then(|e| e.to_str()) != Some("caadoc") {
        return Err("Il documento di destinazione deve avere estensione .caadoc".to_string());
    }
    let parent = target
        .parent()
        .ok_or_else(|| "Percorso di destinazione senza cartella".to_string())?;
    if !parent.exists() {
        return Err(format!("Cartella di destinazione inesistente: {}", parent.display()));
    }
    let canonical_root = root
        .canonicalize()
        .map_err(|e| format!("cartella libreria non accessibile: {e}"))?;
    let canonical_parent = parent
        .canonicalize()
        .map_err(|e| format!("cartella di destinazione non accessibile: {e}"))?;
    if !canonical_parent.starts_with(&canonical_root) {
        return Err("La destinazione esce dalla cartella della libreria".to_string());
    }
    if target.exists() && !enabled() {
        return Err("La libreria è protetta: avvia la modalità sviluppatore per modificarla".to_string());
    }
    Ok(target)
}

pub fn packages(engine: &Engine) -> Result<Vec<DevPackage>, String> {
    ensure_enabled()?;
    let resources = &engine.paths.resources_dir;
    let manifest = read_manifest(resources)?;
    let mut map: std::collections::BTreeMap<String, DevPackage> = std::collections::BTreeMap::new();
    for entry in &manifest.entries {
        let class = template_catalog::classify(entry);
        let category_id = if entry.category_id.is_empty() {
            class.category_id
        } else {
            entry.category_id.clone()
        };
        let category_name = if entry.category.is_empty() {
            class.category
        } else {
            entry.category.clone()
        };
        let package = map.entry(category_id.clone()).or_insert_with(|| DevPackage {
            id: category_id.clone(),
            name: category_name,
            description: String::new(),
            count: 0,
            documents: Vec::new(),
        });
        package.count += 1;
        package.documents.push(DevDocument {
            id: entry.id.clone(),
            title: entry.title.clone(),
            path: entry.path.clone(),
            source_type: entry.source_type.clone(),
            kind: entry.kind.clone(),
            pages: entry.pages,
            elements: entry.elements,
            missing_symbols: entry.missing_symbols,
            category_id: category_id.clone(),
            subcategory_id: if entry.subcategory_id.is_empty() {
                class.subcategory_id
            } else {
                entry.subcategory_id.clone()
            },
            subcategory: if entry.subcategory.is_empty() {
                class.subcategory
            } else {
                entry.subcategory.clone()
            },
        });
    }
    let mut packages: Vec<DevPackage> = map.into_values().collect();
    for package in &mut packages {
        if package.description.is_empty() {
            package.description = format!(
                "Pacchetto «{}» con {} documenti.",
                package.name, package.count
            );
        }
        package.documents.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
    }
    packages.sort_by(|a, b| b.count.cmp(&a.count).then(a.name.cmp(&b.name)));
    Ok(packages)
}

pub fn open_document(engine: &Engine, relative: &str) -> Result<crate::commands::OpenedUniversalDocument, String> {
    ensure_enabled()?;
    let path = resolve_existing(&engine.paths.resources_dir, relative)?;
    let document = crate::universal::load_from_caadoc(&path)?;
    Ok(crate::commands::OpenedUniversalDocument {
        document,
        path: path.to_string_lossy().to_string(),
    })
}

pub fn preview(engine: &Engine, relative: &str) -> Result<String, String> {
    ensure_enabled()?;
    let path = resolve_existing(&engine.paths.resources_dir, relative)?;
    let document = crate::universal::load_from_caadoc(&path)?;
    let rendered = document.render()?;
    rendered
        .pages
        .first()
        .map(crate::universal::render_page_svg)
        .ok_or_else(|| "documento senza pagine".to_string())
}

pub fn save_document(
    engine: &Engine,
    document: &UniversalDocument,
    relative: &str,
) -> Result<DevSaveResult, String> {
    ensure_enabled()?;
    let path = resolve_target(&engine.paths.resources_dir, relative)?;
    let assets = universal_build::assets_from_document(document);
    universal_build::save_universal(&path, document, &assets)?;
    let result = refresh_entry_for_path(engine, &path)?;
    Ok(result)
}

pub fn replace_document(
    engine: &Engine,
    relative: &str,
    source: &Path,
) -> Result<DevSaveResult, String> {
    ensure_enabled()?;
    if source.extension().and_then(|e| e.to_str()) != Some("caadoc") {
        return Err("Il file scelto deve essere un documento .caadoc".to_string());
    }
    let source_document = crate::universal::load_from_caadoc(source)?;
    let target = resolve_target(&engine.paths.resources_dir, relative)?;
    let assets = universal_build::assets_from_document(&source_document);
    universal_build::save_universal(&target, &source_document, &assets)?;
    let result = refresh_entry_for_path_preserving(engine, &target)?;
    Ok(result)
}

pub fn add_document(
    engine: &Engine,
    source: &Path,
    folder: &str,
    category_id: &str,
    subcategory_id: &str,
    category_name: &str,
    subcategory_name: &str,
) -> Result<DevSaveResult, String> {
    ensure_enabled()?;
    if folder != "boards" && folder != "templates" {
        return Err("Cartella di destinazione non valida: usa «boards» o «templates»".to_string());
    }
    if source.extension().and_then(|e| e.to_str()) != Some("caadoc") {
        return Err("Il file scelto deve essere un documento .caadoc".to_string());
    }
    let mut document = crate::universal::load_from_caadoc(source)?;
    let manifest = read_manifest(&engine.paths.resources_dir)?;
    let base_id = if document.id.trim().is_empty() {
        crate::pictogram_import::slugify(&document.title)
    } else {
        crate::pictogram_import::slugify(&document.id)
    };
    let base_id = if base_id.is_empty() { "documento".to_string() } else { base_id };
    let id = unique_id(&manifest, &engine.paths.resources_dir, folder, &base_id);
    document.id = id.clone();
    let relative = format!("library/{folder}/{id}.caadoc");
    let target = resolve_target(&engine.paths.resources_dir, &relative)?;
    let assets = universal_build::assets_from_document(&document);
    universal_build::save_universal(&target, &document, &assets)?;
    let mut entry = entry_from_document(
        &document,
        &id,
        if folder == "boards" { "board" } else { "template" },
        category_id,
        subcategory_id,
        category_name,
        subcategory_name,
        &format!("library/{folder}/{id}.caadoc"),
    );
    let class = template_catalog::classify(&entry);
    if entry.category_id.is_empty() {
        entry.category_id = class.category_id;
    }
    if entry.subcategory_id.is_empty() {
        entry.subcategory_id = class.subcategory_id;
    }
    if entry.subcategory.is_empty() {
        entry.subcategory = class.subcategory;
    }
    if entry.category.is_empty() {
        entry.category = class.category;
    }
    if entry.audience.is_empty() {
        entry.audience = class.audience;
    }
    if entry.objective.is_empty() {
        entry.objective = class.objective;
    }
    if entry.tags.is_empty() {
        entry.tags = class.tags;
    }
    if entry.complexity.is_empty() {
        entry.complexity = class.complexity;
    }
    entry.preview = format!("library/previews/{id}.svg");
    let mut manifest = manifest;
    manifest.entries.push(entry.clone());
    manifest.generated_at = crate::util::now_iso();
    write_manifest(&engine.paths.resources_dir, &manifest)?;
    write_preview(&engine.paths.resources_dir, &entry, &document)?;
    Ok(DevSaveResult {
        path: target.to_string_lossy().to_string(),
        entry,
    })
}

pub fn delete_document(engine: &Engine, relative: &str) -> Result<DevDeleteResult, String> {
    ensure_enabled()?;
    let path = resolve_existing(&engine.paths.resources_dir, relative)?;
    let rel = relative_to_resources(&engine.paths.resources_dir, &path)
        .ok_or_else(|| "Percorso non riconosciuto nella libreria".to_string())?;
    let mut manifest = read_manifest(&engine.paths.resources_dir)?;
    let index = manifest
        .entries
        .iter()
        .position(|entry| entry.path == rel)
        .ok_or_else(|| format!("Voce di catalogo non trovata per {rel}"))?;
    let entry = manifest.entries.remove(index);
    let preview_path = engine.paths.resources_dir.join(&entry.preview);
    if preview_path.exists() {
        let _ = std::fs::remove_file(&preview_path);
    }
    std::fs::remove_file(&path).map_err(|e| format!("eliminazione documento: {e}"))?;
    manifest.generated_at = crate::util::now_iso();
    write_manifest(&engine.paths.resources_dir, &manifest)?;
    Ok(DevDeleteResult {
        removed: true,
        path: rel,
    })
}

/// Aggiorna la voce di catalogo dopo un salvataggio normale dell'editor.
pub fn refresh_entry_for_path(engine: &Engine, path: &Path) -> Result<DevSaveResult, String> {
    ensure_enabled()?;
    refresh_entry_for_path_preserving(engine, path)
}

fn refresh_entry_for_path_preserving(engine: &Engine, path: &Path) -> Result<DevSaveResult, String> {
    let resources = &engine.paths.resources_dir;
    let rel = relative_to_resources(resources, path)
        .ok_or_else(|| "Il documento non è dentro la libreria".to_string())?;
    let document = crate::universal::load_from_caadoc(path)?;
    let mut manifest = read_manifest(resources)?;
    let index = manifest
        .entries
        .iter()
        .position(|entry| entry.path == rel);
    let new_entry = match index {
        Some(index) => {
            let existing = manifest.entries[index].clone();
            let mut entry = entry_from_document(
                &document,
                &existing.id,
                &existing.source_type,
                &existing.category_id,
                &existing.subcategory_id,
                &existing.category,
                &existing.subcategory,
                &rel,
            );
            // Conserva la tassonomia e le scelte editoriali esistenti.
            entry.category_id = existing.category_id;
            entry.subcategory_id = existing.subcategory_id;
            entry.category = existing.category;
            entry.subcategory = existing.subcategory;
            entry.audience = existing.audience;
            entry.objective = existing.objective;
            entry.tags = existing.tags;
            entry.complexity = existing.complexity;
            entry.printable = existing.printable;
            entry.customizable = existing.customizable;
            entry.featured = existing.featured;
            entry.id = existing.id.clone();
            entry.preview = if existing.preview.is_empty() {
                format!("library/previews/{}.svg", existing.id)
            } else {
                existing.preview
            };
            manifest.entries[index] = entry.clone();
            entry
        }
        None => {
            let id = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("documento")
                .to_string();
            let source_type = if path
                .components()
                .any(|component| component.as_os_str() == "boards")
            {
                "board"
            } else {
                "template"
            };
            let mut entry = entry_from_document(
                &document,
                &id,
                source_type,
                "",
                "",
                "",
                "",
                &rel,
            );
            let class = template_catalog::classify(&entry);
            if entry.category_id.is_empty() {
                entry.category_id = class.category_id;
            }
            if entry.subcategory_id.is_empty() {
                entry.subcategory_id = class.subcategory_id;
            }
            if entry.subcategory.is_empty() {
                entry.subcategory = class.subcategory;
            }
            if entry.category.is_empty() {
                entry.category = class.category;
            }
            if entry.audience.is_empty() {
                entry.audience = class.audience;
            }
            if entry.objective.is_empty() {
                entry.objective = class.objective;
            }
            if entry.tags.is_empty() {
                entry.tags = class.tags;
            }
            if entry.complexity.is_empty() {
                entry.complexity = class.complexity;
            }
            entry.preview = format!("library/previews/{id}.svg");
            manifest.entries.push(entry.clone());
            entry
        }
    };
    manifest.generated_at = crate::util::now_iso();
    write_manifest(resources, &manifest)?;
    write_preview(resources, &new_entry, &document)?;
    Ok(DevSaveResult {
        path: path.to_string_lossy().to_string(),
        entry: new_entry,
    })
}

fn write_preview(resources: &Path, entry: &LibraryEntry, document: &UniversalDocument) -> Result<(), String> {
    let preview = if entry.preview.is_empty() {
        format!("library/previews/{}.svg", entry.id)
    } else {
        entry.preview.clone()
    };
    let path = resources.join(&preview);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("cartella anteprima: {e}"))?;
    }
    let svg = universal_build::preview_svg(document)?;
    std::fs::write(&path, svg).map_err(|e| format!("scrittura anteprima: {e}"))
}

fn entry_from_document(
    document: &UniversalDocument,
    id: &str,
    source_type: &str,
    category_id: &str,
    subcategory_id: &str,
    category_name: &str,
    subcategory_name: &str,
    relative: &str,
) -> LibraryEntry {
    let pages = document.pages.len();
    let elements = document.pages.iter().map(|page| page.elements.len()).sum();
    let missing_symbols = count_missing_symbols(document);
    let interactive = document.pages.iter().flat_map(|page| page.elements.iter()).any(|element| {
        element
            .action
            .as_ref()
            .map(|action| action.action_type != "speak" && action.action_type != "none")
            .unwrap_or(false)
    });
    let needs_symbols = document.pages.iter().flat_map(|page| page.elements.iter()).any(|element| {
        element.symbol_id.is_some()
            || element.svg.is_some()
            || element
                .grid
                .as_ref()
                .map(|grid| grid.cells.iter().any(|cell| cell.symbol_id.is_some() || cell.svg.is_some()))
                .unwrap_or(false)
    });
    LibraryEntry {
        id: id.to_string(),
        title: document.title.clone(),
        category: category_name.to_string(),
        description: document.description.clone(),
        source_type: source_type.to_string(),
        kind: document.kind.clone(),
        pages,
        elements,
        interactive,
        missing_symbols,
        path: relative.to_string(),
        preview: format!("library/previews/{id}.svg"),
        category_id: category_id.to_string(),
        subcategory_id: subcategory_id.to_string(),
        subcategory: subcategory_name.to_string(),
        audience: document
            .metadata
            .get("audience")
            .and_then(|value| value.as_str())
            .unwrap_or("primaria")
            .to_string(),
        objective: document.description.clone(),
        tags: Vec::new(),
        complexity: if elements > 24 {
            "articolato".to_string()
        } else if elements > 8 {
            "intermedio".to_string()
        } else {
            "base".to_string()
        },
        needs_symbols,
        printable: true,
        customizable: true,
        featured: false,
    }
}

fn count_missing_symbols(document: &UniversalDocument) -> usize {
    let mut missing = 0usize;
    for page in &document.pages {
        for element in &page.elements {
            if element
                .metadata
                .get("missing_symbol")
                .and_then(|value| value.as_bool())
                .unwrap_or(false)
            {
                missing += 1;
            }
            if element
                .grid
                .as_ref()
                .map(|grid| grid.cells.iter().filter(|cell| cell.missing_symbol).count())
                .unwrap_or(0)
                > 0
            {
                missing += element
                    .grid
                    .as_ref()
                    .map(|grid| grid.cells.iter().filter(|cell| cell.missing_symbol).count())
                    .unwrap_or(0);
            }
        }
    }
    missing
}

fn unique_id(
    manifest: &LibraryManifest,
    resources: &Path,
    folder: &str,
    base: &str,
) -> String {
    let mut candidate = base.to_string();
    let mut counter = 2usize;
    loop {
        let exists_in_manifest = manifest.entries.iter().any(|entry| entry.id == candidate);
        let exists_on_disk = resources
            .join("library")
            .join(folder)
            .join(format!("{candidate}.caadoc"))
            .exists();
        if !exists_in_manifest && !exists_on_disk {
            return candidate;
        }
        candidate = format!("{base}_{counter}");
        counter += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::AppPaths;
    use crate::universal::{add_grid_page, build_document, UniversalDocument};
    use std::sync::Mutex;

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    fn copy_dir(source: &Path, target: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(target)?;
        for entry in std::fs::read_dir(source)? {
            let entry = entry?;
            let path = entry.path();
            let destination = target.join(entry.file_name());
            if path.is_dir() {
                copy_dir(&path, &destination)?;
            } else {
                std::fs::copy(&path, &destination)?;
            }
        }
        Ok(())
    }

    fn sample_document(title: &str) -> UniversalDocument {
        let mut document = build_document("dev-sample", title, "mixed", "A4", "portrait");
        add_grid_page(&mut document, "home", "Home", 2, 2).unwrap();
        document
    }

    fn write_sample(path: &Path, title: &str) {
        let document = sample_document(title);
        let assets = universal_build::assets_from_document(&document);
        universal_build::save_universal(path, &document, &assets).unwrap();
    }

    #[test]
    fn developer_library_add_replace_save_delete_roundtrip() {
        let _guard = TEST_LOCK.lock().unwrap();
        set_enabled(true);
        let temp = tempfile::tempdir().unwrap();
        let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("resources");
        let resources = temp.path().join("resources");
        std::fs::create_dir_all(&resources).unwrap();
        copy_dir(&repo.join("dictionaries"), &resources.join("dictionaries")).unwrap();
        copy_dir(&repo.join("symbol-packs/core"), &resources.join("symbol-packs/core")).unwrap();
        std::fs::create_dir_all(resources.join("library/boards")).unwrap();
        std::fs::create_dir_all(resources.join("library/templates")).unwrap();
        std::fs::create_dir_all(resources.join("library/previews")).unwrap();
        std::fs::write(
            resources.join("library/manifest.json"),
            serde_json::to_string_pretty(&LibraryManifest::default()).unwrap(),
        )
        .unwrap();
        let engine = Engine::new(AppPaths::new(temp.path().join("data"), resources.clone())).unwrap();

        let source = temp.path().join("source.caadoc");
        write_sample(&source, "Documento di prova");
        let added = add_document(
            &engine,
            &source,
            "templates",
            "aac",
            "tabelle",
            "Comunicazione aumentativa e alternativa",
            "Tabelle di comunicazione",
        )
        .unwrap();
        assert!(Path::new(&added.path).exists());
        assert_eq!(added.entry.title, "Documento di prova");

        let packages = packages(&engine).unwrap();
        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].documents.len(), 1);
        assert_eq!(packages[0].documents[0].title, "Documento di prova");

        let opened = open_document(&engine, &added.entry.path).unwrap();
        assert_eq!(opened.document.title, "Documento di prova");
        assert!(!preview(&engine, &added.entry.path).unwrap().is_empty());

        let mut renamed = opened.document;
        renamed.title = "Documento rinominato".to_string();
        let saved = save_document(&engine, &renamed, &added.entry.path).unwrap();
        assert_eq!(saved.entry.title, "Documento rinominato");

        let replacement = temp.path().join("replacement.caadoc");
        write_sample(&replacement, "Sostituto");
        let replaced = replace_document(&engine, &added.entry.path, &replacement).unwrap();
        assert_eq!(replaced.entry.title, "Sostituto");
        assert_eq!(replaced.entry.id, added.entry.id);

        let deleted = delete_document(&engine, &added.entry.path).unwrap();
        assert!(deleted.removed);
        assert!(!Path::new(&added.path).exists());
        let manifest = read_manifest(&resources).unwrap();
        assert!(manifest.entries.is_empty());
        set_enabled(false);
    }

    #[test]
    fn developer_library_refuses_paths_outside_library() {
        let _guard = TEST_LOCK.lock().unwrap();
        set_enabled(true);
        let temp = tempfile::tempdir().unwrap();
        let resources = temp.path().join("resources");
        std::fs::create_dir_all(resources.join("library/templates")).unwrap();
        std::fs::create_dir_all(resources.join("library/previews")).unwrap();
        std::fs::write(
            resources.join("library/manifest.json"),
            serde_json::to_string_pretty(&LibraryManifest::default()).unwrap(),
        )
        .unwrap();
        assert!(resolve_existing(&resources, "../evil.caadoc").is_err());
        assert!(resolve_target(&resources, "library/templates/../evil.caadoc").is_err());
        assert!(resolve_target(&resources, "library/templates/ok.caadoc").is_ok());
        set_enabled(false);
    }
}
