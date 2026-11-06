mod native_bridge;

#[tauri::command]
fn get_macos_major_version() -> i32 {
    native_bridge::macos_major_version()
}

fn main() {
    if std::env::args_os().skip(1).any(|arg| arg == "--native-bridge-check") {
        println!("AETHER_NATIVE_MACOS_MAJOR={}", native_bridge::macos_major_version());
        return;
    }

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![get_macos_major_version])
        .run(tauri::generate_context!())
        .expect("AETHER application failed to start");
}
