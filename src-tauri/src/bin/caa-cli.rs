//! Interfaccia a riga di comando di CAA Writer.
//!
//! Pensata per un coding agent e per l'automazione: ogni metodo riceve e
//! restituisce JSON, non apre finestre e non richiede interazione. Il comando
//! `rpc` mantiene un'unica istanza del motore e legge richieste JSONL da stdin.

use std::io::{BufRead, Read, Write};
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use serde_json::{json, Value};

use caa_writer_lib::automation::{self, AccessMode, DispatchOptions};
use caa_writer_lib::engine::Engine;
use caa_writer_lib::universal::{self, DocumentCommand, UniversalDocument};
use caa_writer_lib::universal_build;

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
    /// Modalità di accesso: read, write o admin (default: admin).
    #[arg(long, global = true, default_value = "admin")]
    access: String,
    /// Mostra cosa farebbe un metodo che modifica dati, senza eseguirlo.
    #[arg(long, global = true)]
    dry_run: bool,
    /// Non usare il bridge della GUI, anche se aperta: forza il motore locale.
    #[arg(long, global = true)]
    no_bridge: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Stato del motore, percorsi e conteggi.
    Info,
    /// Catalogo delle capacità con schemi e modalità di accesso.
    Capabilities {
        #[arg(long, default_value = "json")]
        format: String,
        #[arg(long)]
        category: Option<String>,
    },
    /// Stato applicativo: database, modello, percorsi, job.
    Status,
    /// Elenco dei metodi disponibili.
    Methods,
    /// Impostazioni persistite.
    Settings {
        #[command(subcommand)]
        action: SettingsAction,
    },
    /// Progetti dentro <data-dir>/projects (percorsi relativi, niente `..`).
    Project {
        #[command(subcommand)]
        action: ProjectAction,
    },
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
enum SettingsAction {
    /// Elenca chiavi e valori (filtro opzionale per prefisso).
    List {
        #[arg(long)]
        prefix: Option<String>,
    },
    /// Legge una chiave.
    Get { key: String },
    /// Scrive una chiave.
    Set { key: String, value: String },
}

