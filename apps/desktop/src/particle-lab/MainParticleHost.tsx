import { useEffect, useRef, useState } from 'react';
import type { VisualSnapshot } from '../scene/visual-state';
import { createParticleRenderer, type LabOptions, type LabReport, type ParticleRenderer } from './renderer';
import './main-particle.css';

export function MainParticleHost({snapshot,motion,panelOpen,quality,fault,diagnostics,onReport,onFinished,onCloseDiagnostics}: {
  snapshot:VisualSnapshot|null; motion:boolean; panelOpen:boolean;
  quality:LabOptions['quality'];fault:LabOptions['fault'];diagnostics:boolean;
  onReport:(report:LabReport)=>void;onFinished:()=>void;onCloseDiagnostics:()=>void;
}) {
  const surface=useRef<HTMLDivElement>(null),engine=useRef<ParticleRenderer|null>(null);
  const callbacks=useRef({onReport,onFinished});callbacks.current={onReport,onFinished};
  const diagnosticsVisible=useRef(diagnostics);diagnosticsVisible.current=diagnostics;
  const [revision,setRevision]=useState(0);
  const [health,setHealth]=useState<LabReport['health']>('unavailable');
  const [reason,setReason]=useState('Session 확인 중');
  const [report,setReport]=useState<LabReport|null>(null);
  useEffect(()=>{
    const node=surface.current;if(!node||!snapshot)return;
    let mounted=true;
    const canvas=document.createElement('canvas');canvas.setAttribute('aria-hidden','true');node.append(canvas);
    const instance=createParticleRenderer(canvas,snapshot,{quality,fault,motion,gain:1,forward:true,main:true},value=>{
      if(!mounted)return;
      callbacks.current.onReport(value);
      // Explicit developer overlay only, capped by engine reporting (4 Hz), never the Shell tree.
      if(diagnosticsVisible.current)setReport(value);
      // Frame reports stay in a ref owned by Shell. React only receives semantic changes.
      setHealth(previous=>previous===value.health?previous:value.health);
      if(value.health!=='ready')setReason(value.reason);
    },()=>{if(mounted)setRevision(v=>v+1);},()=>{if(mounted)callbacks.current.onFinished();});
    engine.current=instance;
    const observer=new ResizeObserver(()=>{
      if(mounted){const r=node.getBoundingClientRect();instance.resize(r.width,r.height);}
    });observer.observe(node);
    return()=>{mounted=false;observer.disconnect();instance.dispose();canvas.remove();
      callbacks.current.onReport(instance.stats());if(engine.current===instance)engine.current=null;};
    // Phase changes do not allocate a new context. Identity/epoch and explicit GPU reset do.
  },[snapshot?.instance,snapshot?.contextEpoch,quality,fault,revision]);
  useEffect(()=>{if(snapshot)engine.current?.update(snapshot,motion,1,true);engine.current?.setMotion(motion);},[snapshot,motion]);
  const ready=!!snapshot&&health==='ready';
  return <section className={`particle-main ${panelOpen?'panel-open':''}`} aria-label="로컬 Particle 시각 효과">
    <div className="particle-main-surface" ref={surface}/>
    {!ready&&<div className="particle-main-fallback" role="status"><strong>시각 효과 사용 불가</strong><p>{snapshot?reason:'Runtime Session 확인 중'}</p><p>탐색과 로컬 초안은 사용할 수 있습니다.</p></div>}
    <p className="particle-main-caption" role="status">{snapshot?.source==='preview'?`${snapshot.phase.toUpperCase()} · PREVIEW`:'로컬 시각 효과 · AI 미연결'}</p>
    {diagnostics&&<aside className="particle-main-diagnostics" aria-label="Particle Developer Diagnostics" onKeyDown={e=>{if(e.key==='Escape'){e.stopPropagation();onCloseDiagnostics();}}}>
      <button onClick={onCloseDiagnostics}>진단 닫기</button>
      <div className="fake-actions"><button onClick={()=>setReport(engine.current?.stats()??null)}>지표 새로고침</button>
        <button disabled={!ready} onClick={()=>{engine.current?.lose();setReport(engine.current?.stats()??null);}}>Context loss</button>
        <button disabled={health!=='lost'} onClick={()=>{engine.current?.restore();}}>Context restore</button>
        <button onClick={()=>setRevision(v=>v+1)}>Renderer 재생성</button>
        <button onClick={()=>{engine.current?.seek(5.05);setReport(engine.current?.stats()??null);}}>5.05초 정적 비교</button>
      </div>
      <p role="status">{report?`${report.health} · frames ${report.frames} · RAF ${report.raf} · live ${report.live} · created/disposed ${report.created}/${report.disposed}`:'지표 수집 중'}</p>
      <p>{report?`Scene ${report.width}×${report.height} · DPR ${report.dpr} · ${report.particles} particles · ${report.draws} draw · ${report.bytes} buffer bytes`:''}</p>
      <p>{report?`frame interval p50/p95 ${report.frameP50?.toFixed(2)??'—'}/${report.frameP95?.toFixed(2)??'—'} ms · CPU submit ${report.cpuP50?.toFixed(2)??'—'}/${report.cpuP95?.toFixed(2)??'—'} ms · samples ${report.samples}`:''}</p>
      <p>{report?.reason} · 진단 패널을 열었을 때만 갱신</p>
      <pre>{JSON.stringify(report,null,2)}</pre>
      <p>현재 canvas에만 장애 주입 · GPU time / VRAM / 귀속 RSS: UNAVAILABLE</p>
    </aside>}
  </section>;
}
