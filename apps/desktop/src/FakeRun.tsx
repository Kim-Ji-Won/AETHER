import { useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { FIXTURE_INPUT, runFakeProvider } from './fake-provider';

type RunState = 'created' | 'running' | 'succeeded' | 'failed';
type Trace = [seq: number, type: string, value: string][];
// Native tuples avoid adding a direct serialization dependency for this experiment.
type WireSnapshot = [number, number, number, number, RunState, number, number, number, string, Trace];
type Snapshot = {
  version: number; run_id: number; request_id: number; generation: number;
  state: RunState; seq: number; progress: number; terminalCount: number; result: string; trace: Trace;
};
function decode(wire: WireSnapshot): Snapshot {
  const [version, run_id, request_id, generation, state, seq, progress, terminalCount, result, trace] = wire;
  return { version, run_id, request_id, generation, state, seq, progress, terminalCount, result, trace };
}

export function FakeRun() {
  const [run, setRun] = useState<Snapshot | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState(false);
  const inFlight = useRef(false);

  async function start() {
    if (inFlight.current) return;
    inFlight.current = true;
    setBusy(true);
    setError(false);
    let current: Snapshot | null = null;
    try {
      current = decode(await invoke<WireSnapshot>('create_fake_run'));
      setRun(current);
      await runFakeProvider(FIXTURE_INPUT, async event => {
        const { version, run_id, request_id, generation, seq } = current!;
        current = decode(await invoke<WireSnapshot>('submit_fake_event', {
          version, run_id, request_id, generation, seq: seq + 1, ...event,
        }));
        setRun(current);
      });
      // Independently re-read Rust's canonical snapshot; never infer completion from the timer.
      const confirmed = await invoke<WireSnapshot | null>('get_fake_run');
      if (!confirmed) throw new Error('Missing host snapshot');
      const final = decode(confirmed);
      if (final.run_id !== current!.run_id || final.seq !== current!.seq
        || final.state !== 'succeeded' || final.result !== 'FAKE_OK' || final.terminalCount !== 1) {
        throw new Error('Host snapshot mismatch');
      }
      setRun(final);
    } catch {
      // An IPC failure is not a fabricated host failure/success. Re-read before reporting it.
      setError(true);
      try {
        const latest = await invoke<WireSnapshot | null>('get_fake_run');
        if (latest) {
          const snapshot = decode(latest);
          if (current && snapshot.run_id === current.run_id && ['created', 'running'].includes(snapshot.state)) {
            const { version, run_id, request_id, generation, seq } = snapshot;
            setRun(decode(await invoke<WireSnapshot>('submit_fake_event', {
              version, run_id, request_id, generation, seq: seq + 1, type: 'failed', value: '',
            })));
          } else setRun(snapshot);
        }
      } catch { /* Keep the last confirmed state. Do not claim that Rust changed. */ }
    } finally {
      inFlight.current = false;
      setBusy(false);
    }
  }

  return (
    <section className="fake-run" aria-label="Fake Provider 실험">
      <button type="button" disabled={busy} onClick={start}>Fake Run 실행</button>
      <div role="status" aria-live="polite">
        {run ? <>
          <p>run_id: {run.run_id} · {run.state === 'running' ? '실행 중 · ' : ''}{run.state}
            {run.state === 'succeeded' ? ' · 완료' : ''} · {run.progress}%</p>
          <p>v{run.version} · request_id: {run.request_id} · generation: {run.generation} · seq: {run.seq}</p>
          <p>Rust 확인: {run.trace.map(([seq, type, value]) => `${seq}:${type}${value ? ` ${value}` : ''}`).join(' → ')}</p>
          <p>{run.result || '결과 대기'} · terminal: {run.terminalCount}</p>
        </> : <p>Fake Provider · 실행 전 · 메모리 전용</p>}
        {error && <p>작업 상태를 확인하지 못했어요. 마지막 Rust 확인 상태입니다. 다시 시도할 수 있어요.</p>}
      </div>
    </section>
  );
}
