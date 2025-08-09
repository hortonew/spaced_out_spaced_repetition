# List available commands
default:
    @just --list

# Run Tauri development server
dev:
    cargo tauri dev

# Run Tauri Android development
android:
    cargo tauri android dev

# Build and run Tauri in release mode (desktop)
run:
    cargo tauri build --no-bundle && ./src-tauri/target/release/app

# Build and run Tauri Android in release mode
android-run:
    cargo tauri android build --apk