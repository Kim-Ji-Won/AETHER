//! Fixed E4c bundle entry point. No caller-selected executable, script, args or environment.
use std::{io::{BufRead, BufReader, Read, Write}, path::{Path, PathBuf},
    process::{Child, Command, Stdio}, sync::mpsc, time::{Duration, Instant, SystemTime, UNIX_EPOCH}};
use tauri::ipc::{InvokeBody, InvokeResponseBody};
use crate::fake_run::{self, Frame, Snapshot};

const MAX_FRAME: usize = 512;
const TIMEOUT: Duration = Duration::from_secs(5);
type Reply = Result<(Snapshot, u32, i32), String>;

struct ChildGuard(Child);
impl Drop for ChildGuard {
    fn drop(&mut self) {
        if !matches!(self.0.try_wait(), Ok(Some(_))) { let _ = self.0.kill(); }
        let _ = self.0.wait(); // Always reap only our child, including on protocol errors.
    }
}

fn resource_file(dir: &Path, name: &str) -> Result<PathBuf, String> {
    let path = dir.join(name).canonicalize().map_err(|_| "Missing bundled sidecar file")?;
    if path.parent() != Some(dir) || !path.is_file() { return Err("Sidecar path escaped bundle".into()); }
    Ok(path)
}

fn read_frame(reader: &mut impl BufRead) -> Result<Option<String>, String> {
    let mut bytes = Vec::with_capacity(MAX_FRAME);
    (&mut *reader).take((MAX_FRAME + 1) as u64).read_until(b'\n', &mut bytes)
        .map_err(|_| "Sidecar read failed")?;
    if bytes.is_empty() { return Ok(None); }
    if bytes.len() > MAX_FRAME || bytes.last() != Some(&b'\n') { return Err("Invalid sidecar frame length".into()); }
    String::from_utf8(bytes).map(Some).map_err(|_| "Invalid sidecar UTF-8".into())
}

fn body(line: String) -> Result<InvokeBody, String> {
    Ok(InvokeBody::Json(InvokeResponseBody::Json(line).deserialize().map_err(|_| "Invalid sidecar JSON")?))
}

fn write_frame(writer: &mut impl Write, line: &str) -> Result<(), String> {
    if line.len() > MAX_FRAME { return Err("Host frame too large".into()); }
    writer.write_all(line.as_bytes()).and_then(|_| writer.flush()).map_err(|_| "Sidecar write failed".into())
}

pub fn run(resource_dir: PathBuf) -> Reply {
    // resource_dir comes only from Tauri's app resolver, never from IPC arguments.
    let resources = resource_dir.canonicalize().map_err(|_| "Missing bundle resources")?;
    let dir = resources.join("sidecar").canonicalize().map_err(|_| "Missing bundled sidecar")?;
    if dir.parent() != Some(resources.as_path()) { return Err("Sidecar directory escaped bundle".into()); }
    let node = resource_file(&dir, "node")?;
    let entry = resource_file(&dir, "entry.mjs")?;
    resource_file(&dir, "fake-provider.js")?;
    resource_file(&dir, "LICENSE")?;
    let created = fake_run::create_node()?;
    let result = exchange(&node, &entry, &dir, &created);
    fake_run::finish_node(created.1);
    result
}

