import { useState } from 'react';
import { displayState, safeError } from './conversation-ui';
import type { Conversation } from './useAgentConversation';

export function ConversationResponse({ conversation }: { conversation: Conversation }) {
  const { view, submitted, connection, runtime, reconnect } = conversation;
  const [cancelling, setCancelling] = useState(false);
  const [notice, setNotice] = useState('');
  const run = view?.[1];
  if (!run && connection === 'connected') return null;
  const pending = run && run[10] === 'Pending';
  async function cancel() {
    if (cancelling) return;
    setCancelling(true); setNotice('');
    try { await runtime.cancel(); }
    catch { setNotice('취소 확인을 받지 못했습니다. 마지막 Host 상태를 확인하세요.'); }
    finally { setCancelling(false); }
  }
  return <section className="synthetic-response glass" aria-label="Synthetic Conversation 결과">
    <header><strong>SYNTHETIC / LOCAL TEST</strong><span>AI Provider 미연결</span></header>
    <div className="conversation-status" role="status" aria-live="polite">
      {connection !== 'connected' && <p>{connection} · 연결 확인 중 · 마지막 Host 상태이며 결과를 추정하지 않습니다.</p>}
      <p>{displayState(view)}{pending ? ' · Cleanup 확인 대기' : ''}</p>
      {run?.[10] === 'Unconfirmed' && <p>자원 정리 미확인 · 새 실행 차단</p>}
      {run?.[9] && <p>{safeError(run[9])}</p>}
      {notice && <p>{notice}</p>}
    </div>
    {run && <div className="conversation-output" tabIndex={0} aria-label="로컬 테스트 입력과 출력">
      {submitted?.run === run[0] && <p><span>입력 · </span>{submitted.text}</p>}
      <p><span>{run[5] === 'Succeeded' ? 'Synthetic 응답' : 'Partial Output · 미완료'} · </span>{run[8] || '(출력 없음)'}</p>
    </div>}
    <div className="conversation-actions">
      {run && ['Queued', 'Running', 'CancelRequested'].includes(run[5]) && <button disabled={connection !== 'connected' || cancelling || run[5] === 'CancelRequested'} onClick={() => void cancel()}>Cancel · 실행 취소</button>}
      {connection !== 'connected' && <button disabled={connection === 'reconnecting'} onClick={() => void reconnect()}>연결 다시 확인</button>}
    </div>
  </section>;
}
