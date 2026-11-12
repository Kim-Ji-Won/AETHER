import { useLayoutEffect, useRef, useState, type ReactNode } from 'react';
import { Core, coreStates, type CoreState } from './Core';
import { FakeRun } from './FakeRun';
import { CommandDock, ContextPanel, Icon, Navigation, pages, StatusArea, type Page } from './ShellParts';
import { SceneHost } from './scene/SceneHost';
import { useVisualState } from './scene/useVisualState';
import type { Fault, SceneReport } from './scene/renderer';
import type { Quality } from './scene/framing';
import { useMotion } from './useMotion';

export function Shell({ lit, onLight, nativeChecks }: { lit: boolean; onLight: () => void; nativeChecks: ReactNode }) {
  const [quality, setQuality] = useState<Quality>('balanced');
  const [angle, setAngle] = useState(0);
  const [fault, setFault] = useState<Fault>('none');
  const [sceneReport, setSceneReport] = useState<SceneReport | null>(null);
  const [page, setPage] = useState<Page>('main');
  const [preview, setPreview] = useState<CoreState | ''>('');
  const [motion, setMotion] = useState(true);
  const [draft, setDraft] = useState('');
  const [legacyPreview, setLegacyPreview] = useState(false);
  const [context, setContext] = useState(false);
  const [status, setStatus] = useState(false);
  const heading = useRef<HTMLHeadingElement>(null);
  const firstRender = useRef(true);
  const policy = useMotion(motion, page === 'main' || (page === 'command' && legacyPreview));
  const visualSnapshot = useVisualState(preview, lit);
  useLayoutEffect(() => {
    if (firstRender.current) { firstRender.current = false; return; }
    heading.current?.focus();
  }, [page]);
  function navigate(next: Page) { setPage(next); setContext(false); setStatus(false); }
  function closeContext() { setContext(false); document.querySelector<HTMLButtonElement>('.context-toggle')?.focus(); }
  function closeStatus() { setStatus(false); document.querySelector<HTMLButtonElement>('.status-toggle')?.focus(); }
  const current = pages.find(item => item[0] === page)!;
  return <div className={`shell ${policy.active ? 'motion-on' : 'motion-off'}`}>
    <a className="skip-link" href="#workspace">본문으로 건너뛰기</a>
    <Navigation page={page} navigate={navigate} />
    <main id="workspace" className={`workspace ${page === 'main' ? 'presence-workspace' : ''}`} tabIndex={-1} onKeyDown={e => { if (e.key === 'Escape') { if (context) closeContext(); if (status) closeStatus(); } }}>
      <header className={`page-header ${page === 'main' ? 'presence-header' : ''}`}>
        <div className={page === 'main' ? 'sr-only' : undefined}>{page !== 'main' && <p className="eyebrow">WORKSPACE / {current[2].toUpperCase()}</p>}<h1 ref={heading} tabIndex={-1}>{current[1]}</h1></div>
        {page === 'main' ? <div className="header-actions">
          <button className="connection status-toggle" aria-expanded={status} aria-controls={status ? 'status-panel' : undefined} onClick={() => { setStatus(!status); setContext(false); }}><span className="status-dot" />AI 미연결<span aria-hidden="true">⌄</span></button>
          <button className="context-toggle" aria-expanded={context} aria-controls={context ? 'context-panel' : undefined} onClick={() => { setContext(!context); setStatus(false); }}><Icon name="context" /><span>Context</span></button>
        </div> : <span className="connection"><span className="status-dot" /> AI 미연결</span>}
      </header>
      {/* Mount visible product pages so WebKit rebuilds their accessibility subtree.
          Runtime Console stays last and mounted: navigation must not abort a Fake Run.
          Keeping hidden runtime content after the active page avoids WebKit AX sibling truncation. */}
      {page === 'main' && <section className="main-page" aria-label="Main">
        <div className="main-stage">
          <section className="scene-viewport" aria-label="3D Scene 배치 영역">
            <SceneHost snapshot={visualSnapshot} motion={motion} panelOpen={status || context}
              options={{quality, angle, fault}} onReport={setSceneReport} />
          </section>
          {status && <StatusArea close={closeStatus} />}
          <ContextPanel open={context} close={closeContext} navigate={navigate} />
        </div>
        <CommandDock draft={draft} setDraft={setDraft} />
      </section>}
      {page === 'memory' && <section className="page-scroll" aria-label="Memory Center">
        <label className="search-label">기억 검색<input disabled placeholder="검색 기능 준비 중" type="search" aria-describedby="memory-status" /></label>
        <div className="panel empty-state"><Icon name="memory" /><h2>기억 저장소 미연결</h2><p id="memory-status">저장된 데이터와 건수를 확인할 수 없습니다.</p><button disabled>기억 추가 · 준비 중</button></div>
        <p className="footnote">검색 · 추가 · 편집 · 삭제 기능 준비 중</p>
      </section>}
      {page === 'settings' && <section className="page-scroll" aria-label="Settings">
        <div className="panel"><h2>AI Provider</h2><div className="setting-row"><div><h3>연결된 Provider 없음</h3><p>모델 선택과 API Key 연결 준비 중</p></div><button disabled>연결 · 준비 중</button></div></div>
        <div className="panel"><h2>Permissions</h2><div className="setting-row"><div><h3>마이크 · 미확인</h3><p>음성 기능 미연결 · 권한을 요청하지 않습니다.</p></div><span className="badge">준비 중</span></div><div className="setting-row"><div><h3>손쉬운 사용</h3><p>Native Diagnostics에서 현재 권한을 직접 조회합니다.</p></div><button onClick={() => navigate('command')}>상태 조회로 이동</button></div></div>
        <div className="panel"><h2>Appearance & Motion</h2><div className="setting-row"><div><h3>Deep Space</h3><p>기본 어두운 테마 · 다른 테마 준비 중</p></div><span className="theme-swatch" aria-label="남청색 테마" /></div><div className="setting-row"><div><h3>Core 미리보기 움직임</h3><p>현재 세션에 적용 · 시스템 동작 줄이기 우선</p></div><label className="motion-toggle"><input type="checkbox" checked={motion} onChange={e => setMotion(e.target.checked)} />{motion ? '켜짐' : '꺼짐'}</label></div><p className="motion-status">시스템 동작 줄이기: {policy.reduced ? '적용 중' : '꺼짐'}<br />현재 Motion 정책: {policy.reason}</p></div>
      </section>}
      <section className="page-scroll" hidden={page !== 'command'} aria-label="Command Center">
        <div className="section-intro"><span className="badge">DEVELOPER DIAGNOSTICS</span><p>E4 Fake Runtime의 합성 진단입니다. 실제 AI나 컴퓨터 제어를 수행하지 않습니다.</p></div>
        <div className="panel"><div className="panel-heading"><h2>Runtime Console</h2><span className="badge">FAKE PROVIDER</span></div><FakeRun /></div>
        <div className="panel"><div className="panel-heading"><h2>Neural Scene</h2><span className="badge">P1.2b · PREVIEW</span></div>
          <p>유기적 신경망 구조의 로컬 시각 실험입니다. 실제 AI 처리·최종 Plasma·Bloom은 포함하지 않습니다. 아래 설정은 Main 진입 시 적용됩니다.</p>
          <div className="scene-settings">
            <label>Scene 품질<select value={quality} onChange={e => setQuality(e.target.value as Quality)}><option value="balanced">Balanced · 30fps 목표</option><option value="high">High · 60fps 목표</option></select></label>
            <label>Camera 시점<select value={angle} onChange={e => setAngle(Number(e.target.value))}><option value={-15}>−15°</option><option value={0}>정면</option><option value={15}>+15°</option></select></label>
            <label>GPU 실패 진단<select value={fault} onChange={e => setFault(e.target.value as Fault)}><option value="none">정상 경로</option><option value="shader">Shader compile 실패 주입</option><option value="fallback">DOM fallback 주입</option></select></label>
          </div>
          <pre className="scene-report" aria-label="Scene 최근 진단">{sceneReport ? JSON.stringify(sceneReport, null, 2) : 'Main Scene 실행 전'}</pre>
          <p>마지막 이벤트/수동 조회 값 · CPU submit과 frame interval은 GPU time이 아닙니다. GPU time / 귀속 RSS: UNAVAILABLE. 에너지: NOT_RUN.</p>
          <a href="/THIRD_PARTY_NOTICES.txt" target="_blank" rel="noreferrer">Three.js 라이선스 고지</a>
        </div>
        <div className="panel"><div className="panel-heading"><h2>Core Diagnostics</h2><span className="badge">LOCAL VISUAL</span></div><p>점등과 상태 선택은 로컬 시각 실험입니다. 실제 AI 상태를 변경하지 않습니다.</p>
          <div className="core-controls"><button className="primary" onClick={onLight} disabled={lit}>{lit ? '✓ 코어 점등됨' : '◈ 코어 점등'}</button><label>상태 미리보기<select value={preview} onChange={e => setPreview(e.target.value as CoreState | '')}><option value="">실제 연결 상태</option>{Object.entries(coreStates).map(([key, [label]]) => <option key={key} value={key}>Preview · {key} / {label}</option>)}</select></label></div>
          <button className="text-button" aria-expanded={legacyPreview} onClick={() => setLegacyPreview(!legacyPreview)}>{legacyPreview ? '기존 2D Core 닫기' : '기존 2D Core 미리보기'}</button>
          {legacyPreview && <div className="legacy-preview"><Core state={preview || 'offline'} lit={lit} preview={preview !== ''} /></div>}
          <p>Motion 정책: {policy.reason} · 마지막 자동 정지: {policy.lastPause}</p><p className="muted">현재 Core는 승인되지 않은 2D placeholder입니다. Main은 실제 3D Neural Structure Preview이며 최종 Plasma는 후속 단계입니다.</p>
        </div>
        <div className="panel"><div className="panel-heading"><h2>Native Diagnostics</h2><span className="badge">E3b · READ / APPROVE</span></div>{nativeChecks}</div>
      </section>
    </main>
  </div>;
}
