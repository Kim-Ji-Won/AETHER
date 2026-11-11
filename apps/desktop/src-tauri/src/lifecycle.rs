//! Process/context identity is an IPC freshness boundary, not user authorization.
use std::sync::{Mutex, OnceLock, atomic::{AtomicBool, Ordering}};
use tauri::{Manager, ipc::InvokeBody};
use crate::{approval, fake_run};

struct Gate { instance: String, epoch: u32, closing: bool }
impl Gate {
    fn fresh() -> Self {
        let mut bytes = [0u8; 16];
        unsafe extern "C" { fn arc4random_buf(buffer: *mut std::ffi::c_void, size: usize); }
        // macOS libSystem CSPRNG; no user path, environment or persistent identifier.
        unsafe { arc4random_buf(bytes.as_mut_ptr().cast(), bytes.len()); }
        Self { instance: bytes.iter().map(|b| format!("{b:02x}")).collect(), epoch: 0, closing: false }
    }
    fn check(&self, body: &InvokeBody) -> Result<InvokeBody, &'static str> {
        let InvokeBody::Json(j) = body else { return Err("Expected lifecycle envelope"); };
        if j.to_string().len() > 768 || j.as_object().map(|o| o.len()) != Some(3)
            || j["instance"].as_str() != Some(self.instance.as_str())
            || j["epoch"].as_u64() != Some(self.epoch as u64) || self.closing {
            return Err("Stale or closing lifecycle session");
        }
        let payload = j.get("payload").ok_or("Missing payload")?;
        Ok(InvokeBody::Json(payload.clone()))
    }
}
static GATE: OnceLock<Mutex<Gate>> = OnceLock::new();
fn gate() -> &'static Mutex<Gate> { GATE.get_or_init(|| Mutex::new(Gate::fresh())) }
pub fn instance() -> String { gate().lock().expect("lifecycle gate").instance.clone() }
pub fn session() -> Result<(String, u32, bool), &'static str> {
    let g = gate().lock().map_err(|_| "Session unavailable")?;
    Ok((g.instance.clone(), g.epoch, !g.closing))
}
pub fn with_live<T>(action: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
    let g = gate().lock().map_err(|_| "Session unavailable")?;
    if g.closing { return Err("App closing".into()); }
    action()
}
pub fn with_request<T>(body: &InvokeBody, action: impl FnOnce(&InvokeBody) -> Result<T, &'static str>) -> Result<T, &'static str> {
    let g = gate().lock().map_err(|_| "Session unavailable")?;
    let payload = g.check(body)?;
    // Gate -> ledger is the only lock order. Never hold either lock across waits.
    action(&payload)
}
pub fn context_lost(app: &tauri::AppHandle, reason: &str) {
    if let Ok(mut g) = gate().lock() {
        match g.epoch.checked_add(1) { Some(n) => g.epoch = n, None => g.closing = true }
        fake_run::context_lost(g.epoch);
        eprintln!("AETHER_LIFECYCLE instance={} pid={} event={reason} epoch={} time_ms={}",
            g.instance, std::process::id(), g.epoch, now());
    }
    approval::invalidate(app);
}
pub fn now() -> u128 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis()
}
static EXIT_STARTED: AtomicBool = AtomicBool::new(false);
static EXIT_READY: AtomicBool = AtomicBool::new(false);
pub fn quit(app: &tauri::AppHandle, api: &tauri::ExitRequestApi) {
    if EXIT_READY.load(Ordering::SeqCst) { return; }
    api.prevent_exit();
    if EXIT_STARTED.swap(true, Ordering::SeqCst) { return; }
    if let Ok(mut g) = gate().lock() { g.closing = true; fake_run::shutdown(); }
    approval::invalidate(app);
    eprintln!("AETHER_QUIT instance={} event=stop-intake time_ms={}", instance(), now());
    let app = app.clone();
    std::thread::spawn(move || {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while !fake_run::child_idle() && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        if fake_run::child_idle() {
            eprintln!("AETHER_QUIT instance={} child_idle=true pending=false time_ms={}", instance(), now());
            EXIT_READY.store(true, Ordering::SeqCst); app.exit(0);
        } else {
            // Do not claim success or force an unconfirmed child cleanup.
            eprintln!("AETHER_QUIT cleanup=FAILED host_retained=true");
        }
    });
}
static APP: OnceLock<tauri::AppHandle> = OnceLock::new();
extern "C" fn native_terminate() {
    // AppKit's predefined Quit uses terminate:, bypassing ExitRequested.
    // willTerminate is synchronous on the main thread; bounded child wait holds no mutex.
    if let Some(app) = APP.get() {
        if let Ok(mut g) = gate().lock() { g.closing = true; fake_run::shutdown(); }
        approval::invalidate(app);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while !fake_run::child_idle() && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        eprintln!("AETHER_NATIVE_QUIT instance={} child_idle={} pending=false time_ms={}", instance(), fake_run::child_idle(), now());
    }
}
pub fn install_menu(app: &tauri::AppHandle) -> tauri::Result<()> {
    unsafe extern "C" { fn aether_install_lifecycle_observers(callback: extern "C" fn()); }
    // Tauri setup runs on the AppKit main thread.
    let _ = APP.set(app.clone());
    unsafe { aether_install_lifecycle_observers(native_terminate); }
    use tauri::menu::{Menu, MenuItem, MenuItemKind, Submenu};
    let menu = Menu::default(app)?;
    // Locked Tauri macOS default app submenu ends in predefined Quit (terminate:).
    // A custom native item remains usable while our sheet is pending and enters cleanup.
    if let Some(MenuItemKind::Submenu(application)) = menu.items()?.first() {
        let last = application.items()?.len().checked_sub(1).expect("default app menu");
        application.remove_at(last)?;
        application.append(&MenuItem::with_id(app, "e4e-quit", "Quit AETHER", true, Some("CmdOrCtrl+Q"))?)?;
    }
    let reload = MenuItem::with_id(app, "e4e-reload", "WebView 다시 불러오기", true, Some("CmdOrCtrl+R"))?;
    let close = MenuItem::with_id(app, "e4e-close", "시험 창 닫기", true, None::<&str>)?;
    menu.append(&Submenu::with_items(app, "Lifecycle", true, &[&reload, &close])?)?;
    app.set_menu(menu)?;
    app.on_menu_event(|app, event| {
        if event.id().as_ref() == "e4e-quit" { app.exit(0); return; }
        if let Some(window) = app.get_webview_window("main") {
            match event.id().as_ref() {
                "e4e-reload" => { let _ = window.reload(); },
                "e4e-close" => { let _ = window.close(); },
                _ => {}
            }
        }
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn body(g: &Gate) -> InvokeBody {
        use tauri::ipc::InvokeResponseBody;
        InvokeBody::Json(InvokeResponseBody::Json(format!(r#"{{"instance":"{}","epoch":{},"payload":{{"token":1,"mode":"normal"}}}}"#,g.instance,g.epoch)).deserialize().unwrap())
    }
    #[test]
    fn instance_epoch_and_quit_reject_old_messages_without_executing_action() {
        let mut g = Gate::fresh(); let old = body(&g);
        assert!(g.check(&old).is_ok());
        g.epoch += 1; assert!(g.check(&old).is_err());
        let current = body(&g); assert!(g.check(&current).is_ok());
        assert!(Gate::fresh().check(&current).is_err());
        g.closing = true; assert!(g.check(&current).is_err());
    }
}
