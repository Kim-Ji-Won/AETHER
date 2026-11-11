use super::*;
fn dir() -> PathBuf { PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sidecar/generated").canonicalize().unwrap() }
fn create(mode: &'static str) -> Snapshot {
    let token=fake_run::snapshot().unwrap().map(|r|r.2+1).unwrap_or(1);
    fake_run::begin(fake_run::Start {token,mode},"node-b").unwrap()
}
fn run_test(entry: &Path, created: &Snapshot, timeout: Duration) -> (Reply, ChildReport) {
    let d=dir(); let mut report=ChildReport::default();
    let result=exchange(&d.join("node"),entry,&d,created,timeout,&mut report);
    fake_run::finish_node(created.1, report.reaped);
    assert!(report.reaped,"child was not reaped");
    assert!(!Command::new("/bin/kill").args(["-0",&report.pid.to_string()])
        .stdout(Stdio::null()).stderr(Stdio::null()).status().unwrap().success());
    (result,report)
}
#[test]
fn real_node_cancel_barrier_and_recovery_repeated() {
    let _serial=TEST_SERIAL.lock().unwrap();
    for barrier in ["created","progress25"] {
        for _ in 0..10 {
            let created=create("pause25"); let id=created.1;
            if barrier=="created" { fake_run::cancel(id,"node-b").unwrap(); }
            let worker=std::thread::spawn(move || run_test(&dir().join("entry.mjs"),&created,TIMEOUT));
            if barrier=="progress25" {
                let until=Instant::now()+TIMEOUT;
                loop {
                    let r=fake_run::snapshot().unwrap().unwrap();
                    if r.1==id && r.6==25 { break; }
                    assert!(Instant::now()<until); std::thread::sleep(Duration::from_millis(5));
                }
                let first=fake_run::cancel(id,"node-b").unwrap();
                let second=fake_run::cancel(id,"node-b").unwrap();
                assert!(matches!(second.4,"cancel_requested"|"cancelled")); assert_eq!(first.1,second.1);
            }
            let (result,report)=worker.join().unwrap(); assert!(result.is_err()); assert!(report.reaped);
            let r=fake_run::snapshot().unwrap().unwrap(); assert_eq!((r.4,r.7),("cancelled",1));
            let late=format!("{{\"version\":1,\"run_id\":{},\"request_id\":{},\"generation\":1,\"seq\":{},\"type\":\"terminal\",\"value\":\"FAKE_OK\"}}",r.1,r.2,r.5+1);
            assert!(fake_run::submit_node(Frame::parse(&body(late).unwrap()).unwrap()).is_err());
            assert_eq!(fake_run::snapshot().unwrap().unwrap(),r);
            let created=create("normal"); let (next,_)=run_test(&dir().join("entry.mjs"),&created,TIMEOUT);
            assert_eq!(next.unwrap().0.4,"succeeded");
        }
        println!("E4D real_node barrier={barrier} repeats=10 cancelled=10 late_rejected=10 recovery_success=10 child_reaped=20 failures=0");
    }
}
#[test]
fn real_node_malformed_transport_is_failed_and_reaped() {
    let _serial=TEST_SERIAL.lock().unwrap();
    let temp=std::env::temp_dir().join(format!("aether-e4d-fault-{}",std::process::id()));
    std::fs::create_dir(&temp).unwrap();
    for fault in ["json","utf8","no_lf","oversize","eof","unknown","wrong_id","generation","duplicate","conflict","bad_done","extra","timeout"] {
        let entry=temp.join(format!("{fault}.mjs"));
        std::fs::write(&entry,format!("const FAULT = '{fault}';\n{}",include_str!("../../tests/sidecar-fault.mjs"))).unwrap();
        for _ in 0..5 {
            let created=create("normal");
            let (result,_)=run_test(&entry,&created,if fault=="timeout" {Duration::from_millis(150)} else {TIMEOUT});
            let error=result.unwrap_err();
            let expected=match fault {
                "json"=>"Invalid sidecar JSON", "utf8"=>"Invalid sidecar UTF-8",
                "no_lf"|"oversize"=>"Invalid sidecar frame length", "eof"=>"Unexpected sidecar EOF",
                "unknown"=>"Invalid transition or payload", "wrong_id"|"generation"=>"Invalid identity",
                "duplicate"=>"Duplicate frame", "conflict"=>"Frame conflict", "bad_done"=>"Invalid Node completion",
                "extra"=>"Unexpected trailing sidecar output", "timeout"=>"Sidecar response timeout", _=>unreachable!() };
            assert_eq!(error,expected);
            let r=fake_run::snapshot().unwrap().unwrap(); assert_eq!((r.4,r.7),("failed",1));
            assert_ne!(r.8,"FAKE_OK");
            let created=create("normal"); let (normal,_)=run_test(&dir().join("entry.mjs"),&created,TIMEOUT);
            assert_eq!(normal.unwrap().0.4,"succeeded");
        }
        println!("E4D real_node fault={fault} repeats=5 rejected=5 recovery_success=5 child_reaped=10 failures=0");
    }
    std::fs::remove_dir_all(temp).unwrap(); // Only this test's newly created fixture directory.
}
