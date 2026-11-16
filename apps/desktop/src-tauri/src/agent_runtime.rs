//! Foreground WKWebView bridge. Lock order: lifecycle Gate -> Runtime. Timer takes only Runtime.
//! No network, credentials, logging of input/output, or renderer-owned terminal decisions.
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use aether_desktop::agent_text::{self as contract, synthetic::SyntheticAdapter, *};
use tauri::ipc::InvokeBody;

type SessionWire = (String, u32, u32, u32, u64, u32, bool);
type RunWire = (u32, u32, u32, u32, String, String, u32, u8, String, String, String);
pub type View = (SessionWire, Option<RunWire>, Option<String>);

fn policy() -> Policy {
    Policy { max_input_bytes: 128, max_delta_bytes: 64, max_output_bytes: 2048,
        max_deltas: 128, max_queue_frames: 2, max_queue_bytes: 128,
        run_ms: 12_000, idle_ms: 3_000, cleanup_ms: 2_000,
        rate_window_ms: 1_000, max_starts_per_window: 8, synthetic_budget: 1000 }
}

struct Runtime {
    host: Host,
    adapter: Option<SyntheticAdapter>,
    lease: Option<String>,
    lost_run: Option<u32>,
    epoch: u32,
}
impl Runtime {
    fn new(instance: &str) -> Self {
        Self { host: Host::new(instance, policy()).expect("fixed synthetic policy"),
            adapter: None, lease: None, lost_run: None, epoch: 0 }
    }
    fn attach(&mut self, instance: &str, epoch: u32, live: bool) -> Result<(), ErrorCode> {
        self.host.attach(instance, epoch, live)?;
        self.epoch = epoch;
        Ok(())
    }
    fn tick(&mut self, now: u64) -> Result<(), ErrorCode> {
        self.host.advance_to(now)?;
        // Actual owned Rust queue is cleared before Adapter release is confirmed.
        if self.host.current().is_some_and(|r| r.state == State::CancelRequested || r.state.terminal()) {
            if let Some(adapter) = &mut self.adapter { adapter.abort(&mut self.host)?; }
        }
        Ok(())
    }
    fn bound(&self, lease: &str) -> Result<Identity, ErrorCode> {
        if self.lease.as_deref() != Some(lease) { return Err(ErrorCode::InvalidIdentity); }
        let id = self.host.current().ok_or(ErrorCode::InvalidIdentity)?.identity.clone();
        if id.session_generation != self.host.inspect().session_generation { return Err(ErrorCode::InvalidIdentity); }
        Ok(id)
    }
    fn start(&mut self, body: &InvokeBody, lease: String) -> Result<(), ErrorCode> {
        self.host.start(Start::parse(body, self.host.policy().max_input_bytes)?)?;
        self.adapter = None;
        self.lease = Some(lease);
        Ok(())
    }
    fn runtime_input(&mut self, body: &InvokeBody) -> Result<(), ErrorCode> {
        let InvokeBody::Json(j) = body else { return Err(ErrorCode::InvalidInput); };
        if j.as_object().map(|o| o.len()) != Some(3) { return Err(ErrorCode::InvalidInput); }
        let lease = j["lease"].as_str().ok_or(ErrorCode::InvalidIdentity)?;
        let id = self.bound(lease)?; // before parsing a frame or failing any current run
        let action = j["action"].as_str().ok_or(ErrorCode::InvalidInput)?;
        let data = j.get("data").ok_or(ErrorCode::InvalidInput)?;
        match action {
            "dispatch" if data.as_object().is_some_and(|o| o.is_empty()) => {
                self.adapter = Some(SyntheticAdapter::dispatch(&mut self.host, &id)?);
            }
            "frame" => {
                if self.host.inspect().closing { return Err(ErrorCode::StaleSession); }
                let frame = match Frame::parse(&InvokeBody::Json(data.clone()), self.host.policy().max_delta_bytes) {
                    Ok(frame) => frame,
                    Err(code) => {
                        // Only an already authenticated lease may fail its own malformed stream.
                        if matches!(code, ErrorCode::ProtocolError | ErrorCode::OutputLimit) {
                            self.host.stream_error(&id, code)?;
                        }
                        return Err(code);
                    }
                };
                if frame.identity != id { return Err(ErrorCode::InvalidIdentity); }
                if self.adapter.is_none() { return Err(ErrorCode::InvalidIdentity); }
                // Sequential IPC is a one-frame barrier. Preserve the source sequence;
                // never manufacture a new sequence for an untrusted runtime frame.
                let completed = matches!(frame.payload, AdapterPayload::Completed);
                self.host.ingest(frame)?;
                if completed { self.adapter.as_mut().unwrap().close(&mut self.host)?; }
            }
            "release" if data.as_object().is_some_and(|o| o.is_empty()) => {
                let run = self.host.current().ok_or(ErrorCode::InvalidIdentity)?;
                // Runtime ACK cannot release a still-producing adapter or a queued run.
                if self.adapter.is_none() { return Err(ErrorCode::InvalidIdentity); }
                if run.state == State::Running {
                    self.adapter.as_mut().unwrap().close(&mut self.host)?;
                } else { self.adapter.as_mut().unwrap().abort(&mut self.host)?; }
                self.host.release(&id, CleanupPart::Runtime, true)?;
            }
            _ => return Err(ErrorCode::InvalidInput),
        }
        Ok(())
    }
    fn lost(&mut self) -> Result<(), ErrorCode> {
        self.lost_run = self.host.current().map(|r| r.identity.run_id);
        self.lease = None; // revoke sender before generation changes
        self.adapter = None; // Drop releases the entire owned, memory-only adapter queue
        self.host.runtime_lost()
    }
    fn page_replaced(&mut self) -> Result<(), ErrorCode> {
        // Called only by native PageLoad Finished / Destroyed, not a UI assertion.
        if let Some(id) = self.lost_run.take() { self.host.confirm_owned_cleanup(id)?; }
        Ok(())
    }
    fn shutdown(&mut self) -> Result<(), ErrorCode> {
        self.host.shutdown()?;
        self.tick(self.host.now_ms())
    }
    fn idle(&self) -> bool {
        self.host.current().is_none_or(|r| r.state.terminal()
            && matches!(r.cleanup, Cleanup::Confirmed | Cleanup::NotRequired))
    }
    fn view(&self) -> View {
        let s = self.host.inspect();
        let session = (s.session.host_instance_id, self.epoch, s.session.ui_context_epoch,
            s.session_generation, s.snapshot_revision, s.request_high_water, s.closing);
        let run = s.run.map(|r| (r.identity.run_id, r.identity.request_id, r.identity.attempt_id,
            r.identity.session_generation, r.identity.correlation_id, format!("{:?}", r.state),
            r.last_seq, r.terminal_count, r.text, r.error.map(|e| format!("{:?}", e.code)).unwrap_or_default(),
            format!("{:?}", r.cleanup)));
        (session, run, self.lease.clone())
    }
}

