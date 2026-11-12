import test, { after } from 'node:test';
import assert from 'node:assert/strict';
import * as THREE from 'three';
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
        assert.ok(Number.isFinite(x+y+z)); assert.ok(Math.hypot(x,y,z)+.07<2.65);
        front=Math.max(front,z);back=Math.min(back,z);
      }
      assert.equal(g.attributes.pathPhase.count,p.count);
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
    assert.ok(u.uTime.value>30 && u.uTime.value<34);
    f.neural.update(100);assert.ok(u.uTime.value<34); // Resume cannot catch up a long hidden interval.
    f.owned.geometries.forEach((g,i)=>{
      assert.equal(g.attributes.position,attributes[i]);assert.deepEqual(g.attributes.position.array,before[i]);
    });
    for(const material of f.owned.materials)assert.equal(material.uniforms.uTime,u.uTime);
    const time=u.uTime.value;f.neural.invalidate();f.neural.update(1);assert.equal(u.uTime.value,time);
  } finally {f.close();}
});
