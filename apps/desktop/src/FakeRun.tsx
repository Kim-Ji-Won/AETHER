import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { FIXTURE_INPUT, runFakeProvider } from './fake-provider';

type RunState = 'created' | 'running' | 'cancel_requested' | 'cancelled' | 'succeeded' | 'failed';
type Variant = 'webview-a' | 'node-b';
type Mode = 'normal' | 'pause25' | 'long60';
type Trace = [seq: number, type: string, value: string][];
type WireSnapshot = [number, number, number, number, RunState, number, number, number, string, Trace, Variant, Mode];
type Snapshot = {
  version: number; run_id: number; request_id: number; generation: number;
  state: RunState; seq: number; progress: number; terminalCount: number; result: string; trace: Trace;
  variant: Variant; mode: Mode;
};
function decode(wire: WireSnapshot): Snapshot {
  const [version, run_id, request_id, generation, state, seq, progress, terminalCount, result, trace, variant, mode] = wire;
  return { version, run_id, request_id, generation, state, seq, progress, terminalCount, result, trace, variant, mode };
}

export function FakeRun() {
  const [run, setRun] = useState<Snapshot | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState(false);
  const [session, setSession] = useState<[string, number, boolean] | null>(null);
  const [long60, setLong60] = useState(false);
  const [pause25, setPause25] = useState(false);
  const [childResult, setChildResult] = useState<string | null>(null);
  const inFlight = useRef(false);
  const controller = useRef<AbortController | null>(null);

  useEffect(() => {
    let mounted = true; let refreshing = false;
    invoke<[string, number, boolean]>('get_runtime_session').then(value => { if (mounted) setSession(value); }).catch(() => setError(true));
    const refresh = async () => {
      if (refreshing) return;
      refreshing = true;
      try {
        const wire = await invoke<WireSnapshot | null>('get_fake_run');
        if (mounted && wire) reflect(decode(wire));
      } catch { if (mounted) setError(true); }
      finally { refreshing = false; }
    };
    void refresh();
    const timer = setInterval(refresh, 100);
    return () => { mounted = false; clearInterval(timer); controller.current?.abort(); };
  }, []);
  function envelope(payload: object) {
    if (!session || !session[2]) throw new Error('Session unavailable');
    return { instance: session[0], epoch: session[1], payload };
  }
  const active = busy || (run !== null && ['created', 'running', 'cancel_requested'].includes(run.state));

  function reflect(snapshot: Snapshot) {
    setRun(previous => {
      if (previous && previous.run_id > snapshot.run_id) return previous;
      if (previous?.run_id === snapshot.run_id && (previous.seq > snapshot.seq
        || (previous.state === 'cancel_requested' && ['created', 'running'].includes(snapshot.state)))) return previous;
      return snapshot;
    });
  }
  async function latest() {
    const wire = await invoke<WireSnapshot | null>('get_fake_run');
    const snapshot = wire ? decode(wire) : null;
    if (snapshot) reflect(snapshot);
    return snapshot;
  }
  async function cancel() {
    if (!run) return;
    try {
      const snapshot = decode(await invoke<WireSnapshot>('cancel_fake_run', envelope({ run_id: run.run_id, variant: run.variant })));
      reflect(snapshot);
      if (['cancel_requested', 'cancelled'].includes(snapshot.state)) controller.current?.abort();
    } catch {
      setError(true);
      try { await latest(); } catch { /* Keep the last host-confirmed state. */ }
    }
  }
  async function start(variant: Variant) {
    if (inFlight.current) return;
    inFlight.current = true;
    setBusy(true); setError(false); setChildResult(null); setRun(null);
    const abort = new AbortController(); controller.current = abort;
    let current: Snapshot | null = null;
    let polling = false;
    let reading = false;
    let timer: ReturnType<typeof setInterval> | undefined;
    try {
      const previous = await invoke<WireSnapshot | null>('get_fake_run');
      const token = previous ? previous[2] + 1 : 1;
      const mode: Mode = pause25 ? 'pause25' : long60 ? 'long60' : 'normal';
      if (variant === 'webview-a') {
        current = decode(await invoke<WireSnapshot>('create_fake_run', envelope({ token, mode })));
        reflect(current);
        await runFakeProvider(FIXTURE_INPUT, async event => {
          const { version, run_id, request_id, generation, seq } = current!;
          current = decode(await invoke<WireSnapshot>('submit_fake_event', envelope({
            version, run_id, request_id, generation, seq: seq + 1, ...event,
          })));
          reflect(current);
        }, long60 && !pause25 ? 15_000 : 100, { signal: abort.signal, pause25 });
        const final = await latest();
        if (!final || final.run_id !== current!.run_id || final.state !== 'succeeded'
          || final.result !== 'FAKE_OK' || final.terminalCount !== 1) throw new Error('Host mismatch');
      } else {
        polling = true;
        timer = setInterval(async () => {
          if (reading) return;
          reading = true;
          try {
            const wire = await invoke<WireSnapshot | null>('get_fake_run');
            if (polling && wire && wire[2] === token && wire[10] === variant) reflect(decode(wire));
          } catch { /* The start command reports errors. */ }
          finally { reading = false; }
        }, 50);
        const [wire, , exit] = await invoke<[WireSnapshot, number, number]>('start_node_fake_run', envelope({ token, mode }));
        polling = false; current = decode(wire); reflect(current);
        if (current.state === 'cancelled') setChildResult('Node 중단 · reaped');
        else if (exit === 0 && current.state === 'succeeded' && current.terminalCount === 1) setChildResult('Node exit: 0 · reaped');
        else throw new Error('Host mismatch');
      }
    } catch {
      polling = false;
      setError(true);
      try {
        const snapshot = await latest();
        if (snapshot && ['cancelled', 'cancel_requested'].includes(snapshot.state)) setError(false);
        else if (variant === 'webview-a' && current?.run_id === snapshot?.run_id
          && snapshot && ['created', 'running'].includes(snapshot.state)) {
          const { version, run_id, request_id, generation, seq } = snapshot;
          reflect(decode(await invoke<WireSnapshot>('submit_fake_event', envelope({
            version, run_id, request_id, generation, seq: seq + 1, type: 'failed', value: '',
          }))));
        }
      } catch { /* No invented terminal state if IPC itself is unavailable. */ }
    } finally {
      polling = false; if (timer) clearInterval(timer);
      controller.current = null; inFlight.current = false; setBusy(false);
    }
  }
  return (
    <section className="fake-run" aria-label="Fake Provider 실험">
      <div className="fake-actions">
        <button type="button" disabled={active || !session} onClick={() => start('webview-a')}>WebView A 실행</button>
        <button type="button" disabled={active || !session} onClick={() => start('node-b')}>Node B 실행</button>
        <button type="button" disabled={!active || !run || !['created', 'running', 'cancel_requested'].includes(run.state)} onClick={cancel}>작업 취소</button>
      </div>
      <label><input type="checkbox" checked={pause25} disabled={active || !session} onChange={e => setPause25(e.target.checked)} />25%에서 대기 (취소 실험)</label>
      <label><input type="checkbox" checked={long60} disabled={active || pause25} onChange={e => setLong60(e.target.checked)} />60초 합성 작업</label>
      <p>메모리 전용 · 앱 재실행은 새 세션 · instance: {session?.[0] ?? '연결 중'} · context: {session?.[1] ?? '-'}</p>
      <div role="status" aria-live="polite">
        {run ? <>
          <p>{run.variant === 'node-b' ? 'Node B' : 'WebView A'} · run_id: {run.run_id} · {run.state} · {run.progress}%</p>
          <p>v{run.version} · request_id: {run.request_id} · generation: {run.generation} · seq: {run.seq}</p>
          <p>Rust 확인: {run.trace.map(([seq, type, value]) => `${seq}:${type}${value ? ` ${value}` : ''}`).join(' → ')}</p>
          <p>{run.result || '결과 대기'} · terminal: {run.terminalCount}{childResult ? ` · ${childResult}` : ''}</p>
          {run.state === 'running' && run.progress === 25 && run.mode === 'pause25' && <p>25%에서 대기 중이에요. 작업 취소를 눌러주세요.</p>}
          {run.state === 'cancel_requested' && <p>취소 요청을 확인했어요. 실행 정리 중이에요.</p>}
          {run.state === 'cancelled' && <p>취소했어요. 새 작업을 실행할 수 있어요.</p>}
          {run.state === 'failed' && <p>작업이 실패했어요. 새 작업으로 다시 시도할 수 있어요.</p>}
        </> : <p>Fake Provider · 실행 전 · 메모리 전용</p>}
        {error && <p>요청을 처리하지 못했어요. 마지막 Rust 확인 상태예요. 상태 확인 후 다시 시도해주세요.</p>}
      </div>
    </section>
  );
}
