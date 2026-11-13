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