static RUNTIME: OnceLock<Mutex<Runtime>> = OnceLock::new();
static CLOCK: OnceLock<Instant> = OnceLock::new();
fn runtime() -> Result<&'static Mutex<Runtime>, &'static str> { RUNTIME.get().ok_or("Agent unavailable") }
fn elapsed() -> u64 { CLOCK.get().expect("installed monotonic clock").elapsed().as_millis().min(u64::MAX as u128) as u64 }
fn safe_error(code: ErrorCode) -> String { format!("{code:?}") }

pub fn install(instance: &str) {
    CLOCK.set(Instant::now()).expect("one agent clock");
    assert!(RUNTIME.set(Mutex::new(Runtime::new(instance))).is_ok());
    std::thread::spawn(|| loop {
        std::thread::sleep(Duration::from_millis(10));
        let Ok(mut r) = runtime().unwrap().lock() else { return; };
        if r.tick(elapsed()).is_err() { let _ = r.shutdown(); }
        // Does not depend on WebKit timers, render frames, visibility, or window focus.
    });
}

fn trusted_source(window: &tauri::WebviewWindow) -> Result<(), &'static str> {
    let url = window.url().map_err(|_| "Sender unavailable")?;
    if sender_allowed(window.label(), url.scheme(), url.host_str(), url.port()) { Ok(()) }
    else { Err("Untrusted agent sender") }
}
fn sender_allowed(label: &str, scheme: &str, host: Option<&str>, port: Option<u16>) -> bool {
    label == "main" && ((scheme == "tauri" && host == Some("localhost") && port.is_none())
        || (cfg!(debug_assertions) && scheme == "http" && host == Some("127.0.0.1") && port == Some(1420)))
}

