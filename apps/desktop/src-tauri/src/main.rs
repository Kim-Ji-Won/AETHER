mod native_bridge;
mod approval;
mod fake_run;
mod sidecar;

#[tauri::command]
async fn start_node_fake_run(app: tauri::AppHandle, request: tauri::ipc::Request<'_>) -> Result<(fake_run::Snapshot, u32, i32), String> {
    let start = fake_run::Start::parse(request.body())?;
    use tauri::Manager;
    let resources = app.path().resource_dir().map_err(|_| "Bundle resource directory unavailable")?;
    tauri::async_runtime::spawn_blocking(move || sidecar::run(resources, start))
        .await.map_err(|_| "Sidecar worker failed")?
}

#[tauri::command]
fn create_fake_run(request: tauri::ipc::Request<'_>) -> Result<fake_run::Snapshot, &'static str> {
    fake_run::begin(fake_run::Start::parse(request.body())?, "webview-a")
}

#[tauri::command]
fn cancel_fake_run(request: tauri::ipc::Request<'_>) -> Result<fake_run::Snapshot, &'static str> {
    fake_run::cancel_body(request.body())
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
            cancel_fake_run,
            submit_fake_event,
            get_fake_run,
            start_node_fake_run,
        ])
        .run(tauri::generate_context!())
        .expect("AETHER application failed to start");
}
