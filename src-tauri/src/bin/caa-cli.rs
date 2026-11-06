//! Interfaccia a riga di comando di CAA Writer.
//!
//! Pensata per un coding agent e per l'automazione: ogni metodo riceve e
//! restituisce JSON, non apre finestre e non richiede interazione. Il comando
//! `rpc` mantiene un'unica istanza del motore e legge richieste JSONL da stdin.

use std::io::{BufRead, Read, Write};
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use serde_json::{json, Value};

use caa_writer_lib::database::SymbolFilters;
use caa_writer_lib::engine::Engine;
use caa_writer_lib::model::*;
use caa_writer_lib::paths::AppPaths;
use caa_writer_lib::export::{caadoc, docx, html};
use caa_writer_lib::{online_search, pictogram_import};

#[derive(Parser)]
#[command(
    name = "caa-cli",
    version,
    about = "CAA Writer headless: motore CAA pilotabile da JSON"
)]
struct Cli {
    /// Cartella dati (default: quella dell'app, oppure CAA_WRITER_DATA_DIR).
    #[arg(long, global = true)]
    data_dir: Option<PathBuf>,
    /// Cartella risorse con dictionaries/, symbol-packs/, templates/.
    #[arg(long, global = true)]
    resources: Option<PathBuf>,
    /// JSON compatto (una riga) invece che indentato.
    #[arg(long, global = true)]
    compact: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Stato del motore, percorsi e conteggi.
    Info,
    /// Elenco dei metodi disponibili.
    Methods,
    /// Esegue un metodo con parametri JSON (argomento, file o stdin con `-`).
    Call {
        method: String,
        #[arg(default_value = "-")]
        input: String,
    },
    /// Legge richieste JSONL da stdin: {"id":1,"method":"...","params":{}}.
    Rpc,
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn default_data_dir() -> PathBuf {
    if let Ok(value) = std::env::var("CAA_WRITER_DATA_DIR") {
        return PathBuf::from(value);
    }
    #[cfg(target_os = "macos")]
    {
        home_dir().join("Library/Application Support/org.caawriter.desktop")
    }
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(home_dir)
            .join("org.caawriter.desktop")
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home_dir().join(".local/share"))
            .join("org.caawriter.desktop")
    }
}

fn resolve_resources(explicit: Option<PathBuf>) -> Result<PathBuf, String> {
    if let Some(path) = explicit {
        if path.join("dictionaries/it-core.json").exists() {
            return Ok(path);
        }
        return Err(format!(
            "risorse non valide: {} non contiene dictionaries/it-core.json",
            path.display()
        ));
    }
    if let Ok(value) = std::env::var("CAA_WRITER_RESOURCES") {
        let path = PathBuf::from(value);
        if path.join("dictionaries/it-core.json").exists() {
            return Ok(path);
        }
    }
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| cwd.clone());
    let mut candidates = vec![
        cwd.join("resources"),
        cwd.join("../resources"),
        cwd.join("../../resources"),
        exe_dir.join("resources"),
        exe_dir.join("../resources"),
        exe_dir.join("../../../resources"),
        exe_dir.join("../../../../resources"),
    ];
    for candidate in candidates.drain(..) {
        if candidate.join("dictionaries/it-core.json").exists() {
            return Ok(candidate);
        }
    }
    Err(
        "cartella risorse non trovata: usa --resources o CAA_WRITER_RESOURCES"
            .to_string(),
    )
}

fn engine_for(cli: &Cli) -> Result<Engine, String> {
    let resources = resolve_resources(cli.resources.clone())?;
    let data_dir = cli.data_dir.clone().unwrap_or_else(default_data_dir);
    let paths = AppPaths::new(data_dir, resources);
    Engine::new(paths)
}

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

