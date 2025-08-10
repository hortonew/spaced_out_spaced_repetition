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
apk:
    # or --apk for production
    cargo tauri android build --debug

# Deploy debug build to android phone over USB
apk-install:
    adb install -r src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk


# Build and Deploy debug build to android phone over USB
debug:
    cargo tauri android build --debug
    adb install -r src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk

# Run tests
test:
    cargo test -p app

# Coverage
cov:
    cargo llvm-cov --html --open -p app

# Icon generation for all platforms
icon:
    cargo tauri icon ./Spaced_Out_Icon_circle.png
