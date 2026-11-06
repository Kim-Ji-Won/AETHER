mod native_bridge;
mod approval;

#[tauri::command]
fn get_macos_major_version() -> i32 {
    native_bridge::macos_major_version()
}

#[tauri::command]
fn get_accessibility_trust_status() -> Result<bool, &'static str> {
    native_bridge::accessibility_trust_status()
}

#[tauri::command]
async fn request_counter_increment(window: tauri::WebviewWindow) -> Result<u32, String> {
    approval::request(window).await
}

#[tauri::command]
fn get_experiment_counter() -> Result<u32, String> {
    approval::counter()
}

fn main() {
    if std::env::args_os().skip(1).any(|arg| arg == "--native-bridge-check") {
        println!("AETHER_NATIVE_MACOS_MAJOR={}", native_bridge::macos_major_version());
        return;
    }

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_macos_major_version,
            get_accessibility_trust_status,
            request_counter_increment,
            get_experiment_counter,
        ])
        .run(tauri::generate_context!())
        .expect("AETHER application failed to start");
}
