//! Livello applicativo condiviso da CLI, server MCP e bridge della GUI.
//!
//! Ogni operazione automatizzabile ha qui una sola implementazione. La CLI e il
//! server MCP non duplicano la logica: chiamano `dispatch` con gli stessi
//! parametri JSON e ricevono lo stesso risultato.

use std::path::{Component, Path, PathBuf};

use serde_json::{json, Value};

use crate::database::SymbolFilters;
use crate::engine::Engine;
use crate::universal::UniversalDocument;
use crate::export::{caadoc, docx, html};
use crate::model::*;
use crate::{asset_manager, commands, online_search, pictogram_import};

pub mod bootstrap;
pub mod bridge;
pub mod http;
pub mod jobs;
pub mod policy;
pub mod registry;
pub use policy::{AccessMode, DispatchOptions};
pub use registry::{catalog, input_schema, methods, mcp_method_for_tool, mcp_tools, operation_json, operations_by_category, Operation, API_VERSION};

fn as_string(params: &Value, key: &str) -> Option<String> {
    params.get(key).and_then(|value| value.as_str()).map(str::to_string)
}

fn as_bool(params: &Value, key: &str, default: bool) -> bool {
    params.get(key).and_then(|value| value.as_bool()).unwrap_or(default)
}

fn as_u32(params: &Value, key: &str, default: u32) -> u32 {
    params
        .get(key)
        .and_then(|value| value.as_u64())
        .map(|value| value as u32)
        .unwrap_or(default)
}

fn options_from(params: &Value) -> Result<SymbolizeOptions, String> {
    match params.get("options") {
        Some(value) => serde_json::from_value(value.clone()).map_err(|e| format!("options: {e}")),
        None => Ok(SymbolizeOptions::default()),
    }
}

fn strip_svg(mut value: Value) -> Value {
    if let Some(tokens) = value.get_mut("tokens").and_then(|value| value.as_array_mut()) {
        for token in tokens {
            if let Some(object) = token.as_object_mut() {
                object.remove("svg");
                object.remove("image");
            }
        }
    }
    value
}

fn paragraph_texts(text: &str) -> Vec<String> {
    text.replace("\r\n", "\n")
        .split("\n\n")
        .map(str::trim)
        .filter(|paragraph| !paragraph.is_empty())
        .map(str::to_string)
        .collect()
}

fn analyze_text(engine: &Engine, text: &str, params: &Value) -> Result<Value, String> {
    let include_svg = as_bool(params, "include_svg", false);
    let document_key = as_string(params, "document_key").unwrap_or_else(|| "cli".to_string());
    let mut paragraphs = Vec::new();
    for (index, paragraph) in paragraph_texts(text).iter().enumerate() {
        let mut options = options_from(params)?;
        options.document_key = document_key.clone();
        options.paragraph_id = format!("p{}", index + 1);
        let result = engine.symbolize(paragraph, &options)?;
        let mut value = serde_json::to_value(&result).map_err(|e| e.to_string())?;
        if !include_svg {
            value = strip_svg(value);
        }
        paragraphs.push(json!({
            "index": index + 1,
            "text": paragraph,
            "result": value,
        }));
    }
    Ok(json!({
        "document_key": document_key,
        "paragraphs": paragraphs,
    }))
}

// ------------------------------------------------------------------ progetti

/// Radice dei progetti automatizzabili: `<data-dir>/projects`. Non è mai un
/// percorso arbitrario scelto dal client.
pub fn project_root(engine: &Engine) -> Result<PathBuf, String> {
    let root = engine.paths.data_dir.join("projects");
    std::fs::create_dir_all(&root)
        .map_err(|error| format!("cartella progetti {}: {error}", root.display()))?;
    std::fs::canonicalize(&root)
        .map_err(|error| format!("cartella progetti {}: {error}", root.display()))
}

/// Radice progetti esistente, senza crearla. Usata dalle operazioni di sola
/// lettura per non produrre effetti collaterali.
pub fn project_root_if_exists(engine: &Engine) -> Option<PathBuf> {
    let root = engine.paths.data_dir.join("projects");
    if root.is_dir() {
        std::fs::canonicalize(&root).ok()
    } else {
        None
    }
}

fn clean_relative(relative: &str) -> Result<PathBuf, String> {
    let path = Path::new(relative);
    if path.is_absolute() {
        return Err("percorso assoluto non ammesso: usa un percorso relativo alla radice progetti".to_string());
    }
    let mut clean = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => clean.push(part),
            Component::CurDir => {}
            _ => return Err("il percorso non può contenere `..` o prefissi di unità".to_string()),
        }
    }
    if clean.as_os_str().is_empty() {
        return Err("percorso vuoto".to_string());
    }
    Ok(clean)
}

/// Risolve un percorso relativo senza uscire dalla radice progetti, anche in
/// presenza di collegamenti simbolici. Con `create_parents` crea le cartelle
/// intermedie (sempre dentro la radice) prima di verificare il contenimento.
pub fn resolve_project_path(
    engine: &Engine,
    relative: &str,
    create_parents: bool,
) -> Result<PathBuf, String> {
    let relative = clean_relative(relative)?;
    let root = if create_parents {
        project_root(engine)?
    } else {
        project_root_if_exists(engine)
            .ok_or_else(|| "nessun progetto: la cartella progetti non esiste ancora".to_string())?
    };
    let candidate = root.join(&relative);
    if create_parents {
        if let Some(parent) = candidate.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("cartella {}: {error}", parent.display()))?;
        }
    }
    let parent = candidate.parent().unwrap_or(&root);
    let canonical_parent = std::fs::canonicalize(parent)
        .map_err(|error| format!("cartella {}: {error}", parent.display()))?;
    if !canonical_parent.starts_with(&root) {
        return Err("il percorso esce dalla radice progetti".to_string());
    }
    let name = candidate
        .file_name()
        .ok_or_else(|| "percorso senza nome di file".to_string())?;
    let target = canonical_parent.join(name);
    if target.exists() {
        let canonical = std::fs::canonicalize(&target)
            .map_err(|error| format!("{}: {error}", target.display()))?;
        if !canonical.starts_with(&root) {
            return Err("il percorso esce dalla radice progetti".to_string());
        }
        Ok(canonical)
    } else {
        Ok(target)
    }
}

fn walk_projects(
    dir: &Path,
    out: &mut Vec<Value>,
    limit: usize,
) {
    if out.len() >= limit {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<_> = entries.flatten().collect();
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        if out.len() >= limit {
            return;
        }
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            walk_projects(&path, out, limit);
            continue;
        }
        if path.extension().and_then(|value| value.to_str()) != Some("caadoc") {
            continue;
        }
        let metadata = entry.metadata().ok();
        let bytes = metadata.as_ref().map(|value| value.len()).unwrap_or(0);
        let modified = metadata
            .and_then(|value| value.modified().ok())
            .and_then(|value| value.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|value| value.as_secs());
        out.push(json!({
            "name": entry.file_name().to_string_lossy(),
            "path": path.to_string_lossy(),
            "relative": path.file_name().map(|value| value.to_string_lossy().to_string()),
            "bytes": bytes,
            "modified_unix": modified,
        }));
    }
}

pub fn document_summary(document: &UniversalDocument) -> Value {
    json!({
        "id": document.id,
        "title": document.title,
        "kind": document.kind,
        "schema_version": document.schema_version,
        "pages": document.pages.iter().map(|page| json!({
            "id": page.id,
            "title": page.title,
            "layout": page.layout,
            "elements": page.elements.len(),
            "home": page.is_home,
        })).collect::<Vec<_>>(),
        "licenses": document.licenses.len(),
        "metadata": document.metadata,
    })
}

fn save_universal_document(path: &Path, document: &UniversalDocument) -> Result<Value, String> {
    let assets = crate::universal_build::assets_from_document(document);
    let saved = crate::universal_build::save_universal(path, document, &assets)?;
    Ok(json!({
        "ok": true,
        "path": saved.path,
        "title": document.title,
        "pages": document.pages.len(),
        "elements": document.pages.iter().map(|page| page.elements.len()).sum::<usize>(),
        "warnings": saved.warnings,
    }))
}

