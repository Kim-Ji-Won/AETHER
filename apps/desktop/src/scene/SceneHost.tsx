import { useEffect, useRef, useState } from 'react';
import { createFoundation, type Foundation, type SceneOptions, type SceneReport } from './renderer';
import type { VisualSnapshot } from './visual-state';
import './scene.css';

export function SceneHost({ snapshot, motion, panelOpen, options, onReport }: {
  snapshot: VisualSnapshot | null; motion: boolean; panelOpen: boolean;
  options: Omit<SceneOptions, 'motion'>; onReport: (report: SceneReport) => void;
}) {
  const surface = useRef<HTMLDivElement>(null);
  const engine = useRef<Foundation | null>(null);
  const [revision, setRevision] = useState(0);
  const [report, setReport] = useState<SceneReport | null>(null);
  const publish = useRef(onReport); publish.current=onReport;
  useEffect(() => {
    if (!snapshot || !surface.current) return;
    let mounted = true;
    const node = surface.current;
    const canvas = document.createElement('canvas'); canvas.setAttribute('aria-hidden','true'); node.append(canvas);
    const instance = createFoundation(canvas, snapshot, {...options, motion}, value => {
      publish.current(value); if (mounted) setReport(value);
    }, () => { if (mounted) setRevision(value => value+1); });
    engine.current=instance;
    const observer = new ResizeObserver(() => {
      if (mounted) { const rect=node.getBoundingClientRect(); instance.resize(rect.width,rect.height); }
    });
    observer.observe(node);
    return () => {
      mounted=false; observer.disconnect(); instance.dispose(); canvas.remove();
      if (engine.current===instance) engine.current=null;
    };
    // Snapshot content/motion are updates, only owner identity or explicit reset recreates GPU resources.
  }, [snapshot?.instance, snapshot?.contextEpoch, options.quality, options.angle, options.fault, revision]);
  useEffect(() => { if (snapshot) engine.current?.update(snapshot,motion); }, [snapshot,motion]);
  const ready = !!snapshot && report?.health==='ready';
  return <section className={`foundation ${panelOpen ? 'panel-open' : ''}`} aria-label="Scene Foundation Preview">
    <div className="foundation-caption"><span className="badge">FOUNDATION PREVIEW · P1.2a</span><span>검증용 3D Mesh · 최종 AI Core 아님</span></div>
    <div className="foundation-surface" ref={surface} />
    {!ready && <div className="foundation-fallback" role="status"><strong>DOM Scene Fallback</strong><p>{snapshot ? report?.reason || 'Renderer 확인 중' : 'Runtime Session 확인 불가'}</p><p>AI 미연결 · 로컬 초안과 탐색 사용 가능</p></div>}
    <div className="foundation-status"><span role="status">{ready ? 'WebGL2 · 실제 3D' : '3D 렌더 중지'} · AI 미연결{snapshot?.source==='preview' ? ` · Preview: ${snapshot.phase}` : ''}</span>
      <details className="foundation-diagnostics"><summary onClick={() => { const value=engine.current?.stats(); if(value) setReport(value); }}>Scene Diagnostics</summary>
        <p>{report?.reason ?? '세션 대기'} · generation {report?.generation ?? '-'}<br />
          Scene {Math.round(report?.width ?? 0)}×{Math.round(report?.height ?? 0)} · DPR {report?.dpr ?? '-'}<br />
          frames {report?.frames ?? 0} · RAF {report?.pendingRAF ?? 0} · draw {report?.calls ?? 0} · triangles {report?.triangles ?? 0}<br />
          live {report?.liveRenderers ?? 0} · created {report?.created ?? 0} / disposed {report?.disposed ?? 0}</p>
        <div className="foundation-actions">
          <button onClick={() => { const value=engine.current?.stats(); if(value) {setReport(value); publish.current(value);} }}>지표 새로고침</button>
          <button disabled={!report?.lossSupported || !ready} onClick={() => engine.current?.lose()}>Context loss 주입</button>
          <button disabled={report?.health!=='lost'} onClick={() => engine.current?.restore()}>Context restore</button>
          <button onClick={() => setRevision(value=>value+1)}>Scene 재생성</button>
        </div>
        <p>장애 주입은 이 canvas에만 적용됩니다. 품질·Camera·Shader 실패 설정은 Command Center에 있습니다.</p>
      </details>
    </div>
  </section>;
}
