import * as THREE from 'three';
import { probeShaders } from './capabilities';
import { frameScene, renderPolicy, type Quality } from './framing';
import { VisualGate, type VisualSnapshot } from './visual-state';
import { createNeuralStructure } from './neural';

export type Health = 'ready' | 'lost' | 'unavailable' | 'disposed';
export type Fault = 'none' | 'shader' | 'fallback';
export interface SceneOptions { quality: Quality; angle: number; fault: Fault; motion: boolean; referenceFraming?: boolean }
export interface SceneReport {
  health: Health; reason: string; generation: number; frames: number; pendingRAF: number;
  width: number; height: number; dpr: number; calls: number; triangles: number;
  frameP95: number | null; cpuP95: number | null; samples: number;
  geometries: number; materials: number; textures: number;
  liveRenderers: number; created: number; disposed: number; lossSupported: boolean;
  gpuTime: 'UNAVAILABLE'; rss: 'UNAVAILABLE';
  neuralNodes: number; neuralConnections: number; geometryBytes: number;
}
const audit = { generation: 0, live: 0, created: 0, disposed: 0 };
function p95(values: number[]) {
  return values.length ? [...values].sort((a,b) => a-b)[Math.ceil(values.length * .95) - 1] : null;
}

// No IPC, runtime commands, timers, external assets or audio in this module.
export function createFoundation(canvas: HTMLCanvasElement, initial: VisualSnapshot, options: SceneOptions,
  report: (value: SceneReport) => void, restored: () => void) {
  const generation = ++audit.generation;
  const gate = new VisualGate([initial.instance, initial.contextEpoch, true]);
  let dead = false, lost = false, raf = 0, frames = 0, last = 0, next = 0;
  let health: Health = 'unavailable', reason = '초기화 중';
  let width = 1, height = 1, dpr = 1, calls = 0, triangles = 0;
  let renderer: THREE.WebGLRenderer | null = null;
  let gl: WebGL2RenderingContext | null = null;
  let loss: WEBGL_lose_context | null = null;
  let motion = options.motion;
  const media = matchMedia('(prefers-reduced-motion: reduce)');
  const intervals: number[] = [], cpu: number[] = [];
  const geometries: THREE.BufferGeometry[] = [], materials: THREE.Material[] = [];
  const scene = new THREE.Scene(), camera = new THREE.PerspectiveCamera(38, 1, .1, 100);
  const group = new THREE.Group(); scene.add(group);
  let counted = false, available = false;
  let neural: ReturnType<typeof createNeuralStructure> | null = null;
  function stats(): SceneReport {
    return { health, reason, generation, frames, pendingRAF: raf ? 1 : 0, width, height, dpr, calls, triangles,
      frameP95: p95(intervals), cpuP95: p95(cpu), samples: intervals.length,
      geometries: geometries.length, materials: materials.length, textures: renderer?.info.memory.textures ?? 0,
      liveRenderers: audit.live, created: audit.created, disposed: audit.disposed,
      lossSupported: !!loss, gpuTime: 'UNAVAILABLE', rss: 'UNAVAILABLE',
      neuralNodes: neural?.nodeCount ?? 0, neuralConnections: neural?.connections ?? 0, geometryBytes: neural?.bufferBytes ?? 0 };
  }
  function stop() { if (raf) cancelAnimationFrame(raf); raf = 0; last = 0; next = 0; }
  function release() {
    stop(); neural?.invalidate(); neural=null;
    geometries.splice(0).forEach(g => g.dispose());
    materials.splice(0).forEach(m => m.dispose());
    scene.clear(); group.clear();
    renderer?.dispose(); renderer = null;
    if (counted) { audit.live--; audit.disposed++; counted = false; }
  }
  function fail(message: string) {
    if (dead) return;
    health = 'unavailable'; reason = message; release();
    if (gl && !gl.isContextLost()) loss?.loseContext();
    report(stats());
  }
  function policy() { return renderPolicy(!document.hidden && available, document.hasFocus(), motion, media.matches); }
  function draw(time: number) {
    if (dead || lost || health !== 'ready' || !renderer || !policy().draw) return;
    try {
      const start = performance.now();
      renderer.render(scene, camera);
      cpu.push(performance.now() - start); if (cpu.length > 2400) cpu.shift();
      if (last) { intervals.push(time-last); if (intervals.length > 2400) intervals.shift(); }
      last = time; frames++; calls = renderer.info.render.calls; triangles = renderer.info.render.triangles;
    } catch { fail('Renderer draw 실패 · DOM Shell 유지'); }
  }
  function tick(time: number) {
    raf = 0;
    if (dead || lost || health !== 'ready' || !policy().animate) { last = 0; return; }
    const period = 1000 / (options.quality === 'high' ? 60 : 30);
    if (!next || time + .75 >= next) {
      // Advance local presentation time only while drawing; never catch up after suspension.
      neural?.update(last ? (time-last)/1000 : 0);
      draw(time); next = time + period - (next ? Math.max(0, time-next) % period : 0);
    }
    if (!dead && health === 'ready' && policy().animate) raf = requestAnimationFrame(tick);
  }
  function refresh() {
    stop();
    if (dead || lost || health !== 'ready') return;
    const p = policy();
    reason = !p.draw ? '숨김 / 창 비활성 · 정지' : !p.animate ? 'Motion 정적 · 단발 렌더' : `${options.quality === 'high' ? 60 : 30}fps 목표`;
    if (p.draw) { draw(performance.now()); last = 0; if (p.animate) raf = requestAnimationFrame(tick); }
    report(stats());
  }
  function resize(w: number, h: number) {
    if (dead || lost || !renderer) return;
    available = w > 0 && h > 0;
    if (!available) { refresh(); return; }
    const f = frameScene(w, h, window.devicePixelRatio, options.quality, options.referenceFraming);
    width=f.width; height=f.height; dpr=f.dpr;
    renderer.setPixelRatio(dpr); renderer.setSize(width, height, false);
    camera.aspect=f.aspect; camera.near=f.near; camera.far=f.far;
    const angle=options.angle*Math.PI/180;
    camera.position.set(Math.sin(angle)*f.distance, f.distance*.19, Math.cos(angle)*f.distance);
    camera.lookAt(0, 0, 0); camera.updateProjectionMatrix(); refresh();
  }
  function update(s: VisualSnapshot, enabled: boolean) {
    if (dead) return;
    motion=enabled;
    if (gate.accept(s)) {
      neural?.setVisual(s);
    }
    refresh();
  }
  const onLost = (event: Event) => {
    event.preventDefault(); if (dead || health === 'unavailable') return;
    lost=true; stop(); health='lost'; reason='GPU context loss · DOM fallback'; report(stats());
  };
  const onRestored = () => { if (!dead && lost) restored(); };
  canvas.addEventListener('webglcontextlost', onLost);
  canvas.addEventListener('webglcontextrestored', onRestored);
  document.addEventListener('visibilitychange', refresh);
  window.addEventListener('focus', refresh); window.addEventListener('blur', refresh);
  media.addEventListener('change', refresh);
  try {
    if (options.fault === 'fallback') throw new Error('진단용 DOM fallback');
    gl = canvas.getContext('webgl2', { alpha: true, antialias: true, powerPreference: 'low-power' });
    if (!gl) throw new Error('WebGL2 사용 불가');
    loss = gl.getExtension('WEBGL_lose_context');
    probeShaders(gl, options.fault === 'shader');
    renderer = new THREE.WebGLRenderer({ canvas, context: gl, alpha: true, antialias: true });
    audit.created++; audit.live++; counted=true;
    renderer.outputColorSpace=THREE.SRGBColorSpace;
    renderer.toneMapping=THREE.ACESFilmicToneMapping; renderer.toneMappingExposure=1;
    renderer.setClearColor(0x07101a, 0);
    renderer.debug.onShaderError = () => { throw new Error('Material shader compile/link 실패'); };
    scene.add(new THREE.HemisphereLight(0xb5e7ff, 0x172331, 2));
    const key = new THREE.DirectionalLight(0xc1f9ff, 4); key.position.set(-3,4,5); scene.add(key);
    const rim = new THREE.DirectionalLight(0xada1ee, 2); rim.position.set(3,2,-3); scene.add(rim);
    neural=createNeuralStructure(group, {geometries,materials});
    renderer.compile(scene,camera);
    health='ready'; reason='WebGL2 · Shader compile/link 정상';
    update(initial, motion);
  } catch (error) { fail(error instanceof Error ? error.message : 'WebGL2 초기화 실패'); }
  return {
    stats, resize, update,
    lose: () => { if (!dead && health === 'ready') loss?.loseContext(); },
    restore: () => { if (!dead && lost) loss?.restoreContext(); },
    dispose: () => {
      if (dead) return;
      dead=true; release();
      canvas.removeEventListener('webglcontextlost',onLost); canvas.removeEventListener('webglcontextrestored',onRestored);
      document.removeEventListener('visibilitychange',refresh); window.removeEventListener('focus',refresh); window.removeEventListener('blur',refresh);
      media.removeEventListener('change',refresh);
      // This canvas is owned exclusively by this generation, never a shared WebKit context.
      if (gl && !gl.isContextLost()) loss?.loseContext();
      loss=null; gl=null; health='disposed'; reason='자원 및 listener 해제'; report(stats());
    },
  };
}
export type Foundation = ReturnType<typeof createFoundation>;
