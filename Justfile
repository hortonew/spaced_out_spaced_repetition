# List available commands
default:
    @just --list

# Run Tauri development server
dev:
    cargo tauri dev

# Run Tauri Android development
android:
    cargo tauri android dev