fn exchange(node: &Path, entry: &Path, cwd: &Path, created: &Snapshot) -> Reply {
    let child = Command::new(node).arg(entry).current_dir(cwd).env_clear()
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped())
        .spawn().map_err(|_| "Bundled Node spawn failed")?;
    let mut child = ChildGuard(child);
    let pid = child.0.id();
    let started = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|_| "Clock unavailable")?.as_millis();
    eprintln!("AETHER_SIDECAR pid={pid} parent={} started_unix_ms={started} executable=Contents/Resources/sidecar/node run_id={}", std::process::id(), created.1);
    let mut stdin = child.0.stdin.take().ok_or("Missing child stdin")?;
    let stdout = child.0.stdout.take().ok_or("Missing child stdout")?;
    let stderr = child.0.stderr.take().ok_or("Missing child stderr")?;
    let (tx, rx) = mpsc::channel();
    let output = std::thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        // ready + five events + done + EOF. Queue and each frame are bounded.
        for _ in 0..8 {
            let line = read_frame(&mut reader);
            let stop = !matches!(line, Ok(Some(_)));
            if tx.send(line).is_err() || stop { return; }
        }
    });
    let diagnostics = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stderr.take(4097).read_to_end(&mut bytes).map(|_| bytes.len())
    });
    let receive = || rx.recv_timeout(TIMEOUT).map_err(|_| "Sidecar response timeout/disconnect".to_string())?
        .and_then(|line| line.ok_or("Unexpected sidecar EOF".into()));

    let result = (|| -> Reply {
        write_frame(&mut stdin, &format!("{{\"version\":1,\"type\":\"begin\",\"run_id\":{},\"request_id\":{},\"generation\":{},\"input\":\"테스트 질문\"}}\n", created.1, created.2, created.3))?;
        let ready = body(receive()?)?;
        let InvokeBody::Json(ready) = ready else { unreachable!() };
        if ready.as_object().map(|o| o.len()) != Some(5) || ready["version"] != 1 || ready["type"] != "ready"
            || ready["pid"].as_u64() != Some(pid as u64) || ready["node_version"] != "v22.21.0" || ready["arch"] != "arm64" {
            return Err("Invalid Node handshake".into());
        }
        let mut terminal = None;
        for seq in 1..=5 {
            let frame = Frame::parse(&body(receive()?)?)?;
            if seq == 5 { fake_run::validate_node_terminal(&frame)?; terminal = Some(frame); }
            else { fake_run::submit_node(frame)?; }
            write_frame(&mut stdin, &format!("{{\"version\":1,\"type\":\"ack\",\"run_id\":{},\"seq\":{seq}}}\n", created.1))?;
        }
        let done = body(receive()?)?;
        let InvokeBody::Json(done) = done else { unreachable!() };
        if done.as_object().map(|o| o.len()) != Some(4) || done["version"] != 1 || done["type"] != "done"
            || done["run_id"].as_u64() != Some(created.1 as u64) || done["seq"] != 5 {
            return Err("Invalid Node completion".into());
        }
        if rx.recv_timeout(TIMEOUT).map_err(|_| "Sidecar EOF timeout")??.is_some() {
            return Err("Unexpected trailing sidecar output".into());
        }
        let deadline = Instant::now() + TIMEOUT;
        let status = loop {
            if let Some(status) = child.0.try_wait().map_err(|_| "Child wait failed")? { break status; }
            if Instant::now() >= deadline { return Err("Child exit timeout".into()); }
            std::thread::sleep(Duration::from_millis(10));
        };
        if !status.success() { return Err("Bundled Node exited unsuccessfully".into()); }
        let run = fake_run::submit_node(terminal.ok_or("Missing terminal")?)?;
        eprintln!("AETHER_SIDECAR pid={pid} exit=0 reaped=true run_id={}", created.1);
        Ok((run, pid, 0))
    })();
    if result.is_err() {
        let _ = write_frame(&mut stdin, "{\"version\":1,\"type\":\"error\",\"code\":\"PROTOCOL_REJECTED\"}\n");
    }
    drop(stdin);
    // Close/kill/wait before joining pipe readers on any failed exchange.
    drop(child);
    let _ = output.join();
    let stderr_bytes = diagnostics.join().ok().and_then(Result::ok).unwrap_or(4097);
    if result.is_err() {
        eprintln!("AETHER_SIDECAR pid={pid} failed=true reaped=true stderr_bytes={stderr_bytes}");
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn newline_frame_is_bounded() {
        assert_eq!(read_frame(&mut &b"{}\n"[..]).unwrap(), Some("{}\n".into()));
        assert!(read_frame(&mut &b"{}"[..]).is_err());
        assert!(read_frame(&mut vec![b'x'; MAX_FRAME + 1].as_slice()).is_err());
    }
    #[test]
    fn real_pinned_node_roundtrip_uses_host_ledger_once() {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sidecar/generated").canonicalize().unwrap();
        let created = fake_run::create_node().unwrap();
        // The WebView transport cannot inject into a B-owned run.
        let injected = format!("{{\"version\":1,\"run_id\":{},\"request_id\":{},\"generation\":1,\"seq\":1,\"type\":\"start\",\"value\":\"\"}}", created.1, created.2);
        assert!(fake_run::submit(Frame::parse(&body(injected).unwrap()).unwrap()).is_err());
        let (run, _, exit) = exchange(&dir.join("node"), &dir.join("entry.mjs"), &dir, &created).unwrap();
        assert_eq!((run.4, run.5, run.6, run.7, run.8, run.10, exit), ("succeeded", 5, 75, 1, "FAKE_OK", "node-b", 0));
        assert_eq!(run.9, vec![(1,"start",""), (2,"progress","25"), (3,"progress","50"), (4,"progress","75"), (5,"terminal","FAKE_OK")]);
        fake_run::finish_node(created.1);
    }
}
