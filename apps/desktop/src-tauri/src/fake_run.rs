//! One bounded in-memory run. Rust serializes events, cancellation and terminal decisions.
use std::sync::{Mutex, OnceLock};
use tauri::ipc::InvokeBody;

// JSON tuple, documented in docs/architecture/e4b-normal-path.md.
pub type Snapshot = (u32, u32, u32, u32, &'static str, u32, u8, u8, &'static str,
    Vec<(u32, &'static str, &'static str)>, &'static str, &'static str);

#[derive(Clone, Debug)]
pub struct Frame {
    version: u32,
    run_id: u32,
    request_id: u32,
    generation: u32,
    seq: u32,
    kind: String,
    value: String,
}

impl Frame {
    pub fn parse(body: &InvokeBody) -> Result<Self, &'static str> {
        let InvokeBody::Json(json) = body else { return Err("Expected JSON object") };
        if json.to_string().len() > 511 { return Err("Frame too large after JSON parsing"); }
        let object = json.as_object().ok_or("Expected JSON object")?;
        // Closed schema: no arbitrary question, metadata, nested data or extra fields.
        let keys = ["version", "run_id", "request_id", "generation", "seq", "type", "value"];
        if object.len() != keys.len() || keys.iter().any(|key| !object.contains_key(*key)) {
            return Err("Invalid frame fields");
        }
        let number = |key| object[key].as_u64().and_then(|v| u32::try_from(v).ok())
            .ok_or("Expected u32 identifier");
        let kind = object["type"].as_str().filter(|s| s.len() <= 8).ok_or("Invalid event type")?;
        let value = object["value"].as_str().filter(|s| s.len() <= 7).ok_or("Invalid event value")?;
        Ok(Self { version: number("version")?, run_id: number("run_id")?,
            request_id: number("request_id")?, generation: number("generation")?,
            seq: number("seq")?, kind: kind.into(), value: value.into() })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Start { pub token: u32, pub mode: &'static str }
impl Start {
    pub fn parse(body: &InvokeBody) -> Result<Self, &'static str> {
        let InvokeBody::Json(j) = body else { return Err("Expected JSON"); };
        if j.as_object().map(|o| o.len()) != Some(2) || j.to_string().len() > 80 {
            return Err("Invalid start fields");
        }
        let token = j["token"].as_u64().and_then(|v| u32::try_from(v).ok())
            .filter(|v| *v > 0).ok_or("Invalid start token")?;
        let mode = match j["mode"].as_str() { Some("normal") => "normal", Some("pause25") => "pause25", Some("long60") => "long60",
            _ => return Err("Invalid fixture mode") };
        Ok(Self { token, mode })
    }
}

pub fn cancel_body(body: &InvokeBody) -> Result<Snapshot, &'static str> {
    let InvokeBody::Json(j) = body else { return Err("Expected JSON"); };
    if j.as_object().map(|o| o.len()) != Some(2) || j.to_string().len() > 80 {
        return Err("Invalid cancel fields");
    }
    let id = j["run_id"].as_u64().and_then(|v| u32::try_from(v).ok()).ok_or("Invalid run ID")?;
    let variant = j["variant"].as_str().filter(|v| matches!(*v, "webview-a" | "node-b"))
        .ok_or("Invalid variant")?;
    cancel(id, variant)
}

#[derive(Clone, Default)]
struct Ledger {
    next_id: u32,
    epoch: u32,
    run: Option<Snapshot>,
    sidecar_busy: bool,
    last_start: Option<(Start, &'static str)>,
}

impl Ledger {
    #[cfg(test)]
    fn create(&mut self) -> Result<Snapshot, &'static str> {
        self.create_variant("webview-a")
    }

