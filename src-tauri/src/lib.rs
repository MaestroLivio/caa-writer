//! CAA Writer — libreria applicativa Tauri.
//!
//! Ordine di inizializzazione:
//! 1. percorsi (dati utente separati dalle risorse in sola lettura);
//! 2. database SQLite + migrazioni;
//! 3. importazione del pacchetto simboli `core` se il database è vuoto;
//! 4. registrazione dei comandi.

pub mod ai_engine;
pub mod ai_provider;
pub mod asset_manager;
pub mod commands;
pub mod database;
pub mod downloads;
pub mod diagnostics;
pub mod document_edit;
pub mod engine;
pub mod image_engine;
pub mod export;
pub mod layout;
pub mod layout_edit;
pub mod library_quality;
pub mod linguistics;
pub mod model;
pub mod paths;
pub mod packages;
pub mod profiles;
pub mod pictogram_import;
pub mod online_search;
pub mod process;
pub mod speech;
pub mod story;
pub mod studio;
pub mod template_catalog;
pub mod tts_normalize;
pub mod trace;
pub mod universal;
pub mod universal_build;
pub mod voice;
pub mod symbol_search;
pub mod util;

use tauri::Manager;
#[cfg(target_os = "macos")]
use tauri::Emitter;

#[derive(Default)]
pub struct PendingOpen(pub std::sync::Mutex<Option<String>>);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let pending = PendingOpen::default();
            if let Ok(mut slot) = pending.0.lock() {
                for argument in std::env::args().skip(1) {
                    let lower = argument.to_lowercase();
                    if lower.ends_with(".caadoc") || lower.ends_with(".caaplay") {
                        *slot = Some(argument);
                        break;
                    }
                }
            }
            app.manage(pending);
            let data_dir = app.path().app_data_dir().unwrap_or_else(|_| {
                std::env::temp_dir().join("caa-writer")
            });
            let resource_dir = app.path().resource_dir().ok();
            let exe_dir = std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|p| p.to_path_buf()));
            let resources =
                paths::AppPaths::resolve_resources(resource_dir.as_deref(), exe_dir.as_deref());
            let app_paths = paths::AppPaths::new(data_dir, resources);

            match engine::Engine::new(app_paths) {
                Ok(engine) => {
                    for warning in &engine.startup_warnings {
                        eprintln!("[caa-writer] {warning}");
                    }
                    app.manage(engine);
                    Ok(())
                }
                Err(err) => {
                    eprintln!("[caa-writer] avvio non riuscito: {err}");
                    Err(Box::new(std::io::Error::other(err)))
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::print_window,
            commands::symbolize_paragraph,
            commands::refine_paragraph,
            commands::cancel_refine,
            commands::search_symbols,
            commands::concept_alternatives,
            commands::set_preference,
            commands::delete_preference,
            commands::list_preferences,
            commands::set_document_override,
            commands::clear_document_override,
            commands::set_unit_state,
            commands::save_document,
            commands::remove_recovery_draft,
            commands::load_document,
            commands::export_html,
            commands::export_docx,
            commands::export_attributions,
            commands::list_voices,
            commands::speak,
            commands::stop_speaking,
            commands::speech_active,
            commands::normalize_speech,
            commands::voice_status,
            commands::ai_status,
            commands::ai_chat,
            commands::image_status,
            commands::online_autofill,
            commands::hydrate_symbols,
            commands::image_set_config,
            commands::image_set_key,
            commands::image_clear_key,
            commands::image_generate,
            commands::image_install_runtime,
            commands::image_download_model,
            commands::runtime_diagnostics,
            commands::document_validate,
            commands::document_render,
            commands::document_apply,
            commands::generate_trace_sheet,
            commands::universal_from_plan,
            commands::document_html,
            commands::print_universal,
            commands::ai_provider_status,
            commands::ai_provider_save,
            commands::ai_provider_test,
            commands::ai_provider_clear_key,
            commands::take_pending_open,
            commands::universal_import,
            commands::document_export,
            commands::universal_library,
            commands::library_catalog,
            commands::library_search,
            commands::library_by_ids,
            commands::library_preview,
            commands::library_validate,
            commands::universal_open,
            commands::set_ai_config,
            commands::ai_models,
            commands::ai_install_model,
            commands::ai_install_status,
            commands::ai_cancel_model_install,
            commands::ai_remove_model,
            commands::ai_import_model,
            commands::ai_related_symbols,
            commands::ai_affinity_symbols,
            commands::resolve_missing_symbols,
            commands::ai_select_model,
            commands::ai_benchmark,
            commands::ai_run_tool,
            commands::translate_search_term,
            commands::story_create,
            commands::library_stats,
            commands::list_packs,
            commands::package_catalog,
            commands::download_start,
            commands::download_status,
            commands::download_reset,
            commands::package_set_library,
            commands::package_download,
            commands::package_remove,
            commands::check_updates,
            commands::open_repository_link,
            commands::list_templates,
            commands::plan_resource,
            commands::build_layout,
            commands::edit_layout,
            commands::interpret_document_prompt,
            commands::run_cli,
            commands::profiles_list,
            commands::profile_save,
            commands::profile_delete,
            commands::profile_set_active,
            commands::profile_active,
            commands::profile_boards,
            commands::profile_assign_board,
            commands::profile_remove_board,
            commands::boards_list,
            commands::board_build,
            commands::creator_pin_status,
            commands::creator_set_pin,
            commands::creator_verify_pin,
            commands::import_svg_symbol,
            commands::scan_pictograms,
            commands::import_pictograms,
            commands::import_image_symbol,
            commands::list_pictogram_collections,
            commands::rename_pictogram_collection,
            commands::delete_pictogram_collection,
            commands::delete_pictogram_symbols,
            commands::move_pictogram_symbols,
            commands::list_online_providers,
            commands::save_online_provider,
            commands::delete_online_provider,
            commands::ensure_online_symbols,
            commands::search_online_pictograms,
            commands::online_thumbnail,
            commands::save_online_pictogram,
            commands::english_alternatives,
            commands::hide_symbol,
            commands::import_document_text,
            commands::clear_history,
            commands::get_setting,
            commands::set_setting,
            commands::attributions,
            commands::suggest_symbols,
            commands::word_affinities,
            commands::common_symbols,
        ])
        .build(tauri::generate_context!())
        .expect("errore irreversibile nell'avvio di CAA Writer")
        .run(|app_handle, event| match event {
            // All'uscita si ferma `llama-server`: altrimenti il processo figlio
            // resta attivo e occupa RAM anche dopo la chiusura della finestra.
            tauri::RunEvent::Exit => {
                if let Some(engine) = app_handle.try_state::<engine::Engine>() {
                    engine.shutdown();
                }
            }
            // Doppio clic su `.caadoc`/`.caaplay` (macOS): la finestra può
            // essere già aperta, quindi il percorso viene inviato al frontend.
            #[cfg(target_os = "macos")]
            tauri::RunEvent::Opened { urls } => {
                for url in urls {
                    if let Ok(path) = url.to_file_path() {
                        let value = path.to_string_lossy().to_string();
                        // Se il frontend non è ancora pronto, il percorso resta
                        // in attesa e viene letto al primo mount.
                        if let Some(pending) = app_handle.try_state::<PendingOpen>() {
                            if let Ok(mut slot) = pending.0.lock() {
                                *slot = Some(value.clone());
                            }
                        }
                        let _ = app_handle.emit("open-document", value);
                    }
                }
            }
            _ => {}
        });
}
