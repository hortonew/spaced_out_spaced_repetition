mod commands;
use commands::AppState;
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
            // Legacy demo commands
            commands::say_hello,
            commands::my_custom_command
        ])
        .setup(|app| {
            // Initialize app state with app handle
            let app_state =
                AppState::new(app.handle().clone()).expect("Failed to initialize app state");
            app.manage(app_state);

            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
