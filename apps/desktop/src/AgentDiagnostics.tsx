import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { AgentRuntime, type Mode, type View } from './agent-runtime';

export function AgentDiagnostics() {
  const client = useRef<AgentRuntime | null>(null);
  if (!client.current) client.current = new AgentRuntime((command, payload) => invoke<View>(command, payload));
  const [view, setView] = useState<View | null>(null);
  const [report, setReport] = useState('연결 중');
  const [oldInstance, setOldInstance] = useState('');
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    const runtime = client.current!;
    let mounted = true;
    runtime.onView = value => { if (mounted) setView(value); };
    void runtime.bootstrap().then(() => { if (mounted) setReport('Synthetic 전용 · External Provider 미연결'); })
      .catch(() => { if (mounted) setReport('Bootstrap 거부 · 재전송 없음'); });
    const timer = setInterval(() => { void runtime.refresh().catch(() => {}); }, 100);
    return () => { mounted = false; clearInterval(timer); };
  }, []);
  async function act(action: () => Promise<unknown>, success: string) {
    setBusy(true);
    try { const result = await action(); setReport(typeof result === 'string' ? result : success); }
    catch (error) {
      // Never expose arbitrary transport error details, URLs, paths, or input text.
      const safe = ['DuplicateRequest', 'AlreadyTerminal', 'InvalidIdentity', 'Busy', 'StaleSession'];
      setReport(safe.includes(String(error)) ? `Host 거부: ${String(error)}` : '요청 거부 · 마지막 Host 상태 확인');
    } finally { setBusy(false); }
  }
  const active = !!view?.[1] && (!['Succeeded', 'Failed', 'Cancelled'].includes(view[1][5]) || !['Confirmed', 'NotRequired'].includes(view[1][10]));
  const modes: [Mode, string][] = [['normal', 'Synthetic 실행'], ['queued', 'Queued 대기'], ['idle', 'Idle Timeout'],
    ['run-timeout', 'Run Timeout'], ['cleanup-timeout', 'Cleanup ACK 누락']];
  return <section aria-label="Agent Text Diagnostics">
    <p>agent.text.v1 · WKWebView foreground · Synthetic 응답이며 실제 AI가 아닙니다.</p>
    <div className="fake-actions">{modes.map(([mode, label]) => <button key={mode} disabled={!view || active || busy}
      onClick={() => act(() => client.current!.start(mode), '요청 전송 · Host 상태 관찰 중')}>{label}</button>)}</div>
    <div className="fake-actions">
      <button disabled={!view?.[1] || busy} onClick={() => act(() => client.current!.cancel(), '취소 요청 처리 · Host 상태 확인')}>Agent 취소</button>
      <button disabled={!view?.[1] || busy} onClick={() => act(() => client.current!.duplicateStart(), '중복 Start가 수락됨 · 검사 실패')}>Start 재전송 검사</button>
      <button disabled={!view?.[1] || busy} onClick={() => act(() => client.current!.release(), '정지된 producer의 Cleanup ACK 전송')}>늦은 Cleanup ACK</button>
      <button disabled={!view || active || busy} onClick={() => act(async () => {
        const results = await client.current!.probes(); return results.join(' · ');
      }, 'IPC 계약 검사 완료 · 세부 결과는 아래 상태와 테스트 기록 참조')}>IPC 계약 검사</button>
    </div>
    <label>이전 앱 Instance (재시작 거부 검사)<input value={oldInstance} maxLength={64} onChange={e => setOldInstance(e.target.value)} /></label>
    <button disabled={!view || !oldInstance || busy} onClick={() => act(async () => {
      if (oldInstance === view?.[0][0] || !/^[a-f0-9]{32}$/.test(oldInstance)) throw new Error('Invalid fixture');
      try { await client.current!.oldSession(oldInstance); } catch (error) {
        if (String(error) === 'Stale or closing lifecycle session') return '이전 Session 거부: PASS (Lifecycle Gate)';
        throw error;
      }
      throw new Error('Old session accepted');
    }, '이전 Session 거부 검사 완료')}>이전 Session 검사</button>
    <div role="status" aria-live="polite">
      <p>{report}</p>
      <p>Host instance: {view?.[0][0] ?? '-'} · UI epoch: {view?.[0][1] ?? '-'} → {view?.[0][2] ?? '-'} · generation: {view?.[0][3] ?? '-'} · revision: {view?.[0][4] ?? '-'} · closing: {String(view?.[0][6] ?? false)}</p>
      {view?.[1] ? <>
        <p>Agent run {view[1][0]} · {view[1][5]} · seq {view[1][6]} · terminal {view[1][7]} · cleanup {view[1][10]} · error {view[1][9] || '없음'}</p>
        <pre aria-label="Synthetic Text 결과">{view[1][8] || '(출력 없음)'}</pre>
      </> : <p>Agent Run 없음 · 메모리 전용 · 자동 복원/재시도 없음</p>}
    </div>
  </section>;
}
