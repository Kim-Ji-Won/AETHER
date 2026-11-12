import { useLayoutEffect, useRef, useState, type ReactNode } from 'react';
import { Core, coreStates, type CoreState } from './Core';
import { FakeRun } from './FakeRun';
import { CommandDock, ContextPanel, Icon, Navigation, pages, StatusArea, type Page } from './ShellParts';
import { useMotion } from './useMotion';

export function Shell({ lit, onLight, nativeChecks }: { lit: boolean; onLight: () => void; nativeChecks: ReactNode }) {
  const [page, setPage] = useState<Page>('main');
  const [preview, setPreview] = useState<CoreState | ''>('');
  const [motion, setMotion] = useState(true);
  const [draft, setDraft] = useState('');
  const [legacyPreview, setLegacyPreview] = useState(false);
  const [context, setContext] = useState(false);
  const heading = useRef<HTMLHeadingElement>(null);
  const firstRender = useRef(true);
  const policy = useMotion(motion, page === 'command' && legacyPreview);
  useLayoutEffect(() => {
    if (firstRender.current) { firstRender.current = false; return; }
    heading.current?.focus();
  }, [page]);
  function navigate(next: Page) { setPage(next); setContext(false); }
  const current = pages.find(item => item[0] === page)!;
  return <div className={`shell ${policy.active ? 'motion-on' : 'motion-off'}`}>
    <a className="skip-link" href="#workspace">본문으로 건너뛰기</a>
    <Navigation page={page} navigate={navigate} />
    <main id="workspace" className="workspace" tabIndex={-1}>
      <header className="page-header"><div><p className="eyebrow">WORKSPACE / {current[2].toUpperCase()}</p><h1 ref={heading} tabIndex={-1}>{current[1]}</h1></div>
        <div className="header-actions"><span className="connection"><span className="status-dot" /> AI 미연결</span>{page === 'main' && <button className="context-toggle" aria-expanded={context} aria-controls={context ? 'context-panel' : undefined} onClick={() => setContext(!context)}><Icon name="context" /><span>Context</span></button>}</div>
      </header>
      {/* Mount visible product pages so WebKit rebuilds their accessibility subtree.
          Runtime Console stays last and mounted: navigation must not abort a Fake Run.
          Keeping hidden runtime content after the active page avoids WebKit AX sibling truncation. */}
      {page === 'main' && <section className="main-page" aria-label="Main">
        <div className="main-intro"><p className="eyebrow">PERSONAL INTELLIGENCE</p><h2>당신의 생각이 시작되는 곳.</h2></div>
        <div className="main-stage">
          <StatusArea />
          <section className="scene-viewport" aria-label="향후 3D AI Core 배치 영역">
            <div className="scene-reserve"><span className="eyebrow">AETHER CORE</span><h2>Scene reserved.</h2><p>실제 3D Core는 다음 단계에서 연결됩니다.</p><span className="badge">P1.2 · 미구현</span>
              {preview && <p className="preview-status" role="status">Preview · {preview} / {coreStates[preview][0]}</p>}
            </div>
            <span className="scene-boundary">ORBITAL INTELLIGENCE <span>+ NEURAL PLASMA</span></span>
          </section>
          <ContextPanel open={context} close={() => { setContext(false); document.querySelector<HTMLButtonElement>('.context-toggle')?.focus(); }} navigate={navigate} />
        </div>
        <CommandDock draft={draft} setDraft={setDraft} />
      </section>}
      {page === 'memory' && <section className="page-scroll" aria-label="Memory Center">
        <div className="section-intro"><span className="badge">LONG-TERM MEMORY</span><h2>기억이 연결되는 공간.</h2><p>기억을 검색하고 관리할 공간입니다. 저장소는 아직 연결되지 않았습니다.</p></div>
        <label className="search-label">기억 검색<input disabled placeholder="검색 기능 준비 중" type="search" /></label>
        <div className="panel empty-state"><Icon name="memory" /><h2>아직 저장된 기억이 없어요.</h2><p>기억을 생성하거나 불러오지 않았습니다.<br />연결 후 저장된 정보가 이곳에 표시됩니다.</p><button disabled>기억 추가 · 준비 중</button></div>
        <p className="footnote">데이터 수집 없음 · 검색 / 편집 / 삭제 기능 준비 중</p>
      </section>}
      {page === 'settings' && <section className="page-scroll" aria-label="Settings">
        <div className="section-intro"><span className="badge">YOUR ENVIRONMENT</span><h2>당신에게 맞는 환경.</h2><p>현재 적용되는 UI 설정과 준비 중인 기능을 구분합니다.</p></div>
        <div className="panel"><h2>AI Provider</h2><div className="setting-row"><div><h3>연결된 Provider 없음</h3><p>모델 선택과 API Key 연결 준비 중</p></div><button disabled>연결 · 준비 중</button></div></div>
        <div className="panel"><h2>권한과 시스템</h2><div className="setting-row"><div><h3>마이크 · 미확인</h3><p>음성 기능 미연결 · 권한을 요청하지 않습니다.</p></div><span className="badge">준비 중</span></div><div className="setting-row"><div><h3>손쉬운 사용</h3><p>Native Diagnostics에서 현재 권한을 직접 조회합니다.</p></div><button onClick={() => navigate('command')}>상태 조회로 이동</button></div></div>
        <div className="panel"><h2>Appearance & Motion</h2><div className="setting-row"><div><h3>Deep Space</h3><p>기본 어두운 테마 · 다른 테마 준비 중</p></div><span className="theme-swatch" aria-label="남청색 테마" /></div><div className="setting-row"><div><h3>Core 미리보기 움직임</h3><p>현재 세션에 적용 · 시스템 동작 줄이기 우선</p></div><label className="motion-toggle"><input type="checkbox" checked={motion} onChange={e => setMotion(e.target.checked)} />{motion ? '켜짐' : '꺼짐'}</label></div><p className="motion-status">시스템 동작 줄이기: {policy.reduced ? '적용 중' : '꺼짐'}<br />현재 Motion 정책: {policy.reason}</p></div>
      </section>}
      <section className="page-scroll" hidden={page !== 'command'} aria-label="Command Center">
        <div className="section-intro"><span className="badge">DEVELOPER DIAGNOSTICS</span><h2>실행의 흐름을 한눈에.</h2><p>E4 Fake Runtime의 합성 진단입니다. 실제 AI나 컴퓨터 제어를 수행하지 않습니다.</p></div>
        <div className="panel"><div className="panel-heading"><h2>Runtime Console</h2><span className="badge">FAKE PROVIDER</span></div><FakeRun /></div>
        <div className="panel"><div className="panel-heading"><h2>Core Diagnostics</h2><span className="badge">LOCAL VISUAL</span></div><p>점등과 상태 선택은 로컬 시각 실험입니다. 실제 AI 상태를 변경하지 않습니다.</p>
          <div className="core-controls"><button className="primary" onClick={onLight} disabled={lit}>{lit ? '✓ 코어 점등됨' : '◈ 코어 점등'}</button><label>상태 미리보기<select value={preview} onChange={e => setPreview(e.target.value as CoreState | '')}><option value="">실제 연결 상태</option>{Object.entries(coreStates).map(([key, [label]]) => <option key={key} value={key}>Preview · {key} / {label}</option>)}</select></label></div>
          <button className="text-button" aria-expanded={legacyPreview} onClick={() => setLegacyPreview(!legacyPreview)}>{legacyPreview ? '기존 2D Core 닫기' : '기존 2D Core 미리보기'}</button>
          {legacyPreview && <div className="legacy-preview"><Core state={preview || 'offline'} lit={lit} preview={preview !== ''} /></div>}
          <p>Motion 정책: {policy.reason} · 마지막 자동 정지: {policy.lastPause}</p><p className="muted">현재 Core는 승인되지 않은 2D placeholder입니다. 실제 3D는 P1.2 범위입니다.</p>
        </div>
        <div className="panel"><div className="panel-heading"><h2>Native Diagnostics</h2><span className="badge">E3b · READ / APPROVE</span></div>{nativeChecks}</div>
      </section>
    </main>
  </div>;
}