pub fn export_universal(
    document: &UniversalDocument,
    format: &str,
    out: &Path,
) -> Result<Value, String> {
    let format = format.trim().to_ascii_lowercase();
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("cartella {}: {error}", parent.display()))?;
    }
    match format.as_str() {
        "pdf" => {
            crate::export::universal_pdf::export_pdf(document, out)?;
            Ok(json!({"ok": true, "format": "pdf", "path": out, "warnings": []}))
        }
        "png" => {
            crate::export::universal_pdf::export_first_page_png(document, out)?;
            Ok(json!({"ok": true, "format": "png", "path": out, "warnings": []}))
        }
        "html" => {
            let body = crate::universal_build::export_html(document)?;
            std::fs::write(out, body).map_err(|error| format!("{}: {error}", out.display()))?;
            Ok(json!({"ok": true, "format": "html", "path": out, "warnings": []}))
        }
        "json" => {
            let body = serde_json::to_string_pretty(document).map_err(|e| e.to_string())?;
            std::fs::write(out, body).map_err(|error| format!("{}: {error}", out.display()))?;
            Ok(json!({"ok": true, "format": "json", "path": out, "warnings": []}))
        }
        "svg" => {
            let rendered = document.render()?;
            let page = rendered
                .pages
                .first()
                .ok_or_else(|| "documento senza pagine".to_string())?;
            std::fs::write(out, crate::universal::render_page_svg(page))
                .map_err(|error| format!("{}: {error}", out.display()))?;
            Ok(json!({"ok": true, "format": "svg", "path": out, "warnings": rendered.warnings}))
        }
        "docx" => {
            let result = crate::export::universal_docx::write(out, document)?;
            Ok(json!({"ok": true, "format": "docx", "path": result.path, "warnings": result.warnings}))
        }
        "pptx" => {
            let result = crate::export::universal_pptx::write(out, document)?;
            Ok(json!({"ok": true, "format": "pptx", "path": result.path, "warnings": result.warnings}))
        }
        other => Err(format!("formato di esportazione non supportato: {other}")),
    }
}

fn markdown_catalog(version: &str, category: Option<&str>) -> String {
    let mut out = format!("# CAA Writer — capacità (API {API_VERSION}, versione {version})\n\n");
    for operation in operations_by_category(category) {
        out.push_str(&format!("## `{}` — {}\n\n", operation.name, operation.title));
        out.push_str(&format!("{}\n\n", operation.description));
        out.push_str(&format!("- Categoria: {}\n", operation.category));
        if let Some(tool) = operation.mcp {
            out.push_str(&format!("- Tool MCP: `{tool}`\n"));
        }
        out.push_str(&format!(
            "- Accesso: {}{}\n",
            if operation.mutating { "scrittura" } else { "lettura" },
            if operation.destructive { ", distruttiva (confirm=true, admin)" } else { "" }
        ));
        if !operation.params.is_empty() {
            out.push_str("- Parametri: ");
            let names: Vec<String> = operation
                .params
                .iter()
                .map(|param| {
                    format!(
                        "`{}`{}",
                        param.name,
                        if param.required { " (obbligatorio)" } else { "" }
                    )
                })
                .collect();
            out.push_str(&names.join(", "));
            out.push('\n');
        }
        if let Some(example) = operation.example {
            out.push_str(&format!("- Esempio: `{example}`\n"));
        }
        out.push('\n');
    }
    out
}

/// Elenco dei formati realmente gestiti dal renderer condiviso.
pub const EXPORT_FORMATS: &[&str] = &["pdf", "png", "html", "json", "svg", "docx", "pptx"];

pub fn dispatch(engine: &Engine, method: &str, params: &Value) -> Result<Value, String> {
    dispatch_with_options(engine, method, params, DispatchOptions::default())
}

