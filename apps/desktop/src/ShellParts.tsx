import { useLayoutEffect, useRef, useState } from 'react';

export const pages = [
  ['main', 'Main', 'Overview'], ['command', 'Command Center', 'Operations'],
  ['memory', 'Memory Center', 'Knowledge'], ['settings', 'Settings', 'Preferences'],
] as const;
export type Page = typeof pages[number][0];
type IconName = Page | 'plus' | 'mic' | 'send' | 'context';
const paths: Record<IconName, string> = {
  main: 'm3 10 9-7 9 7v10h-6v-7H9v7H3Z',
  command: 'm3 7 9-4 9 4-9 4Zm0 5 9 4 9-4M3 17l9 4 9-4',
  memory: 'M8 4H5v16h14V4h-3M8 3h8v5H8ZM8 12h8M8 16h5',
  settings: 'M4 6h16M4 12h16M4 18h16M8 3v6M16 9v6M10 15v6',
  plus: 'M12 5v14M5 12h14', mic: 'M9 5a3 3 0 0 1 6 0v7a3 3 0 0 1-6 0ZM6 11v1a6 6 0 0 0 12 0v-1M12 18v4M8 22h8',
  send: 'm3 3 18 9-18 9 4-9Zm4 9h14', context: 'M4 4h16v16H4ZM14 4v16',
};
export function Icon({ name }: { name: IconName }) {
  return <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d={paths[name]} /></svg>;
}
export function Navigation({ page, navigate }: { page: Page; navigate: (page: Page) => void }) {
  const [expanded, setExpanded] = useState(false);
  const toggle = useRef<HTMLButtonElement>(null);
  return <aside className={`sidebar ${expanded ? 'nav-expanded' : ''}`} onKeyDown={e => { if (e.key === 'Escape') { setExpanded(false); toggle.current?.focus(); } }}>
    <div className="brand"><span className="brand-mark" aria-hidden="true">Λ</span><span className="brand-name">AETHER<small>PERSONAL INTELLIGENCE</small></span></div>
    <button ref={toggle} className="nav-toggle" aria-label={expanded ? '탐색 이름 접기' : '탐색 이름 펼치기'} aria-expanded={expanded} onClick={e => { setExpanded(!expanded); e.currentTarget.focus(); }}>☰</button>
    <nav aria-label="주 탐색">{pages.map(([id, name]) => <button key={id} aria-label={name} title={name} aria-current={page === id ? 'page' : undefined} onClick={() => { navigate(id); setExpanded(false); }}><Icon name={id} /><span className="nav-name">{name}</span><span className="nav-tooltip" aria-hidden="true">{name}</span></button>)}</nav>
    <div className="sidebar-bottom"><span className="status-dot" /> <span className="nav-name">LOCAL SHELL<small>P1.1 / Preview</small></span></div>
  </aside>;
}
export function StatusArea({ close }: { close: () => void }) {
  const dismiss = useRef<HTMLButtonElement>(null);
  useLayoutEffect(() => { dismiss.current?.focus(); }, []);
  return <aside id="status-panel" className="status-area glass" aria-label="실제 연결 상태">
    <header className="panel-heading"><h2>연결 상태</h2><button ref={dismiss} onClick={close}>닫기</button></header>
    <div className="status-primary"><span className="status-dot" /><span>AI Provider<strong>연결 안 됨</strong></span></div>
    <div><Icon name="memory" /><span>기억 저장소<strong>미연결</strong></span></div>
    <div><Icon name="mic" /><span>음성 입력<strong>준비 중</strong></span></div>
    <p className="status-footnote">현재는 로컬 UI입니다.</p>
  </aside>;
}
export function ContextPanel({ open, close, navigate }: { open: boolean; close: () => void; navigate: (page: Page) => void }) {
  const dismiss = useRef<HTMLButtonElement>(null);
  useLayoutEffect(() => { if (open) dismiss.current?.focus(); }, [open]);
  if (!open) return null;
  return <aside id="context-panel" className="context-panel glass" aria-label="Context">
    <div className="panel-heading"><h2>Context</h2><button ref={dismiss} onClick={close}>닫기</button></div>
    <div className="context-item"><Icon name="main" /><div>대화 미연결<small>AI Provider 연결 준비 중</small></div></div>
    <div className="context-item"><Icon name="memory" /><div>기억 저장소 미연결<small>데이터 확인 불가</small></div></div>
    <div className="context-item"><Icon name="command" /><div>작업 기록 미지원<small>Fake Runtime 결과는 Command Center에서 확인</small></div></div>
    <button className="text-button" onClick={() => navigate('settings')}>연결 설정 보기 ↗</button>
  </aside>;
}
export function CommandDock({ draft, setDraft }: { draft: string; setDraft: (text: string) => void }) {
  const input = useRef<HTMLTextAreaElement>(null);
  useLayoutEffect(() => {
    const field = input.current;
    if (!field) return;
    const resize = () => {
      field.style.height = '0px';
      const fullHeight = field.scrollHeight + 2;
      field.style.height = `${Math.min(100, Math.max(40, fullHeight))}px`;
      field.style.overflowY = fullHeight > 100 ? 'auto' : 'hidden';
    };
    resize();
    let width = field.clientWidth;
    const observer = new ResizeObserver(() => {
      if (field.clientWidth !== width) { width = field.clientWidth; resize(); }
    });
    observer.observe(field);
    return () => observer.disconnect();
  }, [draft]);
  return <div className="command-dock-wrap">
    <div className="command-dock glass" role="group" aria-label="Command Dock">
      <button disabled aria-label="첨부 준비 중" title="첨부 준비 중"><Icon name="plus" /></button>
      <label className="draft-field"><span className="sr-only">메시지 초안</span><textarea ref={input} rows={1} maxLength={4000} value={draft} onChange={e => setDraft(e.target.value)} placeholder="메시지 초안 입력" aria-describedby="draft-notice" /></label>
      <button disabled aria-label="음성 입력 준비 중" title="음성 입력 준비 중"><Icon name="mic" /></button>
      <button className="send-button" disabled aria-label="전송 준비 중 · AI 미연결" title="AI Provider 연결 준비 중"><Icon name="send" /></button>
    </div>
    <p id="draft-notice" className="dock-notice">로컬 초안 · 전송 및 음성 기능 준비 중 · 앱 종료 시 사라집니다.</p>
  </div>;
}
