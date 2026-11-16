import { activeRun, inputProblem, type View } from './agent-runtime.ts';

// Safari may end composition immediately before the Enter keydown.
export function submitKey(event: { key: string; shiftKey: boolean; isComposing: boolean; keyCode: number; repeat: boolean }, composing: boolean, sinceCompositionEnd: number) {
  return event.key === 'Enter' && !event.shiftKey && !event.isComposing && !composing
    && event.keyCode !== 229 && !event.repeat && sinceCompositionEnd > 50;
}
export function canSubmit(view: View | null, connection: string, busy: boolean, text: string) {
  return connection === 'connected' && !!view && !view[0][6] && !busy && !activeRun(view) && !inputProblem(text);
}
const states: Record<string, string> = {
  Queued: 'queued · 실행 대기', Running: 'running · 로컬 생성 중',
  CancelRequested: 'cancel_requested · 취소 요청됨', Succeeded: 'succeeded · 완료',
  Cancelled: 'cancelled · 취소됨', Failed: 'failed · 실패',
};
const errors: Record<string, string> = {
  RuntimeLost: '화면을 Reload하여 실행이 중단되었습니다. 자동 재전송하지 않습니다.',
  RunTimeout: '실행 제한 시간을 초과했습니다.', IdleTimeout: '출력 대기 시간을 초과했습니다.',
  CleanupUnconfirmed: '정리 제한 시간을 초과했습니다. 마지막 Cleanup 상태를 확인하세요.',
  ProtocolError: '응답 순서 또는 형식이 계약에 맞지 않습니다.',
  UnsupportedContent: '지원하지 않는 응답 형식입니다.',
  OutputLimit: '출력 한도를 초과했습니다.', InputLimit: '입력 한도를 초과했습니다.',
  BufferLimit: '응답 버퍼 한도를 초과했습니다.', InvalidInput: '입력 형식이 유효하지 않습니다.',
  Cancelled: '사용자가 실행을 취소했습니다.',
};
export function displayState(view: View | null) {
  const run = view?.[1];
  if (!run) return '실행 없음';
  if (run[10] === 'Unconfirmed') return 'cleanup_unconfirmed · 정리 미확인';
  return states[run[5]] ?? '알 수 없는 Host 상태 · 실행 차단';
}
export function safeError(code: string) {
  // Do not echo arbitrary bridge errors, input or paths into the UI.
  return errors[code] ? `${code} · ${errors[code]}` : code ? 'Host 오류 · 마지막 상태를 확인하세요.' : '';
}
