import * as THREE from 'three';
import {probeShaders} from '../scene/capabilities';
import {renderPolicy} from '../scene/framing';
import {VisualGate,type VisualSnapshot} from '../scene/visual-state';
import {createMotion,type ParticlePhase} from './motion';
import {createParticles} from './particles';
export interface LabOptions {quality:'balanced'|'high';fault:'none'|'shader'|'fallback';motion:boolean;gain:number;forward?:boolean}
export interface LabReport {
 health:'ready'|'lost'|'unavailable'|'disposed'; reason:string; generation:number;
 frames:number; raf:number; live:number; created:number; disposed:number;
 particles:number; draws:number; bytes:number; dpr:number; width:number; height:number;
 forwardEnabled:boolean; time:number; level:number; raw:number; inputLevel:number; outputLevel:number;
 frameP50:number|null;frameP95:number|null;cpuP50:number|null;cpuP95:number|null;samples:number;lossSupported:boolean;
 gpuTime:'UNAVAILABLE';vram:'UNAVAILABLE';rss:'UNAVAILABLE';
}
const audit={generation:0,live:0,created:0,disposed:0};
const percentile=(a:number[],q:number)=>a.length?[...a].sort((x,y)=>x-y)[Math.ceil(a.length*q)-1]:null;
// Separate opt-in lab: reads only local preview snapshots; no IPC, audio, Runtime or permissions.
export function createParticleRenderer(canvas:HTMLCanvasElement,initial:VisualSnapshot,options:LabOptions,
 publish:(r:LabReport)=>void,restored:()=>void) {
 const generation=++audit.generation,gate=new VisualGate([initial.instance,initial.contextEpoch,true]);
 const media=matchMedia('(prefers-reduced-motion: reduce)'),clock=createMotion();
 const scene=new THREE.Scene(),camera=new THREE.PerspectiveCamera(38,1,.1,40);
 let renderer:THREE.WebGLRenderer|null=null,particles:ReturnType<typeof createParticles>|null=null;
 let gl:WebGL2RenderingContext|null=null,loss:WEBGL_lose_context|null=null;
 let dead=false,lost=false,counted=false,available=false,raf=0,frames=0,draws=0,last=0,next=0,lastPublish=0;
 let width=1,height=1,dpr=1,motion=options.motion,gain=options.gain,phase:ParticlePhase='idle',forward=options.forward??true;
 let health:LabReport['health']='unavailable',reason='초기화 중',sample=clock.step(0,phase,gain);
 const intervals:number[]=[],cpu:number[]=[];
 const stats=():LabReport=>({health,reason,generation,frames,raf:raf?1:0,live:audit.live,created:audit.created,disposed:audit.disposed,
   particles:particles?.count??0,draws,bytes:particles?.bytes??0,dpr,width,height,forwardEnabled:forward,...sample,
   frameP50:percentile(intervals,.5),frameP95:percentile(intervals,.95),cpuP50:percentile(cpu,.5),cpuP95:percentile(cpu,.95),samples:intervals.length,lossSupported:!!loss,gpuTime:'UNAVAILABLE',vram:'UNAVAILABLE',rss:'UNAVAILABLE'});
 function stop(){if(raf)cancelAnimationFrame(raf);raf=0;last=0;next=0;}
 function release(){stop();particles?.dispose();particles=null;scene.clear();renderer?.dispose();renderer=null;
   if(counted){counted=false;audit.live--;audit.disposed++;}}
 function fail(message:string){if(dead)return;health='unavailable';reason=message;release();if(gl&&!gl.isContextLost())loss?.loseContext();publish(stats());}
 const policy=()=>renderPolicy(!document.hidden&&available,document.hasFocus(),motion,media.matches);
 function draw(now:number,advance:boolean){
   if(dead||lost||health!=='ready'||!renderer||!particles||!policy().draw)return;
   try{
     const start=performance.now();
     if(advance)sample=clock.step(last?Math.min(.1,(now-last)/1000):0,phase,gain);
     particles.uniforms.uTime.value=sample.time;particles.uniforms.uLevel.value=sample.level;
     particles.uniforms.uThinking.value=sample.thinking;particles.uniforms.uRadius.value=sample.radius;
     particles.uniforms.uSpeaking.value=sample.speakingWeight;
     particles.uniforms.uForwardFast.value=sample.forwardFast;particles.uniforms.uForwardSlow.value=sample.forwardSlow;
     particles.uniforms.uForwardEnabled.value=forward?1:0;
     renderer.render(scene,camera);cpu.push(performance.now()-start);if(cpu.length>1200)cpu.shift();
     if(advance&&last){intervals.push(now-last);if(intervals.length>1200)intervals.shift();}
     last=now;frames++;draws=renderer.info.render.calls;
   }catch{fail('Particle draw 실패 · DOM controls 유지');}
 }
 function tick(now:number){raf=0;if(dead||lost||health!=='ready'||!policy().animate){last=0;return;}
   const period=1000/(options.quality==='high'?60:30);
   if(!next||now+.75>=next){draw(now,true);next=now+period-(next?Math.max(0,now-next)%period:0);}
   if(!dead&&health==='ready'&&policy().animate)raf=requestAnimationFrame(tick);
   if(now-lastPublish>250){lastPublish=now;publish(stats());}
 }
 function refresh(){stop();if(dead||lost||health!=='ready')return;
   const p=policy();reason=!p.draw?'숨김 / 비활성 · 정지':!p.animate?'Motion 정적 · 단발 렌더':`${options.quality==='high'?60:30}fps 목표`;
   if(p.draw){draw(performance.now(),false);last=0;if(p.animate)raf=requestAnimationFrame(tick);}publish(stats());}
 function resize(w:number,h:number){if(dead||lost||!renderer)return;available=w>0&&h>0;if(!available){refresh();return;}
   width=w;height=h;dpr=Math.min(window.devicePixelRatio||1,options.quality==='high'?1.5:1.25);
   renderer.setPixelRatio(dpr);renderer.setSize(w,h,false);if(particles)particles.uniforms.uDpr.value=dpr;
   camera.aspect=w/h;const half=Math.min(38*Math.PI/360,Math.atan(Math.tan(38*Math.PI/360)*w/h));
   camera.position.set(0,.10,2.25/Math.tan(half));if(particles)particles.uniforms.uCameraDistance.value=camera.position.length();camera.lookAt(0,0,0);camera.updateProjectionMatrix();refresh();}
 function update(s:VisualSnapshot,enabled:boolean,amplitude:number,depthEnabled=forward){if(dead)return;
   if(gate.accept(s)&&s.source==='preview'&&['idle','listening','thinking','speaking'].includes(s.phase)){
     phase=s.phase as ParticlePhase;gain=Number.isFinite(amplitude)?Math.max(0,Math.min(1,amplitude)):0;motion=enabled;forward=depthEnabled;refresh();}}
 const onLost=(e:Event)=>{e.preventDefault();if(dead||health==='unavailable')return;lost=true;stop();health='lost';reason='GPU context loss · DOM fallback';publish(stats());};
 const onRestored=()=>{if(!dead&&lost)restored();};
 canvas.addEventListener('webglcontextlost',onLost);canvas.addEventListener('webglcontextrestored',onRestored);
 document.addEventListener('visibilitychange',refresh);window.addEventListener('focus',refresh);window.addEventListener('blur',refresh);media.addEventListener('change',refresh);
 try{
   if(options.fault==='fallback')throw Error('진단용 DOM fallback');
   gl=canvas.getContext('webgl2',{alpha:false,antialias:false,powerPreference:'low-power'});if(!gl)throw Error('WebGL2 사용 불가');
   loss=gl.getExtension('WEBGL_lose_context');probeShaders(gl,options.fault==='shader');
   renderer=new THREE.WebGLRenderer({canvas,context:gl,alpha:false,antialias:false});counted=true;audit.live++;audit.created++;
   renderer.outputColorSpace=THREE.SRGBColorSpace;renderer.toneMapping=THREE.ACESFilmicToneMapping;renderer.setClearColor(0x02050a,1);
   renderer.debug.onShaderError=()=>{throw Error('Particle shader compile/link 실패');};
   particles=createParticles(options.quality);scene.add(particles.points);renderer.compile(scene,camera);
   health='ready';update(initial,motion,gain);
 }catch(e){fail(e instanceof Error?e.message:'초기화 실패');}
 return {stats,resize,update,replay(){if(dead)return;clock.reset();sample=clock.step(0,phase,gain);refresh();},
   seek(seconds:number){if(dead||!Number.isFinite(seconds))return;clock.reset();sample=clock.step(0,phase,gain);
     const steps=Math.round(Math.min(16,Math.max(0,seconds))*120);for(let i=0;i<steps;i++)sample=clock.step(1/120,phase,gain);motion=false;refresh();},
   lose(){if(!dead&&health==='ready')loss?.loseContext();},restore(){if(!dead&&lost)loss?.restoreContext();},
   dispose(){if(dead)return;dead=true;release();canvas.removeEventListener('webglcontextlost',onLost);canvas.removeEventListener('webglcontextrestored',onRestored);
     document.removeEventListener('visibilitychange',refresh);window.removeEventListener('focus',refresh);window.removeEventListener('blur',refresh);media.removeEventListener('change',refresh);
     if(gl&&!gl.isContextLost())loss?.loseContext();loss=null;gl=null;health='disposed';reason='자원 해제';publish(stats());}};
}
export type ParticleRenderer=ReturnType<typeof createParticleRenderer>;
