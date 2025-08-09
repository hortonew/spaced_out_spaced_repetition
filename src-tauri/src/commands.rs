#[tauri::command]
pub fn say_hello(name: String) -> String {
    format!("Hello, {name} 👋 (from Rust)")
}

#[tauri::command]
pub fn my_custom_command() {
    println!("I was invoked from JavaScript!");
}