    #[cfg(test)]
    fn create_variant(&mut self, variant: &'static str) -> Result<Snapshot, &'static str> {
        self.begin(Start { token: self.next_id + 1, mode: "normal" }, variant)
    }

    fn begin(&mut self, start: Start, variant: &'static str) -> Result<Snapshot, &'static str> {
        if let Some((previous, owner)) = self.last_start {
            if start.token == previous.token {
                return Err(if start == previous && owner == variant { "Duplicate start" } else { "Start conflict" });
            }
            if start.token < previous.token { return Err("Stale start token"); }
        }
        if self.sidecar_busy || self.run.as_ref().is_some_and(|run| matches!(run.4, "created" | "running" | "cancel_requested")) {
            return Err("Run already active");
        }
        let id = self.next_id.checked_add(1).ok_or("Run ID exhausted")?;
        self.next_id = id;
        // IDs remain memory-only; lifecycle envelope scopes IPC to an instance/context.
        let run = (1, id, start.token, self.epoch.max(1), "created", 0, 0, 0, "", vec![], variant, start.mode);
        self.last_start = Some((start, variant));
        self.sidecar_busy = variant == "node-b";
        self.run = Some(run.clone());
        Ok(run)
    }

    fn apply(&mut self, frame: Frame) -> Result<Snapshot, &'static str> {
        let run = self.run.as_mut().ok_or("No run")?;
        if (frame.version, frame.run_id, frame.request_id, frame.generation) != (run.0, run.1, run.2, run.3) {
            return Err("Invalid identity");
        }
        if let Some((_, kind, value)) = run.9.iter().find(|(seq, _, _)| *seq == frame.seq) {
            return Err(if *kind == frame.kind && *value == frame.value { "Duplicate frame" } else { "Frame conflict" });
        }
        if frame.seq != run.5 + 1 || matches!(run.4, "succeeded" | "failed" | "cancel_requested" | "cancelled") {
            return Err("Invalid sequence or terminal state");
        }
        let (state, progress, terminal, result, kind, value) = match
            (run.4, frame.kind.as_str(), frame.value.as_str(), run.6) {
            ("created", "start", "", 0) => ("running", 0, 0, "", "start", ""),
            ("running", "progress", "25", 0) => ("running", 25, 0, "", "progress", "25"),
            ("running", "progress", "50", 25) => ("running", 50, 0, "", "progress", "50"),
            ("running", "progress", "75", 50) => ("running", 75, 0, "", "progress", "75"),
            ("running", "terminal", "FAKE_OK", 75) => ("succeeded", 75, 1, "FAKE_OK", "terminal", "FAKE_OK"),
            ("created" | "running", "failed", "", _) => ("failed", run.6, 1, "", "failed", ""),
            _ => return Err("Invalid transition or payload"),
        };
        run.4 = state;
        run.5 = frame.seq;
        run.6 = progress;
        run.7 = terminal;
        run.8 = result;
        run.9.push((frame.seq, kind, value)); // At most five accepted events.
        Ok(run.clone())
    }
    fn cancel(&mut self, id: u32, variant: &str) -> Result<Snapshot, &'static str> {
        let run = self.run.as_mut().ok_or("No run")?;
        if run.1 != id || run.10 != variant { return Err("Cancel ownership mismatch"); }
        match run.4 {
            "succeeded" | "failed" => return Err("Already terminal"),
            "cancel_requested" | "cancelled" => return Ok(run.clone()), // Idempotent, no trace append.
            _ => {}
        }
        run.4 = "cancel_requested";
        // Cancellation linearizes here. All subsequent provider events are rejected.
        // A has no child; B finalizes only after kill/wait outside the mutex.
        if variant == "webview-a" { Self::cancelled(run); }
        Ok(run.clone())
    }

    fn context_lost(&mut self, epoch: u32) {
        self.epoch = epoch;
        if let Some(run) = self.run.as_mut() {
            if run.10 == "webview-a" && matches!(run.4, "created" | "running") {
                run.4 = "failed"; run.7 = 1; run.5 += 1;
                run.9.push((run.5, "failed", ""));
                eprintln!("AETHER_CONTEXT_LOST run_id={} generation={} state=failed terminal_count=1", run.1, run.3);
            }
        }
    }

    fn cancelled(run: &mut Snapshot) {
        run.4 = "cancelled"; run.7 = 1; run.8 = "";
        run.5 += 1;
        run.9.push((run.5, "cancelled", ""));
    }

}

static LEDGER: OnceLock<Mutex<Ledger>> = OnceLock::new();
fn ledger() -> &'static Mutex<Ledger> { LEDGER.get_or_init(Default::default) }

pub fn begin(start: Start, variant: &'static str) -> Result<Snapshot, &'static str> {
    let received = std::time::Instant::now();
    let run = ledger().lock().map_err(|_| "Ledger unavailable")?.begin(start, variant)?;
    crate::measurement::record("host_accept", Some(&run), Some(received), None, None);
    Ok(run)
}

pub fn cancel(id: u32, variant: &str) -> Result<Snapshot, &'static str> {
    let received = std::time::Instant::now();
    let run = ledger().lock().map_err(|_| "Ledger unavailable")?.cancel(id, variant)?;
    crate::measurement::record("cancel_confirmed", Some(&run), Some(received), None, None);
    eprintln!("AETHER_CANCEL run_id={} variant={} state={} terminal_count={}", run.1, run.10, run.4, run.7);
    Ok(run)
}

