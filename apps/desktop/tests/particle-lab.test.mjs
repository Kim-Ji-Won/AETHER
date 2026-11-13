import test,{after} from 'node:test';
import assert from 'node:assert/strict';
import {execFileSync} from 'node:child_process';
import {mkdtempSync,rmSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {harness,nodes} from './scene-lifecycle-harness.mjs';
const desktop=fileURLToPath(new URL('..',import.meta.url)),output=mkdtempSync(join(tmpdir(),'aether-particle-contract-'));
execFileSync(process.execPath,[join(desktop,'node_modules/typescript/bin/tsc'),'--ignoreConfig','--target','ES2022','--module','ESNext','--moduleResolution','Bundler','--jsx','react-jsx','--types','vite/client','--strict','--skipLibCheck','--rootDir','src','--outDir',output,'src/particle-lab/ParticleLab.tsx'],{cwd:desktop});
after(()=>rmSync(output,{recursive:true}));
async function api(file,name){const h=await harness(output);return h.exported('particle-lab/'+file,name);}
test('synthetic envelope has irregular strengths and silence; smoothing is bounded and returns to rest',async()=>{
 const envelope=await api('motion','syntheticEnvelope'),create=await api('motion','createMotion');
 assert.equal(envelope(0),0);assert.equal(envelope(14),0);assert.ok(envelope(4.9)>.9);
 const a=create(),b=create(),levels=[];for(let i=0;i<480;i++){
   const x=a.step(1/30,'listening',1),y=b.step(1/30,'listening',.35);
   assert.ok(x.level>=0&&x.level<=1);assert.ok(Math.abs(y.level-x.level*.35)<1e-10);levels.push(x.level);
 }
 assert.ok(Math.max(...levels)>.6);assert.ok(levels.at(-1)<.001);
 const c=create();for(let i=0;i<145;i++)c.step(1/30,'speaking',1);
 let sample;for(let i=0;i<120;i++)sample=c.step(1/30,'idle',1);
 assert.ok(sample.level<.001);assert.equal(sample.outputLevel,0);
});
test('particle field is volumetric and deterministic across quality prefixes, without line geometry',async()=>{
 const data=await api('particles','particleData'),create=await api('particles','createParticles');
 const a=data(12000),b=data(20000);assert.deepEqual([...a.positions],[...b.positions.slice(0,a.positions.length)]);
 let inner=0,outer=0;for(let i=0;i<a.positions.length;i+=3){const r=Math.hypot(...a.positions.slice(i,i+3));if(r<.5)inner++;if(r>1)outer++;assert.ok(r<1.29);}
 assert.ok(inner>1000);assert.ok(outer>200);
 const p=create('balanced');assert.equal(p.points.type,'Points');assert.equal(p.count,12000);assert.equal(p.bytes,336000);
 let count=0;p.points.geometry.addEventListener('dispose',()=>count++);p.dispose();p.dispose();assert.equal(count,1);
});
async function lab(){const h=await harness(output),Lab=await h.exported('particle-lab/ParticleLab','ParticleLab');h.mount(Lab,{exit(){}});return h;}
function button(h,label){const b=nodes(h.tree,n=>n.type==='button'&&n.props.children===label)[0];assert.ok(b);b.props.onClick();h.flush();}
function clean(h){const m=h.metrics();for(const k of ['live','pendingRAF','resources','listeners'])assert.equal(m[k],0,k);assert.ok(m.maxLive<=1);assert.ok(m.maxRAF<=1);}
test('contract: lab StrictMode replay and stale resize/restore/RAF cannot affect replacement or disposed renderer',async()=>{
 const h=await lab(),resize=h.observers[0].callback,restore=h.canvases[0].callbacks('webglcontextrestored')[0],raf=[...h.raf.values()][0];
 h.replay();const before=h.metrics();resize();restore();raf(1000);assert.deepEqual(h.metrics(),before);
 h.unmount();resize();restore();raf(2000);clean(h);assert.equal(h.requests.length,0);
});
test('contract: reduced-motion/hidden stop RAF, restore resumes, context loss restores once and fallback stays usable',async()=>{
 const h=await lab();h.tick(100);h.tick(150);assert.equal(h.raf.size,1);
 h.setReduced(true);assert.equal(h.raf.size,0);h.setReduced(false);assert.equal(h.raf.size,1);
 h.setHidden(true);assert.equal(h.raf.size,0);h.setHidden(false);assert.equal(h.raf.size,1);
 button(h,'Renderer Diagnostics');button(h,'Context loss');assert.equal(h.raf.size,0);
 h.failNextContext();button(h,'Context restore');assert.equal(h.raf.size,0);assert.equal(h.metrics().live,0);
 assert.equal(nodes(h.tree,n=>n.props?.className==='lab-fallback').length,1);
 h.unmount();clean(h);
});

test('spatial radius contracts during long rests, expands with gain and returns to Idle without a periodic loop',async()=>{
 const create=await api('motion','createMotion'),env=await api('motion','syntheticEnvelope');
 const big=create(),small=create(),samples=[];let maximum=0,minimum=2,smallMax=0;
 for(let i=0;i<2400;i++){const a=big.step(1/120,'listening',1),b=small.step(1/120,'listening',.35);samples.push(a);
 if(a.time<12){maximum=Math.max(maximum,a.radius);smallMax=Math.max(smallMax,b.radius);}if(a.time>12&&a.time<14)minimum=Math.min(minimum,a.radius);}
 assert.ok(maximum/minimum>1.25);assert.ok(maximum>smallMax+.05);assert.ok(Math.abs(samples.at(-1).radius-1)<.002);
 assert.ok(samples.every((v,i)=>!i||Math.abs(v.radius-samples[i-1].radius)<.025));
 assert.equal(env(20.9),0);assert.notEqual(env(1.1,'listening'),env(1.1,'speaking'));
 big.reset();small.reset();assert.deepEqual(big.step(.05,'speaking',1),small.step(.05,'speaking',1));
});

test('forward channels have separate bounded inertia, amplitude hierarchy and no Idle/Thinking burst',async()=>{
 const create=await api('motion','createMotion');
 const run=(phase,gain)=>{const m=create(),samples=[];for(let i=0;i<1920;i++)samples.push(m.step(1/120,phase,gain));return samples;};
 for(const phase of ['idle','thinking'])assert.ok(run(phase,1).every(s=>s.forwardFast===0&&s.forwardSlow===0));
 const big=run('listening',1),small=run('listening',.35),speak=run('speaking',1);
 const peak=a=>Math.max(...a.map(s=>s.forwardFast));assert.ok(peak(big)>peak(small)*5);assert.ok(peak(speak)>.7);
 assert.ok(big.every(s=>s.forwardFast>=0&&s.forwardFast<=1&&s.forwardSlow>=0&&s.forwardSlow<=1));
 assert.ok(big.some(s=>s.raw===0&&s.forwardSlow>s.forwardFast+.1)); // independent slow return
 assert.ok(big.every((s,i)=>!i||Math.abs(s.forwardFast-big[i-1].forwardFast)<.08));
 assert.ok(big.at(-1).forwardSlow<.001);
 const transition=create();for(let i=0;i<600;i++)transition.step(1/120,'listening',1);
 for(let i=0;i<600;i++)transition.step(1/120,'thinking',1);
 assert.ok(transition.step(0,'thinking',1).forwardSlow<.001);
});
test('forward model preserves most of the volume, bounds displacement and guards fixed-camera projection',async()=>{
 const data=await api('particles','particleData'),distance=await api('depth','forwardDistance'),safe=await api('depth','safeForward'),bound=await api('depth','particleBound');
 const {positions,traits}=data(20000);let still=0,max=0;
 for(let i=0;i<20000;i++){
   const t=[...traits.slice(i*4,i*4+4)],p=[...positions.slice(i*3,i*3+3)];
   const d=distance(t,p,1,1,1);if(d===0)still++;max=Math.max(max,d);
   assert.ok(d>=0&&d<=2.8*1.12);assert.ok(2.4+d<bound);
   const view=[p[0]*1.3,p[1]*1.3,-6.53+p[2]*1.3],z=view[2]+safe(d,view,2.9,2.9);
   assert.ok(z<=-1.5);assert.ok(Math.abs(view[0]*2.9/z)<=.86);assert.ok(Math.abs(view[1]*2.9/z)<=.82);
 }
 assert.ok(still/20000>.60&&still/20000<.64);assert.ok(max>2);
 assert.ok(Math.abs(safe(3,[0,0,-1.6],2.9,2.9)-.1)<1e-12);
});
test('forward ON/OFF keeps one renderer and immutable geometry, and does not reset the preview',async()=>{
 const h=await lab();button(h,'Renderer Diagnostics');h.tick(100);h.tick(200);h.tick(400);h.flush();
 const report=()=>JSON.parse(nodes(h.tree,n=>n.type==='pre')[0].props.children);
 const initial=report();assert.ok(initial.time>0);
 const toggle=nodes(h.tree,n=>n.type==='input'&&n.props?.type==='checkbox').at(-1);
 const before=h.metrics();toggle.props.onChange({target:{checked:false}});h.flush();
 const after=h.metrics(),updated=report();assert.equal(updated.created,initial.created);assert.equal(updated.generation,initial.generation);
 assert.equal(updated.time,initial.time);assert.equal(updated.forwardEnabled,false);assert.equal(after.resources,before.resources);
 assert.ok(after.maxLive<=1&&after.maxRAF<=1);assert.equal(h.requests.length,0);h.unmount();clean(h);
 const create=await api('particles','createParticles'),p=create('high');
 const position=p.points.geometry.attributes.position.array,copy=position.slice();
 p.uniforms.uForwardFast.value=1;p.uniforms.uForwardSlow.value=.8;p.uniforms.uForwardEnabled.value=0;
 assert.deepEqual(position,copy);assert.match(p.points.material.vertexShader,/view.z\+=uForwardEnabled/);
 assert.match(p.points.material.vertexShader,/5.5\*uDpr/);p.dispose();
});
