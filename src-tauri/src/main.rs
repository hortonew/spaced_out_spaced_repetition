// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![cfg_attr(mobile, tauri::mobile_entry_point)]
use tauri::Manager;

#[tauri::command]
fn say_hello(name: String) -> String {
    format!("Hello, {name} 👋 (from Rust)")
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![say_hello])
        .run(tauri::generate_context!())
        .expect("error running app");
}