pub fn cancel_pending(id: u32) -> bool {
    ledger().lock().map(|s| s.run.as_ref().is_some_and(|r| r.1 == id && r.4 == "cancel_requested")).unwrap_or(true)
}

pub fn submit(frame: Frame) -> Result<Snapshot, &'static str> {
    submit_from(frame, "webview-a")
}

fn submit_from(frame: Frame, variant: &str) -> Result<Snapshot, &'static str> {
    let received = std::time::Instant::now();
    let mut state = ledger().lock().map_err(|_| "Ledger unavailable")?;
    if state.run.as_ref().map(|r| r.10) != Some(variant) { return Err("Runtime owner mismatch"); }
    let run = state.apply(frame)?;
    drop(state);
    crate::measurement::record("event_accepted", Some(&run), Some(received), None, None);
    let (_, kind, value) = run.9.last().unwrap();
    eprintln!("AETHER_FAKE variant={} version={} run_id={} request_id={} generation={} seq={} type={} value={} state={} terminal_count={} time_ms={} pid={}",
        run.10, run.0, run.1, run.2, run.3, run.5, kind, value, run.4, run.7, crate::lifecycle::now(), std::process::id());
    Ok(run)
}

#[cfg(test)]
pub fn create_node() -> Result<Snapshot, &'static str> {
    ledger().lock().map_err(|_| "Ledger unavailable")?.create_variant("node-b")
}

pub fn submit_node(frame: Frame) -> Result<Snapshot, &'static str> { submit_from(frame, "node-b") }

pub fn validate_node_terminal(frame: &Frame) -> Result<(), &'static str> {
    let state = ledger().lock().map_err(|_| "Ledger unavailable")?;
    if !state.sidecar_busy || state.run.as_ref().map(|r| r.10) != Some("node-b")
        || frame.kind != "terminal" { return Err("Invalid sidecar terminal"); }
    // Validate without publishing success; success requires a normal child exit as well.
    state.clone().apply(frame.clone())?;
    Ok(())
}

pub fn finish_node(id: u32, cleanup_complete: bool) {
    // If OS cleanup could not be confirmed, keep the run busy instead of claiming cancellation.
    if !cleanup_complete { return; }
    if let Ok(mut state) = ledger().lock() {
        if let Some(run) = &state.run {
            if run.1 != id || run.10 != "node-b" { return; }
            if matches!(run.4, "created" | "running") {
                let frame = Frame { version: run.0, run_id: run.1, request_id: run.2,
                    generation: run.3, seq: run.5 + 1, kind: "failed".into(), value: "".into() };
                let _ = state.apply(frame);
            } else if run.4 == "cancel_requested" {
                Ledger::cancelled(state.run.as_mut().unwrap());
            }
        }
        state.sidecar_busy = false;
    }
}

pub fn snapshot() -> Result<Option<Snapshot>, &'static str> {
    Ok(ledger().lock().map_err(|_| "Ledger unavailable")?.run.clone())
}

// Called under lifecycle's gate, never while waiting on a child or native sheet.
pub fn context_lost(epoch: u32) {
    if let Ok(mut state) = ledger().lock() {
        state.context_lost(epoch);
    }
}
pub fn shutdown() {
    if let Ok(mut state) = ledger().lock() {
        if let Some(run) = state.run.clone() {
            if matches!(run.4, "created" | "running") { let _ = state.cancel(run.1, run.10); }
        }
    }
}
pub fn child_idle() -> bool { ledger().lock().map(|s| !s.sidecar_busy).unwrap_or(false) }

