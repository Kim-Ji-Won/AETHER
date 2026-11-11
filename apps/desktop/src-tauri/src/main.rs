mod native_bridge;
mod approval;
mod fake_run;

#[tauri::command]
fn create_fake_run() -> Result<fake_run::Snapshot, &'static str> {
    fake_run::create()
}

#[tauri::command]
fn submit_fake_event(request: tauri::ipc::Request<'_>) -> Result<fake_run::Snapshot, &'static str> {
    fake_run::submit(fake_run::Frame::parse(request.body())?)
}

#[tauri::command]
fn get_fake_run() -> Result<Option<fake_run::Snapshot>, &'static str> {
    fake_run::snapshot()
}

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
            create_fake_run,
            submit_fake_event,
            get_fake_run,
        ])
        .run(tauri::generate_context!())
        .expect("AETHER application failed to start");
}
