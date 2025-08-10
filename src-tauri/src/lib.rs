mod card_service;
mod commands;
mod models;
mod spaced_repetition;
mod storage;

use card_service::CardService;
use storage::Storage;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            // Card management commands
            commands::create_card,
            commands::get_cards,
            commands::get_card,
            commands::update_card,
            commands::delete_card,
            // Review session commands
            commands::get_due_cards,
            commands::review_card,
            commands::get_review_stats,
            // Organization and search commands
            commands::search_cards,
            commands::get_tags,
            commands::get_tag_stats,
            commands::bulk_update_tag,
            commands::delete_multiple_cards,
            // Settings commands
            commands::get_settings,
            commands::update_settings,
        ])
        .setup(|app| {
            // Initialize storage and card service
            let storage = Storage::new(app.handle().clone()).expect("Failed to initialize storage");
            let card_service = CardService::new(storage).expect("Failed to initialize card service");
            app.manage(card_service);

            if cfg!(debug_assertions) {
                app.handle()
                    .plugin(tauri_plugin_log::Builder::default().level(log::LevelFilter::Info).build())?;
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