#[cfg(test)]
mod tests {
    use super::*;
    fn frame(id: u32, seq: u32, kind: &str, value: &str) -> Frame {
        Frame { version: 1, run_id: id, request_id: id, generation: 1,
            seq, kind: kind.into(), value: value.into() }
    }
    fn finish(ledger: &mut Ledger, id: u32) -> Snapshot {
        for (seq, kind, value) in [(1,"start",""), (2,"progress","25"),
            (3,"progress","50"), (4,"progress","75"), (5,"terminal","FAKE_OK")] {
            let run = ledger.apply(frame(id, seq, kind, value)).unwrap();
            assert_eq!((run.0, run.1, run.2, run.3, run.5), (1, id, id, 1, seq));
        }
        ledger.run.clone().unwrap()
    }
    #[test]
    fn reload_fails_a_once_preserves_b_and_snapshot_then_new_generation() {
        for variant in ["webview-a", "node-b"] {
            let mut ledger = Ledger::default();
            ledger.create_variant(variant).unwrap();
            ledger.apply(frame(1,1,"start","")).unwrap();
            let before = ledger.apply(frame(1,2,"progress","25")).unwrap();
            ledger.context_lost(2);
            if variant == "webview-a" {
                let failed = ledger.run.clone().unwrap();
                assert_eq!((failed.4,failed.7),("failed",1));
                ledger.context_lost(3); assert_eq!(ledger.run.as_ref(),Some(&failed));
                assert!(ledger.apply(frame(1,3,"progress","50")).is_err());
                let next = ledger.create().unwrap(); assert_eq!(next.3,3);
            } else {
                assert_eq!(ledger.run.as_ref(),Some(&before));
                assert_eq!(ledger.apply(frame(1,3,"progress","50")).unwrap().6,50);
            }
        }
    }
    #[test]
    fn normal_order_identity_and_single_terminal() {
        let mut ledger = Ledger::default();
        let created = ledger.create().unwrap();
        assert_eq!((created.4, created.5), ("created", 0));
        assert!(ledger.create().is_err());
        let run = finish(&mut ledger, created.1);
        assert_eq!((run.4, run.6, run.7, run.8), ("succeeded", 75, 1, "FAKE_OK"));
        assert_eq!(run.9, vec![(1,"start",""), (2,"progress","25"),
            (3,"progress","50"), (4,"progress","75"), (5,"terminal","FAKE_OK")]);
        for seq in [5, 6] { assert!(ledger.apply(frame(run.1, seq, "terminal", "FAKE_OK")).is_err()); }
        assert_eq!(ledger.run.as_ref(), Some(&run));
        assert_eq!(ledger.create().unwrap().1, run.1 + 1);
    }
    #[test]
    fn wrong_identity_sequence_and_regression_leave_state_unchanged() {
        let mut ledger = Ledger::default();
        ledger.create().unwrap();
        ledger.apply(frame(1, 1, "start", "")).unwrap();
        let before = ledger.apply(frame(1, 2, "progress", "25")).unwrap();
        let mut wrong_request = frame(1, 3, "progress", "50"); wrong_request.request_id = 2;
        let mut wrong_generation = wrong_request.clone(); wrong_generation.request_id = 1; wrong_generation.generation = 2;
        let mut wrong_version = wrong_generation.clone(); wrong_version.generation = 1; wrong_version.version = 2;
        for invalid in [frame(2, 3, "progress", "50"), frame(1, 4, "progress", "50"),
            frame(1, 3, "progress", "25"), frame(1, 3, "terminal", "FAKE_OK"),
            wrong_request, wrong_generation, wrong_version] {
            assert!(ledger.apply(invalid).is_err());
            assert_eq!(ledger.run.as_ref(), Some(&before));
        }
    }
    #[test]
    fn failure_is_terminal_and_new_run_is_explicit() {
        let mut ledger = Ledger::default();
        ledger.create().unwrap();
        let failed = ledger.apply(frame(1, 1, "failed", "")).unwrap();
        assert_eq!((failed.4, failed.7, failed.8), ("failed", 1, ""));
        assert!(ledger.apply(frame(1, 2, "start", "")).is_err());
        assert_eq!(ledger.create().unwrap().1, 2);
    }

    #[test]
    fn frame_schema_is_bounded_and_closed() {
        let valid = r#"{"version":1,"run_id":1,"request_id":1,"generation":1,"seq":1,"type":"start","value":""}"#;
        let parse = |json: &str| {
            let body = InvokeBody::Json(tauri::ipc::InvokeResponseBody::Json(json.into()).deserialize().unwrap());
            Frame::parse(&body)
        };
        assert!(parse(valid).is_ok());
        for invalid in [valid.replace("\"seq\":1", "\"seq\":4294967296"),
            valid.replace("\"seq\":1", "\"seq\":-1"),
            valid.replace("\"start\"", "\"oversized_type\""),
            valid.replace("\"value\":\"\"", "\"value\":\"TOO_LONG\""),
            valid.replace("\"value\":\"\"", "\"value\":{}"),
            valid.replace("\"value\":\"\"", "\"value\":\"\",\"approved\":true")] {
            assert!(parse(&invalid).is_err());
        }
        assert!(Frame::parse(&InvokeBody::Raw(vec![0; 512])).is_err());
    }
}

#[cfg(test)]
#[path = "fake_run_tests.rs"]
mod safety_tests;
