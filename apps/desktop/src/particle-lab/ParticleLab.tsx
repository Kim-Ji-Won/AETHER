import {useEffect,useRef,useState} from 'react';
import type {VisualSnapshot} from '../scene/visual-state';
import {createParticleRenderer,type ParticleRenderer,type LabReport,type LabOptions} from './renderer';
import type {ParticlePhase} from './motion';
import './particle-lab.css';
export function ParticleLab({exit}:{exit:()=>void}) {
 const surface=useRef<HTMLDivElement>(null),engine=useRef<ParticleRenderer|null>(null),diagnosticTrigger=useRef<HTMLButtonElement>(null);
 const identity=useRef(`particle-preview-${crypto.randomUUID()}`),sequence=useRef(0);
 const [phase,setPhase]=useState<ParticlePhase>('idle'),[motion,setMotion]=useState(true),[gain,setGain]=useState(1),[forward,setForward]=useState(true);
 const [quality,setQuality]=useState<LabOptions['quality']>('balanced'),[fault,setFault]=useState<LabOptions['fault']>('none');
 const [revision,setRevision]=useState(0),[report,setReport]=useState<LabReport|null>(null),[diagnostics,setDiagnostics]=useState(false);
 const latest=useRef({phase,motion,gain,forward});latest.current={phase,motion,gain,forward};
 const snapshot=():VisualSnapshot=>({instance:identity.current,contextEpoch:1,visualSequence:++sequence.current,source:'preview',phase:latest.current.phase,
   connection:'disconnected',illuminationEnabled:false,inputLevel:null,outputLevel:null});
 useEffect(()=>{
   const node=surface.current;if(!node)return;let mounted=true;
   const canvas=document.createElement('canvas');canvas.setAttribute('aria-hidden','true');node.append(canvas);
   const instance=createParticleRenderer(canvas,snapshot(),{quality,fault,motion:latest.current.motion,gain:latest.current.gain,forward:latest.current.forward},r=>{if(mounted)setReport(r);},()=>{if(mounted)setRevision(r=>r+1);});
   engine.current=instance;
   const observer=new ResizeObserver(()=>{if(mounted){const r=node.getBoundingClientRect();instance.resize(r.width,r.height);}});observer.observe(node);
   return()=>{mounted=false;observer.disconnect();instance.dispose();canvas.remove();if(engine.current===instance)engine.current=null;};
 },[quality,fault,revision]);
 useEffect(()=>{engine.current?.update(snapshot(),motion,gain,forward);},[phase,motion,gain,forward]);
 const closeDiagnostics=()=>{setDiagnostics(false);diagnosticTrigger.current?.focus();};
 const ready=report?.health==='ready';
 return <main className="particle-lab">
   <header><div><p className="lab-kicker">AETHER · VISUAL LAB</p><h1>Living Particle Intelligence</h1></div><button onClick={exit}>AETHER Shell 열기</button></header>
   <section className="lab-stage" aria-label="Particle Orb Local Preview">
     <div className="lab-canvas" ref={surface}/>
     {!ready&&<div className="lab-fallback" role="status"><strong>DOM Fallback</strong><p>{report?.reason??'Renderer 확인 중'}</p><p>Preview 설정과 Shell 이동 사용 가능</p></div>}
     <div className="lab-state"><span>{phase.toUpperCase()} · PREVIEW</span><span>실제 오디오·AI 미연결</span></div>
     <div className="lab-envelope"><span>Synthetic envelope · {(report?.time??0).toFixed(1)}s</span><meter aria-label="합성 음성 강도" min={0} max={1} value={report?.level??0}/><output>{(report?.level??0).toFixed(2)}</output></div>
   </section>
   <section className="lab-controls" aria-label="Preview Controls">
     <label>상태<select value={phase} onChange={e=>setPhase(e.target.value as ParticlePhase)}><option value="idle">Idle Preview</option><option value="listening">Listening Preview</option><option value="thinking">Thinking Preview</option><option value="speaking">Speaking Preview</option></select></label>
     <label>합성 음성 강도<select value={gain} onChange={e=>setGain(Number(e.target.value))}><option value={.35}>작게 · 35%</option><option value={.65}>중간 · 65%</option><option value={1}>크게 · 100%</option></select></label>
     <label>품질<select value={quality} onChange={e=>setQuality(e.target.value as LabOptions['quality'])}><option value="balanced">Balanced · 12,000</option><option value="high">High · 20,000</option></select></label>
     <label className="lab-motion"><input type="checkbox" checked={motion} onChange={e=>setMotion(e.target.checked)}/>Motion ON</label>
     <button onClick={()=>{setMotion(true);engine.current?.replay();}}>16초 Preview 다시 시작</button>
     <label>시점 고정<select defaultValue="" onChange={e=>{engine.current?.seek(Number(e.target.value));setMotion(false);e.target.value="";}}><option value="" disabled>정적 비교</option><option value="0">0s · 시작</option><option value="1">1s · 작은 발화</option><option value="2.3">2.3s · 짧은 쉼</option><option value="5.05">5.05s · 강한 발화</option><option value="7.95">7.95s · 수축</option><option value="9.25">9.25s · 재확산</option><option value="13.9">13.9s · 긴 쉼</option><option value="16">16s · Idle 복귀</option></select></label>
     <button ref={diagnosticTrigger} aria-expanded={diagnostics} onClick={()=>setDiagnostics(v=>!v)}>Renderer Diagnostics</button>
   </section>
   <footer>Local Visual Preview · 16초 단회 합성 패턴 · 마이크 접근 / 녹음 / 음성 합성 없음</footer>
   {diagnostics&&<aside className="lab-diagnostics" aria-label="Renderer Diagnostics" onKeyDown={e=>{if(e.key==='Escape')closeDiagnostics();}}>
     <button onClick={closeDiagnostics}>진단 닫기</button><p>{report?.reason}</p>
     <label><input type="checkbox" checked={forward} onChange={e=>setForward(e.target.checked)}/>전방 이동 · Camera 고정</label>
     <pre>{JSON.stringify(report,null,2)}</pre>
     <label>GPU 오류 검사<select value={fault} onChange={e=>setFault(e.target.value as LabOptions['fault'])}><option value="none">정상</option><option value="shader">Shader 실패</option><option value="fallback">DOM fallback</option></select></label>
     <div><button disabled={!report?.lossSupported||!ready} onClick={()=>engine.current?.lose()}>Context loss</button><button disabled={report?.health!=='lost'} onClick={()=>engine.current?.restore()}>Context restore</button><button onClick={()=>setRevision(v=>v+1)}>Renderer 재생성</button><button onClick={()=>{const r=engine.current?.stats();if(r)setReport(r);}}>진단 갱신</button></div>
     <p>GPU time / VRAM / 귀속 RSS: UNAVAILABLE · 앱 frame / CPU submit과 구분</p>
   </aside>}
 </main>;
}
