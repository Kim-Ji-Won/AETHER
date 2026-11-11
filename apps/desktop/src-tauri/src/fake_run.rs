//! E4b: one in-memory run, fixed synthetic events, no external effects.
use std::sync::{Mutex, OnceLock};
use tauri::ipc::InvokeBody;

// JSON tuple, documented in docs/architecture/e4b-normal-path.md.
pub type Snapshot = (u32, u32, u32, u32, &'static str, u32, u8, u8, &'static str,
    Vec<(u32, &'static str, &'static str)>);

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

#[derive(Default)]
struct Ledger {
    next_id: u32,
    run: Option<Snapshot>,
}

impl Ledger {
    fn create(&mut self) -> Result<Snapshot, &'static str> {
        if self.run.as_ref().is_some_and(|run| matches!(run.4, "created" | "running")) {
            return Err("Run already active");
        }
        let id = self.next_id.checked_add(1).ok_or("Run ID exhausted")?;
        self.next_id = id;
        // IDs are scoped to this host process. No restart/reload recovery claim.
        let run = (1, id, id, 1, "created", 0, 0, 0, "", vec![]);
        self.run = Some(run.clone());
        Ok(run)
    }

    fn apply(&mut self, frame: Frame) -> Result<Snapshot, &'static str> {
        let run = self.run.as_mut().ok_or("No run")?;
        if (frame.version, frame.run_id, frame.request_id, frame.generation) != (run.0, run.1, run.2, run.3)
            || frame.seq != run.5 + 1 || matches!(run.4, "succeeded" | "failed") {
            return Err("Invalid identity, sequence or terminal state");
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
        eprintln!("AETHER_FAKE version={} run_id={} request_id={} generation={} seq={} type={} value={} state={} terminal_count={}",
            run.0, run.1, run.2, run.3, run.5, kind, value, run.4, run.7);
        Ok(run.clone())
    }
}

static LEDGER: OnceLock<Mutex<Ledger>> = OnceLock::new();
fn ledger() -> &'static Mutex<Ledger> { LEDGER.get_or_init(Default::default) }

pub fn create() -> Result<Snapshot, &'static str> {
    ledger().lock().map_err(|_| "Ledger unavailable")?.create()
}

pub fn submit(frame: Frame) -> Result<Snapshot, &'static str> {
    ledger().lock().map_err(|_| "Ledger unavailable")?.apply(frame)
}

pub fn snapshot() -> Result<Option<Snapshot>, &'static str> {
    Ok(ledger().lock().map_err(|_| "Ledger unavailable")?.run.clone())
}

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
