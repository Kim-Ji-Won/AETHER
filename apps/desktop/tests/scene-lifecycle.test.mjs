import test, { after } from 'node:test';
import assert from 'node:assert/strict';
import { compileScene, harness, nodes } from './scene-lifecycle-harness.mjs';

// These are source-executing contract tests with controlled platform/hooks, not GUI tests.
const compiled = compileScene(); after(() => compiled.dispose());
const snapshot = { instance: 'owner', contextEpoch: 2, visualSequence: 1, source: 'actual',
  phase: 'offline', connection: 'disconnected', illuminationEnabled: false, inputLevel: null, outputLevel: null };
async function scene() {
  const h = await harness(compiled.output), reports = [];
  const SceneHost = await h.exported('SceneHost', 'SceneHost');
  const props = { snapshot, motion: true, panelOpen: false,
    options: { quality: 'balanced', angle: 0, fault: 'none' }, onReport: r => reports.push(r) };
  h.mount(SceneHost, props);
  return { h, reports, props };
}
function button(h, label) {
  const matches = nodes(h.tree, n => n.type === 'button' && n.props.children === label);
  assert.equal(matches.length, 1); assert.equal(!!matches[0].props.disabled, false);
  matches[0].props.onClick(); h.flush();
}
function fallback(h, expected) {
  assert.equal(nodes(h.tree, n => n.props?.className === 'foundation-fallback').length, expected ? 1 : 0);
}
function cleaned(h) {
  const m = h.metrics();
  for (const key of ['live', 'pendingRAF', 'resources', 'listeners']) assert.equal(m[key], 0, key);
  assert.equal(h.surface.children.length, 0);
  assert.ok(h.observers.every(o => !o.active));
  assert.ok(m.maxLive <= 1); assert.ok(m.maxRAF <= 1);
}

test('contract: missing restore event stays lost with fallback and no autonomous retry/RAF', async () => {
  const { h, reports, props } = await scene(); const before = JSON.stringify(props.snapshot);
  fallback(h, false); h.silenceRestore();
  button(h, 'Context loss 주입'); button(h, 'Context restore');
  for (let i = 0; i < 30; i++) { h.observers[0].callback(); h.flush(); }
  fallback(h, true); assert.equal(reports.at(-1).health, 'lost');
  assert.equal(h.metrics().pendingRAF, 0); assert.equal(h.canvases.length, 1);
  assert.equal(h.canvases[0].restores, 1); // Only the explicit user action, never an automatic retry.
  assert.equal(JSON.stringify(props.snapshot), before); assert.equal(h.requests.length, 0);
  h.unmount(); cleaned(h);
});

test('contract: restored event followed by replacement WebGL2 failure stays unavailable', async () => {
  const { h, reports } = await scene();
  button(h, 'Context loss 주입'); h.failNextContext(); button(h, 'Context restore');
  const failed = reports.at(-1);
  assert.equal(failed.health, 'unavailable'); assert.match(failed.reason, /WebGL2/);
  assert.equal(failed.liveRenderers, 0); assert.equal(failed.pendingRAF, 0); fallback(h, true);
  for (let i = 0; i < 30; i++) { h.observers.at(-1).callback(); h.flush(); }
  assert.equal(h.canvases.length, 2); assert.equal(h.requests.length, 0);
  assert.equal(reports.at(-1).health, 'unavailable');
  h.unmount(); cleaned(h);
});

test('contract: captured ResizeObserver callback delivered after unmount has no effects', async () => {
  const { h, reports } = await scene(); const queued = h.observers[0].callback;
  h.unmount(); const baseline = h.metrics(), count = reports.length;
  queued(); queued();
  assert.deepEqual(h.metrics(), baseline); assert.equal(reports.length, count); cleaned(h);
});

test('contract: captured restore event delivered after dispose cannot recreate or publish', async () => {
  const { h, reports } = await scene();
  button(h, 'Context loss 주입');
  const queued = h.canvases[0].callbacks('webglcontextrestored'); assert.equal(queued.length, 1);
  h.unmount(); const baseline = h.metrics(), count = reports.length;
  queued[0](); queued[0]();
  assert.deepEqual(h.metrics(), baseline); assert.equal(reports.length, count); cleaned(h);
});

test('contract: StrictMode-style effect replay invalidates old resize/restore/RAF callbacks', async () => {
  const { h, reports } = await scene();
  const oldResize = h.observers[0].callback;
  const oldRestore = h.canvases[0].callbacks('webglcontextrestored')[0];
  const oldRAF = [...h.raf.values()][0]; assert.equal(typeof oldRAF, 'function');
  button(h, 'Context loss 주입'); const firstGeneration = reports.at(-1).generation;
  h.replay();
  assert.ok(reports.at(-1).generation > firstGeneration); fallback(h, false);
  const baseline = h.metrics(), count = reports.length;
  oldResize(); oldRestore(); oldRAF(performance.now());
  assert.deepEqual(h.metrics(), baseline); assert.equal(reports.length, count);
  assert.equal(h.metrics().live, 1); assert.equal(h.metrics().pendingRAF, 1);
  h.unmount(); cleaned(h);
});

test('contract: old generation events cannot overwrite a successfully restored Scene', async () => {
  const { h, reports } = await scene(); const oldCanvas = h.canvases[0];
  const oldResize = h.observers[0].callback, oldRestore = oldCanvas.callbacks('webglcontextrestored')[0];
  button(h, 'Context loss 주입'); button(h, 'Context restore');
  const generation = reports.at(-1).generation, count = reports.length, baseline = h.metrics();
  oldResize(); oldRestore(); h.flush();
  assert.deepEqual(h.metrics(), baseline); assert.equal(reports.length, count);
  assert.equal(reports.at(-1).generation, generation); assert.equal(reports.at(-1).health, 'ready');
  assert.equal(h.canvases.length, 2); fallback(h, false);
  h.unmount(); cleaned(h);
});

test('contract: actual adapter hook rejects late prior setup session Promise and unmount result', async () => {
  const h = await harness(compiled.output), useVisualState = await h.exported('useVisualState', 'useVisualState');
  h.mount(() => useVisualState('', false)); h.replay();
  assert.equal(h.requests.length, 2);
  h.requests[1].resolve(['current', 8, true]); await Promise.resolve(); h.flush();
  assert.equal(h.tree.instance, 'current'); assert.equal(h.tree.contextEpoch, 8);
  const writes = h.metrics().writes;
  h.requests[0].resolve(['obsolete', 1, true]); await Promise.resolve(); h.flush();
  assert.equal(h.metrics().writes, writes); assert.equal(h.tree.instance, 'current');
  h.replay(); const late = h.requests.at(-1); h.unmount(); const afterUnmount = h.metrics().writes;
  late.resolve(['after-unmount', 9, true]); await Promise.resolve();
  assert.equal(h.metrics().writes, afterUnmount);
});
