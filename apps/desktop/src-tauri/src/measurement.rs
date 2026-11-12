//! Opt-in, bounded diagnostic output. Never a ledger, authority, or cross-process clock.
use std::{io::Write, sync::{Mutex, OnceLock}, time::Instant};
use crate::fake_run::Snapshot;

struct Session { trial: String, variant: &'static str, instance: String, start: Instant, count: u32 }
static SESSION: OnceLock<Mutex<Option<Session>>> = OnceLock::new();
fn valid_trial(value: &str) -> bool {
    value.len() == 32 && value.bytes().all(|b| b.is_ascii_hexdigit())
}
pub fn init() {
    let enabled = std::env::args_os().any(|arg| arg == "--benchmark-smoke");
    let trial = std::env::var("AETHER_BENCH_TRIAL").unwrap_or_default();
    let variant = match std::env::var("AETHER_BENCH_VARIANT").as_deref() {
        Ok("webview-a") => Some("webview-a"), Ok("node-b") => Some("node-b"), _ => None,
    };
    let session = if enabled && valid_trial(&trial) && variant.is_some() {
        Some(Session { trial, variant: variant.unwrap(), instance: crate::lifecycle::instance(),
            start: Instant::now(), count: 0 })
    } else { None };
    let _ = SESSION.set(Mutex::new(session));
    record("host_main", None, None, None, None);
}

// All text below comes from fixed host enums / validated ledger values, never input text.
// A separate lock serializes timestamps and records; callers release ledger locks first.
pub fn record(event: &'static str, run: Option<&Snapshot>, since: Option<Instant>,
    child: Option<u32>, exit: Option<i32>) {
    let Some(lock) = SESSION.get() else { return; };
    let Ok(mut session) = lock.lock() else { return; };
    let Some(s) = session.as_mut() else { return; };
    if s.count >= 128 { return; }
    s.count += 1;
    let (id, request, generation, seq, progress, terminals, outcome, result, mode, variant) =
        run.map(|r| (r.1,r.2,r.3,r.5,r.6,r.7,r.4,r.8,r.11,r.10))
        .unwrap_or((0,0,0,0,0,0,"none","","none",s.variant));
    let number = |n: Option<u128>| n.map(|v| v.to_string()).unwrap_or_else(|| "null".into());
    let line = format!(concat!("AETHER_MEASURE {{\"schema_version\":1,\"trial_id\":\"{}\",",
        "\"variant\":\"{}\",\"component\":\"host\",\"app_instance\":\"{}\",",
        "\"process_pid\":{},\"run_id\":{},\"request_id\":{},\"generation\":{},\"seq\":{},",
        "\"event_type\":\"{}\",\"monotonic_ns\":{},\"operation_ns\":{},",
        "\"outcome\":\"{}\",\"exit_code\":{},\"child_pid\":{},",
        "\"progress\":{},\"terminal_count\":{},\"result\":\"{}\",\"fixture\":\"{}\"}}\n"),
        s.trial,variant,s.instance,std::process::id(),id,request,generation,seq,event,
        s.start.elapsed().as_nanos(),number(since.map(|t| t.elapsed().as_nanos())),outcome,
        exit.map(|n| n.to_string()).unwrap_or_else(|| "null".into()),
        number(child.map(|p| p as u128)),progress,terminals,result,mode);
    // Broken diagnostic pipes must not change app state or panic.
    let _ = std::io::stderr().lock().write_all(line.as_bytes());
}

#[cfg(test)]
mod tests {
    #[test]
    fn trial_id_is_bounded_and_cannot_inject_json_or_paths() {
        assert!(super::valid_trial("0123456789abcdef0123456789abcdef"));
        for value in ["", "../private", "arbitrary text", "0123456789abcdef0123456789abcde\"", "0123456789abcdef0123456789abcdef0"] {
            assert!(!super::valid_trial(value));
        }
    }
}
