//! CAA Writer — libreria applicativa Tauri.
//!
//! Ordine di inizializzazione:
//! 1. percorsi (dati utente separati dalle risorse in sola lettura);
//! 2. database SQLite + migrazioni;
//! 3. importazione del pacchetto simboli `core` se il database è vuoto;
//! 4. registrazione dei comandi.

pub mod ai_engine;
pub mod asset_manager;
pub mod commands;
pub mod database;
pub mod diagnostics;
pub mod document_edit;
pub mod engine;
pub mod export;
pub mod layout;
pub mod layout_edit;
pub mod linguistics;
pub mod model;
pub mod paths;
pub mod profiles;
pub mod pictogram_import;
pub mod online_search;
pub mod process;
pub mod speech;
pub mod studio;
pub mod tts_normalize;
pub mod voice;
pub mod symbol_search;
pub mod util;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
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
            commands::runtime_diagnostics,
            commands::set_ai_config,
            commands::ai_models,
            commands::ai_install_model,
            commands::ai_install_status,
            commands::ai_cancel_model_install,
            commands::ai_remove_model,
            commands::ai_import_model,
            commands::ai_related_symbols,
            commands::ai_select_model,
            commands::ai_benchmark,
            commands::ai_run_tool,
            commands::library_stats,
            commands::list_packs,
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
            commands::list_pictogram_collections,
            commands::rename_pictogram_collection,
            commands::delete_pictogram_collection,
            commands::delete_pictogram_symbols,
            commands::move_pictogram_symbols,
            commands::list_online_providers,
            commands::search_online_pictograms,
            commands::online_thumbnail,
            commands::save_online_pictogram,
            commands::hide_symbol,
            commands::import_document_text,
            commands::clear_history,
            commands::get_setting,
            commands::set_setting,
            commands::attributions,
            commands::suggest_symbols,
            commands::word_affinities,
        ])
        .build(tauri::generate_context!())
        .expect("errore irreversibile nell'avvio di CAA Writer")
        .run(|app_handle, event| {
            // All'uscita si ferma `llama-server`: altrimenti il processo figlio
            // resta attivo e occupa RAM anche dopo la chiusura della finestra.
            if matches!(event, tauri::RunEvent::Exit) {
                if let Some(engine) = app_handle.try_state::<engine::Engine>() {
                    engine.shutdown();
                }
            }
        });
}
