import test, { after } from 'node:test';
import assert from 'node:assert/strict';
import * as THREE from 'three';
import {frameScene,sceneFitRadius} from '../src/scene/framing.ts';
import { compileScene, harness } from './scene-lifecycle-harness.mjs';

const compiled=compileScene(); after(()=>compiled.dispose());
async function fixture() {
  const h=await harness(compiled.output);
  const create=await h.exported('neural','createNeuralStructure');
  const group=new THREE.Group(), owned={geometries:[],materials:[]};
  const neural=create(group,owned);
  return { group,owned,neural,close() { neural.invalidate(); owned.geometries.forEach(g=>g.dispose()); owned.materials.forEach(m=>m.dispose()); group.clear(); } };
}
test('neural topology has volumetric extent and bounded geometry/memory with four draw batches',async()=>{
  const f=await fixture();
  try {
    assert.equal(f.group.children.length,4);
    assert.ok(f.neural.nodeCount>=100); assert.ok(f.neural.connections>f.neural.nodeCount);
    assert.ok(f.neural.triangles<150000); assert.ok(f.neural.bufferBytes<8*1024*1024);
    let front=-Infinity,back=Infinity;
    for(const g of f.owned.geometries) {
      const p=g.attributes.position;
      for(let i=0;i<p.count;i++) {
        const x=p.getX(i),y=p.getY(i),z=p.getZ(i);
        assert.ok(Number.isFinite(x+y+z)); assert.ok(Math.hypot(x,y,z)+.07<sceneFitRadius);
        front=Math.max(front,z);back=Math.min(back,z);
      }
      assert.equal(g.attributes.signalRoute.count,p.count);
    }
    assert.ok(front>1.3 && back< -1.3);
  } finally { f.close(); }
});
test('remount preserves deterministic positions and connectivity',async()=>{
  const a=await fixture(),b=await fixture();
  try {
    for(let i=0;i<a.owned.geometries.length;i++) {
      assert.deepEqual(a.owned.geometries[i].attributes.position.array,b.owned.geometries[i].attributes.position.array);
      assert.deepEqual(a.owned.geometries[i].index.array,b.owned.geometries[i].index.array);
    }
  } finally {a.close();b.close();}
});
test('energy updates use shared uniforms without geometry allocations or buffer mutation; invalidate freezes updates',async()=>{
  const f=await fixture();
  try {
    const before=f.owned.geometries.map(g=>g.attributes.position.array.slice());
    const attributes=f.owned.geometries.map(g=>g.attributes.position);
    const u=f.neural.inspection.uniforms;
    for(let i=0;i<1000;i++)f.neural.update(1/30);
    assert.ok(u.uTime.value>28 && u.uTime.value<29);
    f.neural.update(100);assert.ok(u.uTime.value<34); // Resume cannot catch up a long hidden interval.
    f.owned.geometries.forEach((g,i)=>{
      assert.equal(g.attributes.position,attributes[i]);assert.deepEqual(g.attributes.position.array,before[i]);
    });
    for(const material of f.owned.materials)assert.equal(material.uniforms.uTime,u.uTime);
    const time=u.uTime.value;f.neural.invalidate();f.neural.update(1);assert.equal(u.uTime.value,time);
  } finally {f.close();}
});

test('nucleus density and primary signal arrivals form a connected outward hierarchy',async()=>{
  const f=await fixture();
  try {
    const {nodes,primaryRoutes,layerConnections,uniforms}=f.neural.inspection;
    assert.equal(nodes.length,114);assert.ok(f.neural.connections<=198);
    assert.equal(nodes.filter(n=>n.layer<2).length,48);
    const nucleus=nodes.filter(n=>n.layer<2);
    const centroid=nucleus.reduce((sum,n)=>sum.add(n.position),new THREE.Vector3()).divideScalar(nucleus.length);
    assert.ok(centroid.length()<.12); // Radius must not correlate with the polar sampling order.
    assert.ok(nucleus.some(n=>n.position.length()<.35) && nucleus.some(n=>n.position.length()>.85));
    assert.ok(layerConnections[0]>layerConnections[1]+layerConnections[2]);
    for(const n of nodes.filter(n=>n.layer>0)) {
      const route=primaryRoutes.find(r=>r.to===n);assert.ok(route);
      assert.equal(route.from.channel,n.channel);assert.ok(route.from.arrival<n.arrival);
    }
    const phase=uniforms.uTime.value;
    f.neural.setVisual({source:'preview',phase:'thinking',illuminationEnabled:false});
    assert.equal(uniforms.uTime.value,phase);assert.equal(uniforms.uSpeed.value,1.35);
    f.neural.setVisual({source:'actual',phase:'offline',illuminationEnabled:false});
    assert.equal(uniforms.uSpeed.value,.85);
  } finally {f.close();}
});
test('actual neural vertices plus deformation fit all camera angles and safe rectangles',async()=>{
  const f=await fixture();
  try {
    for(const [w,h] of [[316,356],[756,410],[1056,450],[316,296]])for(const degrees of [-15,0,15]) {
      const frame=frameScene(w,h,2,'balanced'), angle=degrees*Math.PI/180;
      const camera=new THREE.PerspectiveCamera(frame.fov,frame.aspect,frame.near,frame.far);
      camera.position.set(Math.sin(angle)*frame.distance,frame.distance*.19,Math.cos(angle)*frame.distance);
      camera.lookAt(0,0,0);camera.updateMatrixWorld();
      for(const geometry of f.owned.geometries) {
        const p=geometry.attributes.position;
        for(let i=0;i<p.count;i+=3) {
          const v=new THREE.Vector3(p.getX(i),p.getY(i),p.getZ(i));
          v.multiplyScalar(1+.07/v.length()).project(camera);
          assert.ok(Math.abs(v.x)<.98 && Math.abs(v.y)<.98 && Math.abs(v.z)<1);
        }
      }
    }
  } finally {f.close();}
});
