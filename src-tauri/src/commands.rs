//! Comandi Tauri: unica superficie tra frontend e backend.
//!
//! Nessun comando esegue operazioni implicite onerose: la simbolizzazione è
//! per paragrafo, la rifinitura IA è esplicita, le esportazioni ricevono un
//! percorso scelto dall'utente.

use std::path::Path;

use serde::Serialize;
use tauri::{Manager, State};

use crate::asset_manager::{alternative_from_record, svg as svg_tool};
use crate::database::{Database, SymbolFilters};
use crate::engine::Engine;
use crate::export::{caadoc, docx, html};
use crate::model::*;
use crate::pictogram_import;
use crate::profiles;
use crate::online_search;
use crate::speech;
use crate::studio;
use crate::symbol_search;
use crate::util::now_iso;

#[derive(Debug, Serialize)]
pub struct AppInfo {
    pub version: String,
    pub schema_version: i64,
    pub lexicon_concepts: usize,
    pub lexicon_lemmas: usize,
    pub data_dir: String,
    pub resources_dir: String,
    pub warnings: Vec<String>,
    pub platform: String,
}

/// Apre il dialogo di stampa nativo della finestra. `window.print()` nel
/// WebView non è affidabile su tutte le piattaforme: questo comando usa
/// direttamente l'API del webview.
#[tauri::command]
pub async fn print_window(window: tauri::WebviewWindow) -> Result<(), String> {
    window.print().map_err(|e| format!("stampa non disponibile: {e}"))
}

#[tauri::command]
pub fn app_info(state: State<'_, Engine>) -> Result<AppInfo, String> {
    let schema_version = state.db()?.schema_version().map_err(|e| e.to_string())?;
    Ok(AppInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        schema_version,
        lexicon_concepts: state.lexicon.concept_count(),
        lexicon_lemmas: state.lexicon.lemma_count(),
        data_dir: state.paths.data_dir.to_string_lossy().to_string(),
        resources_dir: state.paths.resources_dir.to_string_lossy().to_string(),
        warnings: state.startup_warnings.clone(),
        platform: format!("{} / {}", std::env::consts::OS, std::env::consts::ARCH),
    })
}

/// Comandi sincroni di Tauri girano sul thread principale: un'analisi da
/// 20–90 ms per paragrafo o una sintesi vocale da secondi bloccherebbe la
/// digitazione. Le operazioni pesanti passano quindi su `spawn_blocking`, che
/// non tocca mai il thread dell'interfaccia.
fn on_blocking<T, F>(work: F) -> impl std::future::Future<Output = Result<T, String>>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    async move {
        match tauri::async_runtime::spawn_blocking(work).await {
            Ok(result) => result,
            Err(error) => Err(format!("operazione interrotta: {error}")),
        }
    }
}

#[tauri::command]
pub async fn symbolize_paragraph(
    app: tauri::AppHandle,
    text: String,
    options: SymbolizeOptions,
) -> Result<ParagraphResult, String> {
    on_blocking(move || {
        let state = app.state::<Engine>();
        state.symbolize(&text, &options)
    })
    .await
}

#[tauri::command]
pub async fn refine_paragraph(
    app: tauri::AppHandle,
    text: String,
    options: SymbolizeOptions,
    request_id: Option<String>,
) -> Result<ParagraphResult, String> {
    on_blocking(move || {
        let state = app.state::<Engine>();
        state.refine_cancellable(&text, &options, request_id.as_deref())
    })
    .await
}

/// Annulla una rifinitura neurale superata. Se il modello sta già generando,
/// il lavoro termina ma il risultato viene scartato: non sovrascrive mai il
/// testo nuovo.
#[tauri::command]
pub fn cancel_refine(state: State<'_, Engine>, request_id: String) {
    state.cancel_request(&request_id);
}

