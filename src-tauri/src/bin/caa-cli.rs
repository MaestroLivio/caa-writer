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
use caa_writer_lib::universal::{self, DocumentCommand, UniversalDocument};
use caa_writer_lib::{asset_manager, commands, online_search, pictogram_import, universal_build};

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
    /// Documento universale: crea, valida, ispeziona, migra, applica comandi.
    Document {
        #[command(subcommand)]
        action: DocumentAction,
    },
    /// Gestione delle pagine di un documento nativo.
    Page {
        #[command(subcommand)]
        action: PageAction,
    },
    /// Gestione degli elementi di un documento nativo.
    Element {
        #[command(subcommand)]
        action: ElementAction,
    },
    /// Tabelle e sistemi comunicativi nativi.
    Board {
        #[command(subcommand)]
        action: BoardAction,
    },
    /// Pipeline riproducibile della biblioteca.
    Template {
        #[command(subcommand)]
        action: TemplateAction,
    },
    /// Catalogo organizzato, ricerca e controllo qualità della biblioteca.
    Library {
        #[command(subcommand)]
        action: LibraryAction,
    },
    /// Storie illustrate in CAA con scene vettoriali modificabili.
    Story {
        #[command(subcommand)]
        action: StoryAction,
    },
}

#[derive(Subcommand)]
enum StoryAction {
    /// Crea un .caadoc multipagina da una specifica JSON.
    Create {
        #[arg(long)]
        spec: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    /// Valida una storia già creata.
    Validate { input: PathBuf },
    /// Esporta PDF o PNG usando il renderer dell'editor.
    Render {
        input: PathBuf,
        #[arg(long, default_value = "pdf")]
        format: String,
        #[arg(long)]
        out: PathBuf,
    },
    /// Copia la storia in un nuovo .caadoc.
    Package {
        input: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
}

#[derive(Subcommand)]
enum LibraryAction {
    /// Categorie, sottocategorie e conteggi reali.
    Catalog,
    /// Ricerca per parola, categoria, pubblico e tipo di interazione.
    Search {
        #[arg(long, default_value = "")]
        query: String,
        #[arg(long)]
        category: Option<String>,
        #[arg(long)]
        subcategory: Option<String>,
        #[arg(long)]
        audience: Option<String>,
        #[arg(long)]
        interactive: bool,
        #[arg(long, default_value = "rilevanza")]
        sort: String,
        #[arg(long, default_value_t = 24)]
        limit: usize,
        #[arg(long, default_value_t = 0)]
        offset: usize,
    },
    /// Controlli strutturali ed editoriali su tutti i documenti.
    Validate {
        #[arg(long)]
        only_pilots: bool,
        #[arg(long, default_value_t = 80)]
        max_issues: usize,
    },
}

#[derive(Subcommand)]
enum DocumentAction {
    /// Crea un .caadoc da una specifica JSON.
    Create {
        #[arg(long)]
        spec: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    /// Valida schema, pagine, elementi, risorse e collegamenti.
    Validate { file: PathBuf },
    /// Riepilogo leggibile del documento.
    Inspect { file: PathBuf },
    /// Calcola la geometria e scrive JSON + SVG per ogni pagina.
    Render {
        file: PathBuf,
        #[arg(long)]
        out: PathBuf,
        /// Scrive anche un PNG per ogni pagina (controllo visivo).
        #[arg(long)]
        png: bool,
    },
    /// Esporta HTML interattivo, SVG o JSON.
    Export {
        file: PathBuf,
        #[arg(long, default_value = "html")]
        format: String,
        #[arg(long)]
        out: PathBuf,
    },
    /// Converte un vecchio layout in documento universale.
    Migrate {
        file: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    /// Applica una transazione JSON di comandi tipizzati.
    Apply {
        file: PathBuf,
        #[arg(long)]
        commands: PathBuf,
        #[arg(long)]
        out: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum PageAction {
    Add {
        file: PathBuf,
        #[arg(long)]
        id: Option<String>,
        #[arg(long)]
        title: Option<String>,
        #[arg(long, default_value = "flow")]
        layout: String,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Remove {
        file: PathBuf,
        #[arg(long)]
        id: String,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Reorder {
        file: PathBuf,
        #[arg(long)]
        id: String,
        #[arg(long)]
        index: usize,
        #[arg(long)]
        out: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum ElementAction {
    Add {
        file: PathBuf,
        #[arg(long)]
        page: String,
        #[arg(long)]
        element: PathBuf,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Update {
        file: PathBuf,
        #[arg(long)]
        page: String,
        #[arg(long)]
        id: String,
        #[arg(long)]
        patch: PathBuf,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Remove {
        file: PathBuf,
        #[arg(long)]
        page: String,
        #[arg(long)]
        id: String,
        #[arg(long)]
        out: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum BoardAction {
    Create {
        #[arg(long)]
        spec: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    Link {
        file: PathBuf,
        #[arg(long)]
        page: String,
        #[arg(long)]
        id: String,
        #[arg(long)]
        target: String,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Validate { file: PathBuf },
}

#[derive(Subcommand)]
enum TemplateAction {
    Build {
        #[arg(long)]
        spec: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    Validate {
        #[arg(long)]
        spec: PathBuf,
    },
    Preview {
        file: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    Package {
        file: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    BuildAll {
        #[arg(long, default_value = "templates/sources")]
        sources: PathBuf,
        #[arg(long, default_value = "resources/library")]
        out: PathBuf,
    },
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
        "packages.catalog" => {
            let catalog = caa_writer_lib::packages::catalog(engine)?;
            Ok(serde_json::to_value(catalog).map_err(|e| e.to_string())?)
        }
        "packages.updates" => {
            let info = caa_writer_lib::packages::check_updates(engine);
            Ok(serde_json::to_value(info).map_err(|e| e.to_string())?)
        }
        "packages.download" => {
            let id = as_string(params, "id").ok_or_else(|| "serve il parametro id".to_string())?;
            let limit = params.get("limit").and_then(|v| v.as_u64()).unwrap_or(5) as usize;
            let report = match id.as_str() {
                "symbols:arasaac" => caa_writer_lib::packages::download_arasaac(engine, limit.max(1))?,
                "symbols:globalsymbols" => {
                    caa_writer_lib::packages::download_globalsymbols(engine, limit.max(1))?
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
            caa_writer_lib::packages::set_library_enabled(
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
            caa_writer_lib::packages::remove_symbol_pack(engine, pack)?;
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
            Ok(serde_json::to_value(caa_writer_lib::asset_manager::alternative_from_record(&record))
                .map_err(|e| e.to_string())?)
        }
        "boards.list" => Ok(serde_json::to_value(caa_writer_lib::profiles::list_boards(
            &engine.paths.boards_dir(),
        )?)
        .map_err(|e| e.to_string())?),
        "boards.build" => {
            let id = as_string(params, "id").ok_or_else(|| "serve id".to_string())?;
            let board = caa_writer_lib::profiles::build_board(
                engine,
                &engine.paths.boards_dir(),
                &id,
            )?;
            Ok(serde_json::to_value(&board).map_err(|e| e.to_string())?)
        }
        "profiles.list" => {
            let db = engine.db()?;
            Ok(serde_json::to_value(caa_writer_lib::profiles::list_profiles(&db)?)
                .map_err(|e| e.to_string())?)
        }
        "prompt.interpret" => {
            let prompt = as_string(params, "prompt").ok_or_else(|| "serve prompt".to_string())?;
            let plan = engine.interpret_document_prompt(&prompt, as_bool(params, "use_neural", true));
            Ok(serde_json::to_value(&plan).map_err(|e| e.to_string())?)
        }
        "library.catalog" => {
            let manifest = caa_writer_lib::packages::filtered_manifest(
                &engine.paths.resources_dir,
                &engine.paths.data_dir,
            );
            Ok(serde_json::to_value(caa_writer_lib::template_catalog::overview(&manifest))
                .map_err(|e| e.to_string())?)
        }
        "library.search" => {
            let manifest = caa_writer_lib::packages::filtered_manifest(
                &engine.paths.resources_dir,
                &engine.paths.data_dir,
            );
            let raw = params
                .get("query")
                .cloned()
                .unwrap_or_else(|| params.clone());
            let query: caa_writer_lib::template_catalog::CatalogQuery =
                serde_json::from_value(raw).map_err(|e| format!("query: {e}"))?;
            let query = if query.query.is_none() {
                caa_writer_lib::template_catalog::CatalogQuery {
                    query: as_string(params, "q"),
                    ..query
                }
            } else {
                query
            };
            let mut page = caa_writer_lib::template_catalog::search(&manifest, &query);
            for item in &mut page.items {
                item.preview_svg =
                    caa_writer_lib::template_catalog::preview_svg(&engine.paths.resources_dir, &item.entry);
            }
            Ok(serde_json::to_value(page).map_err(|e| e.to_string())?)
        }
        "library.validate" => {
            let manifest = caa_writer_lib::universal::load_library_manifest(&engine.paths.resources_dir);
            let report = caa_writer_lib::library_quality::validate_manifest(
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
        "ai.chat" => {
            let messages: Vec<caa_writer_lib::ai_provider::ChatMessage> = params
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
            caa_writer_lib::universal::validate(&document)?;
            let out = as_string(params, "out")
                .ok_or_else(|| "serve out".to_string())?;
            let bytes = caa_writer_lib::export::universal_pdf::build_pdf(&document, 2.0)?;
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
            let status = caa_writer_lib::image_engine::status(&engine.paths, &db);
            Ok(serde_json::to_value(&status).map_err(|e| e.to_string())?)
        }
        "image.generate" => {
            let prompt = as_string(params, "prompt").ok_or_else(|| "serve prompt".to_string())?;
            let negative = as_string(params, "negative").unwrap_or_default();
            let db = engine.db()?;
            let image = caa_writer_lib::image_engine::generate(
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
            let message = caa_writer_lib::image_engine::install_runtime(&engine.paths)?;
            Ok(json!({ "ok": true, "message": message }))
        }
        "image.download" => {
            let model = as_string(params, "model").ok_or_else(|| "serve model".to_string())?;
            let message = caa_writer_lib::image_engine::download_model(&engine.paths, &model)?;
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
                                caa_writer_lib::asset_manager::alternative_from_record(&record);
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
    "packages.catalog",
    "packages.updates",
    "packages.download",
    "packages.set_library",
    "packages.remove",
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
    "boards.list",
    "boards.build",
    "profiles.list",
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
    "online.ensure",
    "online.autofill",
    "online.global",
    "online.english",
    "symbol.resolve",
    "symbol.hydrate",
    "symbol.get",
    "online.save",
    "prompt.interpret",
    "library.catalog",
    "library.search",
    "library.validate",
    "export.html",
    "export.docx",
    "voice.status",
    "voice.speak",
    "speak",
    "ai.status",
    "ai.chat",
    "ai.models",
    "ai.tool",
    "ai.affinity",
    "ai.words",
    "document.pdf",
    "translate.search",
    "image.status",
    "image.generate",
    "image.install",
    "image.download",
    "translate.english",
    "english.alternatives",
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

fn read_json_file(path: &Path) -> Result<Value, String> {
    let raw = std::fs::read_to_string(path)
        .map_err(|error| format!("{}: {error}", path.display()))?;
    serde_json::from_str(&raw).map_err(|error| format!("{}: JSON non valido: {error}", path.display()))
}

fn load_document(path: &Path) -> Result<UniversalDocument, String> {
    universal_build::load_universal(path)
}

fn save_document(path: &Path, document: &UniversalDocument) -> Result<Value, String> {
    let assets = universal_build::assets_from_document(document);
    let saved = universal_build::save_universal(path, document, &assets)?;
    Ok(json!({
        "ok": true,
        "path": saved.path,
        "title": document.title,
        "pages": document.pages.len(),
        "elements": document.pages.iter().map(|page| page.elements.len()).sum::<usize>(),
        "warnings": saved.warnings,
    }))
}

fn apply_to_file(
    file: &Path,
    out: Option<PathBuf>,
    commands: Vec<DocumentCommand>,
) -> Result<Value, String> {
    let document = load_document(file)?;
    let result = universal::apply_transaction(&document, &commands)?;
    let target = out.unwrap_or_else(|| file.to_path_buf());
    save_document(&target, &result.document)
}

fn document_summary(document: &UniversalDocument) -> Value {
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

fn render_to_dir(document: &UniversalDocument, out: &Path) -> Result<Value, String> {
    render_to_dir_with_png(document, out, false)
}

fn render_to_dir_with_png(
    document: &UniversalDocument,
    out: &Path,
    png: bool,
) -> Result<Value, String> {
    std::fs::create_dir_all(out).map_err(|error| format!("{}: {error}", out.display()))?;
    let rendered = document.render()?;
    std::fs::write(
        out.join("rendered.json"),
        serde_json::to_string_pretty(&rendered).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let mut files = Vec::new();
    for page in &rendered.pages {
        let path = out.join(format!("page-{}.svg", page.index + 1));
        std::fs::write(&path, universal::render_page_svg(page)).map_err(|e| e.to_string())?;
        files.push(path.to_string_lossy().to_string());
        if png {
            let bitmap = caa_writer_lib::export::universal_pdf::page_to_png(page, 2.0)?;
            let png_path = out.join(format!("page-{}.png", page.index + 1));
            std::fs::write(&png_path, bitmap).map_err(|e| e.to_string())?;
            files.push(png_path.to_string_lossy().to_string());
        }
    }
    Ok(json!({
        "ok": true,
        "pages": rendered.pages.len(),
        "warnings": rendered.warnings,
        "files": files,
    }))
}

fn universal_pdf_export(document: &UniversalDocument, out: &Path) -> Result<(), String> {
    caa_writer_lib::export::universal_pdf::export_pdf(document, out)
}

fn universal_png_export(document: &UniversalDocument, out: &Path) -> Result<(), String> {
    caa_writer_lib::export::universal_pdf::export_first_page_png(document, out)
}

fn export_document(document: &UniversalDocument, format: &str, out: &Path) -> Result<Value, String> {
    match format {
        "html" => {
            let html = universal_build::export_html(document)?;
            std::fs::write(out, html).map_err(|e| format!("{}: {e}", out.display()))?;
            Ok(json!({"ok": true, "format": "html", "path": out}))
        }
        "json" => {
            std::fs::write(
                out,
                serde_json::to_string_pretty(document).map_err(|e| e.to_string())?,
            )
            .map_err(|e| format!("{}: {e}", out.display()))?;
            Ok(json!({"ok": true, "format": "json", "path": out}))
        }
        "svg" => render_to_dir(document, out),
        "pdf" => {
            universal_pdf_export(document, out)?;
            Ok(json!({"ok": true, "format": "pdf", "path": out}))
        }
        "png" => {
            universal_png_export(document, out)?;
            Ok(json!({"ok": true, "format": "png", "path": out}))
        }
        "docx" => {
            let result = caa_writer_lib::export::universal_docx::write(out, document)?;
            Ok(json!({"ok": true, "format": "docx", "path": result.path, "warnings": result.warnings}))
        }
        "pptx" => {
            let result = caa_writer_lib::export::universal_pptx::write(out, document)?;
            Ok(json!({"ok": true, "format": "pptx", "path": result.path, "warnings": result.warnings}))
        }
        other => Err(format!("formato di esportazione non supportato: {other}")),
    }
}

fn build_spec_file(engine: &Engine, spec_path: &Path, out: Option<&Path>) -> Result<Value, String> {
    let spec = read_json_file(spec_path)?;
    let outcome = universal_build::build_from_spec(engine, &spec)?;
    let summary = document_summary(&outcome.document);
    if let Some(out) = out {
        let saved = universal_build::save_universal(out, &outcome.document, &outcome.assets)?;
        Ok(json!({
            "ok": true,
            "document": summary,
            "path": saved.path,
            "notes": outcome.notes,
            "assets": outcome.assets.len(),
        }))
    } else {
        Ok(json!({
            "ok": true,
            "document": summary,
            "notes": outcome.notes,
            "assets": outcome.assets.len(),
        }))
    }
}

fn output(result: Result<Value, String>, compact: bool) {
    match result {
        Ok(value) => {
            let _ = print_json(&value, compact);
        }
        Err(error) => {
            eprintln!("{}", json!({"ok": false, "error": error}));
            std::process::exit(1);
        }
    }
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
        Command::Document { action } => output(
            (|| -> Result<Value, String> {
                match action {
                    DocumentAction::Create { spec, out } => build_spec_file(&engine, &spec, Some(&out)),
                    DocumentAction::Validate { file } => {
                        let document = load_document(&file)?;
                        universal::validate(&document)?;
                        Ok(json!({"ok": true, "valid": true, "document": document_summary(&document)}))
                    }
                    DocumentAction::Inspect { file } => {
                        let document = load_document(&file)?;
                        Ok(json!({"ok": true, "document": document_summary(&document)}))
                    }
                    DocumentAction::Render { file, out, png } => {
                        let document = load_document(&file)?;
                        render_to_dir_with_png(&document, &out, png)
                    }
                    DocumentAction::Export { file, format, out } => {
                        let document = load_document(&file)?;
                        export_document(&document, &format, &out)
                    }
                    DocumentAction::Migrate { file, out } => {
                        let document = load_document(&file)?;
                        save_document(&out, &document)
                    }
                    DocumentAction::Apply { file, commands, out } => {
                        let value = read_json_file(&commands)?;
                        let commands: Vec<DocumentCommand> = serde_json::from_value(value)
                            .map_err(|error| format!("comandi non validi: {error}"))?;
                        apply_to_file(&file, out, commands)
                    }
                }
            })(),
            cli.compact,
        ),
        Command::Page { action } => output(
            (|| -> Result<Value, String> {
                match action {
                    PageAction::Add { file, id, title, layout, out } => apply_to_file(
                        &file,
                        out,
                        vec![DocumentCommand::AddPage { id, title, layout: Some(layout), after: None }],
                    ),
                    PageAction::Remove { file, id, out } => {
                        apply_to_file(&file, out, vec![DocumentCommand::RemovePage { id }])
                    }
                    PageAction::Reorder { file, id, index, out } => {
                        apply_to_file(&file, out, vec![DocumentCommand::MovePage { id, index }])
                    }
                }
            })(),
            cli.compact,
        ),
        Command::Element { action } => output(
            (|| -> Result<Value, String> {
                match action {
                    ElementAction::Add { file, page, element, out } => {
                        let value = read_json_file(&element)?;
                        apply_to_file(
                            &file,
                            out,
                            vec![DocumentCommand::AddElement { page, element: value, index: None }],
                        )
                    }
                    ElementAction::Update { file, page, id, patch, out } => {
                        let value = read_json_file(&patch)?;
                        apply_to_file(
                            &file,
                            out,
                            vec![DocumentCommand::UpdateElement { page, id, patch: value }],
                        )
                    }
                    ElementAction::Remove { file, page, id, out } => {
                        apply_to_file(&file, out, vec![DocumentCommand::RemoveElement { page, id }])
                    }
                }
            })(),
            cli.compact,
        ),
        Command::Board { action } => output(
            (|| -> Result<Value, String> {
                match action {
                    BoardAction::Create { spec, out } => {
                        let mut value = read_json_file(&spec)?;
                        if let Some(object) = value.as_object_mut() {
                            object.insert("source_type".to_string(), json!("board"));
                            object.insert("kind".to_string(), json!("communication_board"));
                        }
                        let outcome = universal_build::build_from_spec(&engine, &value)?;
                        let saved = universal_build::save_universal(
                            &out,
                            &outcome.document,
                            &outcome.assets,
                        )?;
                        Ok(json!({"ok": true, "path": saved.path, "document": document_summary(&outcome.document)}))
                    }
                    BoardAction::Link { file, page, id, target, out } => apply_to_file(
                        &file,
                        out,
                        vec![DocumentCommand::CreateLink { page, id, target_page: target }],
                    ),
                    BoardAction::Validate { file } => {
                        let document = load_document(&file)?;
                        universal::validate(&document)?;
                        if document.kind != "communication_board" {
                            return Err(format!("il documento non è un sistema comunicativo: {}", document.kind));
                        }
                        Ok(json!({"ok": true, "valid": true, "document": document_summary(&document)}))
                    }
                }
            })(),
            cli.compact,
        ),
        Command::Template { action } => output(
            (|| -> Result<Value, String> {
                match action {
                    TemplateAction::Build { spec, out } => build_spec_file(&engine, &spec, Some(&out)),
                    TemplateAction::Validate { spec } => build_spec_file(&engine, &spec, None),
                    TemplateAction::Preview { file, out } => {
                        let document = load_document(&file)?;
                        let svg = universal_build::preview_svg(&document)?;
                        std::fs::write(&out, svg).map_err(|e| format!("{}: {e}", out.display()))?;
                        Ok(json!({"ok": true, "path": out}))
                    }
                    TemplateAction::Package { file, out } => {
                        let document = load_document(&file)?;
                        save_document(&out, &document)
                    }
                    TemplateAction::BuildAll { sources, out } => {
                        let resources_root = engine.paths.resources_dir.clone();
                        let result = universal_build::build_all(
                            &engine,
                            &sources,
                            &out,
                            &resources_root,
                        )?;
                        let report = serde_json::to_value(&result.report).map_err(|e| e.to_string())?;
                        let failed = result.report.failed;
                        let missing = result.report.templates_expected < 300
                            || result.report.boards_expected < 30;
                        if failed > 0 || missing {
                            return Err(format!(
                                "biblioteca incompleta: {} template, {} sistemi, {} falliti{}",
                                result.report.templates_expected,
                                result.report.boards_expected,
                                failed,
                                if missing { " (attesi almeno 300 template e 30 sistemi)" } else { "" }
                            ));
                        }
                        Ok(json!({
                            "ok": true,
                            "report": report,
                            "manifest": result.manifest,
                        }))
                    }
                }
            })(),
            cli.compact,
        ),
        Command::Library { action } => output(
            (|| -> Result<Value, String> {
                match action {
                    LibraryAction::Catalog => {
                        let manifest = caa_writer_lib::universal::load_library_manifest(
                            &engine.paths.resources_dir,
                        );
                        serde_json::to_value(caa_writer_lib::template_catalog::overview(&manifest))
                            .map_err(|e| e.to_string())
                    }
                    LibraryAction::Search {
                        query,
                        category,
                        subcategory,
                        audience,
                        interactive,
                        sort,
                        limit,
                        offset,
                    } => {
                        let manifest = caa_writer_lib::universal::load_library_manifest(
                            &engine.paths.resources_dir,
                        );
                        let request = caa_writer_lib::template_catalog::CatalogQuery {
                            query: Some(query),
                            category,
                            subcategory,
                            audience,
                            interactive: if interactive { Some(true) } else { None },
                            printable: None,
                            needs_symbols: None,
                            source_type: None,
                            sort: Some(sort),
                            offset: Some(offset),
                            limit: Some(limit),
                        };
                        let mut page =
                            caa_writer_lib::template_catalog::search(&manifest, &request);
                        for item in &mut page.items {
                            item.preview_svg = caa_writer_lib::template_catalog::preview_svg(
                                &engine.paths.resources_dir,
                                &item.entry,
                            );
                        }
                        serde_json::to_value(page).map_err(|e| e.to_string())
                    }
                    LibraryAction::Validate {
                        only_pilots,
                        max_issues,
                    } => {
                        let manifest = caa_writer_lib::universal::load_library_manifest(
                            &engine.paths.resources_dir,
                        );
                        let mut report = caa_writer_lib::library_quality::validate_manifest(
                            &engine.paths.resources_dir,
                            &manifest,
                        );
                        if only_pilots {
                            report.issues.retain(|issue| issue.document_id.starts_with("pilota_"));
                            report.documents = manifest
                                .entries
                                .iter()
                                .filter(|entry| entry.id.starts_with("pilota_"))
                                .count();
                            report.errors = report
                                .issues
                                .iter()
                                .filter(|issue| issue.severity == "errore")
                                .count();
                            report.warnings = report
                                .issues
                                .iter()
                                .filter(|issue| issue.severity == "avviso")
                                .count();
                            let with_issues: std::collections::HashSet<&str> = report
                                .issues
                                .iter()
                                .map(|issue| issue.document_id.as_str())
                                .collect();
                            report.valid = report.documents.saturating_sub(with_issues.len());
                            report.by_category.clear();
                        }
                        let total_issues = report.issues.len();
                        report.issues.truncate(max_issues);
                        let mut value =
                            serde_json::to_value(report).map_err(|e| e.to_string())?;
                        if let Some(object) = value.as_object_mut() {
                            object.insert("issues_total".to_string(), json!(total_issues));
                        }
                        Ok(value)
                    }
                }
            })(),
            cli.compact,
        ),
        Command::Story { action } => output(
            (|| -> Result<Value, String> {
                match action {
                    StoryAction::Create { spec, out } => {
                        let value = read_json_file(&spec)?;
                        let outcome = caa_writer_lib::story::build_story(&engine, &value)?;
                        let saved = caa_writer_lib::story::save_story(
                            &out,
                            &outcome.document,
                            &outcome.assets,
                        )?;
                        Ok(json!({
                            "ok": true,
                            "path": saved.path,
                            "document": document_summary(&outcome.document),
                            "notes": outcome.notes,
                            "assets": outcome.assets.len()
                        }))
                    }
                    StoryAction::Validate { input } => {
                        let document = load_document(&input)?;
                        universal::validate(&document)?;
                        let rendered = document.render()?;
                        Ok(json!({
                            "ok": true,
                            "valid": true,
                            "document": document_summary(&document),
                            "pages": rendered.pages.len(),
                            "warnings": rendered.warnings
                        }))
                    }
                    StoryAction::Render { input, format, out } => {
                        let document = load_document(&input)?;
                        export_document(&document, &format, &out)
                    }
                    StoryAction::Package { input, out } => {
                        let document = load_document(&input)?;
                        save_document(&out, &document)
                    }
                }
            })(),
            cli.compact,
        ),
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