fn dispatch(engine: &Engine, method: &str, params: &Value) -> Result<Value, String> {
    match method {
        "info" => {
            let stats = engine.db()?.statistics().map_err(|e| e.to_string())?;
            let templates = caa_writer_lib::studio::list_templates(&engine.paths.templates_dir())
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
        "methods" => Ok(json!({ "methods": METHODS })),
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
            let templates = caa_writer_lib::studio::list_templates(&engine.paths.templates_dir())?;
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
            let template = caa_writer_lib::studio::find_template(
                &engine.paths.templates_dir(),
                &id,
            )?
            .ok_or_else(|| format!("modello {id} non trovato"))?;
            Ok(serde_json::to_value(&template).map_err(|e| e.to_string())?)
        }
        "templates.plan" => {
            let id = as_string(params, "template_id");
            let prompt = as_string(params, "prompt");
            let templates = caa_writer_lib::studio::list_templates(&engine.paths.templates_dir())?;
            let template = match id {
                Some(id) => templates.into_iter().find(|template| template.id == id),
                None => prompt
                    .as_deref()
                    .map(|prompt| caa_writer_lib::studio::interpret_prompt(prompt, &templates).0)
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
            let spec: caa_writer_lib::layout::LayoutSpecification = params
                .get("spec")
                .cloned()
                .ok_or_else(|| "serve il parametro spec".to_string())
                .and_then(|value| serde_json::from_value(value).map_err(|e| format!("spec: {e}")))?;
            let document = engine.layout_document(spec)?;
            Ok(serde_json::to_value(&document).map_err(|e| e.to_string())?)
        }
        "layout.edit" => {
            let spec: caa_writer_lib::layout::LayoutSpecification = params
                .get("spec")
                .cloned()
                .ok_or_else(|| "serve il parametro spec".to_string())
                .and_then(|value| serde_json::from_value(value).map_err(|e| format!("spec: {e}")))?;
            let prompt = as_string(params, "prompt");
            let operations = match params.get("operations") {
                Some(value) => Some(
                    serde_json::from_value::<Vec<caa_writer_lib::layout_edit::LayoutOperation>>(
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
            let terms = online_search::known_terms(&engine.lexicon, &terms);
            if terms.is_empty() {
                return Err("nessun termine generico riconosciuto".to_string());
            }
            let db = engine.db()?;
            let response = online_search::search(&db, &terms, as_bool(params, "force_refresh", false))?;
            Ok(serde_json::to_value(&response).map_err(|e| e.to_string())?)
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
            Ok(serde_json::to_value(caa_writer_lib::asset_manager::alternative_from_record(&record))
                .map_err(|e| e.to_string())?)
        }
        "prompt.interpret" => {
            let prompt = as_string(params, "prompt").ok_or_else(|| "serve prompt".to_string())?;
            let plan = engine.interpret_document_prompt(&prompt, as_bool(params, "use_neural", true));
            Ok(serde_json::to_value(&plan).map_err(|e| e.to_string())?)
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
            let status = caa_writer_lib::voice::status(
                &engine.paths.resources_dir,
                &engine.paths.data_dir,
                as_string(params, "language").as_deref(),
            );
            Ok(serde_json::to_value(&status).map_err(|e| e.to_string())?)
        }
        "voice.speak" => {
            let text = as_string(params, "text").ok_or_else(|| "serve text".to_string())?;
            let language = as_string(params, "language");
            if let Some(paths) = caa_writer_lib::voice::resolve(
                &engine.paths.resources_dir,
                &engine.paths.data_dir,
                language.as_deref(),
            ) {
                match caa_writer_lib::voice::synthesize(&text, &paths) {
                    Ok(wav) => {
                        caa_writer_lib::voice::play(&wav)?;
                        return Ok(json!({ "engine": "neural", "wav": wav }));
                    }
                    Err(error) => {
                        caa_writer_lib::speech::speak(&text, None, None)?;
                        return Ok(json!({ "engine": "system", "fallback_reason": error }));
                    }
                }
            }
            caa_writer_lib::speech::speak(&text, None, None)?;
            Ok(json!({ "engine": "system" }))
        }
        "ai.status" => Ok(serde_json::to_value(engine.ai_status()).map_err(|e| e.to_string())?),
        "ai.models" => Ok(serde_json::to_value(engine.ai_models()).map_err(|e| e.to_string())?),
        "ai.tool" => {
            let tool = as_string(params, "tool").ok_or_else(|| "serve tool".to_string())?;
            let input = params.get("input").cloned().unwrap_or_else(|| json!({}));
            let outcome = engine.run_ai_tool(&tool, &input)?;
            Ok(serde_json::to_value(&outcome).map_err(|e| e.to_string())?)
        }
        "speak" => {
            let text = as_string(params, "text").ok_or_else(|| "serve text".to_string())?;
            let language = as_string(params, "language");
            if let Some(paths) = caa_writer_lib::voice::resolve(
                &engine.paths.resources_dir,
                &engine.paths.data_dir,
                language.as_deref(),
            ) {
                if let Ok(wav) = caa_writer_lib::voice::synthesize(&text, &paths) {
                    caa_writer_lib::voice::play(&wav)?;
                    return Ok(json!({ "engine": "neural", "wav": wav }));
                }
            }
            caa_writer_lib::speech::speak(&text, None, None)?;
            Ok(json!({ "engine": "system" }))
        }
        other => Err(format!("metodo sconosciuto: {other}")),
    }
}

const METHODS: &[&str] = &[
    "info",
    "methods",
    "symbolize",
    "analyze.text",
    "analyze.caadoc",
    "document.open",
    "document.save",
    "search",
    "affinities",
    "templates.list",
    "templates.get",
    "templates.plan",
    "layout.build",
    "layout.edit",
    "import.scan",
    "import.run",
    "collections.list",
    "collections.rename",
    "collections.delete",
    "collections.delete_symbols",
    "collections.move",
    "online.providers",
    "online.search",
    "online.save",
    "prompt.interpret",
    "export.html",
    "export.docx",
    "voice.status",
    "voice.speak",
    "speak",
    "ai.status",
    "ai.models",
    "ai.tool",
];

fn read_input(input: &str) -> Result<Value, String> {
    if input == "-" {
        let mut buffer = String::new();
        std::io::stdin()
            .read_to_string(&mut buffer)
            .map_err(|e| format!("stdin: {e}"))?;
        let trimmed = buffer.trim();
        if trimmed.is_empty() {
            return Ok(json!({}));
        }
        return serde_json::from_str(trimmed).map_err(|e| format!("JSON non valido: {e}"));
    }
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(json!({}));
    }
    serde_json::from_str(trimmed).map_err(|e| format!("JSON non valido: {e}"))
}

fn print_json(value: &Value, compact: bool) -> Result<(), String> {
    let text = if compact {
        serde_json::to_string(value)
    } else {
        serde_json::to_string_pretty(value)
    }
    .map_err(|e| e.to_string())?;
    println!("{text}");
    Ok(())
}

fn main() {
    let cli = Cli::parse();
    let engine = match engine_for(&cli) {
        Ok(engine) => engine,
        Err(error) => {
            eprintln!("{}", serde_json::to_string(&json!({ "ok": false, "error": error })).unwrap());
            std::process::exit(1);
        }
    };

    match cli.command {
        Command::Info => match dispatch(&engine, "info", &json!({})) {
            Ok(value) => {
                let _ = print_json(&value, cli.compact);
            }
            Err(error) => {
                eprintln!("{}", json!({ "ok": false, "error": error }));
                std::process::exit(1);
            }
        },
        Command::Methods => {
            let _ = print_json(&json!({ "methods": METHODS }), cli.compact);
        }
        Command::Call { method, input } => match read_input(&input)
            .and_then(|params| dispatch(&engine, &method, &params))
        {
            Ok(value) => {
                let _ = print_json(&value, cli.compact);
            }
            Err(error) => {
                eprintln!("{}", serde_json::to_string(&json!({ "ok": false, "error": error })).unwrap());
                std::process::exit(1);
            }
        },
        Command::Rpc => {
            let stdin = std::io::stdin();
            let mut stdout = std::io::stdout();
            for line in stdin.lock().lines() {
                let Ok(line) = line else { break };
                if line.trim().is_empty() {
                    continue;
                }
                let request: Value = match serde_json::from_str(&line) {
                    Ok(value) => value,
                    Err(error) => {
                        let response = json!({ "id": null, "ok": false, "error": format!("JSON non valido: {error}") });
                        let _ = writeln!(stdout, "{response}");
                        let _ = stdout.flush();
                        continue;
                    }
                };
                let id = request.get("id").cloned().unwrap_or(Value::Null);
                let method = request
                    .get("method")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default()
                    .to_string();
                let params = request.get("params").cloned().unwrap_or_else(|| json!({}));
                let response = match dispatch(&engine, &method, &params) {
                    Ok(result) => json!({ "id": id, "ok": true, "result": result }),
                    Err(error) => json!({ "id": id, "ok": false, "error": error }),
                };
                let _ = writeln!(stdout, "{response}");
                let _ = stdout.flush();
            }
        }
    }
}
