use tauri::Manager;
mod native_bridge;
mod approval;
mod fake_run;
mod sidecar;
mod lifecycle;
mod measurement;
mod agent_runtime;

#[tauri::command]
fn agent_bootstrap(window: tauri::WebviewWindow) -> Result<agent_runtime::View, String> {
    agent_runtime::bootstrap(&window)
}
#[tauri::command]
fn agent_start(window: tauri::WebviewWindow, request: tauri::ipc::Request<'_>) -> Result<agent_runtime::View, String> {
    agent_runtime::command(&window, request.body(), "start")
}
#[tauri::command]
fn agent_cancel(window: tauri::WebviewWindow, request: tauri::ipc::Request<'_>) -> Result<agent_runtime::View, String> {
    agent_runtime::command(&window, request.body(), "cancel")
}
#[tauri::command]
fn agent_snapshot(window: tauri::WebviewWindow, request: tauri::ipc::Request<'_>) -> Result<agent_runtime::View, String> {
    agent_runtime::command(&window, request.body(), "snapshot")
}
#[tauri::command]
fn agent_runtime_input(window: tauri::WebviewWindow, request: tauri::ipc::Request<'_>) -> Result<agent_runtime::View, String> {
    agent_runtime::command(&window, request.body(), "runtime")
}

#[tauri::command]
async fn start_node_fake_run(app: tauri::AppHandle, request: tauri::ipc::Request<'_>) -> Result<(fake_run::Snapshot, u32, i32), String> {
    use tauri::Manager;
    let resources = app.path().resource_dir().map_err(|_| "Bundle resource directory unavailable")?;
    let created = lifecycle::with_request(request.body(), |body| fake_run::begin(fake_run::Start::parse(body)?, "node-b"))?;
    tauri::async_runtime::spawn_blocking(move || sidecar::run_created(resources, created))
        .await.map_err(|_| "Sidecar worker failed")?
}

#[tauri::command]
fn create_fake_run(request: tauri::ipc::Request<'_>) -> Result<fake_run::Snapshot, &'static str> {
    lifecycle::with_request(request.body(), |body| fake_run::begin(fake_run::Start::parse(body)?, "webview-a"))
}

#[tauri::command]
fn cancel_fake_run(request: tauri::ipc::Request<'_>) -> Result<fake_run::Snapshot, &'static str> {
    lifecycle::with_request(request.body(), fake_run::cancel_body)
}

#[tauri::command]
fn submit_fake_event(request: tauri::ipc::Request<'_>) -> Result<fake_run::Snapshot, &'static str> {
    lifecycle::with_request(request.body(), |body| fake_run::submit(fake_run::Frame::parse(body)?))
}

#[tauri::command]
fn get_runtime_session() -> Result<(String, u32, bool), &'static str> {
    let session = lifecycle::session()?;
    measurement::record("session_ready", None, None, None, None);
    Ok(session)
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

    measurement::init();
    eprintln!("AETHER_INSTANCE instance={} pid={} time_ms={} executable=Contents/MacOS/aether-desktop", lifecycle::instance(), std::process::id(), lifecycle::now());
    tauri::Builder::default()
        .setup(|app| {
            agent_runtime::install(&lifecycle::instance());
            lifecycle::install_menu(app.handle())?;
            Ok(())
        })
        .on_page_load(|webview, payload| {
            if webview.label() == "main" && matches!(payload.event(), tauri::webview::PageLoadEvent::Started) {
                lifecycle::context_lost(webview.app_handle(), "page-started");
            }
            if webview.label() == "main" && matches!(payload.event(), tauri::webview::PageLoadEvent::Finished) {
                lifecycle::page_replaced();
            }
        })
        .on_window_event(|window, event| {
            use tauri::Manager;
            if window.label() == "main" && matches!(event, tauri::WindowEvent::CloseRequested { .. }) {
                lifecycle::window_closing(window.app_handle());
            }
            if window.label() == "main" && matches!(event, tauri::WindowEvent::Destroyed) {
                lifecycle::context_lost(window.app_handle(), "window-close");
                lifecycle::page_replaced();
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_macos_major_version,
            get_accessibility_trust_status,
            request_counter_increment,
            get_experiment_counter,
            create_fake_run,
            cancel_fake_run,
            submit_fake_event,
            get_fake_run,
            get_runtime_session,
            start_node_fake_run,
            agent_bootstrap,
            agent_start,
            agent_cancel,
            agent_snapshot,
            agent_runtime_input,
        ])
        .build(tauri::generate_context!())
        .expect("AETHER application failed to start")
        .run(|app, event| {
            if let tauri::RunEvent::ExitRequested { api, .. } = event { lifecycle::quit(app, &api); }
        });
}