pub fn dispatch_with_options(
    engine: &Engine,
    method: &str,
    params: &Value,
    options: DispatchOptions,
) -> Result<Value, String> {
    let method = if method == "symbolize_paragraph" {
        "symbolize"
    } else {
        method
    };
    if method != "methods" {
        let operation = registry::find(method)
            .ok_or_else(|| format!("metodo sconosciuto: {method}"))?;
        if let Some(dry_run) = policy::check(operation, params, options)? {
            return Ok(dry_run);
        }
    }
    match method {
        "app.capabilities" => {
            let category = as_string(params, "category");
            let format = as_string(params, "format").unwrap_or_else(|| "json".to_string());
            if format.eq_ignore_ascii_case("markdown") {
                return Ok(json!({
                    "format": "markdown",
                    "markdown": markdown_catalog(env!("CARGO_PKG_VERSION"), category.as_deref()),
                }));
            }
            let mut value = catalog(env!("CARGO_PKG_VERSION"), category.as_deref());
            value["access_mode"] = json!(options.access.as_str());
            value["export_formats"] = json!(EXPORT_FORMATS);
            Ok(value)
        }
        "app.status" => {
            let stats = engine.db()?.statistics().map_err(|e| e.to_string())?;
            Ok(json!({
                "application": "CAA Writer",
                "version": env!("CARGO_PKG_VERSION"),
                "api_version": API_VERSION,
                "data_dir": engine.paths.data_dir,
                "resources_dir": engine.paths.resources_dir,
                "project_root": project_root_if_exists(engine).map(|path| path.to_string_lossy().to_string()),
                "database": engine.paths.db_path,
                "stats": stats,
                "lexicon_concepts": engine.lexicon.concept_count(),
                "lexicon_lemmas": engine.lexicon.lemma_count(),
                "ai": engine.ai_status(),
                "warnings": engine.startup_warnings,
                "access_mode": options.access.as_str(),
                "gui_bridge": bridge::local_status(),
                "limitations": [
                    "Il bridge della GUI ascolta solo su localhost e richiede il token scritto nella cartella dati; senza GUI aperta CLI e MCP lavorano in headless.",
                    "I job vivono nel processo che li avvia: usa `caa-cli rpc` o il server MCP per interrogarli.",
                    "Le operazioni di rete falliscono esplicitamente quando la rete non è disponibile."
                ]
            }))
        }
        "app.diagnostics" => {
            Ok(serde_json::to_value(engine.diagnostics()).map_err(|e| e.to_string())?)
        }
        "settings.list" => {
            let prefix = as_string(params, "prefix").unwrap_or_default();
            let db = engine.db()?;
            let mut stmt = db
                .conn()
                .prepare("SELECT key, value FROM settings ORDER BY key")
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })
                .map_err(|e| e.to_string())?;
            let mut items = Vec::new();
            for row in rows {
                let (key, value) = row.map_err(|e| e.to_string())?;
                if !prefix.is_empty() && !key.starts_with(&prefix) {
                    continue;
                }
                items.push(json!({"key": key, "value": value}));
            }
            Ok(json!({"items": items, "count": items.len()}))
        }
        "settings.get" => {
            let key = as_string(params, "key").ok_or_else(|| "serve key".to_string())?;
            let db = engine.db()?;
            Ok(json!({"key": key.clone(), "value": db.setting(&key)}))
        }
        "settings.set" => {
            let key = as_string(params, "key").ok_or_else(|| "serve key".to_string())?;
            let value = params
                .get("value")
                .map(|value| match value {
                    Value::String(text) => text.clone(),
                    other => other.to_string(),
                })
                .ok_or_else(|| "serve value".to_string())?;
            let db = engine.db()?;
            let previous = db.setting(&key);
            db.set_setting(&key, &value).map_err(|e| e.to_string())?;
            Ok(json!({
                "ok": true,
                "key": key,
                "value": value,
                "changed": previous.as_deref() != Some(value.as_str()),
            }))
        }
        "project.list" => {
            let Some(root) = project_root_if_exists(engine) else {
                return Ok(json!({"root": engine.paths.data_dir.join("projects"), "items": [], "count": 0}));
            };
            let relative = as_string(params, "root").unwrap_or_default();
            let dir = if relative.is_empty() {
                root.clone()
            } else {
                resolve_project_path(engine, &relative, false)?
            };
            if !dir.is_dir() {
                return Err(format!("cartella progetti non trovata: {relative}"));
            }
            let limit = as_u32(params, "limit", 200).clamp(1, 2000) as usize;
            let mut items = Vec::new();
            walk_projects(&dir, &mut items, limit);
            Ok(json!({"root": root, "items": items, "count": items.len()}))
        }
        "project.create" => {
            let relative = as_string(params, "path").ok_or_else(|| "serve path".to_string())?;
            let target = resolve_project_path(engine, &relative, true)?;
            if target.exists() {
                return Err(format!("il progetto esiste già: {relative}"));
            }
            let title = as_string(params, "title").ok_or_else(|| "serve title".to_string())?;
            let kind = as_string(params, "kind").unwrap_or_else(|| "document".to_string());
            let layout = as_string(params, "layout").unwrap_or_else(|| "flow".to_string());
            if !matches!(layout.as_str(), "flow" | "absolute" | "grid") {
                return Err(format!("layout non valido: {layout}"));
            }
            let slug = crate::pictogram_import::slugify(&title);
            let mut document = UniversalDocument::empty(
                &format!("doc-{}-{slug}", crate::util::now_iso().replace([':', '-'], "")),
                &title,
                &kind,
            );
            if let Some(page) = document.pages.first_mut() {
                page.layout = layout;
            }
            let saved = save_universal_document(&target, &document)?;
            Ok(json!({
                "ok": true,
                "path": saved["path"],
                "relative": relative,
                "document": document_summary(&document),
                "warnings": saved["warnings"],
            }))
        }
        "project.open" => {
            let relative = as_string(params, "path").ok_or_else(|| "serve path".to_string())?;
            let target = resolve_project_path(engine, &relative, false)?;
            let document = crate::universal_build::load_universal(&target)?;
            Ok(json!({
                "ok": true,
                "path": target,
                "document": document,
                "summary": document_summary(&document),
            }))
        }
        "project.validate" => {
            let relative = as_string(params, "path").ok_or_else(|| "serve path".to_string())?;
            let target = resolve_project_path(engine, &relative, false)?;
            let document = crate::universal_build::load_universal(&target)?;
            crate::universal::validate(&document)?;
            let rendered = document.render()?;
            Ok(json!({
                "ok": true,
                "valid": true,
                "path": target,
                "document": document_summary(&document),
                "warnings": rendered.warnings,
                "pages": rendered.pages.len(),
            }))
        }
        "project.save" => {
            let relative = as_string(params, "path").ok_or_else(|| "serve path".to_string())?;
            let target = resolve_project_path(engine, &relative, true)?;
            let overwrite = as_bool(params, "overwrite", false);
            if target.exists() && !overwrite {
                return Err(format!(
                    "il progetto esiste già: usa overwrite=true per sostituirlo ({relative})"
                ));
            }
            let document: UniversalDocument = params
                .get("document")
                .cloned()
                .ok_or_else(|| "serve document".to_string())
                .and_then(|value| serde_json::from_value(value).map_err(|e| format!("document: {e}")))?;
            crate::universal::validate(&document)?;
            let saved = save_universal_document(&target, &document)?;
            Ok(json!({
                "ok": true,
                "path": saved["path"],
                "relative": relative,
                "document": document_summary(&document),
                "warnings": saved["warnings"],
            }))
        }
        "project.duplicate" | "project.rename" => {
            let relative = as_string(params, "path").ok_or_else(|| "serve path".to_string())?;
            let target_relative = as_string(params, "target").ok_or_else(|| "serve target".to_string())?;
            let source = resolve_project_path(engine, &relative, false)?;
            if !source.is_file() {
                return Err(format!("progetto non trovato: {relative}"));
            }
            let target = resolve_project_path(engine, &target_relative, true)?;
            if target.exists() {
                return Err(format!("la destinazione esiste già: {target_relative}"));
            }
            if method == "project.duplicate" {
                std::fs::copy(&source, &target)
                    .map_err(|error| format!("copia {}: {error}", source.display()))?;
            } else {
                std::fs::rename(&source, &target)
                    .map_err(|error| format!("rinomina {}: {error}", source.display()))?;
            }
            Ok(json!({
                "ok": true,
                "source": source,
                "path": target,
                "relative": target_relative,
            }))
        }
        "project.delete" => {
            let relative = as_string(params, "path").ok_or_else(|| "serve path".to_string())?;
            let target = resolve_project_path(engine, &relative, false)?;
            if !target.is_file() {
                return Err(format!("progetto non trovato: {relative}"));
            }
            std::fs::remove_file(&target)
                .map_err(|error| format!("eliminazione {}: {error}", target.display()))?;
            Ok(json!({"ok": true, "removed": target, "relative": relative}))
        }
        "project.export" => {
            let relative = as_string(params, "path").ok_or_else(|| "serve path".to_string())?;
            let format = as_string(params, "format").unwrap_or_else(|| "pdf".to_string());
            if !EXPORT_FORMATS.contains(&format.to_ascii_lowercase().as_str()) {
                return Err(format!("formato non supportato: {format}"));
            }
            let source = resolve_project_path(engine, &relative, false)?;
            let document = crate::universal_build::load_universal(&source)?;
            let out_relative = as_string(params, "out").unwrap_or_else(|| {
                let stem = Path::new(&relative)
                    .file_stem()
                    .and_then(|value| value.to_str())
                    .unwrap_or("progetto");
                format!("exports/{stem}.{}", format.to_ascii_lowercase())
            });
            let out = resolve_project_path(engine, &out_relative, true)?;
            let mut value = export_universal(&document, &format, &out)?;
            value["source"] = json!(source);
            value["relative"] = json!(out_relative);
            Ok(value)
        }
        "document.create" => {
            let document = if let Some(spec) = params.get("spec") {
                let outcome = crate::universal_build::build_from_spec(engine, spec)?;
                (outcome.document, outcome.assets, outcome.notes)
            } else {
                let title = as_string(params, "title").unwrap_or_else(|| "Documento senza titolo".to_string());
                let kind = as_string(params, "kind").unwrap_or_else(|| "document".to_string());
                let layout = as_string(params, "layout").unwrap_or_else(|| "flow".to_string());
                let slug = crate::pictogram_import::slugify(&title);
                let mut document = UniversalDocument::empty(
                    &format!("doc-{}-{slug}", crate::util::now_iso().replace([':', '-'], "")),
                    &title,
                    &kind,
                );
                if let Some(page) = document.pages.first_mut() {
                    page.layout = layout;
                }
                let assets = crate::universal_build::assets_from_document(&document);
                (document, assets, Vec::new())
            };
            let (document, assets, notes) = document;
            crate::universal::validate(&document)?;
            let mut value = json!({
                "ok": true,
                "document": document_summary(&document),
                "full": document,
                "notes": notes,
            });
            if let Some(path) = as_string(params, "path") {
                let saved = crate::universal_build::save_universal(Path::new(&path), &document, &assets)?;
                value["path"] = json!(saved.path);
                value["warnings"] = json!(saved.warnings);
            }
            Ok(value)
        }
        "document.get" => {
            let path = as_string(params, "path").ok_or_else(|| "serve path".to_string())?;
            let document = crate::universal_build::load_universal(Path::new(&path))?;
            if as_bool(params, "summary_only", false) {
                Ok(json!({"ok": true, "path": path, "document": document_summary(&document)}))
            } else {
                Ok(json!({"ok": true, "path": path, "document": document}))
            }
        }
        "document.update" => {
            let path = as_string(params, "path").ok_or_else(|| "serve path".to_string())?;
            let commands: Vec<crate::universal::DocumentCommand> = params
                .get("commands")
                .cloned()
                .ok_or_else(|| "serve commands".to_string())
                .and_then(|value| serde_json::from_value(value).map_err(|e| format!("commands: {e}")))?;
            let document = crate::universal_build::load_universal(Path::new(&path))?;
            let result = crate::universal::apply_transaction(&document, &commands)?;
            let out = as_string(params, "out").unwrap_or_else(|| path.clone());
            let saved = save_universal_document(Path::new(&out), &result.document)?;
            Ok(json!({
                "ok": true,
                "path": saved["path"],
                "document": document_summary(&result.document),
                "changed": result.changed,
                "warnings": saved["warnings"],
            }))
        }
        "document.export" => {
            let path = as_string(params, "path").ok_or_else(|| "serve path".to_string())?;
            let format = as_string(params, "format").unwrap_or_else(|| "pdf".to_string());
            let out = as_string(params, "out")
                .unwrap_or_else(|| format!("{}.{}", path.trim_end_matches(".caadoc"), format));
            let document = crate::universal_build::load_universal(Path::new(&path))?;
            export_universal(&document, &format, Path::new(&out))
        }
        "element.add" | "element.update" | "element.delete" | "element.move" | "element.duplicate" => {
            let path = as_string(params, "path").ok_or_else(|| "serve path".to_string())?;
            let page = as_string(params, "page").ok_or_else(|| "serve page".to_string())?;
            let mut document = crate::universal_build::load_universal(Path::new(&path))?;
            let command = match method {
                "element.add" => {
                    let element = params
                        .get("element")
                        .cloned()
                        .ok_or_else(|| "serve element".to_string())?;
                    crate::universal::DocumentCommand::AddElement {
                        page,
                        element,
                        index: params
                            .get("index")
                            .and_then(|value| value.as_u64())
                            .map(|value| value as usize),
                    }
                }
                "element.update" => {
                    let id = as_string(params, "id").ok_or_else(|| "serve id".to_string())?;
                    let patch = params
                        .get("patch")
                        .cloned()
                        .ok_or_else(|| "serve patch".to_string())?;
                    crate::universal::DocumentCommand::UpdateElement { page, id, patch }
                }
                "element.delete" => {
                    let id = as_string(params, "id").ok_or_else(|| "serve id".to_string())?;
                    crate::universal::DocumentCommand::RemoveElement { page, id }
                }
                "element.move" => {
                    let id = as_string(params, "id").ok_or_else(|| "serve id".to_string())?;
                    let x_mm = params
                        .get("x_mm")
                        .and_then(|value| value.as_f64())
                        .ok_or_else(|| "serve x_mm".to_string())? as f32;
                    let y_mm = params
                        .get("y_mm")
                        .and_then(|value| value.as_f64())
                        .ok_or_else(|| "serve y_mm".to_string())? as f32;
                    crate::universal::DocumentCommand::SetPosition { page, id, x_mm, y_mm }
                }
                "element.duplicate" => {
                    let id = as_string(params, "id").ok_or_else(|| "serve id".to_string())?;
                    let source = document
                        .element(&page, &id)
                        .ok_or_else(|| format!("elemento non trovato: {page}/{id}"))?
                        .clone();
                    let mut value = serde_json::to_value(&source).map_err(|e| e.to_string())?;
                    let new_id = as_string(params, "new_id")
                        .unwrap_or_else(|| format!("{id}-copy"));
                    if document.element(&page, &new_id).is_some() {
                        return Err(format!("l'ID del duplicato esiste già: {new_id}"));
                    }
                    if let Some(object) = value.as_object_mut() {
                        object.insert("id".to_string(), json!(new_id));
                        if let Some(x) = object.get("x_mm").and_then(|value| value.as_f64()) {
                            object.insert("x_mm".to_string(), json!(x + 5.0));
                        }
                        if let Some(y) = object.get("y_mm").and_then(|value| value.as_f64()) {
                            object.insert("y_mm".to_string(), json!(y + 5.0));
                        }
                    }
                    crate::universal::DocumentCommand::AddElement {
                        page,
                        element: value,
                        index: None,
                    }
                }
                _ => unreachable!(),
            };
            crate::universal::apply_command(&mut document, &command)?;
            crate::universal::validate(&document)?;
            let out = as_string(params, "out").unwrap_or_else(|| path.clone());
            let saved = save_universal_document(Path::new(&out), &document)?;
            Ok(json!({
                "ok": true,
                "path": saved["path"],
                "document": document_summary(&document),
                "warnings": saved["warnings"],
            }))
        }
        "content.generate" => {
            let kind = as_string(params, "kind").ok_or_else(|| "serve kind".to_string())?;
            let out = as_string(params, "out");
            match kind.as_str() {
                "material" => {
                    let prompt = as_string(params, "prompt")
                        .ok_or_else(|| "serve prompt per un materiale".to_string())?;
                    let template_id = as_string(params, "template_id");
                    let templates = crate::studio::list_templates(&engine.paths.templates_dir())?;
                    if templates.is_empty() {
                        return Err("nessun modello disponibile in resources/templates".to_string());
                    }
                    let (template, mut labels, mut notes) = match template_id {
                        Some(id) => (
                            templates.iter().find(|template| template.id == id).cloned(),
                            Vec::new(),
                            Vec::new(),
                        ),
                        None => {
                            let (template, labels, notes) = crate::studio::interpret_prompt(&prompt, &templates);
                            (template, labels, notes)
                        }
                    };
                    let template = template.ok_or_else(|| "modello non trovato".to_string())?;
                    if labels.is_empty() {
                        labels = template.items.iter().map(|item| item.label.clone()).collect();
                        notes.push("Contenuti predefiniti del modello.".to_string());
                    }
                    labels.truncate(60);
                    let options = SymbolizeOptions {
                        paragraph_id: "studio".to_string(),
                        intensity: Intensity::Minima,
                        mode: CaaMode::Sopra,
                        ..Default::default()
                    };
                    let mut items = Vec::new();
                    for label in labels {
                        let result = engine.symbolize(&label, &options)?;
                        let token = result.tokens.iter().find(|token| {
                            !token.is_punct
                                && (token.symbol_id.is_some() || !token.composition.is_empty())
                        });
                        match token {
                            Some(token) => items.push(crate::studio::PlannedItem {
                                label: label.clone(),
                                concept_id: token.concept_id.clone(),
                                strategy: token.strategy.clone(),
                                symbol_id: token.symbol_id.clone(),
                                svg: token.svg.clone(),
                                image: token.image.clone(),
                                approximate: token.approximate,
                            }),
                            None => {
                                items.push(crate::studio::PlannedItem {
                                    label: label.clone(),
                                    concept_id: None,
                                    strategy: "none".to_string(),
                                    symbol_id: None,
                                    svg: None,
                                    image: None,
                                    approximate: false,
                                });
                                notes.push(format!("«{label}» non ha un pittogramma adeguato."));
                            }
                        }
                    }
                    let plan = crate::studio::ResourcePlan {
                        template_id: template.id.clone(),
                        title: template.name.clone(),
                        kind: template.kind.clone(),
                        grid: template.grid.clone().unwrap_or(crate::studio::GridSpec {
                            columns: 3,
                            rows: 3,
                            caption: true,
                            symbol: true,
                            border: true,
                            color_coding: "categorie".to_string(),
                        }),
                        items,
                        notes,
                        source: "deterministico".to_string(),
                    };
                    let document = crate::universal_build::from_plan(&plan)?;
                    let mut value = json!({
                        "ok": true,
                        "kind": "material",
                        "plan": plan,
                        "document": document_summary(&document),
                        "warnings": [],
                    });
                    if let Some(out) = out {
                        let assets = crate::universal_build::assets_from_document(&document);
                        let saved = crate::universal_build::save_universal(Path::new(&out), &document, &assets)?;
                        value["path"] = json!(saved.path);
                        value["warnings"] = json!(saved.warnings);
                    }
                    Ok(value)
                }
                "story" => {
                    let spec = params
                        .get("spec")
                        .cloned()
                        .ok_or_else(|| "story richiede il parametro spec".to_string())?;
                    let outcome = crate::story::build_story(engine, &spec)?;
                    let mut value = json!({
                        "ok": true,
                        "kind": "story",
                        "document": document_summary(&outcome.document),
                        "notes": outcome.notes,
                        "warnings": [],
                    });
                    if let Some(out) = out {
                        let saved = crate::story::save_story(Path::new(&out), &outcome.document, &outcome.assets)?;
                        value["path"] = json!(saved.path);
                        value["warnings"] = json!(saved.warnings);
                    }
                    Ok(value)
                }
                "layout" => {
                    let spec: crate::layout::LayoutSpecification = params
                        .get("spec")
                        .cloned()
                        .ok_or_else(|| "layout richiede spec".to_string())
                        .and_then(|value| serde_json::from_value(value).map_err(|e| format!("spec: {e}")))?;
                    let layout = engine.layout_document(spec)?;
                    Ok(json!({"ok": true, "kind": "layout", "layout": layout}))
                }
                "image" => {
                    let prompt = as_string(params, "prompt")
                        .ok_or_else(|| "serve prompt per l'immagine".to_string())?;
                    let negative = as_string(params, "negative").unwrap_or_default();
                    let db = engine.db()?;
                    let image = crate::image_engine::generate(&engine.paths, &db, &prompt, &negative)?;
                    let mut value = serde_json::to_value(&image).map_err(|e| e.to_string())?;
                    if let Some(out) = out {
                        std::fs::copy(&image.path, &out)
                            .map_err(|error| format!("copia immagine in {out}: {error}"))?;
                        value["saved_to"] = json!(out);
                    }
                    value["ok"] = json!(true);
                    value["kind"] = json!("image");
                    Ok(value)
                }
                other => Err(format!("tipo di contenuto non supportato: {other}")),
            }
        }
        "content.transform" => {
            let prompt = as_string(params, "prompt").ok_or_else(|| "serve prompt".to_string())?;
            let plan = engine.interpret_document_prompt(&prompt, as_bool(params, "use_neural", true));
            let mut value = json!({
                "ok": true,
                "plan": plan,
                "applied": false,
            });
            if let Some(path) = as_string(params, "document_path") {
                let commands: Vec<crate::universal::DocumentCommand> = params
                    .get("commands")
                    .cloned()
                    .ok_or_else(|| "con document_path serve commands".to_string())
                    .and_then(|value| serde_json::from_value(value).map_err(|e| format!("commands: {e}")))?;
                let document = crate::universal_build::load_universal(Path::new(&path))?;
                let result = crate::universal::apply_transaction(&document, &commands)?;
                let out = as_string(params, "out").unwrap_or_else(|| path.clone());
                let saved = save_universal_document(Path::new(&out), &result.document)?;
                value["applied"] = json!(true);
                value["path"] = saved["path"].clone();
                value["document"] = document_summary(&result.document);
                value["changed"] = json!(result.changed);
            }
            Ok(value)
        }
        "content.validate" => {
            if let Some(path) = as_string(params, "path") {
                let document = crate::universal_build::load_universal(Path::new(&path))?;
                let mut value = json!({"ok": true, "kind": "document", "path": path});
                match crate::universal::validate(&document) {
                    Ok(()) => {
                        value["valid"] = json!(true);
                        let rendered = document.render()?;
                        value["warnings"] = json!(rendered.warnings);
                        value["pages"] = json!(rendered.pages.len());
                    }
                    Err(error) => {
                        value["valid"] = json!(false);
                        value["errors"] = json!([error]);
                    }
                }
                return Ok(value);
            }
            let text = as_string(params, "text").ok_or_else(|| "serve text o path".to_string())?;
            let mut missing = Vec::new();
            let mut tokens = 0usize;
            for (index, paragraph) in paragraph_texts(&text).iter().enumerate() {
                let mut options = options_from(params)?;
                options.paragraph_id = format!("p{}", index + 1);
                let result = engine.symbolize(paragraph, &options)?;
                for token in &result.tokens {
                    if token.is_punct {
                        continue;
                    }
                    tokens += 1;
                    if token.symbol_id.is_none() && token.composition.is_empty() {
                        missing.push(json!({"paragraph": index + 1, "word": token.surface, "strategy": token.strategy}));
                    }
                }
            }
            Ok(json!({
                "ok": true,
                "kind": "text",
                "valid": missing.is_empty(),
                "tokens": tokens,
                "missing": missing,
                "warnings": [],
            }))
        }
        "job.start" => {
            let method = as_string(params, "method").ok_or_else(|| "serve method".to_string())?;
            let inner = params.get("params").cloned().unwrap_or_else(|| json!({}));
            jobs::start(
                &method,
                &inner,
                engine.paths.data_dir.clone(),
                engine.paths.resources_dir.clone(),
                options.access,
            )
        }
        "job.status" => {
            let id = as_string(params, "id").ok_or_else(|| "serve id".to_string())?;
            jobs::status(&id)
        }
        "job.result" => {
            let id = as_string(params, "id").ok_or_else(|| "serve id".to_string())?;
            jobs::result(&id)
        }
        "job.cancel" => {
            let id = as_string(params, "id").ok_or_else(|| "serve id".to_string())?;
            jobs::cancel(&id)
        }
        "job.list" => Ok(jobs::list()),
        "info" => {
            let stats = engine.db()?.statistics().map_err(|e| e.to_string())?;
            let templates = crate::studio::list_templates(&engine.paths.templates_dir())
                .map(|items| items.len())
                .unwrap_or(0);
            Ok(json!({
                "version": env!("CARGO_PKG_VERSION"),
                "data_dir": engine.paths.data_dir,
                "resources_dir": engine.paths.resources_dir,
                "stats": stats,
                "lexicon_concepts": engine.lexicon.concept_count(),
                "lexicon_lemmas": engine.lexicon.lemma_count(),
                "templates": templates,
                "warnings": engine.startup_warnings,
            }))
        }
        "methods" => Ok(json!({
            "methods": registry::methods(),
            "api_version": API_VERSION,
            "hint": "Usa app.capabilities per schemi, categorie e modalità di accesso."
        })),
        "packages.catalog" => {
            let catalog = crate::packages::catalog(engine)?;
            Ok(serde_json::to_value(catalog).map_err(|e| e.to_string())?)
        }
        "packages.updates" => {
            let info = crate::packages::check_updates(engine);
            Ok(serde_json::to_value(info).map_err(|e| e.to_string())?)
        }
        "packages.download" => {
            let id = as_string(params, "id").ok_or_else(|| "serve il parametro id".to_string())?;
            let limit = params.get("limit").and_then(|v| v.as_u64()).unwrap_or(5) as usize;
            let report = match id.as_str() {
                "symbols:arasaac" => crate::packages::download_arasaac(engine, limit.max(1))?,
                "symbols:globalsymbols" => {
                    crate::packages::download_globalsymbols(engine, limit.max(1))?
                }
                _ => return Err(format!("pacchetto non scaricabile: {id}")),
            };
            Ok(serde_json::to_value(report).map_err(|e| e.to_string())?)
        }
        "packages.set_library" => {
            let id = as_string(params, "id").ok_or_else(|| "serve il parametro id".to_string())?;
            let enabled = params
                .get("enabled")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            crate::packages::set_library_enabled(
                &engine.paths.data_dir,
                &id,
                enabled,
            )?;
            Ok(json!({ "id": id, "enabled": enabled }))
        }
        "packages.remove" => {
            let id = as_string(params, "id").ok_or_else(|| "serve il parametro id".to_string())?;
            let pack = id
                .strip_prefix("symbols:")
                .ok_or_else(|| "si rimuovono solo pacchetti symbols:".to_string())?;
            crate::packages::remove_symbol_pack(engine, pack)?;
            Ok(json!({ "removed": id }))
        }
        "symbolize" | "symbolize_paragraph" => {
            let text = as_string(params, "text").ok_or_else(|| "serve il parametro text".to_string())?;
            let options = options_from(params)?;
            let result = engine.symbolize(&text, &options)?;
            let value = serde_json::to_value(&result).map_err(|e| e.to_string())?;
            Ok(if as_bool(params, "include_svg", false) {
                value
            } else {
                strip_svg(value)
            })
        }
        "analyze.text" => {
            let text = as_string(params, "text").ok_or_else(|| "serve il parametro text".to_string())?;
            analyze_text(engine, &text, params)
        }
        "analyze.caadoc" => {
            let path = as_string(params, "path").ok_or_else(|| "serve il parametro path".to_string())?;
            let loaded = caadoc::load(Path::new(&path))?;
            let mut value = analyze_text(engine, &loaded.plain_text, params)?;
            if let Some(object) = value.as_object_mut() {
                object.insert("title".to_string(), json!(loaded.title));
                object.insert("warnings".to_string(), json!(loaded.warnings));
            }
            Ok(value)
        }
        "document.open" => {
            let path = as_string(params, "path").ok_or_else(|| "serve il parametro path".to_string())?;
            let loaded = caadoc::load(Path::new(&path))?;
            Ok(serde_json::to_value(&loaded).map_err(|e| e.to_string())?)
        }
        "document.save" => {
            let path = as_string(params, "path").ok_or_else(|| "serve il parametro path".to_string())?;
            let payload: DocumentPayload = params
                .get("payload")
                .cloned()
                .ok_or_else(|| "serve il parametro payload".to_string())
                .and_then(|value| serde_json::from_value(value).map_err(|e| format!("payload: {e}")))?;
            let result = caadoc::save(Path::new(&path), &payload, &[])?;
            Ok(serde_json::to_value(&result).map_err(|e| e.to_string())?)
        }
        "search" => {
            let query = as_string(params, "query").unwrap_or_default();
            let filters = SymbolFilters {
                pack_id: as_string(params, "pack_id"),
                commercial_only: as_bool(params, "commercial_only", false),
                collection_id: as_string(params, "collection_id"),
            };
            let page = engine
                .db()?
                .search_symbols(
                    &query,
                    as_u32(params, "limit", 40),
                    as_u32(params, "offset", 0),
                    &filters,
                )
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(&page).map_err(|e| e.to_string())?)
        }
        "affinities" => {
            let word = as_string(params, "word").ok_or_else(|| "serve il parametro word".to_string())?;
            let affinities = engine.word_affinities(
                &word,
                as_string(params, "pack_id"),
                as_bool(params, "commercial_only", false),
            )?;
            Ok(serde_json::to_value(&affinities).map_err(|e| e.to_string())?)
        }
        "templates.list" => {
            let templates = crate::studio::list_templates(&engine.paths.templates_dir())?;
            let category = as_string(params, "category").map(|value| value.to_lowercase());
            let query = as_string(params, "query").map(|value| value.to_lowercase());
            let filtered: Vec<_> = templates
                .into_iter()
                .filter(|template| {
                    category
                        .as_ref()
                        .map(|value| template.category.to_lowercase() == *value)
                        .unwrap_or(true)
                        && query
                            .as_ref()
                            .map(|value| {
                                format!(
                                    "{} {} {}",
                                    template.name, template.description, template.category
                                )
                                .to_lowercase()
                                .contains(value)
                            })
                            .unwrap_or(true)
                })
                .collect();
            Ok(serde_json::to_value(&filtered).map_err(|e| e.to_string())?)
        }
        "templates.get" => {
            let id = as_string(params, "id").ok_or_else(|| "serve il parametro id".to_string())?;
            let template = crate::studio::find_template(
                &engine.paths.templates_dir(),
                &id,
            )?
            .ok_or_else(|| format!("modello {id} non trovato"))?;
            Ok(serde_json::to_value(&template).map_err(|e| e.to_string())?)
        }
        "templates.plan" => {
            let id = as_string(params, "template_id");
            let prompt = as_string(params, "prompt");
            let templates = crate::studio::list_templates(&engine.paths.templates_dir())?;
            let template = match id {
                Some(id) => templates.into_iter().find(|template| template.id == id),
                None => prompt
                    .as_deref()
                    .map(|prompt| crate::studio::interpret_prompt(prompt, &templates).0)
                    .unwrap_or(None),
            }
            .ok_or_else(|| "modello non trovato o richiesta senza modello".to_string())?;
            let mut options = SymbolizeOptions {
                paragraph_id: "studio".to_string(),
                intensity: Intensity::Minima,
                mode: CaaMode::Sopra,
                ..Default::default()
            };
            options.pack_id = as_string(params, "pack_id");
            let mut items = Vec::new();
            let mut notes = Vec::new();
            for item in &template.items {
                let result = engine.symbolize(&item.label, &options)?;
                let token = result
                    .tokens
                    .iter()
                    .find(|token| !token.is_punct && (token.symbol_id.is_some() || !token.composition.is_empty()));
                items.push(json!({
                    "label": item.label,
                    "concept": item.concept,
                    "token": token,
                }));
            }
            if items.is_empty() {
                notes.push("Il modello non contiene elementi.".to_string());
            }
            Ok(json!({
                "template": template,
                "items": items,
                "notes": notes,
            }))
        }
        "layout.build" => {
            let spec: crate::layout::LayoutSpecification = params
                .get("spec")
                .cloned()
                .ok_or_else(|| "serve il parametro spec".to_string())
                .and_then(|value| serde_json::from_value(value).map_err(|e| format!("spec: {e}")))?;
            let document = engine.layout_document(spec)?;
            Ok(serde_json::to_value(&document).map_err(|e| e.to_string())?)
        }
        "layout.edit" => {
            let spec: crate::layout::LayoutSpecification = params
                .get("spec")
                .cloned()
                .ok_or_else(|| "serve il parametro spec".to_string())
                .and_then(|value| serde_json::from_value(value).map_err(|e| format!("spec: {e}")))?;
            let prompt = as_string(params, "prompt");
            let operations = match params.get("operations") {
                Some(value) => Some(
                    serde_json::from_value::<Vec<crate::layout_edit::LayoutOperation>>(
                        value.clone(),
                    )
                    .map_err(|e| format!("operations: {e}"))?,
                ),
                None => None,
            };
            let result = engine.edit_layout(spec, prompt, operations)?;
            Ok(serde_json::to_value(&result).map_err(|e| e.to_string())?)
        }
        "import.scan" => {
            let paths: Vec<String> = params
                .get("paths")
                .and_then(|value| value.as_array())
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| item.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default();
            if paths.is_empty() {
                return Err("serve il parametro paths (array di file o cartelle)".to_string());
            }
            let mut preview = pictogram_import::scan(&paths, 160);
            let db = engine.db()?;
            let mut known = std::collections::HashSet::new();
            if let Ok(mut stmt) = db.conn().prepare("SELECT checksum FROM symbols WHERE checksum IS NOT NULL") {
                if let Ok(rows) = stmt.query_map([], |row| row.get::<_, String>(0)) {
                    for row in rows.flatten() {
                        known.insert(row);
                    }
                }
            }
            for entry in &mut preview.entries {
                if let Some(hash) = &entry.checksum {
                    if known.contains(hash) && !entry.duplicate {
                        entry.duplicate = true;
                        preview.duplicates += 1;
                    }
                }
            }
            Ok(serde_json::to_value(&preview).map_err(|e| e.to_string())?)
        }
        "import.run" => {
            let entries: Vec<pictogram_import::ImportItem> = params
                .get("entries")
                .cloned()
                .ok_or_else(|| "serve il parametro entries".to_string())
                .and_then(|value| serde_json::from_value(value).map_err(|e| format!("entries: {e}")))?;
            let collection = as_string(params, "collection").unwrap_or_else(|| "Pittogrammi importati".to_string());
            let license: pictogram_import::ImportLicense = match params.get("license") {
                Some(value) => serde_json::from_value(value.clone())
                    .map_err(|e| format!("license: {e}"))?,
                None => pictogram_import::ImportLicense::default(),
            };
            let db = engine.db()?;
            let report = pictogram_import::import_batch(
                &db,
                &engine.paths.assets_dir,
                &entries,
                &license,
                &collection,
            );
            engine.clear_caches();
            Ok(serde_json::to_value(&report).map_err(|e| e.to_string())?)
        }
        "collections.list" => {
            let db = engine.db()?;
            Ok(serde_json::to_value(pictogram_import::list_collections(&db)?)
                .map_err(|e| e.to_string())?)
        }
        "collections.rename" => {
            let id = as_string(params, "id").ok_or_else(|| "serve id".to_string())?;
            let name = as_string(params, "name").ok_or_else(|| "serve name".to_string())?;
            let db = engine.db()?;
            pictogram_import::rename_collection(&db, &id, &name)?;
            Ok(json!({ "ok": true }))
        }
        "collections.delete" => {
            let id = as_string(params, "id").ok_or_else(|| "serve id".to_string())?;
            let db = engine.db()?;
            let removed = pictogram_import::delete_collection(&db, &engine.paths.assets_dir, &id)?;
            engine.clear_caches();
            Ok(json!({ "removed": removed }))
        }
        "collections.delete_symbols" => {
            let ids: Vec<String> = params
                .get("ids")
                .and_then(|value| value.as_array())
                .map(|items| items.iter().filter_map(|item| item.as_str().map(str::to_string)).collect())
                .unwrap_or_default();
            let db = engine.db()?;
            let removed = pictogram_import::delete_symbols(&db, &engine.paths.assets_dir, &ids)?;
            engine.clear_caches();
            Ok(json!({ "removed": removed }))
        }
        "collections.move" => {
            let ids: Vec<String> = params
                .get("ids")
                .and_then(|value| value.as_array())
                .map(|items| items.iter().filter_map(|item| item.as_str().map(str::to_string)).collect())
                .unwrap_or_default();
            let collection_id = as_string(params, "collection_id").ok_or_else(|| "serve collection_id".to_string())?;
            let db = engine.db()?;
            let moved = pictogram_import::move_symbols(&db, &ids, &collection_id)?;
            engine.clear_caches();
            Ok(json!({ "moved": moved }))
        }
        "online.providers" => Ok(json!({ "providers": online_search::providers() })),
        "online.search" => {
            let terms: Vec<String> = params
                .get("terms")
                .and_then(|value| value.as_array())
                .map(|items| items.iter().filter_map(|item| item.as_str().map(str::to_string)).collect())
                .unwrap_or_default();
            let terms = online_search::terms_for_search(&engine.lexicon, &terms);
            if terms.is_empty() {
                return Err("nessun termine valido per la ricerca online".to_string());
            }
            let db = engine.db()?;
            let response = online_search::search(&db, &terms, as_bool(params, "force_refresh", false))?;
            Ok(serde_json::to_value(&response).map_err(|e| e.to_string())?)
        }
        "online.ensure" => {
            let words: Vec<String> = params
                .get("words")
                .and_then(|value| value.as_array())
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| item.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default();
            let db = engine.db()?;
            let saved = online_search::ensure_arasaac_symbols(
                &db,
                &engine.paths.assets_dir,
                &words,
                12,
            )?;
            if !saved.is_empty() {
                engine.clear_caches();
            }
            Ok(serde_json::json!({ "saved": saved }))
        }
        "online.save" => {
            let result: online_search::OnlineResult = params
                .get("result")
                .cloned()
                .ok_or_else(|| "serve result".to_string())
                .and_then(|value| serde_json::from_value(value).map_err(|e| format!("result: {e}")))?;
            let lemma = as_string(params, "lemma").ok_or_else(|| "serve lemma".to_string())?;
            let collection = as_string(params, "collection").unwrap_or_else(|| "Online".to_string());
            let db = engine.db()?;
            let symbol_id = online_search::save_result(
                &db,
                &engine.paths.assets_dir,
                &result,
                &lemma,
                &collection,
            )?;
            engine.clear_caches();
            let record = db.symbol_by_id(&symbol_id)?.ok_or_else(|| "simbolo non trovato".to_string())?;
            Ok(serde_json::to_value(crate::asset_manager::alternative_from_record(&record))
                .map_err(|e| e.to_string())?)
        }
        "boards.list" => Ok(serde_json::to_value(crate::profiles::list_boards(
            &engine.paths.boards_dir(),
        )?)
        .map_err(|e| e.to_string())?),
        "boards.build" => {
            let id = as_string(params, "id").ok_or_else(|| "serve id".to_string())?;
            let board = crate::profiles::build_board(
                engine,
                &engine.paths.boards_dir(),
                &id,
            )?;
            Ok(serde_json::to_value(&board).map_err(|e| e.to_string())?)
        }
        "profiles.list" => {
            let db = engine.db()?;
            Ok(serde_json::to_value(crate::profiles::list_profiles(&db)?)
                .map_err(|e| e.to_string())?)
        }
        "prompt.interpret" => {
            let prompt = as_string(params, "prompt").ok_or_else(|| "serve prompt".to_string())?;
            let plan = engine.interpret_document_prompt(&prompt, as_bool(params, "use_neural", true));
            Ok(serde_json::to_value(&plan).map_err(|e| e.to_string())?)
        }
        "library.catalog" => {
            let manifest = crate::packages::filtered_manifest(
                &engine.paths.resources_dir,
                &engine.paths.data_dir,
            );
            Ok(serde_json::to_value(crate::template_catalog::overview(&manifest))
                .map_err(|e| e.to_string())?)
        }
        "library.search" => {
            let manifest = crate::packages::filtered_manifest(
                &engine.paths.resources_dir,
                &engine.paths.data_dir,
            );
            let raw = params
                .get("query")
                .cloned()
                .unwrap_or_else(|| params.clone());
            let query: crate::template_catalog::CatalogQuery =
                serde_json::from_value(raw).map_err(|e| format!("query: {e}"))?;
            let query = if query.query.is_none() {
                crate::template_catalog::CatalogQuery {
                    query: as_string(params, "q"),
                    ..query
                }
            } else {
                query
            };
            let mut page = crate::template_catalog::search(&manifest, &query);
            for item in &mut page.items {
                item.preview_svg =
                    crate::template_catalog::preview_svg(&engine.paths.resources_dir, &item.entry);
            }
            Ok(serde_json::to_value(page).map_err(|e| e.to_string())?)
        }
        "library.validate" => {
            let manifest = crate::universal::load_library_manifest(&engine.paths.resources_dir);
            let report = crate::library_quality::validate_manifest(
                &engine.paths.resources_dir,
                &manifest,
            );
            Ok(serde_json::to_value(report).map_err(|e| e.to_string())?)
        }
        "export.html" | "export.docx" => {
            let path = as_string(params, "path").ok_or_else(|| "serve path".to_string())?;
            let document: ExportDoc = params
                .get("document")
                .cloned()
                .ok_or_else(|| "serve document".to_string())
                .and_then(|value| serde_json::from_value(value).map_err(|e| format!("document: {e}")))?;
            let result = if method == "export.html" {
                html::write(Path::new(&path), &document)?
            } else {
                docx::write(Path::new(&path), &document)?
            };
            Ok(serde_json::to_value(&result).map_err(|e| e.to_string())?)
        }
        "voice.status" => {
            let status = crate::voice::status(
                &engine.paths.resources_dir,
                &engine.paths.data_dir,
                as_string(params, "language").as_deref(),
            );
            Ok(serde_json::to_value(&status).map_err(|e| e.to_string())?)
        }
        "voice.speak" => {
            let text = as_string(params, "text").ok_or_else(|| "serve text".to_string())?;
            let language = as_string(params, "language");
            if let Some(paths) = crate::voice::resolve(
                &engine.paths.resources_dir,
                &engine.paths.data_dir,
                language.as_deref(),
            ) {
                match crate::voice::synthesize(&text, &paths) {
                    Ok(wav) => {
                        crate::voice::play(&wav)?;
                        return Ok(json!({ "engine": "neural", "wav": wav }));
                    }
                    Err(error) => {
                        crate::speech::speak(&text, None, None)?;
                        return Ok(json!({ "engine": "system", "fallback_reason": error }));
                    }
                }
            }
            crate::speech::speak(&text, None, None)?;
            Ok(json!({ "engine": "system" }))
        }
        "ai.chat" => {
            let messages: Vec<crate::ai_provider::ChatMessage> = params
                .get("messages")
                .cloned()
                .ok_or_else(|| "serve messages".to_string())
                .and_then(|value| serde_json::from_value(value).map_err(|e| format!("messages: {e}")))?;
            let reply = engine.chat(messages)?;
            Ok(serde_json::to_value(&reply).map_err(|e| e.to_string())?)
        }
        "ai.status" => Ok(serde_json::to_value(engine.ai_status()).map_err(|e| e.to_string())?),
        "ai.models" => Ok(serde_json::to_value(engine.ai_models()).map_err(|e| e.to_string())?),
        "ai.words" => {
            let word = as_string(params, "word").ok_or_else(|| "serve il parametro word".to_string())?;
            Ok(json!({ "word": word, "words": engine.ai_suggested_words(&word) }))
        }
        "ai.affinity" => {
            let word = as_string(params, "word").ok_or_else(|| "serve il parametro word".to_string())?;
            let items = engine.ai_affinity_symbols(&word);
            Ok(json!({ "word": word, "items": items }))
        }
        "ai.tool" => {
            let tool = as_string(params, "tool").ok_or_else(|| "serve tool".to_string())?;
            let input = params.get("input").cloned().unwrap_or_else(|| json!({}));
            let outcome = engine.run_ai_tool(&tool, &input)?;
            Ok(serde_json::to_value(&outcome).map_err(|e| e.to_string())?)
        }
        "translate.search" => {
            let text = as_string(params, "text").ok_or_else(|| "serve text".to_string())?;
            let translated = engine.translate_search_term(&text)?;
            Ok(json!({ "source": text, "translated": translated }))
        }
        "document.pdf" => {
            let document: Value = params
                .get("document")
                .cloned()
                .ok_or_else(|| "serve document".to_string())?;
            let document: UniversalDocument = serde_json::from_value(document)
                .map_err(|e| format!("documento non valido: {e}"))?;
            crate::universal::validate(&document)?;
            let out = as_string(params, "out")
                .ok_or_else(|| "serve out".to_string())?;
            let bytes = crate::export::universal_pdf::build_pdf(&document, 2.0)?;
            std::fs::write(&out, &bytes).map_err(|e| format!("scrittura PDF: {e}"))?;
            Ok(json!({ "ok": true, "path": out, "bytes": bytes.len() }))
        }
        "symbol.get" => {
            let id = as_string(params, "id").ok_or_else(|| "serve id".to_string())?;
            let db = engine.db()?;
            let record = db.symbol_by_id(&id)?.ok_or_else(|| "simbolo non trovato".to_string())?;
            Ok(json!({
                "id": record.id,
                "label": record.label,
                "pack_id": record.pack_id,
                "svg_len": record.svg.as_ref().map(|value| value.len()).unwrap_or(0),
                "image_len": record.image.as_ref().map(|value| value.len()).unwrap_or(0),
            }))
        }
        "symbol.resolve" => {
            let words: Vec<String> = params
                .get("words")
                .and_then(|value| value.as_array())
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| item.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default();
            let results = commands::resolve_missing_symbols_for_engine(&engine, &words)?;
            Ok(json!({ "items": results }))
        }
        "symbol.hydrate" => {
            let ids: Vec<String> = params
                .get("ids")
                .and_then(|value| value.as_array())
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| item.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default();
            let db = engine.db()?;
            let map = online_search::hydrate_arasaac_symbols(
                &db,
                &engine.paths.assets_dir,
                &ids,
            )?;
            if !map.is_empty() {
                engine.clear_caches();
            }
            Ok(json!({ "hydrated": map.keys().collect::<Vec<_>>(), "count": map.len() }))
        }
        "online.english" => {
            let word = as_string(params, "word").ok_or_else(|| "serve il parametro word".to_string())?;
            let limit = params
                .get("limit")
                .and_then(|value| value.as_u64())
                .unwrap_or(3)
                .clamp(1, 8) as usize;
            let term = engine.translate_to_english(&word)?;
            let db = engine.db()?;
            let response = online_search::search_global_symbols(&db, &term)?;
            let mut saved = Vec::new();
            let mut warnings = response.warnings;
            for result in response
                .results
                .into_iter()
                .filter(|result| {
                    result.kind == "pittogramma"
                        && !result.source.to_ascii_lowercase().contains("arasaac")
                })
                .take(limit)
            {
                let label = if result.title.trim().is_empty() {
                    term.clone()
                } else {
                    result.title.clone()
                };
                match online_search::save_result(
                    &db,
                    &engine.paths.assets_dir,
                    &result,
                    &label,
                    "Inglese (Global Symbols)",
                ) {
                    Ok(symbol_id) => {
                        if let Ok(Some(record)) = db.symbol_by_id(&symbol_id) {
                            let mut alternative = asset_manager::alternative_from_record(&record);
                            alternative.pack_name = "Global Symbols (EN)".to_string();
                            saved.push(alternative);
                        }
                    }
                    Err(error) => warnings.push(error),
                }
            }
            engine.clear_caches();
            Ok(json!({ "term": term, "items": saved, "warnings": warnings }))
        }
        "online.global" => {
            let term = as_string(params, "term").ok_or_else(|| "serve il parametro term".to_string())?;
            let db = engine.db()?;
            let response = online_search::search_global_symbols(&db, &term)?;
            Ok(json!({
                "term": term,
                "results": response.results,
                "warnings": response.warnings,
            }))
        }
        "online.autofill" => {
            let words: Vec<String> = params
                .get("words")
                .and_then(|value| value.as_array())
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| item.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default();
            let db = engine.db()?;
            let saved = online_search::autofill_arasaac(
                &db,
                &engine.paths.assets_dir,
                &words,
                12,
            )?;
            if !saved.is_empty() {
                engine.clear_caches();
            }
            Ok(json!({ "saved": saved }))
        }
        "image.status" => {
            let db = engine.db()?;
            let status = crate::image_engine::status(&engine.paths, &db);
            Ok(serde_json::to_value(&status).map_err(|e| e.to_string())?)
        }
        "image.generate" => {
            let prompt = as_string(params, "prompt").ok_or_else(|| "serve prompt".to_string())?;
            let negative = as_string(params, "negative").unwrap_or_default();
            let db = engine.db()?;
            let image = crate::image_engine::generate(
                &engine.paths,
                &db,
                &prompt,
                &negative,
            )?;
            let mut value = serde_json::to_value(&image).map_err(|e| e.to_string())?;
            if let Some(out) = as_string(params, "out") {
                std::fs::copy(&image.path, &out)
                    .map_err(|e| format!("copia immagine in {out}: {e}"))?;
                value["saved_to"] = serde_json::Value::String(out);
            }
            if !as_bool(params, "include_base64", false) {
                value["png_base64"] = serde_json::Value::String(String::new());
            }
            Ok(value)
        }
        "image.install" => {
            let message = crate::image_engine::install_runtime(&engine.paths)?;
            Ok(json!({ "ok": true, "message": message }))
        }
        "image.download" => {
            let model = as_string(params, "model").ok_or_else(|| "serve model".to_string())?;
            let message = crate::image_engine::download_model(&engine.paths, &model)?;
            Ok(json!({ "ok": true, "message": message }))
        }
        "translate.english" => {
            let text = as_string(params, "text").ok_or_else(|| "serve text".to_string())?;
            let translated = engine.translate_to_english(&text)?;
            Ok(json!({ "source": text, "translated": translated }))
        }
        "english.alternatives" => {
            let word = as_string(params, "word").ok_or_else(|| "serve word".to_string())?;
            let limit = params
                .get("limit")
                .and_then(|value| value.as_u64())
                .unwrap_or(4) as usize;
            let term = engine.translate_to_english(&word)?;
            let response = online_search::fetch_arasaac_in_language(&term, "en")?;
            let db = engine.db()?;
            let lemma = term.clone();
            let mut items = Vec::new();
            let mut warnings = response.warnings;
            for result in response.results.into_iter().take(limit.clamp(1, 8)) {
                match online_search::save_result(
                    &db,
                    &engine.paths.assets_dir,
                    &result,
                    &lemma,
                    "Inglese (ARASAAC)",
                ) {
                    Ok(symbol_id) => {
                        if let Ok(Some(record)) = db.symbol_by_id(&symbol_id) {
                            let alternative =
                                crate::asset_manager::alternative_from_record(&record);
                            if items
                                .iter()
                                .all(|item: &serde_json::Value| item["symbol_id"] != alternative.symbol_id)
                            {
                                items.push(serde_json::to_value(&alternative).map_err(|e| e.to_string())?);
                            }
                        }
                    }
                    Err(error) => warnings.push(error),
                }
            }
            if !items.is_empty() {
                engine.clear_caches();
            }
            Ok(json!({ "term": term, "items": items, "warnings": warnings }))
        }
        "speak" => {
            let text = as_string(params, "text").ok_or_else(|| "serve text".to_string())?;
            let language = as_string(params, "language");
            if let Some(paths) = crate::voice::resolve(
                &engine.paths.resources_dir,
                &engine.paths.data_dir,
                language.as_deref(),
            ) {
                if let Ok(wav) = crate::voice::synthesize(&text, &paths) {
                    crate::voice::play(&wav)?;
                    return Ok(json!({ "engine": "neural", "wav": wav }));
                }
            }
            crate::speech::speak(&text, None, None)?;
            Ok(json!({ "engine": "system" }))
        }
        other => Err(format!("metodo sconosciuto: {other}")),
    }
}




