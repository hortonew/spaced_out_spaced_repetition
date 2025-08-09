mod commands;
use commands::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize app state
    let app_state = AppState::new().expect("Failed to initialize app state");

    tauri::Builder::default()
        .manage(app_state)
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
