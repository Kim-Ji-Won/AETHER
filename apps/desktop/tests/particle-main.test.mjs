// Production Host/adapter contracts with DOM/React/GPU doubles, not WKWebView evidence.
import test,{after} from 'node:test';
import assert from 'node:assert/strict';
import {execFileSync} from 'node:child_process';
import {mkdtempSync,rmSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {harness,nodes} from './scene-lifecycle-harness.mjs';
const desktop=fileURLToPath(new URL('..',import.meta.url)),output=mkdtempSync(join(tmpdir(),'aether-particle-main-'));
execFileSync(process.execPath,[join(desktop,'node_modules/typescript/bin/tsc'),'--ignoreConfig','--target','ES2022','--module','ESNext','--moduleResolution','Bundler','--jsx','react-jsx','--types','vite/client','--strict','--skipLibCheck','--rootDir','src','--outDir',output,'src/particle-lab/MainParticleHost.tsx'],{cwd:desktop});
after(()=>rmSync(output,{recursive:true}));
const snapshot=(seq=1,phase='offline',source='actual')=>({instance:'main-42',contextEpoch:2,visualSequence:seq,source,phase,connection:'disconnected',illuminationEnabled:false,inputLevel:null,outputLevel:null});
async function host(s=snapshot()){
 const h=await harness(output),Host=await h.exported('particle-lab/MainParticleHost','MainParticleHost');
 const reports=[];let finished=0;
 const props={snapshot:s,motion:true,panelOpen:false,quality:'balanced',fault:'none',diagnostics:true,onReport:r=>reports.push(r),onFinished:()=>finished++,onCloseDiagnostics(){}};
 h.mount(Host,props);return {h,reports,props,update:patch=>{Object.assign(props,patch);h.mount(Host,props);},get finished(){return finished;}};
}
function clean(h){for(const k of ['live','pendingRAF','resources','listeners'])assert.equal(h.metrics()[k],0,k);assert.ok(h.metrics().maxRAF<=1);assert.ok(h.metrics().maxLive<=1);}
test('adapter preserves source/session/epoch/sequence boundaries, rejects fake audio and unsupported phases',async()=>{
 const h=await harness(output),Adapter=await h.exported('particle-lab/visual-adapter','ParticleVisualAdapter');
 const a=new Adapter(snapshot(),true);
 assert.deepEqual(JSON.parse(JSON.stringify(a.accept(snapshot()))),{phase:'idle',preview:false});
 assert.equal(a.accept(snapshot()),null);
 assert.equal(a.accept({...snapshot(2),instance:'old'}),null);
 assert.equal(a.accept({...snapshot(2),contextEpoch:1}),null);
 assert.equal(a.accept({...snapshot(2),inputLevel:.5}),null);
 assert.equal(a.accept(snapshot(2,'speaking','actual')),null);
 assert.equal(a.accept(snapshot(2,'executing','preview')),null);
 assert.equal(a.accept(snapshot(2,'speaking','preview')).phase,'speaking');
 assert.equal(a.accept(snapshot(3)).phase,'idle');
 assert.equal(a.accept(snapshot(2,'speaking','preview')),null);
 assert.equal(new Adapter(snapshot(),false).accept(snapshot()),null);
});
test('actual idle has no synthetic input; Preview and end return never allocate a new renderer',async()=>{
 const x=await host();for(let i=0;i<30;i++)x.h.tick(100+i*34);
 assert.equal(x.reports.at(-1).level,0);const generation=x.reports.at(-1).generation;
 x.update({snapshot:snapshot(2,'speaking','preview')});
 for(let i=0;i<500;i++)x.h.tick(1200+i*34);
 assert.equal(x.finished,1);assert.equal(x.reports.at(-1).generation,generation);
 x.update({snapshot:snapshot(3)});for(let i=0;i<60;i++)x.h.tick(19000+i*34);
 assert.equal(x.reports.at(-1).inputLevel,null);assert.equal(x.reports.at(-1).outputLevel,null);
 assert.equal(x.reports.at(-1).generation,generation);
 x.update({motion:false});assert.equal(x.h.raf.size,0);
 x.update({motion:true});assert.equal(x.h.raf.size,1);
 x.h.unmount();clean(x.h);
});
test('Host Strict replay, late resize/restore/RAF and 20 ownership cycles preserve one context',async()=>{
 const x=await host(),h=x.h,resize=h.observers[0].callback,restore=h.canvases[0].callbacks('webglcontextrestored')[0],raf=[...h.raf.values()][0];
 h.replay();const before=h.metrics(),report=x.reports.at(-1);
 resize();restore({preventDefault(){}});raf(2000);h.flush();
 assert.equal(h.metrics().draws,before.draws);assert.equal(x.reports.at(-1),report);
 for(let i=0;i<20;i++)h.replay();assert.equal(h.metrics().live,1);assert.equal(h.raf.size,1);
 h.unmount();const disposed=h.metrics();resize();restore({preventDefault(){}});raf(5000);
 assert.deepEqual(h.metrics(),disposed);clean(h);
});
test('Main hide/reduced policy stops RAF, failed restore remains fallback with no retry',async()=>{
 const x=await host(),h=x.h;
 h.setHidden(true);assert.equal(h.raf.size,0);h.setHidden(false);assert.equal(h.raf.size,1);
 h.setReduced(true);assert.equal(h.raf.size,0);h.setReduced(false);assert.equal(h.raf.size,1);
 const button=label=>{nodes(h.tree,n=>n.type==='button'&&n.props.children===label)[0].props.onClick();h.flush();};
 button('Context loss');assert.equal(h.raf.size,0);h.failNextContext();button('Context restore');
 assert.equal(h.metrics().live,0);assert.equal(h.raf.size,0);
 assert.equal(nodes(h.tree,n=>n.props?.className==='particle-main-fallback').length,1);
 const count=h.canvases.length;for(let i=0;i<20;i++)h.flush();assert.equal(h.canvases.length,count);
 h.unmount();clean(h);
});
test('transparent composition preserves data and shader color pipeline; Lab remains opaque material',async()=>{
 const h=await harness(output),create=await h.exported('particle-lab/particles','createParticles');
 const lab=create('balanced'),main=create('balanced',true);
 assert.deepEqual([...main.points.geometry.attributes.position.array],[...lab.points.geometry.attributes.position.array]);
 assert.equal(lab.points.material.premultipliedAlpha,false);assert.equal(main.points.material.premultipliedAlpha,true);
 assert.equal(main.points.material.fragmentShader.split('#include <tonemapping_fragment>').length,2);
 assert.equal(main.points.material.fragmentShader.split('#include <colorspace_fragment>').length,2);
 assert.equal(main.points.material.fragmentShader.split('#include <premultiplied_alpha_fragment>').length,2);
 main.dispose();lab.dispose();
});

// Projection-only framing: unchanged Lab/mobile, continuous layout response and
// stable time/generation/data when toggling the comparison mode.
test('Main projection respects small/layout bounds and never changes Lab framing or preview time',async()=>{
 const x=await host(),h=x.h,zoom=await h.exported('particle-lab/main-framing','mainProjectionZoom');
 assert.equal(zoom(376,442),1);assert.equal(zoom(376,382),1);
 assert.equal(zoom(788,510),1.12);assert.equal(zoom(1088,550),1.12);
 assert.equal(zoom(788,280),1);assert.equal(zoom(NaN,500),1);
 for(let w=375;w<900;w++)assert.ok(Math.abs(zoom(w+1,450)-zoom(w,450))<.001);
 const create=await h.exported('particle-lab/renderer','createParticleRenderer');
 h.unmount();clean(h);
 const canvas=h.canvases[0];
 const lab=create(canvas,snapshot(1,'speaking','preview'),{quality:'balanced',motion:false,gain:1,fault:'none'},()=>{},()=>{});
 lab.resize(788,510);assert.equal(lab.stats().projectionZoom,1);lab.setReferenceFraming(false);assert.equal(lab.stats().projectionZoom,1);lab.dispose();
 const main=create(canvas,snapshot(1,'speaking','preview'),{quality:'balanced',motion:false,gain:1,fault:'none',main:true},()=>{},()=>{});
 main.resize(788,510);main.seek(5.05);const after=main.stats();main.setReferenceFraming(true);const before=main.stats();
 assert.equal(after.projectionZoom,1.12);assert.equal(before.projectionZoom,1);
 for(const k of ['generation','time','cameraY','cameraZ','particles','bytes','level','radius','forwardFast','forwardSlow'])assert.equal(after[k],before[k],k);
 main.setReferenceFraming(false);assert.equal(main.stats().time,after.time);main.dispose();clean(h);
});