#[derive(Subcommand)]
enum ProjectAction {
    /// Elenca i .caadoc nella radice progetti.
    List {
        #[arg(long)]
        root: Option<String>,
        #[arg(long, default_value_t = 200)]
        limit: u32,
    },
    /// Crea un documento vuoto.
    Create {
        #[arg(long)]
        path: String,
        #[arg(long)]
        title: String,
        #[arg(long, default_value = "document")]
        kind: String,
        #[arg(long, default_value = "flow")]
        layout: String,
    },
    /// Apre un progetto e ne stampa il documento completo.
    Open { path: String },
    /// Valida un progetto.
    Validate { path: String },
    /// Salva un documento JSON (UniversalDocument) in un progetto.
    Save {
        #[arg(long)]
        path: String,
        #[arg(long)]
        document: PathBuf,
        #[arg(long)]
        overwrite: bool,
    },
    /// Duplica un progetto.
    Duplicate {
        path: String,
        #[arg(long)]
        target: String,
    },
    /// Rinomina o sposta un progetto.
    Rename {
        path: String,
        #[arg(long)]
        target: String,
    },
    /// Elimina un progetto (richiede --confirm).
    Delete {
        path: String,
        #[arg(long)]
        confirm: bool,
    },
    /// Esporta un progetto nella radice progetti.
    Export {
        path: String,
        #[arg(long, default_value = "pdf")]
        format: String,
        #[arg(long)]
        out: Option<String>,
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

fn engine_for(cli: &Cli) -> Result<Engine, String> {
    automation::bootstrap::build_engine(cli.data_dir.clone(), cli.resources.clone())
}

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
    let access = match AccessMode::parse(&cli.access) {
        Ok(access) => access,
        Err(error) => {
            eprintln!("{}", json!({ "ok": false, "error": error }));
            std::process::exit(2);
        }
    };
    let options = DispatchOptions {
        access,
        dry_run: cli.dry_run,
    };
    let engine = match engine_for(&cli) {
        Ok(engine) => engine,
        Err(error) => {
            eprintln!("{}", serde_json::to_string(&json!({ "ok": false, "error": error })).unwrap());
            std::process::exit(1);
        }
    };
    let bridge = if cli.no_bridge {
        None
    } else {
        caa_writer_lib::automation::bridge::BridgeClient::discover(
            &engine.paths.data_dir,
            cli.resources.as_deref(),
        )
    };
    let dispatch_with_options =
        |engine: &Engine, method: &str, params: &Value, options: DispatchOptions| {
            caa_writer_lib::automation::bridge::execute(engine, bridge.as_ref(), method, params, options)
        };
    match cli.command {
        Command::Info => output(
            dispatch_with_options(&engine, "info", &json!({}), options),
            cli.compact,
        ),
        Command::Capabilities { format, category } => {
            let params = json!({
                "format": format,
                "category": category,
            });
            let result = dispatch_with_options(
                &engine,
                "app.capabilities",
                &params,
                options,
            );
            match result {
                Ok(value) if value.get("markdown").and_then(|value| value.as_str()).is_some() => {
                    println!("{}", value["markdown"].as_str().unwrap_or_default());
                }
                other => output(other, cli.compact),
            }
        }
        Command::Status => output(
            dispatch_with_options(&engine, "app.status", &json!({}), options),
            cli.compact,
        ),
        Command::Methods => {
            let value = json!({
                "methods": caa_writer_lib::automation::registry::methods(),
                "api_version": caa_writer_lib::automation::API_VERSION,
            });
            let _ = print_json(&value, cli.compact);
        }
        Command::Settings { action } => output(
            (|| -> Result<Value, String> {
                match action {
                    SettingsAction::List { prefix } => {
                        let params = prefix.map(|prefix| json!({ "prefix": prefix })).unwrap_or_else(|| json!({}));
                        dispatch_with_options(&engine, "settings.list", &params, options)
                    }
                    SettingsAction::Get { key } => dispatch_with_options(
                        &engine,
                        "settings.get",
                        &json!({ "key": key }),
                        options,
                    ),
                    SettingsAction::Set { key, value } => dispatch_with_options(
                        &engine,
                        "settings.set",
                        &json!({ "key": key, "value": value }),
                        options,
                    ),
                }
            })(),
            cli.compact,
        ),
        Command::Project { action } => output(
            (|| -> Result<Value, String> {
                match action {
                    ProjectAction::List { root, limit } => {
                        let mut params = json!({ "limit": limit });
                        if let Some(root) = root {
                            params["root"] = json!(root);
                        }
                        dispatch_with_options(&engine, "project.list", &params, options)
                    }
                    ProjectAction::Create { path, title, kind, layout } => {
                        dispatch_with_options(
                            &engine,
                            "project.create",
                            &json!({ "path": path, "title": title, "kind": kind, "layout": layout }),
                            options,
                        )
                    }
                    ProjectAction::Open { path } => dispatch_with_options(
                        &engine,
                        "project.open",
                        &json!({ "path": path }),
                        options,
                    ),
                    ProjectAction::Validate { path } => dispatch_with_options(
                        &engine,
                        "project.validate",
                        &json!({ "path": path }),
                        options,
                    ),
                    ProjectAction::Save { path, document, overwrite } => {
                        let value = read_json_file(&document)?;
                        dispatch_with_options(
                            &engine,
                            "project.save",
                            &json!({ "path": path, "document": value, "overwrite": overwrite }),
                            options,
                        )
                    }
                    ProjectAction::Duplicate { path, target } => dispatch_with_options(
                        &engine,
                        "project.duplicate",
                        &json!({ "path": path, "target": target }),
                        options,
                    ),
                    ProjectAction::Rename { path, target } => dispatch_with_options(
                        &engine,
                        "project.rename",
                        &json!({ "path": path, "target": target }),
                        options,
                    ),
                    ProjectAction::Delete { path, confirm } => dispatch_with_options(
                        &engine,
                        "project.delete",
                        &json!({ "path": path, "confirm": confirm }),
                        options,
                    ),
                    ProjectAction::Export { path, format, out } => dispatch_with_options(
                        &engine,
                        "project.export",
                        &json!({ "path": path, "format": format, "out": out }),
                        options,
                    ),
                }
            })(),
            cli.compact,
        ),
        Command::Call { method, input } => match read_input(&input)
            .and_then(|params| dispatch_with_options(&engine, &method, &params, options))
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
                let request_options = match request
                    .get("access")
                    .and_then(|value| value.as_str())
                    .map(AccessMode::parse)
                    .transpose()
                {
                    Ok(Some(access)) => DispatchOptions {
                        access,
                        dry_run: request
                            .get("dry_run")
                            .and_then(|value| value.as_bool())
                            .unwrap_or(options.dry_run),
                    },
                    Ok(None) => DispatchOptions {
                        dry_run: request
                            .get("dry_run")
                            .and_then(|value| value.as_bool())
                            .unwrap_or(options.dry_run),
                        ..options
                    },
                    Err(error) => {
                        let response = json!({ "id": id, "ok": false, "error": error });
                        let _ = writeln!(stdout, "{response}");
                        let _ = stdout.flush();
                        continue;
                    }
                };
                let response = match dispatch_with_options(
                    &engine,
                    &method,
                    &params,
                    request_options,
                ) {
                    Ok(result) => json!({ "id": id, "ok": true, "result": result }),
                    Err(error) => json!({ "id": id, "ok": false, "error": error }),
                };
                let _ = writeln!(stdout, "{response}");
                let _ = stdout.flush();
            }
        }
    }
}
