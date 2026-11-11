use super::*;
use std::sync::{Arc, Barrier};

fn frame(run: &Snapshot, seq: u32, kind: &str, value: &str) -> Frame {
    Frame { version: run.0, run_id: run.1, request_id: run.2, generation: run.3,
        seq, kind: kind.into(), value: value.into() }
}
fn begin(variant: &'static str) -> (Ledger, Snapshot) {
    let mut l = Ledger::default();
    let r = l.begin(Start { token: 1, mode: "normal" }, variant).unwrap(); (l, r)
}
fn advance(l: &mut Ledger, run: &Snapshot, count: usize) {
    for (i, (kind, value)) in [("start", ""), ("progress", "25"), ("progress", "50"),
        ("progress", "75"), ("terminal", "FAKE_OK")].into_iter().take(count).enumerate() {
        l.apply(frame(run, i as u32 + 1, kind, value)).unwrap();
    }
}
fn finish_cancel(l: &mut Ledger) {
    if l.run.as_ref().unwrap().4 == "cancel_requested" { Ledger::cancelled(l.run.as_mut().unwrap()); }
    l.sidecar_busy = false;
}
#[test]
fn repeated_parity_cancel_replay_and_new_run() {
    for variant in ["webview-a", "node-b"] {
        for _ in 0..10 {
            for barrier in [0, 2, 4] {
                let (mut l, r) = begin(variant); advance(&mut l, &r, barrier);
                assert!(l.cancel(r.1 + 1, variant).is_err());
                assert!(l.cancel(r.1, if variant == "node-b" { "webview-a" } else { "node-b" }).is_err());
                let requested = l.cancel(r.1, variant).unwrap();
                assert_eq!(l.cancel(r.1, variant).unwrap(), requested);
                // Cancellation wins before any late frame, including before B's cleanup.
                let late = frame(&r, barrier as u32 + 1, if barrier == 4 { "terminal" } else { "progress" },
                    if barrier == 4 { "FAKE_OK" } else { "50" });
                assert!(l.apply(late.clone()).is_err()); finish_cancel(&mut l);
                let terminal = l.run.clone().unwrap(); assert_eq!((terminal.4, terminal.7), ("cancelled", 1));
                assert!(l.apply(late).is_err()); assert_eq!(l.cancel(r.1, variant).unwrap(), terminal);
                let next = l.begin(Start { token: 2, mode: "normal" }, variant).unwrap();
                assert!(l.cancel(r.1, variant).is_err());
                assert!(l.apply(frame(&r, 1, "start", "")).is_err());
                advance(&mut l, &next, 5); let done = l.run.clone().unwrap();
                assert_eq!((done.4, done.7, done.8, done.9.len()), ("succeeded", 1, "FAKE_OK", 5));
                assert!(l.cancel(next.1, variant).is_err()); assert_eq!(l.run.as_ref(), Some(&done));
            }
            let (mut l, r) = begin(variant);
            for (i, (kind, value)) in [("start", ""), ("progress", "25"), ("progress", "50"),
                ("progress", "75"), ("terminal", "FAKE_OK")].into_iter().enumerate() {
                let f = frame(&r, i as u32 + 1, kind, value); let accepted = l.apply(f.clone()).unwrap();
                assert_eq!(l.apply(f.clone()).unwrap_err(), "Duplicate frame");
                let mut conflict = f; conflict.value = "bad".into();
                assert_eq!(l.apply(conflict).unwrap_err(), "Frame conflict");
                assert_eq!(l.run.as_ref(), Some(&accepted));
            }
            assert_eq!(l.run.as_ref().unwrap().9.len(), 5);
        }
        println!("E4D ledger variant={variant} cancel_barriers=created,25,75 repeats=10 late_rejected=60 duplicate_cancel_unchanged=60 replay_rejected=50 conflict_rejected=50 recovery_success=30 failures=0");
    }
}
#[test]
fn duplicate_start_and_terminal_failure_cannot_restart_work() {
    for variant in ["webview-a", "node-b"] {
        for _ in 0..10 {
            let (mut l, r) = begin(variant);
            let start = Start { token: 1, mode: "normal" };
            assert_eq!(l.begin(start, variant).unwrap_err(), "Duplicate start");
            assert_eq!(l.begin(Start { mode: "pause25", ..start }, variant).unwrap_err(), "Start conflict");
            assert_eq!(l.begin(start, if variant == "webview-a" { "node-b" } else { "webview-a" }).unwrap_err(), "Start conflict");
            let failed = l.apply(frame(&r, 1, "failed", "")).unwrap();
            assert!(l.apply(frame(&r, 2, "progress", "25")).is_err());
            assert!(l.apply(frame(&r, 2, "terminal", "FAKE_OK")).is_err());
            assert!(l.cancel(r.1, variant).is_err()); assert_eq!(l.run.as_ref(), Some(&failed));
            l.sidecar_busy = false;
            assert_eq!(l.begin(start, variant).unwrap_err(), "Duplicate start");
            let new = l.begin(Start { token: 2, mode: "normal" }, variant).unwrap();
            assert_eq!(l.begin(start, variant).unwrap_err(), "Stale start token");
            assert_eq!((new.1, l.next_id), (2, 2));
        }
    }
    println!("E4D duplicate_start/failure variants=2 repeats=10 failures=0");
}
#[test]
fn completion_cancel_race_has_one_winner() {
    for variant in ["webview-a", "node-b"] {
        let mut winners = [0, 0];
        for _ in 0..10 {
            let (mut l, r) = begin(variant); advance(&mut l, &r, 4);
            let shared = Arc::new(Mutex::new(l)); let gate = Arc::new(Barrier::new(3));
            let complete = { let state=shared.clone(); let gate=gate.clone(); let r=r.clone();
                std::thread::spawn(move || { gate.wait(); state.lock().unwrap().apply(frame(&r, 5, "terminal", "FAKE_OK")) }) };
            let cancel = { let state=shared.clone(); let gate=gate.clone();
                std::thread::spawn(move || { gate.wait(); state.lock().unwrap().cancel(r.1, variant) }) };
            gate.wait(); let succeeded = complete.join().unwrap().is_ok(); let cancelled = cancel.join().unwrap().is_ok();
            assert_ne!(succeeded, cancelled); let mut l=shared.lock().unwrap(); finish_cancel(&mut l);
            let done=l.run.as_ref().unwrap(); assert_eq!(done.7,1); assert_eq!(done.9.len(),5);
            winners[usize::from(cancelled)] += 1;
        }
        // Deterministic opposite order, not a scheduler probability claim.
        for cancel_first in [true, false] {
            let (mut l,r)=begin(variant); advance(&mut l,&r,4);
            if cancel_first { l.cancel(r.1,variant).unwrap(); assert!(l.apply(frame(&r,5,"terminal","FAKE_OK")).is_err()); }
            else { l.apply(frame(&r,5,"terminal","FAKE_OK")).unwrap(); assert!(l.cancel(r.1,variant).is_err()); }
            finish_cancel(&mut l); assert_eq!(l.run.as_ref().unwrap().7,1);
        }
        println!("E4D race variant={variant} barrier=before_terminal concurrent=10 success_wins={} cancel_wins={} forced_orders=2 failures=0",winners[0],winners[1]);
    }
}
#[test]
fn malformed_matrix_shared_by_a_and_b_preserves_ledger() {
    use tauri::ipc::InvokeResponseBody;
    let valid=r#"{"version":1,"run_id":1,"request_id":1,"generation":1,"seq":1,"type":"start","value":""}"#;
    let mut cases=vec![];
    for key in ["version","run_id","request_id","generation","seq"] {
        for value in ["0","2","-1","4294967296","1.5","null"] {
            cases.push(valid.replace(&format!("\"{key}\":1"),&format!("\"{key}\":{value}")));
        }
        cases.push(valid.replace(&format!("\"{key}\":1,"),""));
    }
    cases.extend([valid.replace("\"start\"","\"unknown\""),valid.replace("\"value\":\"\"","\"value\":\"bad\""),
        valid.replace("\"value\":\"\"","\"value\":\"\",\"extra\":true"),
        valid.replace("\"value\":\"\"",&format!("\"value\":\"{}\"","x".repeat(600)))]);
    for variant in ["webview-a","node-b"] {
        for _ in 0..5 {
            let (mut l, r)=begin(variant);
            for case in &cases {
                let body=InvokeBody::Json(InvokeResponseBody::Json(case.clone()).deserialize().unwrap());
                assert!(Frame::parse(&body).and_then(|f|l.apply(f)).is_err(),"{case}");
                assert_eq!(l.run.as_ref(),Some(&r));
            }
            advance(&mut l,&r,2); let before=l.run.clone();
            for f in [frame(&r,0,"progress","50"), frame(&r,4,"progress","50"),frame(&r,3,"progress","25")] {
                assert!(l.apply(f).is_err()); assert_eq!(l.run,before);
            }
            advance_remaining(&mut l,&r);
        }
        println!("E4D malformed variant={variant} cases={} repeats=5 accepted_invalid=0 failures=0",cases.len()+3);
    }
}
fn advance_remaining(l:&mut Ledger,r:&Snapshot) {
    l.apply(frame(r,3,"progress","50")).unwrap(); l.apply(frame(r,4,"progress","75")).unwrap();
    l.apply(frame(r,5,"terminal","FAKE_OK")).unwrap();
}

#[test]
fn start_and_cancel_commands_have_closed_bounded_schemas() {
    use tauri::ipc::InvokeResponseBody;
    let body=|s:&str| InvokeBody::Json(InvokeResponseBody::Json(s.into()).deserialize().unwrap());
    for s in [r#"{"token":1,"mode":"normal"}"#,r#"{"token":2,"mode":"pause25"}"#] {
        assert!(Start::parse(&body(s)).is_ok());
    }
    for s in [r#"{}"#,r#"{"token":0,"mode":"normal"}"#,r#"{"token":4294967296,"mode":"normal"}"#,
        r#"{"token":1,"mode":"shell"}"#,r#"{"token":1,"mode":"normal","approved":true}"#,
        r#"{"token":1}"#] { assert!(Start::parse(&body(s)).is_err()); }
    // All rejected before touching the process-global ledger.
    for s in [r#"{}"#,r#"{"run_id":1,"variant":"other"}"#,
        r#"{"run_id":4294967296,"variant":"node-b"}"#,
        r#"{"run_id":1,"variant":"node-b","pid":1}"#] { assert!(cancel_body(&body(s)).is_err()); }
}