#[tauri::command]
pub fn search_symbols(
    state: State<'_, Engine>,
    query: String,
    limit: Option<u32>,
    offset: Option<u32>,
    pack_id: Option<String>,
    commercial_only: Option<bool>,
    collection_id: Option<String>,
) -> Result<SearchResultPage, String> {
    let db = state.db()?;
    db.record_search(&query).ok();
    db.search_symbols(
        &query,
        limit.unwrap_or(30),
        offset.unwrap_or(0),
        &SymbolFilters {
            pack_id,
            commercial_only: commercial_only.unwrap_or(false),
            collection_id,
        },
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn concept_alternatives(
    state: State<'_, Engine>,
    concept_id: String,
    pack_id: Option<String>,
    commercial_only: Option<bool>,
) -> Result<Vec<Alternative>, String> {
    let db = state.db()?;
    let filters = SymbolFilters {
        pack_id,
        commercial_only: commercial_only.unwrap_or(false),
        collection_id: None,
    };
    let symbols = db
        .symbols_for_concept(&concept_id, &filters)
        .map_err(|e| e.to_string())?;
    Ok(symbols.iter().map(alternative_from_record).collect())
}

#[tauri::command]
pub fn set_preference(state: State<'_, Engine>, preference: Preference) -> Result<(), String> {
    let db = state.db()?;
    db.upsert_preference(&Preference {
        created_at: if preference.created_at.is_empty() {
            now_iso()
        } else {
            preference.created_at.clone()
        },
        ..preference
    })
    .map_err(|e| e.to_string())?;
    state.clear_caches();
    Ok(())
}

#[tauri::command]
pub fn delete_preference(
    state: State<'_, Engine>,
    scope: String,
    lemma: String,
    context_key: Option<String>,
) -> Result<(), String> {
    let db = state.db()?;
    db.delete_preference(&scope, &lemma, context_key.as_deref().unwrap_or(""))
        .map_err(|e| e.to_string())?;
    state.clear_caches();
    Ok(())
}

#[tauri::command]
pub fn list_preferences(state: State<'_, Engine>) -> Result<Vec<Preference>, String> {
    state.db()?.list_preferences().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_document_override(
    state: State<'_, Engine>,
    document_key: String,
    unit_key: String,
    symbol_id: Option<String>,
    concept_id: Option<String>,
) -> Result<(), String> {
    let db = state.db()?;
    db.set_document_override(
        &document_key,
        &unit_key,
        symbol_id.as_deref(),
        concept_id.as_deref(),
    )
    .map_err(|e| e.to_string())?;
    state.clear_caches();
    Ok(())
}

#[tauri::command]
pub fn clear_document_override(
    state: State<'_, Engine>,
    document_key: String,
    unit_key: String,
) -> Result<(), String> {
    state
        .db()?
        .clear_document_override(&document_key, &unit_key)
        .map_err(|e| e.to_string())?;
    state.clear_caches();
    Ok(())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn set_unit_state(
    state: State<'_, Engine>,
    document_key: String,
    unit_key: String,
    locked: bool,
    hidden: bool,
    modifiers: UnitModifiers,
) -> Result<(), String> {
    state
        .db()?
        .set_unit_state(&document_key, &unit_key, locked, hidden, &modifiers)
        .map_err(|e| e.to_string())?;
    state.clear_caches();
    Ok(())
}

#[tauri::command]
pub fn save_document(
    state: State<'_, Engine>,
    path: String,
    payload: DocumentPayload,
) -> Result<ExportResult, String> {
    let db = state.db()?;
    let all = db.attributions().map_err(|e| e.to_string())?;
    let used: std::collections::HashSet<&str> = payload
        .assets
        .iter()
        .map(|a| a.symbol_id.as_str())
        .collect();
    let attributions: Vec<AttributionEntry> = all
        .into_iter()
        .filter(|a| used.contains(a.symbol_id.as_str()))
        .collect();
    caadoc::save(Path::new(&path), &payload, &attributions)
}

/// Elimina la bozza automatica. Non accetta un percorso arbitrario: agisce
/// solo sul file dentro la directory dati dell'app, così l'installazione non
/// può proporre di recuperare un lavoro mai fatto.
#[tauri::command]
pub fn remove_recovery_draft(state: State<'_, Engine>) -> Result<bool, String> {
    let path = state.paths.data_dir.join("recovery.caadoc");
    if !path.exists() {
        return Ok(false);
    }
    std::fs::remove_file(&path).map_err(|e| e.to_string())?;
    Ok(true)
}

#[tauri::command]
pub fn load_document(state: State<'_, Engine>, path: String) -> Result<LoadedDocument, String> {    let loaded = caadoc::load(Path::new(&path))?;
    // Le preferenze salvate nel documento tornano nel database locale.
    if let Ok(db) = state.db() {
        for pref in &loaded.preferences {
            // Il simbolo scelto torna solo se era bloccato: un'unità sbloccata
            // deve poter essere rivalutata alla riapertura.
            if pref.locked {
                let _ = db.set_document_override(
                    &loaded.path,
                    &pref.unit_key,
                    pref.symbol_id.as_deref(),
                    pref.concept_id.as_deref(),
                );
            }
            // Visibilità e modificatori si ripristinano sempre.
            let _ = db.set_unit_state(
                &loaded.path,
                &pref.unit_key,
                pref.locked,
                pref.hidden,
                &pref.modifiers,
            );
        }
    }
    state.clear_caches();
    Ok(loaded)
}

#[tauri::command]
pub fn export_html(path: String, doc: ExportDoc) -> Result<ExportResult, String> {
    html::write(Path::new(&path), &doc)
}

#[tauri::command]
pub fn export_docx(path: String, doc: ExportDoc) -> Result<ExportResult, String> {
    docx::write(Path::new(&path), &doc)
}

#[tauri::command]
pub fn export_attributions(
    state: State<'_, Engine>,
    path: String,
) -> Result<ExportResult, String> {
    let entries = state.db()?.attributions().map_err(|e| e.to_string())?;
    let mut out = String::from("# Attribuzioni dei pittogrammi — CAA Writer\n\n");
    out.push_str(
        "Questo registro elenca ogni pittogramma presente nella libreria locale, il repertorio, \
l'autore e la licenza. Conservalo insieme ai materiali distribuiti.\n\n",
    );
    for entry in &entries {
        out.push_str(&format!(
            "## {} (`{}`)\n\n- Repertorio: {} (`{}`)\n- Autore: {}\n- Titolare dei diritti: {}\n- Licenza: {}\n- URL licenza: {}\n- Fonte: {}\n- Vincoli: {}\n- Modifiche: {}\n\n",
            entry.label,
            entry.symbol_id,
            entry.pack_name,
            entry.pack_id,
            entry.author.as_deref().unwrap_or("—"),
            entry.rights_holder.as_deref().unwrap_or("—"),
            entry.license,
            entry.license_url.as_deref().unwrap_or("—"),
            entry.source_url.as_deref().unwrap_or("—"),
            entry.constraints.as_deref().unwrap_or("—"),
            entry.modifications.as_deref().unwrap_or("—"),
        ));
    }
    crate::export::write_atomic(Path::new(&path), out.as_bytes())?;
    Ok(ExportResult {
        path,
        warnings: vec![format!("{} simboli documentati", entries.len())],
    })
}

#[tauri::command]
pub fn list_voices() -> Vec<VoiceInfo> {
    speech::list_voices()
}

#[tauri::command]
pub async fn speak(
    app: tauri::AppHandle,
    text: String,
    voice: Option<String>,
    rate: Option<i32>,
    language: Option<String>,
) -> Result<(), String> {
    // La sintesi può richiedere secondi: non deve bloccare né l'editor né il
    // thread principale. Il processo di riproduzione viene registrato e può
    // essere interrotto da `stop_speaking`.
    on_blocking(move || {
        let state = app.state::<Engine>();
        // Default prudente: la voce è disattivata finché l'utente non la abilita.
        if state.db()?.setting("speech.enabled").as_deref() != Some("true") {
            return Err(
                "sintesi vocale disattivata: attivala in Impostazioni → Accessibilità".to_string(),
            );
        }
        // Voce neurale locale quando disponibile; altrimenti la voce di sistema
        // resta un fallback reale, non un pulsante finto.
        if let Some(paths) = crate::voice::resolve(
            &state.paths.resources_dir,
            &state.paths.data_dir,
            language.as_deref(),
        ) {
            match crate::voice::speak(&text, &paths) {
                Ok(_) => return Ok(()),
                Err(error) => {
                    eprintln!("voce neurale non disponibile ({error}): uso la voce di sistema")
                }
            }
        }
        speech::speak(&text, voice.as_deref(), rate)
    })
    .await
}

/// Interrompe subito la voce in corso (richiesta esplicita dell'utente).
#[tauri::command]
pub fn stop_speaking() {
    speech::stop();
}

#[tauri::command]
pub fn speech_active() -> bool {
    speech::is_speaking()
}

/// Stato reale del motore vocale: installato o fallback di sistema.
#[tauri::command]
pub fn voice_status(
    state: State<'_, Engine>,
    language: Option<String>,
) -> crate::voice::VoiceStatus {
    crate::voice::status(
        &state.paths.resources_dir,
        &state.paths.data_dir,
        language.as_deref(),
    )
}

/// Mostra la forma esatta che la sintesi pronuncerà (numeri, orari, date,
/// abbreviazioni). Il testo del documento non viene modificato.
#[tauri::command]
pub fn normalize_speech(text: String) -> String {
    crate::tts_normalize::normalize(&text)
}

#[tauri::command]
pub fn ai_status(state: State<'_, Engine>) -> AiStatus {
    state.ai_status()
}

#[tauri::command]
pub async fn ai_chat(
    app: tauri::AppHandle,
    messages: Vec<crate::ai_provider::ChatMessage>,
) -> Result<crate::ai_provider::ChatReply, String> {
    on_blocking(move || {
        let state = app.state::<Engine>();
        state.chat(messages)
    })
    .await
}

/// Riempimento automatico online: per le parole senza simbolo personale
/// cerca ARASAAC nella lingua italiana e importa il **primo pittogramma**
/// disponibile come copia offline etichettata con la parola. È una scelta
/// automatica approssimativa: il pannello mostra comunque le alternative.
#[tauri::command]
pub async fn hydrate_symbols(
    app: tauri::AppHandle,
    ids: Vec<String>,
) -> Result<std::collections::HashMap<String, String>, String> {
    on_blocking(move || {
        let state = app.state::<Engine>();
        let db = state.db()?;
        let map = crate::online_search::hydrate_arasaac_symbols(
            &db,
            &state.paths.assets_dir,
            &ids,
        )?;
        if !map.is_empty() {
            state.clear_caches();
        }
        Ok(map)
    })
    .await
}

#[tauri::command]
pub async fn online_autofill(
    app: tauri::AppHandle,
    words: Vec<String>,
) -> Result<Vec<String>, String> {
    on_blocking(move || {
        let state = app.state::<Engine>();
        let db = state.db()?;
        let saved = crate::online_search::autofill_arasaac(
            &db,
            &state.paths.assets_dir,
            &words,
            12,
        )?;
        if !saved.is_empty() {
            state.clear_caches();
        }
        Ok(saved)
    })
    .await
}

#[tauri::command]
pub fn image_status(
    state: State<'_, Engine>,
) -> Result<crate::image_engine::ImageStatus, String> {
    let db = state.db()?;
    Ok(crate::image_engine::status(&state.paths, &db))
}

#[tauri::command]
pub fn image_set_config(
    state: State<'_, Engine>,
    config: crate::image_engine::ImageConfig,
) -> Result<crate::image_engine::ImageStatus, String> {
    let db = state.db()?;
    crate::image_engine::save(&db, &config)?;
    Ok(crate::image_engine::status(&state.paths, &db))
}

#[tauri::command]
pub fn image_set_key(state: State<'_, Engine>, key: String) -> Result<crate::image_engine::ImageStatus, String> {
    crate::image_engine::set_key(&key)?;
    let db = state.db()?;
    Ok(crate::image_engine::status(&state.paths, &db))
}

#[tauri::command]
pub fn image_clear_key(state: State<'_, Engine>) -> Result<crate::image_engine::ImageStatus, String> {
    crate::image_engine::clear_key()?;
    let db = state.db()?;
    Ok(crate::image_engine::status(&state.paths, &db))
}

#[tauri::command]
pub async fn image_generate(
    app: tauri::AppHandle,
    prompt: String,
    negative: Option<String>,
) -> Result<crate::image_engine::GeneratedImage, String> {
    on_blocking(move || {
        let state = app.state::<Engine>();
        let db = state.db()?;
        crate::image_engine::generate(
            &state.paths,
            &db,
            &prompt,
            negative.as_deref().unwrap_or(""),
        )
    })
    .await
}

#[tauri::command]
pub async fn image_install_runtime(app: tauri::AppHandle) -> Result<String, String> {
    on_blocking(move || {
        let state = app.state::<Engine>();
        crate::image_engine::install_runtime(&state.paths)
    })
    .await
}

#[tauri::command]
pub async fn image_download_model(
    app: tauri::AppHandle,
    model: String,
) -> Result<String, String> {
    on_blocking(move || {
        let state = app.state::<Engine>();
        crate::image_engine::download_model(&state.paths, &model)
    })
    .await
}

/// Diagnostica misurata: CPU/RAM dei processi, coda neurale, tempi reali e
/// richieste obsolete. Nessun valore è stimato o inventato.
#[tauri::command]
pub fn ai_provider_status(state: State<'_, Engine>) -> Result<crate::ai_provider::ProviderStatus, String> {
    let db = state.db()?;
    Ok(crate::ai_provider::status(&db))
}

#[tauri::command]
pub fn ai_provider_save(
    state: State<'_, Engine>,
    config: crate::ai_provider::ProviderConfig,
    api_key: Option<String>,
) -> Result<crate::ai_provider::ProviderStatus, String> {
    let db = state.db()?;
    crate::ai_provider::save(&db, &config)?;
    if let Some(key) = api_key.filter(|value| !value.trim().is_empty()) {
        crate::ai_provider::set_key(&key)?;
    }
    Ok(crate::ai_provider::status(&db))
}

#[tauri::command]
pub fn ai_provider_test(
    state: State<'_, Engine>,
) -> Result<crate::ai_provider::ProviderTest, String> {
    let db = state.db()?;
    crate::ai_provider::test(&db)
}

#[tauri::command]
pub fn ai_provider_clear_key(
    state: State<'_, Engine>,
) -> Result<crate::ai_provider::ProviderStatus, String> {
    crate::ai_provider::clear_key()?;
    let db = state.db()?;
    Ok(crate::ai_provider::status(&db))
}

#[tauri::command]
pub fn document_validate(document: serde_json::Value) -> Result<serde_json::Value, String> {
    let document: crate::universal::UniversalDocument =
        serde_json::from_value(document).map_err(|e| format!("documento non valido: {e}"))?;
    crate::universal::validate(&document)?;
    Ok(serde_json::json!({
        "valid": true,
        "pages": document.pages.len(),
        "elements": document.pages.iter().map(|page| page.elements.len()).sum::<usize>(),
        "schema_version": document.schema_version,
    }))
}

#[tauri::command]
pub fn document_render(document: serde_json::Value) -> Result<crate::universal::RenderedDocument, String> {
    let document: crate::universal::UniversalDocument =
        serde_json::from_value(document).map_err(|e| format!("documento non valido: {e}"))?;
    document.render()
}

#[tauri::command]
pub fn generate_trace_sheet(config: crate::trace::TraceConfig) -> Result<String, String> {
    if config.text.trim().is_empty() {
        return Err("Scrivi almeno una lettera o una parola da tracciare.".to_string());
    }
    Ok(crate::trace::trace_sheet_svg(&config))
}

/// Applica una transazione di comandi tipizzati al documento. Il frontend
/// conserva lo stato precedente per undo/redo; il backend valida tutto prima
/// di restituire il nuovo documento.
#[tauri::command]
pub fn document_apply(
    document: serde_json::Value,
    commands: serde_json::Value,
) -> Result<crate::universal::ApplyResult, String> {
    let document: crate::universal::UniversalDocument =
        serde_json::from_value(document).map_err(|e| format!("documento non valido: {e}"))?;
    let commands: Vec<crate::universal::DocumentCommand> =
        serde_json::from_value(commands).map_err(|e| format!("comandi non validi: {e}"))?;
    crate::universal::apply_transaction(&document, &commands)
}

#[tauri::command]
pub fn universal_from_plan(
    plan: studio::ResourcePlan,
) -> Result<crate::universal::UniversalDocument, String> {
    crate::universal_build::from_plan(&plan)
}

/// HTML completo del documento, usato per la stampa di sistema senza
/// dipendere dal DOM dell'editor.
/// Stampa di un documento universale: genera un PDF con il renderer reale
/// (che rispetta formato e orientamento della pagina) e lo apre nel
/// visualizzatore di sistema, da cui si stampa. Su macOS WKWebView non offre
/// un dialogo di stampa affidabile via `window.print()`.
#[tauri::command]
pub async fn print_universal(
    app: tauri::AppHandle,
    document: serde_json::Value,
) -> Result<String, String> {
    on_blocking(move || {
        let document: crate::universal::UniversalDocument = serde_json::from_value(document)
            .map_err(|e| format!("documento non valido: {e}"))?;
        crate::universal::validate(&document)?;
        let bytes = crate::export::universal_pdf::build_pdf(&document, 2.0)?;
        let state = app.state::<Engine>();
        let dir = state.paths.data_dir.join("print");
        std::fs::create_dir_all(&dir).map_err(|e| format!("cartella stampa: {e}"))?;
        let name = format!(
            "stampa-{}.pdf",
            crate::util::now_iso().replace([':', '-', '.'], "").replace('T', "-")
        );
        let path = dir.join(name);
        std::fs::write(&path, bytes).map_err(|e| format!("scrittura PDF di stampa: {e}"))?;
        open_external(&path)?;
        Ok(path.to_string_lossy().to_string())
    })
    .await
}

fn open_external(path: &std::path::Path) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let mut command = crate::process::command("open");
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = crate::process::command("cmd");
        command.args(["/C", "start", ""]);
        command
    };
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = crate::process::command("xdg-open");
    command
        .arg(path)
        .spawn()
        .map_err(|e| format!("apertura del PDF di stampa: {e}"))?;
    Ok(())
}

#[tauri::command]
pub fn document_html(document: serde_json::Value) -> Result<String, String> {
    let document: crate::universal::UniversalDocument =
        serde_json::from_value(document).map_err(|e| format!("documento non valido: {e}"))?;
    crate::universal::validate(&document)?;
    crate::universal_build::export_html(&document)
}

#[tauri::command]
pub fn document_export(
    document: serde_json::Value,
    format: String,
    path: String,
) -> Result<crate::model::ExportResult, String> {
    let document: crate::universal::UniversalDocument =
        serde_json::from_value(document).map_err(|e| format!("documento non valido: {e}"))?;
    crate::universal::validate(&document)?;
    let target = std::path::Path::new(&path);
    let content = match format.as_str() {
        "html" => crate::universal_build::export_html(&document)?,
        "svg" => {
            let rendered = document.render()?;
            let page = rendered
                .pages
                .first()
                .ok_or_else(|| "documento senza pagine".to_string())?;
            crate::universal::render_page_svg(page)
        }
        "pdf" => {
            crate::export::universal_pdf::export_pdf(&document, target)?;
            return Ok(crate::model::ExportResult {
                path,
                warnings: Vec::new(),
            });
        }
        "png" => {
            crate::export::universal_pdf::export_first_page_png(&document, target)?;
            return Ok(crate::model::ExportResult {
                path,
                warnings: vec!["PNG esportato per la prima pagina; usa PDF per il documento completo".to_string()],
            });
        }
        "docx" => {
            return crate::export::universal_docx::write(target, &document);
        }
        "pptx" => {
            return crate::export::universal_pptx::write(target, &document);
        }
        other => return Err(format!("formato di esportazione non supportato: {other}")),
    };
    std::fs::write(target, content).map_err(|e| format!("{}: {e}", target.display()))?;
    Ok(crate::model::ExportResult {
        path,
        warnings: Vec::new(),
    })
}

#[tauri::command]
pub fn take_pending_open(
    pending: State<'_, crate::PendingOpen>,
) -> Option<String> {
    pending.0.lock().ok().and_then(|mut slot| slot.take())
}

/// Importa una copia modificabile nella biblioteca dell'utente. Non sovrascrive
/// il file originale e non tocca le risorse del pacchetto.
#[tauri::command]
pub fn universal_import(
    state: State<'_, Engine>,
    source: String,
) -> Result<OpenedUniversalDocument, String> {
    let source_path = std::path::PathBuf::from(&source);
    let document = crate::universal::load_from_caadoc(&source_path)?;
    let folder = state.paths.data_dir.join("library/imported");
    std::fs::create_dir_all(&folder).map_err(|e| format!("cartella import: {e}"))?;
    let slug = crate::pictogram_import::slugify(&document.title);
    let slug = if slug.is_empty() { document.id.clone() } else { slug };
    let target = folder.join(format!("{slug}.caadoc"));
    let assets = crate::universal_build::assets_from_document(&document);
    crate::universal_build::save_universal(&target, &document, &assets)?;
    Ok(OpenedUniversalDocument {
        document,
        path: target.to_string_lossy().to_string(),
    })
}

#[tauri::command]
pub fn universal_library(state: State<'_, Engine>) -> crate::universal::LibraryManifest {
    crate::universal::load_library_manifest(&state.paths.resources_dir)
}

#[tauri::command]
pub fn library_catalog(
    state: State<'_, Engine>,
) -> crate::template_catalog::CatalogOverview {
    let manifest = crate::packages::filtered_manifest(
        &state.paths.resources_dir,
        &state.paths.data_dir,
    );
    crate::template_catalog::overview(&manifest)
}

#[tauri::command]
pub fn library_search(
    state: State<'_, Engine>,
    query: crate::template_catalog::CatalogQuery,
) -> crate::template_catalog::CatalogPage {
    let manifest = crate::packages::filtered_manifest(
        &state.paths.resources_dir,
        &state.paths.data_dir,
    );
    let mut page = crate::template_catalog::search(&manifest, &query);
    for item in &mut page.items {
        item.preview_svg =
            crate::template_catalog::preview_svg(&state.paths.resources_dir, &item.entry);
    }
    page
}

#[tauri::command]
pub fn library_by_ids(
    state: State<'_, Engine>,
    ids: Vec<String>,
) -> Vec<crate::template_catalog::CatalogItem> {
    let manifest = crate::packages::filtered_manifest(
        &state.paths.resources_dir,
        &state.paths.data_dir,
    );
    let mut items = crate::template_catalog::by_ids(&manifest, &ids);
    for item in &mut items {
        item.preview_svg =
            crate::template_catalog::preview_svg(&state.paths.resources_dir, &item.entry);
    }
    items
}

#[tauri::command]
pub fn library_preview(
    state: State<'_, Engine>,
    id: String,
    source_type: Option<String>,
) -> Result<String, String> {
    let manifest = crate::universal::load_library_manifest(&state.paths.resources_dir);
    let entry = manifest
        .entries
        .iter()
        .find(|entry| {
            entry.id == id
                && source_type
                    .as_deref()
                    .map(|kind| entry.source_type == kind)
                    .unwrap_or(true)
        })
        .ok_or_else(|| format!("documento «{id}» non trovato nella biblioteca"))?;
    if let Some(svg) = crate::template_catalog::preview_svg(&state.paths.resources_dir, entry) {
        return Ok(svg);
    }
    let path = crate::universal::resolve_library_path(
        &manifest,
        &id,
        source_type.as_deref(),
        &state.paths.resources_dir,
    )?;
    let document = crate::universal::load_from_caadoc(&path)?;
    let rendered = document.render()?;
    rendered
        .pages
        .first()
        .map(crate::universal::render_page_svg)
        .ok_or_else(|| "documento senza pagine".to_string())
}

#[tauri::command]
pub fn library_validate(
    state: State<'_, Engine>,
) -> crate::library_quality::LibraryQualityReport {
    let manifest = crate::universal::load_library_manifest(&state.paths.resources_dir);
    crate::library_quality::validate_manifest(&state.paths.resources_dir, &manifest)
}

#[derive(Serialize)]
pub struct OpenedUniversalDocument {
    pub document: crate::universal::UniversalDocument,
    pub path: String,
}

#[tauri::command]
pub fn universal_open(
    state: State<'_, Engine>,
    id: String,
    source_type: Option<String>,
) -> Result<OpenedUniversalDocument, String> {
    let manifest = crate::universal::load_library_manifest(&state.paths.resources_dir);
    let path = crate::universal::resolve_library_path(
        &manifest,
        &id,
        source_type.as_deref(),
        &state.paths.resources_dir,
    )?;
    let document = crate::universal::load_from_caadoc(&path)?;
    Ok(OpenedUniversalDocument {
        document,
        path: path.to_string_lossy().to_string(),
    })
}

#[tauri::command]
pub fn translate_search_term(state: State<'_, Engine>, text: String) -> Result<String, String> {
    state.translate_search_term(&text)
}

#[tauri::command]
pub fn story_create(
    state: State<'_, Engine>,
    spec: serde_json::Value,
) -> Result<OpenedUniversalDocument, String> {
    let outcome = crate::story::build_story(&state, &spec)?;
    let id = outcome.document.id.clone();
    let folder = state.paths.data_dir.join("documents");
    std::fs::create_dir_all(&folder).map_err(|e| format!("cartella documenti: {e}"))?;
    let path = folder.join(format!("{id}.caadoc"));
    crate::story::save_story(&path, &outcome.document, &outcome.assets)?;
    Ok(OpenedUniversalDocument {
        document: outcome.document,
        path: path.to_string_lossy().to_string(),
    })
}

#[tauri::command]
pub fn runtime_diagnostics(state: State<'_, Engine>) -> crate::engine::RuntimeDiagnostics {
    state.diagnostics()
}

#[tauri::command]
pub fn set_ai_config(state: State<'_, Engine>, config: AiConfig) -> Result<AiStatus, String> {
    state.set_ai_config(config)?;
    Ok(state.ai_status())
}

#[tauri::command]
pub fn ai_models(state: State<'_, Engine>) -> Vec<crate::ai_engine::catalog::ModelEntry> {
    state.ai_models()
}

#[tauri::command]
pub fn ai_install_model(
    state: State<'_, Engine>,
    model_id: String,
) -> Result<crate::ai_engine::installer::InstallStatus, String> {
    state.ai_install_model(&model_id)
}

#[tauri::command]
pub fn ai_install_status(
    state: State<'_, Engine>,
) -> crate::ai_engine::installer::InstallStatus {
    state.ai_install_status()
}

#[tauri::command]
pub fn ai_cancel_model_install(
    state: State<'_, Engine>,
) -> crate::ai_engine::installer::InstallStatus {
    state.ai_cancel_install();
    state.ai_install_status()
}

#[tauri::command]
pub fn ai_remove_model(
    state: State<'_, Engine>,
    model_id: String,
) -> Result<AiStatus, String> {
    state.ai_remove_model(&model_id)
}

#[tauri::command]
pub fn ai_import_model(state: State<'_, Engine>, path: String) -> Result<AiStatus, String> {
    state.ai_import_model(&path)
}

/// Sinonimi e concetti affini con il modello locale, per arricchire le
/// alternative in background. Non restituisce errori quando il motore è
/// spento: il pannello continua a usare il lessico deterministico.
#[tauri::command]
pub async fn ai_affinity_symbols(
    app: tauri::AppHandle,
    word: String,
) -> Result<Vec<Alternative>, String> {
    on_blocking(move || Ok(app.state::<Engine>().ai_affinity_symbols(&word))).await
}

#[tauri::command]
pub async fn ai_related_symbols(
    app: tauri::AppHandle,
    text: String,
) -> Result<Vec<Alternative>, String> {
    on_blocking(move || app.state::<Engine>().ai_related_symbols(&text)).await
}

#[tauri::command]
pub fn ai_select_model(state: State<'_, Engine>, model_id: String) -> Result<AiStatus, String> {
    state.ai_select_model(&model_id)?;
    Ok(state.ai_status())
}

#[tauri::command]
pub async fn ai_benchmark(app: tauri::AppHandle) -> Result<BenchmarkResult, String> {
    on_blocking(move || app.state::<Engine>().ai_benchmark()).await
}

#[tauri::command]
pub async fn ai_run_tool(
    app: tauri::AppHandle,
    tool: String,
    input: serde_json::Value,
) -> Result<crate::ai_engine::tools::ToolOutcome, String> {
    on_blocking(move || app.state::<Engine>().run_ai_tool(&tool, &input)).await
}

#[tauri::command]
pub fn library_stats(state: State<'_, Engine>) -> Result<LibraryStats, String> {
    state.db()?.statistics().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_packs(state: State<'_, Engine>) -> Result<Vec<PackInfo>, String> {
    state.db()?.list_packs().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn package_catalog(
    state: State<'_, Engine>,
) -> Result<crate::packages::PackageCatalog, String> {
    crate::packages::catalog(&state)
}

#[tauri::command]
pub fn package_set_library(
    state: State<'_, Engine>,
    id: String,
    enabled: bool,
) -> Result<(), String> {
    crate::packages::set_library_enabled(&state.paths.data_dir, &id, enabled)
}

#[tauri::command]
pub fn download_start(app: tauri::AppHandle, id: String) -> Result<(), String> {
    crate::downloads::start(app, id)
}

#[tauri::command]
pub fn download_status() -> crate::downloads::DownloadSnapshot {
    crate::downloads::status()
}

#[tauri::command]
pub fn download_reset() {
    crate::downloads::reset_finished();
}

#[tauri::command]
pub async fn package_download(
    app: tauri::AppHandle,
    id: String,
) -> Result<crate::packages::PackageReport, String> {
    on_blocking(move || {
        let state = app.state::<Engine>();
        match id.as_str() {
            "symbols:arasaac" => crate::packages::download_arasaac(&state, 900),
            "symbols:globalsymbols" => crate::packages::download_globalsymbols(&state, 80),
            _ => Err(format!(
                "Il pacchetto «{id}» è incluso nell'app e funziona già offline: non serve scaricarlo."
            )),
        }
    })
    .await
}

#[tauri::command]
pub fn package_remove(state: State<'_, Engine>, id: String) -> Result<(), String> {
    let pack = id
        .strip_prefix("symbols:")
        .ok_or_else(|| "Solo i pacchetti di pittogrammi si possono rimuovere.".to_string())?;
    crate::packages::remove_symbol_pack(&state, pack)
}

#[tauri::command]
pub async fn check_updates(
    app: tauri::AppHandle,
) -> Result<crate::packages::UpdateInfo, String> {
    on_blocking(move || {
        let state = app.state::<Engine>();
        Ok(crate::packages::check_updates(&state))
    })
    .await
}

#[tauri::command]
pub fn open_repository_link(url: String) -> Result<(), String> {
    crate::packages::open_external(&url)
}

#[tauri::command]
pub fn list_templates(state: State<'_, Engine>) -> Result<Vec<studio::TemplateInfo>, String> {
    studio::list_templates(&state.paths.templates_dir())
}

/// Interpreta una richiesta in linguaggio naturale e produce una specifica
/// JSON validata; il layout viene costruito dal frontend, non dal modello.
/// Impagina una specifica di layout: valida, risolve i pittogrammi con il
/// motore condiviso e restituisce le pagine con le misure in millimetri.
/// Applica una modifica strutturata al layout, da richiesta in linguaggio
/// naturale o da un elenco di operazioni. Restituisce la specifica aggiornata e
/// le pagine già ricalcolate.
#[tauri::command]
pub fn edit_layout(
    state: State<'_, Engine>,
    spec: crate::layout::LayoutSpecification,
    prompt: Option<String>,
    operations: Option<Vec<crate::layout_edit::LayoutOperation>>,
) -> Result<crate::layout_edit::LayoutEditResult, String> {
    state.edit_layout(spec, prompt, operations)
}

#[tauri::command]
pub fn build_layout(
    state: State<'_, Engine>,
    spec: crate::layout::LayoutSpecification,
) -> Result<crate::layout::LayoutDocument, String> {
    state.layout_document(spec)
}

#[tauri::command]
pub fn plan_resource(
    state: State<'_, Engine>,
    prompt: String,
    template_id: Option<String>,
) -> Result<studio::ResourcePlan, String> {
    let templates = studio::list_templates(&state.paths.templates_dir())?;
    if templates.is_empty() {
        return Err("nessun modello disponibile nella cartella resources/templates".to_string());
    }

    let (template, mut labels, mut notes) = match template_id {
        Some(id) => (
            templates.iter().find(|t| t.id == id).cloned(),
            Vec::new(),
            Vec::new(),
        ),
        None => {
            let (template, labels, notes) = studio::interpret_prompt(&prompt, &templates);
            (template, labels, notes)
        }
    };

    let Some(template) = template else {
        return Err("modello non trovato".to_string());
    };
    if labels.is_empty() {
        labels = template.items.iter().map(|i| i.label.clone()).collect();
        notes.push("Contenuti predefiniti del modello: modificali nell'editor.".to_string());
    }
    if labels.len() > 60 {
        labels.truncate(60);
        notes.push("Elenco ridotto a 60 elementi per mantenere leggibile il layout".to_string());
    }

    let opts = SymbolizeOptions {
        paragraph_id: "studio".to_string(),
        intensity: Intensity::Minima,
        mode: CaaMode::Sopra,
        ..Default::default()
    };

    let mut items = Vec::new();
    for label in labels {
        let result = state.symbolize(&label, &opts)?;
        let token = result
            .tokens
            .iter()
            .find(|t| !t.is_punct && (t.symbol_id.is_some() || !t.composition.is_empty()));
        match token {
            Some(token) => items.push(studio::PlannedItem {
                label: label.clone(),
                concept_id: token.concept_id.clone(),
                strategy: token.strategy.clone(),
                symbol_id: token.symbol_id.clone(),
                svg: token.svg.clone(),
                image: token.image.clone(),
                approximate: token.approximate,
            }),
            None => {
                items.push(studio::PlannedItem {
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

    Ok(studio::ResourcePlan {
        template_id: template.id.clone(),
        title: template.name.clone(),
        kind: template.kind.clone(),
        grid: template.grid.clone().unwrap_or(studio::GridSpec {
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
    })
}

/// Analizza file e directory per l'importazione: formato reale, lemma
/// proposto, duplicati rispetto alla libreria e miniature dei primi elementi.
#[tauri::command]
pub fn scan_pictograms(
    state: State<'_, Engine>,
    paths: Vec<String>,
) -> Result<pictogram_import::ImportPreview, String> {
    let mut preview = pictogram_import::scan(&paths, 160);
    let db = state.db()?;
    let mut known: std::collections::HashSet<String> = std::collections::HashSet::new();
    if let Ok(mut stmt) = db.conn().prepare("SELECT checksum FROM symbols WHERE checksum IS NOT NULL") {
        if let Ok(rows) = stmt.query_map([], |row| row.get::<_, String>(0)) {
            for row in rows.flatten() {
                known.insert(row);
            }
        }
    }
    let mut duplicates = preview.duplicates;
    for entry in &mut preview.entries {
        if let Some(hash) = &entry.checksum {
            if known.contains(hash) && !entry.duplicate {
                entry.duplicate = true;
                duplicates += 1;
            }
        }
    }
    preview.duplicates = duplicates;
    Ok(preview)
}

/// Importa i pittogrammi confermati dall'utente. Copia i file nella libreria
/// dell'app e li indicizza nell'indice semantico condiviso.
#[tauri::command]
pub fn import_pictograms(
    state: State<'_, Engine>,
    entries: Vec<pictogram_import::ImportItem>,
    collection: String,
    license: pictogram_import::ImportLicense,
) -> Result<pictogram_import::ImportReport, String> {
    let db = state.db()?;
    let report = pictogram_import::import_batch(
        &db,
        &state.paths.assets_dir,
        &entries,
        &license,
        &collection,
    );
    state.clear_caches();
    Ok(report)
}

/// Importa un'immagine (PNG, JPEG, WebP, GIF, BMP, TIFF, ICO, AVIF) come
/// pittogramma personale. Il file resta un'immagine: nessuna conversione in
/// SVG. Restituisce l'identificativo del simbolo creato.
#[tauri::command]
pub fn import_image_symbol(
    state: State<'_, Engine>,
    name: String,
    data_base64: String,
    lemma: String,
    meaning: Option<String>,
    collection: Option<String>,
) -> Result<String, String> {
    use base64::Engine as _;
    let payload = data_base64
        .split_once(',')
        .map(|(_, data)| data)
        .unwrap_or(&data_base64);
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(payload)
        .map_err(|error| format!("immagine base64 non valida: {error}"))?;
    if bytes.len() > 16 * 1024 * 1024 {
        return Err("immagine oltre il limite di 16 MB".to_string());
    }
    let detected = pictogram_import::detect_format(&bytes);
    if !detected.decoder_available() {
        return Err(format!(
            "formato {} non supportato da questo sistema",
            detected.as_str()
        ));
    }
    let tmp_dir = state.paths.assets_dir.join("tmp");
    std::fs::create_dir_all(&tmp_dir).map_err(|e| format!("cartella temporanea: {e}"))?;
    let tmp_path = tmp_dir.join(format!(
        "import-{}.{}",
        now_iso().replace([':', '-', '.'], ""),
        detected.as_str()
    ));
    std::fs::write(&tmp_path, &bytes).map_err(|e| format!("salvataggio temporaneo: {e}"))?;
    let item = pictogram_import::ImportItem {
        path: tmp_path.to_string_lossy().to_string(),
        lemma,
        collection: collection.or_else(|| Some("Personali".to_string())),
        category: meaning.or_else(|| Some("importato".to_string())),
        synonyms: Vec::new(),
        relative_path: Some(name.clone()),
        original_name: Some(name),
    };
    let license = pictogram_import::ImportLicense {
        name: Some("CC BY-NC-SA 4.0".to_string()),
        url: Some("https://creativecommons.org/licenses/by-nc-sa/4.0/".to_string()),
        author: Some("Utente".to_string()),
        source_url: None,
        commercial_ok: true,
    };
    let db = state.db()?;
    let report = pictogram_import::import_batch(
        &db,
        &state.paths.assets_dir,
        &[item],
        &license,
        "Personali",
    );
    let _ = std::fs::remove_file(&tmp_path);
    state.clear_caches();
    report
        .items
        .into_iter()
        .find_map(|outcome| outcome.symbol_id)
        .ok_or_else(|| {
            if report.warnings.is_empty() {
                "importazione non riuscita".to_string()
            } else {
                report.warnings.join("; ")
            }
        })
}

#[tauri::command]
pub fn list_pictogram_collections(
    state: State<'_, Engine>,
) -> Result<Vec<pictogram_import::CollectionInfo>, String> {
    let db = state.db()?;
    pictogram_import::list_collections(&db)
}

#[tauri::command]
pub fn rename_pictogram_collection(
    state: State<'_, Engine>,
    id: String,
    name: String,
) -> Result<(), String> {
    let db = state.db()?;
    pictogram_import::rename_collection(&db, &id, &name)
}

#[tauri::command]
pub fn delete_pictogram_collection(
    state: State<'_, Engine>,
    id: String,
) -> Result<usize, String> {
    let db = state.db()?;
    let removed = pictogram_import::delete_collection(&db, &state.paths.assets_dir, &id)?;
    state.clear_caches();
    Ok(removed)
}

#[tauri::command]
pub fn delete_pictogram_symbols(
    state: State<'_, Engine>,
    ids: Vec<String>,
) -> Result<usize, String> {
    let db = state.db()?;
    let removed = pictogram_import::delete_symbols(&db, &state.paths.assets_dir, &ids)?;
    state.clear_caches();
    Ok(removed)
}

#[tauri::command]
pub fn move_pictogram_symbols(
    state: State<'_, Engine>,
    ids: Vec<String>,
    collection_id: String,
) -> Result<usize, String> {
    let db = state.db()?;
    let moved = pictogram_import::move_symbols(&db, &ids, &collection_id)?;
    state.clear_caches();
    Ok(moved)
}

#[derive(Debug, Serialize)]
pub struct CliRunResult {
    pub ok: bool,
    pub stdout: String,
    pub stderr: String,
    pub json: Option<serde_json::Value>,
    pub binary: String,
}

fn find_cli_binary(resources_dir: &Path) -> Option<std::path::PathBuf> {
    let name = if cfg!(windows) { "caa-cli.exe" } else { "caa-cli" };
    let mut candidates: Vec<std::path::PathBuf> = vec![resources_dir.join("cli").join(name)];
    if let Ok(executable) = std::env::current_exe() {
        if let Some(dir) = executable.parent() {
            candidates.push(dir.join(name));
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        for ancestor in cwd.ancestors().take(5) {
            for profile in ["release", "debug"] {
                candidates.push(ancestor.join("target").join(profile).join(name));
                candidates.push(
                    ancestor
                        .join("src-tauri/target")
                        .join(profile)
                        .join(name),
                );
            }
        }
    }
    candidates.into_iter().find(|path| path.is_file())
}

/// Esegue un metodo della CLI headless e restituisce l'output alla GUI.
/// La console integrata non usa una shell: metodo e parametri sono dati,
/// non comandi di sistema.
#[tauri::command]
pub async fn run_cli(
    app: tauri::AppHandle,
    method: String,
    params: serde_json::Value,
) -> Result<CliRunResult, String> {
    on_blocking(move || {
        let state = app.state::<Engine>();
        let binary = find_cli_binary(&state.paths.resources_dir)
            .ok_or_else(|| "CLI non trovata: compila `caa-cli` o reinstalla l'app.".to_string())?;
        let payload = serde_json::to_string(&params).map_err(|e| format!("parametri: {e}"))?;
        let output = crate::process::command(&binary)
            .arg("--compact")
            .arg("--data-dir")
            .arg(&state.paths.data_dir)
            .arg("--resources")
            .arg(&state.paths.resources_dir)
            .arg("call")
            .arg(&method)
            .arg(&payload)
            .output()
            .map_err(|e| format!("avvio CLI: {e}"))?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let json = serde_json::from_str::<serde_json::Value>(stdout.trim()).ok();
        Ok(CliRunResult {
            ok: output.status.success(),
            stdout,
            stderr,
            json,
            binary: binary.to_string_lossy().to_string(),
        })
    })
    .await
}

#[tauri::command]
pub fn profiles_list(state: State<'_, Engine>) -> Result<Vec<profiles::Profile>, String> {
    let db = state.db()?;
    profiles::list_profiles(&db)
}

#[tauri::command]
pub fn profile_save(
    state: State<'_, Engine>,
    profile: profiles::Profile,
    creator_pin: Option<String>,
) -> Result<(), String> {
    let db = state.db()?;
    profiles::require_creator(&db, creator_pin.as_deref().unwrap_or(""))?;
    profiles::save_profile(&db, &profile)
}

#[tauri::command]
pub fn profile_delete(
    state: State<'_, Engine>,
    id: String,
    creator_pin: Option<String>,
) -> Result<(), String> {
    let db = state.db()?;
    profiles::require_creator(&db, creator_pin.as_deref().unwrap_or(""))?;
    profiles::delete_profile(&db, &id)
}

#[tauri::command]
pub fn profile_set_active(state: State<'_, Engine>, id: String) -> Result<(), String> {
    let db = state.db()?;
    profiles::set_active_profile(&db, &id)
}

#[tauri::command]
pub fn profile_active(state: State<'_, Engine>) -> Result<Option<profiles::Profile>, String> {
    let db = state.db()?;
    match profiles::active_profile(&db) {
        Some(id) => profiles::get_profile(&db, &id),
        None => Ok(None),
    }
}

#[tauri::command]
pub fn profile_boards(
    state: State<'_, Engine>,
    profile_id: String,
) -> Result<Vec<profiles::ProfileBoard>, String> {
    let db = state.db()?;
    let mut boards = profiles::list_profile_boards(&db, &profile_id)?;
    let meta = profiles::list_boards(&state.paths.boards_dir()).unwrap_or_default();
    // Oltre ai 30 sistemi in resources/boards si possono assegnare i 330
    // documenti della biblioteca universale (template e materiali didattici).
    let library = crate::universal::load_library_manifest(&state.paths.resources_dir);
    for board in &mut boards {
        if let Some(found) = meta.iter().find(|item| item.id == board.board_id) {
            board.title = found.title.clone();
            board.description = found.description.clone();
            board.pages = found.pages;
        } else if let Some(found) = library
            .entries
            .iter()
            .find(|entry| entry.id == board.board_id)
        {
            board.title = found.title.clone();
            board.description = found.description.clone();
            board.pages = found.pages;
        }
    }
    Ok(boards)
}

#[tauri::command]
pub fn profile_assign_board(
    state: State<'_, Engine>,
    profile_id: String,
    board_id: String,
    enabled: bool,
    position: i64,
    creator_pin: Option<String>,
) -> Result<(), String> {
    let db = state.db()?;
    profiles::require_creator(&db, creator_pin.as_deref().unwrap_or(""))?;
    profiles::assign_board(&db, &profile_id, &board_id, enabled, position)
}

#[tauri::command]
pub fn profile_remove_board(
    state: State<'_, Engine>,
    profile_id: String,
    board_id: String,
    creator_pin: Option<String>,
) -> Result<(), String> {
    let db = state.db()?;
    profiles::require_creator(&db, creator_pin.as_deref().unwrap_or(""))?;
    profiles::remove_board(&db, &profile_id, &board_id)
}

#[tauri::command]
pub fn boards_list(state: State<'_, Engine>) -> Result<Vec<profiles::BoardMeta>, String> {
    profiles::list_boards(&state.paths.boards_dir())
}

#[tauri::command]
pub fn board_build(state: State<'_, Engine>, id: String) -> Result<profiles::BoardSystem, String> {
    profiles::build_board(&state, &state.paths.boards_dir(), &id)
}

#[tauri::command]
pub fn creator_pin_status(state: State<'_, Engine>) -> Result<bool, String> {
    let db = state.db()?;
    Ok(profiles::pin_enabled(&db))
}

#[tauri::command]
pub fn creator_set_pin(
    state: State<'_, Engine>,
    pin: Option<String>,
    current_pin: Option<String>,
) -> Result<(), String> {
    let db = state.db()?;
    profiles::require_creator(&db, current_pin.as_deref().unwrap_or(""))?;
    profiles::set_pin(&db, pin.as_deref())
}

#[tauri::command]
pub fn creator_verify_pin(state: State<'_, Engine>, pin: String) -> Result<bool, String> {
    let db = state.db()?;
    Ok(profiles::verify_pin(&db, &pin))
}

#[tauri::command]
pub fn interpret_document_prompt(
    state: State<'_, Engine>,
    prompt: String,
    use_neural: Option<bool>,
) -> Result<crate::document_edit::DocumentEditPlan, String> {
    if prompt.trim().is_empty() {
        return Err("Scrivi una richiesta di modifica.".to_string());
    }
    Ok(state.interpret_document_prompt(&prompt, use_neural.unwrap_or(true)))
}

#[tauri::command]
pub fn list_online_providers(
    state: State<'_, Engine>,
) -> Result<Vec<online_search::OnlineProviderInfo>, String> {
    let db = state.db()?;
    Ok(online_search::providers_with_db(&db))
}

#[tauri::command]
pub fn save_online_provider(
    state: State<'_, Engine>,
    provider: online_search::CustomProvider,
) -> Result<Vec<online_search::OnlineProviderInfo>, String> {
    let db = state.db()?;
    online_search::save_custom_provider(&db, provider)?;
    Ok(online_search::providers_with_db(&db))
}

#[tauri::command]
pub fn delete_online_provider(
    state: State<'_, Engine>,
    id: String,
) -> Result<Vec<online_search::OnlineProviderInfo>, String> {
    let db = state.db()?;
    online_search::delete_custom_provider(&db, &id)?;
    Ok(online_search::providers_with_db(&db))
}

/// Cerca online usando solo lemmi/concetti generici già estratti localmente.
/// La rete non deve bloccare il thread dell'interfaccia: un provider lento
/// lascerebbe il pannello su «Cerco…» senza spiegare perché.
#[tauri::command]
pub async fn search_online_pictograms(
    app: tauri::AppHandle,
    terms: Vec<String>,
    force_refresh: Option<bool>,
) -> Result<online_search::OnlineResponse, String> {
    on_blocking(move || {
        let state = app.state::<Engine>();
        // I termini noti al lessico sono la prima scelta; se l'utente cerca
        // esplicitamente una parola nuova, si invia solo quella parola pulita
        // (niente frasi, email, URL o numeri), non il documento.
        let terms = online_search::terms_for_search(&state.lexicon, &terms);
        if terms.is_empty() {
            return Err(
                "Nessun termine valido: la ricerca online non invia testo libero, frasi o dati personali."
                    .to_string(),
            );
        }
        let db = state.db()?;
        online_search::search(&db, &terms, force_refresh.unwrap_or(false))
    })
    .await
}

/// Inserimento automatico durante la scrittura: cerca solo pittogrammi ARASAAC
/// per le parole fuori lessico, li salva come copie offline e restituisce le
/// parole importate. Usa una connessione separata per non tenere bloccato il
/// database principale durante la rete.
#[tauri::command]
pub async fn ensure_online_symbols(
    app: tauri::AppHandle,
    words: Vec<String>,
) -> Result<Vec<String>, String> {
    on_blocking(move || {
        let state = app.state::<Engine>();
        let db = Database::open(&state.paths.db_path)
            .map_err(|e| format!("database online: {e}"))?;
        let saved = online_search::ensure_arasaac_symbols(
            &db,
            &state.paths.assets_dir,
            &words,
            6,
        )?;
        if !saved.is_empty() {
            state.clear_caches();
        }
        Ok(saved)
    })
    .await
}

/// Scarica un risultato, lo salva nella libreria personale (copia offline) e
/// restituisce l'alternativa pronta da applicare all'unità selezionata.
#[tauri::command]
pub async fn online_thumbnail(url: String) -> Result<String, String> {
    on_blocking(move || online_search::download_thumbnail(&url)).await
}

#[tauri::command]
pub async fn save_online_pictogram(
    app: tauri::AppHandle,
    result: online_search::OnlineResult,
    lemma: String,
    collection: String,
) -> Result<Alternative, String> {
    on_blocking(move || {
        let state = app.state::<Engine>();
        let db = state.db()?;
        let symbol_id = online_search::save_result(
            &db,
            &state.paths.assets_dir,
            &result,
            &lemma,
            &collection,
        )?;
        state.clear_caches();
        let record = db
            .symbol_by_id(&symbol_id)?
            .ok_or_else(|| "simbolo importato non trovato".to_string())?;
        Ok(alternative_from_record(&record))
    })
    .await
}

/// Risultato della ricerca inglese usata per arricchire le alternative.
#[derive(Debug, Clone, serde::Serialize)]
pub struct EnglishAlternatives {
    pub term: String,
    pub items: Vec<Alternative>,
    pub warnings: Vec<String>,
}

/// Traduce la parola in inglese (glossario o modello locale) e cerca
/// pittogrammi su Global Symbols, l'unico repertorio inglese utile: ARASAAC
/// in inglese duplica il catalogo italiano. I risultati vengono importati
/// come copie locali e restituiti come alternative per la parola italiana.
#[tauri::command]
pub async fn english_alternatives(
    app: tauri::AppHandle,
    word: String,
    limit: Option<usize>,
) -> Result<EnglishAlternatives, String> {
    on_blocking(move || {
        let state = app.state::<Engine>();
        let term = state.translate_to_english(&word)?;
        let db = state.db()?;
        let response = crate::online_search::search_global_symbols(&db, &term)?;
        let mut items: Vec<Alternative> = Vec::new();
        let mut warnings = response.warnings;
        let take = limit.unwrap_or(5).clamp(1, 8);
        for result in response
            .results
            .into_iter()
            // I symbolset ARASAAC di Global Symbols duplicano il catalogo
            // italiano: si escludono, come richiesto.
            .filter(|result| {
                result.kind == "pittogramma"
                    && !result.source.to_ascii_lowercase().contains("arasaac")
            })
            .take(take)
        {
            // L'etichetta inglese descrive il prestito dal repertorio; la
            // didascalia sotto il pittogramma resta sempre la parola italiana
            // scritta dall'utente.
            let label = if result.title.trim().is_empty() {
                term.clone()
            } else {
                result.title.clone()
            };
            match crate::online_search::save_result(
                &db,
                &state.paths.assets_dir,
                &result,
                &label,
                "Inglese (Global Symbols)",
            ) {
                Ok(symbol_id) => match db.symbol_by_id(&symbol_id).map_err(|e| e.to_string())? {
                    Some(record) => {
                        let mut alternative = crate::asset_manager::alternative_from_record(&record);
                        alternative.pack_name = "Global Symbols (EN)".to_string();
                        if items
                            .iter()
                            .all(|item| item.symbol_id != alternative.symbol_id)
                        {
                            items.push(alternative);
                        }
                    }
                    None => warnings.push(format!(
                        "il pittogramma «{}» non è stato trovato dopo l'importazione",
                        result.title
                    )),
                },
                Err(error) => warnings.push(error),
            }
        }
        // Ripiego: se il termine tradotto non basta, si provano sinonimi
        // inglesi del modello locale, sempre su Global Symbols. Restano
        // alternative dichiarate (badge EN) e non scelte automatiche.
        if items.len() < 4 {
            for synonym in state.ai_english_synonyms(&term).into_iter().take(3) {
                if items.len() >= 6 {
                    break;
                }
                let Ok(response) = crate::online_search::search_global_symbols(&db, &synonym)
                else {
                    continue;
                };
                for result in response
                    .results
                    .into_iter()
                    .filter(|result| {
                        result.kind == "pittogramma"
                            && !result.source.to_ascii_lowercase().contains("arasaac")
                    })
                    .take(2)
                {
                    let label = if result.title.trim().is_empty() {
                        synonym.clone()
                    } else {
                        result.title.clone()
                    };
                    let Ok(symbol_id) = crate::online_search::save_result(
                        &db,
                        &state.paths.assets_dir,
                        &result,
                        &label,
                        "Inglese (Global Symbols)",
                    ) else {
                        continue;
                    };
                    let Some(record) = db.symbol_by_id(&symbol_id).map_err(|e| e.to_string())?
                    else {
                        continue;
                    };
                    let mut alternative = crate::asset_manager::alternative_from_record(&record);
                    alternative.pack_name = "Global Symbols (EN)".to_string();
                    alternative.neural = true;
                    if items
                        .iter()
                        .all(|item| item.symbol_id != alternative.symbol_id)
                    {
                        items.push(alternative);
                    }
                }
            }
        }
        state.clear_caches();
        Ok(EnglishAlternatives {
            term,
            items,
            warnings,
        })
    })
    .await
}

/// Risoluzione automatica di una parola rimasta senza pittogramma dopo la
/// ricerca locale e ARASAAC online. Ordine: sinonimi italiani (modello
/// locale) → Global Symbols con traduzione inglese → sinonimi inglesi su
/// Global Symbols. Ogni risultato è già una copia locale o un simbolo del
/// database; il frontend lo applica all'occorrenza.
#[derive(Debug, Serialize)]
pub struct ResolvedSymbol {
    pub word: String,
    pub alternative: Alternative,
    pub stage: String,
}

fn global_symbol_for_label(
    state: &Engine,
    label: &str,
    query: &str,
) -> Result<Option<Alternative>, String> {
    let db = state.db()?;
    let response = match online_search::search_global_symbols(&db, query) {
        Ok(response) => response,
        Err(_) => return Ok(None),
    };
    let Some(result) = response.results.into_iter().find(|result| {
        result.kind == "pittogramma"
            && !result.source.to_ascii_lowercase().contains("arasaac")
    }) else {
        return Ok(None);
    };
    let symbol_id = online_search::save_result(
        &db,
        &state.paths.assets_dir,
        &result,
        label,
        "Global Symbols automatico",
    )?;
    let Some(record) = db.symbol_by_id(&symbol_id)? else {
        return Ok(None);
    };
    let mut alternative = alternative_from_record(&record);
    alternative.pack_name = "Global Symbols (EN)".to_string();
    alternative.approximate = true;
    Ok(Some(alternative))
}

/// Implementazione condivisa da Tauri e CLI. Ordine: copia personale esatta →
/// sinonimi italiani → Global Symbols con traduzione → sinonimi inglesi.
pub fn resolve_missing_symbols_for_engine(
    state: &Engine,
    words: &[String],
) -> Result<Vec<ResolvedSymbol>, String> {
    let personal = SymbolFilters {
        pack_id: Some("personal".to_string()),
        ..Default::default()
    };
    let mut out = Vec::new();
    for raw in words.iter().take(4) {
        let word = crate::linguistics::normalize::normalize_word(raw);
        if word.is_empty() {
            continue;
        }
        {
            let db = state.db()?;
            if let Some(record) = db.symbol_by_label(&word, &personal)? {
                let mut alternative = alternative_from_record(&record);
                alternative.approximate = false;
                out.push(ResolvedSymbol {
                    word: word.clone(),
                    alternative,
                    stage: "locale".to_string(),
                });
                continue;
            }
        }
        if let Some(mut alternative) = state.ai_affinity_symbols(&word).into_iter().next() {
            alternative.approximate = true;
            alternative.neural = true;
            out.push(ResolvedSymbol {
                word: word.clone(),
                alternative,
                stage: "sinonimo italiano".to_string(),
            });
            continue;
        }
        let term = state.translate_to_english(&word).ok();
        if let Some(term) = &term {
            if let Some(mut alternative) = global_symbol_for_label(state, &word, term)? {
                alternative.neural = false;
                out.push(ResolvedSymbol {
                    word: word.clone(),
                    alternative,
                    stage: "Global Symbols".to_string(),
                });
                continue;
            }
        }
        let base = term.clone().unwrap_or_else(|| word.clone());
        for synonym in state.ai_english_synonyms(&base).into_iter().take(4) {
            if let Some(mut alternative) = global_symbol_for_label(state, &word, &synonym)? {
                alternative.neural = true;
                out.push(ResolvedSymbol {
                    word: word.clone(),
                    alternative,
                    stage: "sinonimo inglese".to_string(),
                });
                break;
            }
        }
    }
    state.clear_caches();
    Ok(out)
}

#[tauri::command]
pub async fn resolve_missing_symbols(
    app: tauri::AppHandle,
    words: Vec<String>,
) -> Result<Vec<ResolvedSymbol>, String> {
    on_blocking(move || {
        let state = app.state::<Engine>();
        resolve_missing_symbols_for_engine(&state, &words)
    })
    .await
}

/// Importa un SVG originale come simbolo personalizzato locale.
#[tauri::command]
pub fn import_svg_symbol(
    state: State<'_, Engine>,
    name: String,
    svg: String,
    lemma: String,
    meaning: Option<String>,
    concept_id: Option<String>,
) -> Result<String, String> {
    // Le immagini generate dal motore neurale sono PNG incorporati in un SVG:
    // 4 MB coprono 512×512 senza permettere file arbitrariamente grandi.
    let sanitized = svg_tool::sanitize(&svg, 4 * 1024 * 1024)?;
    let db = state.db()?;
    let now = now_iso();

    db.conn()
        .execute(
            "INSERT INTO symbol_packs(id, name, version, description, author, rights_holder, license,
                license_url, source_url, constraints, commercial_ok, share_alike, no_derivatives, builtin, installed_at)
             VALUES ('custom','Simboli personali','1.0','Simboli aggiunti dall''utente con licenza dichiarata',
                'Utente','Utente','CC BY-NC-SA 4.0','https://creativecommons.org/licenses/by-nc-sa/4.0/',NULL,
                'Licenza scelta dall''utente al momento dell''importazione',1,0,0,0,?1)
             ON CONFLICT(id) DO NOTHING",
            rusqlite::params![now],
        )
        .map_err(|e| e.to_string())?;

    let slug = slugify(&lemma);
    let concept = concept_id.unwrap_or_else(|| format!("custom.{slug}"));
    let verdict = meaning.unwrap_or_else(|| format!("simbolo personalizzato per «{lemma}»"));

    db.conn()
        .execute(
            "INSERT INTO concepts(id, label, meaning, category, broader, pos, source, commercial_ok)
             VALUES (?1, ?2, ?3, 'personalizzato', NULL, 'noun', 'custom', 1)
             ON CONFLICT(id) DO UPDATE SET label=excluded.label, meaning=excluded.meaning",
            rusqlite::params![concept, lemma, verdict],
        )
        .map_err(|e| e.to_string())?;

    let hash_hex = {
        use sha2::Digest;
        let digest = sha2::Sha256::digest(sanitized.svg.as_bytes());
        crate::asset_manager::hex(&digest)
    };
    let symbol_id = format!("custom.{slug}.{}", &hash_hex[..8]);

    db.conn()
        .execute(
            "INSERT INTO symbols(id, concept_id, pack_id, label, meaning, author, rights_holder, license,
                license_url, source_url, constraints, modifications, format, storage, data_path,
                status, hide, checksum, created_at)
             VALUES (?1, ?2, 'custom', ?3, ?4, 'Utente', 'Utente', 'CC BY-NC-SA 4.0',
                'https://creativecommons.org/licenses/by-nc-sa/4.0/', NULL, NULL, 'Importato dall''utente',
                'svg', 'db', NULL, 'approved', 0, ?5, ?6)
             ON CONFLICT(id) DO UPDATE SET label=excluded.label, meaning=excluded.meaning",
            rusqlite::params![symbol_id, concept, name, verdict, hash_hex, now],
        )
        .map_err(|e| e.to_string())?;

    db.conn()
        .execute(
            "DELETE FROM symbol_assets WHERE symbol_id = ?1",
            rusqlite::params![symbol_id],
        )
        .map_err(|e| e.to_string())?;
    db.conn()
        .execute(
            "INSERT INTO symbol_assets(symbol_id, kind, storage, data, path, width, height, bytes)
             VALUES (?1, 'original', 'db', ?2, NULL, 100, 100, ?3)",
            rusqlite::params![symbol_id, sanitized.svg, sanitized.bytes as i64],
        )
        .map_err(|e| e.to_string())?;

    db.conn()
        .execute(
            "INSERT INTO symbol_licenses(symbol_id, license, license_url, attribution, commercial_ok,
                share_alike, no_derivatives, source_url)
             VALUES (?1, 'CC BY-NC-SA 4.0', 'https://creativecommons.org/licenses/by-nc-sa/4.0/',
                'Simbolo personalizzato dell''utente', 1, 0, 0, NULL)
             ON CONFLICT(symbol_id) DO NOTHING",
            rusqlite::params![symbol_id],
        )
        .map_err(|e| e.to_string())?;

    db.conn()
        .execute(
            "DELETE FROM symbol_fts WHERE symbol_id = ?1",
            rusqlite::params![symbol_id],
        )
        .map_err(|e| e.to_string())?;
    db.conn()
        .execute(
            "INSERT INTO symbol_fts(symbol_id, text) VALUES (?1, ?2)",
            rusqlite::params![symbol_id, format!("{name} {lemma} {verdict} {concept}")],
        )
        .map_err(|e| e.to_string())?;

    db.insert_import_log(Some("custom"), "import_svg", &symbol_id, "");
    state.clear_caches();
    Ok(symbol_id)
}

#[tauri::command]
pub fn hide_symbol(
    state: State<'_, Engine>,
    symbol_id: String,
    hide: bool,
) -> Result<(), String> {
    state
        .db()?
        .hide_symbol(&symbol_id, hide)
        .map_err(|e| e.to_string())?;
    state.clear_caches();
    Ok(())
}

/// Importa testo da TXT, DOCX o HTML. Restituisce testo semplice con paragrafi.
#[tauri::command]
pub fn import_document_text(path: String) -> Result<String, String> {
    let p = Path::new(&path);
    let ext = p
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    match ext.as_str() {
        "txt" | "md" => std::fs::read_to_string(p).map_err(|e| format!("lettura: {e}")),
        "docx" => {
            let file = std::fs::File::open(p).map_err(|e| format!("apertura DOCX: {e}"))?;
            let mut archive =
                zip::ZipArchive::new(file).map_err(|e| format!("DOCX non valido: {e}"))?;
            let mut xml = String::new();
            {
                use std::io::Read;
                archive
                    .by_name("word/document.xml")
                    .map_err(|_| "DOCX senza word/document.xml".to_string())?
                    .read_to_string(&mut xml)
                    .map_err(|e| e.to_string())?;
            }
            Ok(docx_xml_to_text(&xml))
        }
        "html" | "htm" => {
            let raw = std::fs::read_to_string(p).map_err(|e| format!("lettura: {e}"))?;
            Ok(html_to_text(&raw))
        }
        _ => Err(format!(
            "formato non supportato: .{ext} (usa TXT, DOCX o HTML)"
        )),
    }
}

#[tauri::command]
pub fn clear_history(state: State<'_, Engine>) -> Result<(), String> {
    state.db()?.clear_history().map_err(|e| e.to_string())?;
    state.clear_caches();
    Ok(())
}

#[tauri::command]
pub fn get_setting(state: State<'_, Engine>, key: String) -> Result<Option<String>, String> {
    Ok(state.db()?.setting(&key))
}

#[tauri::command]
pub fn set_setting(state: State<'_, Engine>, key: String, value: String) -> Result<(), String> {
    state
        .db()?
        .set_setting(&key, &value)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn attributions(state: State<'_, Engine>) -> Result<Vec<AttributionEntry>, String> {
    state.db()?.attributions().map_err(|e| e.to_string())
}

/// Vocabolario di base per l'editor Utilizzatore (livelli 1–3).
#[tauri::command]
pub fn common_symbols(
    state: State<'_, Engine>,
    limit: Option<u32>,
) -> Result<Vec<Alternative>, String> {
    state.common_symbols(limit.unwrap_or(48).clamp(8, 96) as usize)
}

/// Simboli usati dalle alternative di un testo libero (ricerca rapida).
#[tauri::command]
pub fn word_affinities(
    state: State<'_, Engine>,
    word: String,
    pack_id: Option<String>,
    commercial_only: Option<bool>,
) -> Result<WordAffinity, String> {
    state.word_affinities(&word, pack_id, commercial_only.unwrap_or(false))
}

#[tauri::command]
pub fn suggest_symbols(
    state: State<'_, Engine>,
    text: String,
    commercial_only: Option<bool>,
) -> Result<Vec<Alternative>, String> {
    let opts = SymbolizeOptions {
        paragraph_id: "suggest".to_string(),
        intensity: Intensity::Minima,
        mode: CaaMode::Sopra,
        commercial_only: commercial_only.unwrap_or(false),
        ..Default::default()
    };
    let result = state.symbolize(&text, &opts)?;
    Ok(symbol_search::suggest_alternatives(&result, 16))
}

fn slugify(value: &str) -> String {
    let mut out = String::new();
    for c in value.to_lowercase().chars() {
        if c.is_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    out.trim_matches('-').to_string()
}

fn docx_xml_to_text(xml: &str) -> String {
    let mut out = String::with_capacity(xml.len() / 4);
    let mut inside_text = false;
    let mut chars = xml.char_indices().peekable();
    while let Some((idx, _)) = chars.next() {
        let rest = &xml[idx..];
        if rest.starts_with("<w:t") {
            inside_text = true;
            // salta fino a '>'
            while let Some((_, c)) = chars.next() {
                if c == '>' {
                    break;
                }
            }
            continue;
        }
        if rest.starts_with("</w:t>") {
            inside_text = false;
            continue;
        }
        if rest.starts_with("</w:p>") {
            out.push('\n');
            continue;
        }
        if rest.starts_with("<w:tab") {
            out.push('\t');
            continue;
        }
        if inside_text {
            out.push(rest.chars().next().unwrap_or(' '));
        }
    }
    out.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .lines()
        .map(|l| l.trim_end())
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

fn html_to_text(html: &str) -> String {
    let mut out = String::with_capacity(html.len() / 2);
    let mut in_tag = false;
    for c in html.chars() {
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
    out.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_is_safe() {
        assert_eq!(slugify("Pesca (frutto)"), "pesca-frutto");
        assert_eq!(slugify("Città"), "città");
    }

    #[test]
    fn docx_text_extraction_keeps_paragraphs() {
        let xml = r#"<w:document><w:body><w:p><w:r><w:t>Prima frase.</w:t></w:r></w:p><w:p><w:r><w:t>Seconda.</w:t></w:r></w:p></w:body></w:document>"#;
        let text = docx_xml_to_text(xml);
        assert!(text.contains("Prima frase."));
        assert!(text.contains("Seconda."));
        assert_eq!(text.lines().count(), 2);
    }

    #[test]
    fn html_text_extraction_strips_tags() {
        let text = html_to_text("<p>Ciao <b>mamma</b></p><p>Come stai?</p>");
        assert!(!text.contains('<'));
        assert!(text.contains("Ciao"));
        assert!(text.contains("Come stai?"));
    }
}
