import test from 'node:test';
import assert from 'node:assert/strict';
import { VisualGate, Generation, validSession } from '../src/scene/visual-state.ts';
import { frameScene, renderPolicy, sceneFitRadius } from '../src/scene/framing.ts';

const snapshot = (overrides={}) => ({instance:'a',contextEpoch:2,visualSequence:1,source:'actual',phase:'offline',connection:'disconnected',illuminationEnabled:false,inputLevel:null,outputLevel:null,...overrides});
test('unconfirmed/closing sessions cannot enable the renderer', () => {
  for(const value of [null, ['a',2,false],['a',-1,true],['',2,true],['a',2.5,true]]) assert.equal(validSession(value),false);
  assert.equal(validSession(['a',2,true]),true);
});
test('sequence, epoch, instance and actual/preview boundaries reject stale or fictitious state', () => {
  const gate=new VisualGate(['a',2,true]);
  assert.equal(gate.accept(snapshot()),true);
  for(const value of [snapshot(),snapshot({visualSequence:0}),snapshot({instance:'b',visualSequence:3}),snapshot({contextEpoch:3,visualSequence:3}),snapshot({visualSequence:3,phase:'thinking'})]) assert.equal(gate.accept(value),false);
  assert.equal(gate.accept(snapshot({visualSequence:2,source:'preview',phase:'thinking'})),true);
  assert.equal(gate.accept(snapshot({visualSequence:3,source:'unknown'})),false);
  assert.equal(gate.accept(snapshot({visualSequence:3,source:'preview',phase:'invented'})),false);
  const next=new VisualGate(['b',3,true]);
  assert.equal(next.accept(snapshot({visualSequence:999})),false);
  assert.equal(next.accept(snapshot({instance:'b',contextEpoch:3})),true);
});
test('cleanup invalidates callbacks across setup/cleanup/setup and restore generations', () => {
  const generation=new Generation(); const first=generation.next(); generation.next(); const second=generation.next();
  assert.equal(generation.current(first),false); assert.equal(generation.current(second),true);
  generation.next(); assert.equal(generation.current(second),false);
});
test('bounding sphere fits both perspective axes at small/default/wide safe rectangles', () => {
  for(const [w,h] of [[290,340],[740,480],[1040,520],[290,280],[414,480]]) {
    const f=frameScene(w,h,2,'balanced');
    const half=Math.min(f.fov*Math.PI/360,Math.atan(Math.tan(f.fov*Math.PI/360)*f.aspect));
    assert.ok(Math.sin(half)*f.distance>sceneFitRadius);
    assert.equal(f.dpr,1.25); assert.ok(f.far>f.distance+sceneFitRadius);
  }
  assert.equal(frameScene(100,100,2,'high').dpr,1.5);
  assert.equal(frameScene(100,100,1,'high').dpr,1);
});
test('hidden or unfocused scenes never draw; reduced motion and OFF only permit static draws', () => {
  for(const [v,f] of [[false,true],[true,false],[false,false]]) assert.deepEqual(renderPolicy(v,f,true,false),{draw:false,animate:false});
  assert.deepEqual(renderPolicy(true,true,true,true),{draw:true,animate:false});
  assert.deepEqual(renderPolicy(true,true,false,false),{draw:true,animate:false});
  assert.deepEqual(renderPolicy(true,true,true,false),{draw:true,animate:true});
});

test('reference framing reproduces the reviewed baseline camera distance for fair comparisons',()=>{
  for(const [w,h] of [[756,410],[316,356],[1056,450]]) {
    const baseline=frameScene(w,h,2,'balanced',true);
    const half=Math.min(38*Math.PI/360,Math.atan(Math.tan(38*Math.PI/360)*w/h));
    assert.equal(baseline.distance,2.65/Math.sin(half)*1.08);
    assert.ok(frameScene(w,h,2,'balanced').distance<baseline.distance);
  }
});
