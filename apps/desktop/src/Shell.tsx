import { useEffect, useRef, useState, type ReactNode } from 'react';
import { Core, coreStates, type CoreState } from './Core';
import { FakeRun } from './FakeRun';
const pages = [ ['main', 'Main', 'Overview', '◉'], ['command', 'Command Center', 'Operations', '⌘'], ['memory', 'Memory Center', 'Knowledge', '◇'], ['settings', 'Settings', 'Preferences', '⚙'] ] as const;
type Page = typeof pages[number][0];
export function Shell({ lit, onLight, nativeChecks }: { lit: boolean; onLight: () => void; nativeChecks: ReactNode }) {
  const [page, setPage] = useState<Page>('main');
  const [preview, setPreview] = useState<CoreState | ''>('');
  const [motion, setMotion] = useState(true);
  const [visible, setVisible] = useState(!document.hidden);
  const heading = useRef<HTMLHeadingElement>(null);
  useEffect(() => {
    const changed = () => setVisible(!document.hidden);
    document.addEventListener('visibilitychange', changed);
    return () => document.removeEventListener('visibilitychange', changed);
  }, []);
  function navigate(next: Page) { setPage(next); requestAnimationFrame(() => heading.current?.focus()); }
  const current = pages.find(item => item[0] === page)!;
  return <div className={`shell ${motion && visible && page === 'main' ? 'motion-on' : 'motion-off'}`}>
    <a className="skip-link" href="#workspace">본문으로 건너뛰기</a>
    <aside className="sidebar">
      <div className="brand"><span className="brand-mark">Λ</span><div>AETHER<small>PERSONAL INTELLIGENCE</small></div></div>
      <p className="nav-caption">WORKSPACE</p>
      <nav aria-label="주 탐색">{pages.map(([id, name, subtitle, icon]) => <button key={id} title={name} aria-label={name} aria-current={page === id ? 'page' : undefined} onClick={() => navigate(id)}><span className="nav-icon" aria-hidden="true">{icon}</span><span>{name}<small>{subtitle}</small></span></button>)}</nav>
      <div className="sidebar-bottom"><span className="status-light" />LOCAL SHELL<small>P1.1 / Development preview</small><p>당신의 공간, 새로운 가능성.</p></div>
    </aside>
    <main id="workspace" className="workspace" tabIndex={-1}>
      <header className="page-header"><div><p className="eyebrow">WORKSPACE / {current[2].toUpperCase()}</p><h1 ref={heading} tabIndex={-1}>{current[1]}</h1></div><span className="connection"><span className="status-light" /> AI 미연결</span></header>
      <section hidden={page !== 'main'} aria-label="Main">
        <div className="hero cinematic"><div className="hud-rail" aria-hidden="true"><span>CORE ENVIRONMENT</span><span>LOCAL VISUAL</span></div>
          <Core state={preview || 'offline'} lit={lit} preview={preview !== ''} />

        </div>
        <div className="input-dock"><span aria-hidden="true">＋</span><div>어떤 생각을 함께할까요?<small>텍스트·음성 입력은 준비 중입니다.</small></div><button disabled aria-label="음성 입력 준비 중">음성 · 준비 중</button></div>
        <div className="presence-rail"><span>AI 활동 기록 없음</span><span>기억 저장소 · 준비 중</span><button className="text-button" onClick={() => navigate('command')}>Developer Diagnostics ↗</button></div>
      </section>
      <section hidden={page !== 'command'} aria-label="Command Center">
        <div className="section-intro"><span className="badge">DEVELOPER DIAGNOSTICS</span><h2>실행의 흐름을 한눈에.</h2><p>아래 작업은 E4 Fake Runtime의 합성 진단입니다. 실제 AI나 컴퓨터 제어를 수행하지 않습니다.</p></div>
        <div className="panel runtime-panel"><div className="panel-title"><h2>Runtime Console</h2><span className="badge">FAKE PROVIDER</span></div><FakeRun /></div>
        <div className="panel core-diagnostics"><div className="panel-title"><h2>Core Diagnostics</h2><span className="badge">LOCAL VISUAL / PREVIEW</span></div><p>점등과 상태 선택은 로컬 시각 실험입니다. 실제 AI 상태를 변경하지 않습니다.</p><div className="core-controls"><button className="primary" onClick={onLight} disabled={lit}>{lit ? '✓ 코어 점등됨' : '◈ 코어 점등'}</button><label>상태 미리보기<select value={preview} onChange={e => setPreview(e.target.value as CoreState | '')}><option value="">실제 연결 상태</option>{Object.entries(coreStates).map(([key, [label]]) => <option key={key} value={key}>Preview · {key} / {label}</option>)}</select></label></div><p className="muted">선택한 시각 상태는 Main에서 Preview로 표시됩니다.</p></div>
        <div className="panel native-panel"><div className="panel-title"><h2>Native Diagnostics</h2><span className="badge">E3b · READ / APPROVE</span></div>{nativeChecks}</div>
      </section>
      <section hidden={page !== 'memory'} aria-label="Memory Center">
        <div className="section-intro"><span className="badge">LONG-TERM MEMORY / 준비 중</span><h2>기억이 연결되는 공간.</h2><p>앞으로 기억을 검색하고 관리할 공간입니다. 저장소는 아직 연결되지 않았습니다.</p></div>
        <label className="search-label">기억 검색<input disabled placeholder="검색 기능 준비 중" type="search" /></label>
        <div className="panel empty-state"><span className="empty-symbol" aria-hidden="true">◇</span><h2>아직 저장된 기억이 없어요.</h2><p>기억을 생성하거나 불러오지 않았습니다.<br />연결 후 저장된 정보가 이곳에 표시됩니다.</p><button disabled>기억 추가 · 준비 중</button></div>
        <p className="footnote">데이터 수집 없음 · 검색 / 편집 / 삭제 기능 준비 중</p>
      </section>
      <section hidden={page !== 'settings'} aria-label="Settings">
        <div className="section-intro"><span className="badge">YOUR ENVIRONMENT</span><h2>당신에게 맞는 AETHER.</h2><p>현재 적용되는 UI 설정과 준비 중인 기능을 구분합니다.</p></div>
        <div className="panel settings-panel"><h2>AI Provider</h2><div className="setting-row"><div><h3>연결된 Provider 없음</h3><p>모델 선택과 API Key 연결 준비 중</p></div><button disabled>연결 · 준비 중</button></div></div>
        <div className="panel settings-panel"><h2>권한과 시스템</h2><div className="setting-row"><div><h3>마이크 · 미확인</h3><p>음성 기능 미연결 · 권한을 요청하지 않습니다.</p></div><span className="badge">준비 중</span></div><div className="setting-row"><div><h3>손쉬운 사용</h3><p>현재 권한은 Native Diagnostics에서 직접 조회합니다.</p></div><button onClick={() => navigate('command')}>상태 조회로 이동</button></div></div>
        <div className="panel settings-panel"><h2>Appearance & Motion</h2><div className="setting-row"><div><h3>Deep Space</h3><p>기본 어두운 테마 · 다른 테마 준비 중</p></div><span className="theme-swatch" aria-label="남청색 테마" /></div><div className="setting-row"><div><h3>은은한 Core 움직임</h3><p>현재 세션에 적용 · 시스템 동작 줄이기 우선</p></div><label className="motion-toggle"><input type="checkbox" checked={motion} onChange={e => setMotion(e.target.checked)} />{motion ? '켜짐' : '꺼짐'}</label></div></div>
      </section>
      <footer><span>UI SHELL · DEVELOPMENT PREVIEW</span><span>LOCAL FIRST · P1.1</span></footer>
    </main>
  </div>;
}