#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::AppPaths;

    fn test_engine() -> (tempfile::TempDir, Engine) {
        let dir = tempfile::tempdir().expect("tempdir");
        let resources = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("radice repository")
            .join("resources");
        let paths = AppPaths::new(dir.path().to_path_buf(), resources);
        let engine = Engine::new(paths).expect("motore di test");
        (dir, engine)
    }

    #[test]
    fn every_registered_operation_is_routed() {
        let (_dir, engine) = test_engine();
        for operation in registry::OPERATIONS {
            // `packages.updates` interroga la rete: la sua presenza nel registro
            // è sufficiente, l'esecuzione reale è verificata dai test di rete.
            if operation.name == "packages.updates" {
                continue;
            }
            let outcome = dispatch_with_options(
                &engine,
                operation.name,
                &json!({}),
                DispatchOptions::read_only(),
            );
            if let Err(error) = outcome {
                assert_ne!(
                    error,
                    format!("metodo sconosciuto: {}", operation.name),
                    "operazione registrata ma non instradata: {}",
                    operation.name
                );
            }
        }
    }

    #[test]
    fn unknown_methods_are_rejected() {
        let (_dir, engine) = test_engine();
        let error = dispatch(&engine, "does.not.exist", &json!({})).unwrap_err();
        assert!(error.contains("metodo sconosciuto"));
    }

    #[test]
    fn project_paths_cannot_escape_root() {
        let (_dir, engine) = test_engine();
        assert!(resolve_project_path(&engine, "../fuori.caadoc", false).is_err());
        assert!(resolve_project_path(&engine, "/tmp/fuori.caadoc", false).is_err());
        assert!(resolve_project_path(&engine, "a/../../fuori.caadoc", false).is_err());
        assert!(resolve_project_path(&engine, "ok.caadoc", true).is_ok());
    }

    #[cfg(unix)]
    #[test]
    fn project_symlinks_cannot_escape_root() {
        let (dir, engine) = test_engine();
        let outside = dir.path().join("outside");
        std::fs::create_dir_all(&outside).unwrap();
        let root = project_root(&engine).unwrap();
        std::os::unix::fs::symlink(&outside, root.join("link")).unwrap();
        assert!(resolve_project_path(&engine, "link/evil.caadoc", true).is_err());
    }

    #[test]
    fn project_lifecycle_and_confirmation() {
        let (_dir, engine) = test_engine();
        let created = dispatch(
            &engine,
            "project.create",
            &json!({"path": "esercizi/cane.caadoc", "title": "Il cane", "kind": "document"}),
        )
        .expect("creazione progetto");
        assert_eq!(created["document"]["title"], "Il cane");

        let opened = dispatch(&engine, "project.open", &json!({"path": "esercizi/cane.caadoc"}))
            .expect("apertura");
        assert_eq!(opened["document"]["title"], "Il cane");

        dispatch(&engine, "project.validate", &json!({"path": "esercizi/cane.caadoc"}))
            .expect("validazione");

        let duplicate = dispatch(
            &engine,
            "project.duplicate",
            &json!({"path": "esercizi/cane.caadoc", "target": "esercizi/cane-2.caadoc"}),
        )
        .expect("duplicazione");
        assert!(duplicate["path"].as_str().unwrap().ends_with("cane-2.caadoc"));

        let list = dispatch(&engine, "project.list", &json!({})).expect("elenco");
        assert_eq!(list["count"], 2);

        let html = dispatch(
            &engine,
            "project.export",
            &json!({"path": "esercizi/cane.caadoc", "format": "html"}),
        )
        .expect("esportazione html");
        assert_eq!(html["format"], "html");

        let denied = dispatch(
            &engine,
            "project.delete",
            &json!({"path": "esercizi/cane-2.caadoc"}),
        )
        .unwrap_err();
        assert!(denied.contains("conferma"));

        dispatch(
            &engine,
            "project.delete",
            &json!({"path": "esercizi/cane-2.caadoc", "confirm": true}),
        )
        .expect("eliminazione confermata");
    }

    #[test]
    fn dry_run_does_not_create_files() {
        let (_dir, engine) = test_engine();
        let value = dispatch_with_options(
            &engine,
            "project.create",
            &json!({"path": "dry.caadoc", "title": "Dry"}),
            DispatchOptions::default().with_dry_run(true),
        )
        .expect("dry run");
        assert_eq!(value["dry_run"], true);
        let list = dispatch(&engine, "project.list", &json!({})).unwrap();
        assert_eq!(list["count"], 0);
    }

    #[test]
    fn read_mode_rejects_writes() {
        let (_dir, engine) = test_engine();
        let error = dispatch_with_options(
            &engine,
            "project.create",
            &json!({"path": "x.caadoc", "title": "X"}),
            DispatchOptions::read_only(),
        )
        .unwrap_err();
        assert!(error.contains("read"));
    }

    #[test]
    fn document_and_element_workflow() {
        let (dir, engine) = test_engine();
        let path = dir.path().join("elementi.caadoc");
        let path_text = path.to_string_lossy().to_string();
        dispatch(
            &engine,
            "document.create",
            &json!({"title": "Elementi", "path": path_text}),
        )
        .expect("creazione documento");

        let loaded = dispatch(&engine, "document.get", &json!({"path": path_text.clone()}))
            .expect("lettura documento");
        let page = loaded["document"]["pages"][0]["id"]
            .as_str()
            .unwrap_or("page-1")
            .to_string();

        dispatch(
            &engine,
            "element.add",
            &json!({
                "path": path_text.clone(),
                "page": page,
                "element": {"id": "e1", "kind": "text", "text": "Ciao", "x_mm": 10, "y_mm": 10, "w_mm": 40, "h_mm": 12}
            }),
        )
        .expect("aggiunta elemento");

        dispatch(
            &engine,
            "element.update",
            &json!({"path": path_text.clone(), "page": page, "id": "e1", "patch": {"text": "Ciao mondo"}}),
        )
        .expect("modifica elemento");

        let duplicated = dispatch(
            &engine,
            "element.duplicate",
            &json!({"path": path_text.clone(), "page": page, "id": "e1", "new_id": "e2"}),
        )
        .expect("duplicazione elemento");
        assert_eq!(duplicated["document"]["pages"][0]["elements"], 2);

        let validation = dispatch(
            &engine,
            "content.validate",
            &json!({"path": path_text.clone()}),
        )
        .expect("validazione contenuto");
        assert_eq!(validation["valid"], true);

        dispatch_with_options(
            &engine,
            "element.delete",
            &json!({"path": path_text, "page": page, "id": "e2", "confirm": true}),
            DispatchOptions::default(),
        )
        .expect("eliminazione elemento");
    }

    #[test]
    fn settings_roundtrip_and_catalog() {
        let (_dir, engine) = test_engine();
        dispatch(&engine, "settings.set", &json!({"key": "test.chiave", "value": "valore"}))
            .expect("scrittura impostazione");
        let read = dispatch(&engine, "settings.get", &json!({"key": "test.chiave"})).unwrap();
        assert_eq!(read["value"], "valore");
        let catalog = dispatch(&engine, "app.capabilities", &json!({})).unwrap();
        assert_eq!(catalog["api_version"], API_VERSION);
        assert!(catalog["operations"].as_array().unwrap().len() > 50);
        let markdown = dispatch(
            &engine,
            "app.capabilities",
            &json!({"format": "markdown"}),
        )
        .unwrap();
        assert!(markdown["markdown"].as_str().unwrap().contains("project.create"));
    }
}