pub fn bootstrap(window: &tauri::WebviewWindow) -> Result<View, String> {
    crate::lifecycle::with_agent_bootstrap(|instance, epoch, live| {
        trusted_source(window)?;
        let mut r = runtime()?.lock().map_err(|_| "Agent unavailable")?;
        r.tick(elapsed()).map_err(safe_error)?;
        r.attach(instance, epoch, live).map_err(safe_error)?;
        Ok(r.view())
    })
}
pub fn command(window: &tauri::WebviewWindow, body: &InvokeBody, action: &str) -> Result<View, String> {
    // Only snapshot/release remain available during shutdown. Start/cancel/dispatch/frame
    // still fail under the original closing gate. Cleanup is not new intake authority.
    let cleanup = action == "snapshot" || (action == "runtime" && matches!(body,
        InvokeBody::Json(j) if j["payload"]["action"].as_str() == Some("release")));
    crate::lifecycle::with_agent_request(body, cleanup, |instance, epoch, live, payload| {
        trusted_source(window)?;
        let mut r = runtime()?.lock().map_err(|_| "Agent unavailable")?;
        r.tick(elapsed()).map_err(safe_error)?;
        // Attach only through bootstrap. A command cannot create a trusted epoch.
        let s = r.host.inspect();
        if s.session.host_instance_id != instance || r.epoch != epoch || s.session.ui_context_epoch == 0 {
            return Err("StaleSession".into());
        }
        if !live && !cleanup { return Err("StaleSession".into()); }
        let result = match action {
            "start" => r.start(payload, crate::lifecycle::fresh_id()),
            "cancel" => contract::wire::Cancel::parse(payload).and_then(|c| r.host.cancel_command(c)),
            "runtime" => r.runtime_input(payload),
            "snapshot" => contract::wire::snapshot_session(payload).and_then(|session| {
                if session == s.session { Ok(()) } else { Err(ErrorCode::StaleSession) }
            }),
            _ => Err(ErrorCode::InvalidInput),
        };
        r.tick(elapsed()).map_err(safe_error)?;
        result.map_err(safe_error)?;
        Ok(r.view())
    })
}
pub fn context_lost() {
    if let Ok(lock) = runtime() { if let Ok(mut r) = lock.lock() { let _ = r.tick(elapsed()); let _ = r.lost(); } }
}
pub fn page_replaced() {
    if let Ok(lock) = runtime() { if let Ok(mut r) = lock.lock() { let _ = r.tick(elapsed()); let _ = r.page_replaced(); } }
}
pub fn shutdown() {
    if let Ok(lock) = runtime() { if let Ok(mut r) = lock.lock() { let _ = r.tick(elapsed()); let _ = r.shutdown(); } }
}
pub fn idle() -> bool {
    runtime().ok().and_then(|lock| lock.lock().ok().map(|r| r.idle())).unwrap_or(false)
}

#[cfg(test)]
mod tests;
