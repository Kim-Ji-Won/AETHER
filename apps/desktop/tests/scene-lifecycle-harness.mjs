// Contract harness: executes compiled production modules, NOT ReactDOM, WKWebView or a GPU.
// React effect ordering, DOM events, ResizeObserver, RAF and WebGLRenderer are controlled doubles.
import { execFileSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import vm from 'node:vm';
import * as THREE from 'three';
import * as geometryUtils from 'three/addons/utils/BufferGeometryUtils.js';

export function compileScene() {
  const desktop = fileURLToPath(new URL('..', import.meta.url));
  const output = mkdtempSync(join(tmpdir(), 'aether-scene-contract-'));
  try {
    execFileSync(process.execPath, [join(desktop, 'node_modules/typescript/bin/tsc'),
      '--ignoreConfig', '--target', 'ES2022', '--module', 'ESNext', '--moduleResolution', 'Bundler',
      '--jsx', 'react-jsx', '--types', 'vite/client', '--strict', '--skipLibCheck', '--outDir', output,
      'src/scene/SceneHost.tsx', 'src/scene/useVisualState.ts'], { cwd: desktop, stdio: 'pipe' });
  } catch (error) { rmSync(output, { recursive: true }); throw error; }
  return { output, dispose: () => rmSync(output, { recursive: true }) };
}

class Events {
  listeners = new Map();
  addEventListener(name, fn) {
    if (!this.listeners.has(name)) this.listeners.set(name, new Set());
    this.listeners.get(name).add(fn);
  }
  removeEventListener(name, fn) { this.listeners.get(name)?.delete(fn); }
  callbacks(name) { return [...(this.listeners.get(name) ?? [])]; }
  emit(name) { for (const fn of this.callbacks(name)) fn({ preventDefault() {} }); }
  count() { return [...this.listeners.values()].reduce((n, set) => n + set.size, 0); }
}

export async function harness(output) {
  const raf = new Map(), canvases = [], observers = [], requests = [], resources = new Set();
  const document = new Events(), window = new Events(), media = new Events();
  const surface = { children: [], getBoundingClientRect: () => ({ width: 740, height: 480 }),
    append(canvas) { this.children.push(canvas); } };
  let nextRAF = 0, maxRAF = 0, draws = 0, resizeCalls = 0, live = 0, maxLive = 0;
  let failNextContext = false, restoreMode = 'normal';
  document.hidden = false; document.hasFocus = () => true; window.devicePixelRatio = 2; media.matches = false;
  document.createElement = name => {
    if (name !== 'canvas') throw new Error(`Unexpected DOM allocation: ${name}`);
    const canvas = new Events(); let lost = false;
    canvas.setAttribute = () => {};
    canvas.remove = () => { surface.children = surface.children.filter(c => c !== canvas); };
    canvas.restores = 0;
    const loss = {
      loseContext() { lost = true; canvas.emit('webglcontextlost'); },
      restoreContext() {
        canvas.restores++;
        if (restoreMode === 'silent') return; // Browser never delivers restored.
        lost = false; canvas.emit('webglcontextrestored');
      },
    };
    const gl = { getExtension: () => loss, isContextLost: () => lost,
      createProgram: () => ({}), createShader: () => ({}), shaderSource() {}, compileShader() {},
      getShaderParameter: () => true, attachShader() {}, linkProgram() {}, getProgramParameter: () => true,
      deleteProgram() {}, deleteShader() {}, VERTEX_SHADER: 1, FRAGMENT_SHADER: 2, COMPILE_STATUS: 3, LINK_STATUS: 4 };
    canvas.getContext = () => { if (failNextContext) { failNextContext = false; return null; } return gl; };
    canvases.push(canvas); return canvas;
  };
  class Observer {
    constructor(callback) { this.callback = callback; this.active = true; observers.push(this); }
    observe() { this.callback(); }
    disconnect() { this.active = false; }
  }
  class Renderer {
    info = { memory: { textures: 0 }, render: { calls: 4, triangles: 408 } };
    debug = {}; disposed = false;
    constructor() { live++; maxLive = Math.max(maxLive, live); }
    setClearColor() {} setPixelRatio() {}
    setSize() { resizeCalls++; }
    compile(scene) {
      scene.traverse(object => {
        for (const item of [object.geometry, object.material]) if (item && !resources.has(item)) {
          resources.add(item); item.addEventListener('dispose', () => resources.delete(item));
        }
      });
    }
    render() { draws++; }
    dispose() { if (!this.disposed) { this.disposed = true; live--; } }
  }
  // Minimal hook/effect driver. Strict replay is explicit; no claim of React scheduler coverage.
  let slots = [], cursor = 0, pending = [], dirty = false, component, args, tree, writes = 0;
  const react = {
    useRef(value) { const i = cursor++; return slots[i] ??= { current: value }; },
    useState(value) {
      const i = cursor++; slots[i] ??= { value };
      return [slots[i].value, next => { slots[i].value = typeof next === 'function' ? next(slots[i].value) : next; dirty = true; writes++; }];
    },
    useEffect(setup, deps) {
      const i = cursor++, old = slots[i];
      if (!old || deps.some((d, j) => !Object.is(d, old.deps[j]))) {
        const entry = { setup, deps, cleanup: old?.cleanup };
        slots[i] = entry; pending.push(entry);
      }
    },
  };
  function jsx(type, props) {
    if (props?.ref && ['foundation-surface','lab-canvas'].includes(props.className)) props.ref.current = surface;
    return { type, props };
  }
  const context = vm.createContext({ document, window, ResizeObserver: Observer, matchMedia: () => media,
    performance, console, crypto: globalThis.crypto,
    requestAnimationFrame(fn) { const id = ++nextRAF; raf.set(id, fn); maxRAF = Math.max(maxRAF, raf.size); return id; },
    cancelAnimationFrame(id) { raf.delete(id); },
    setTimeout() { throw new Error('Unexpected retry timer'); },
    setInterval() { throw new Error('Unexpected retry interval'); },
  });
  const mocks = {
    'three/addons/utils/BufferGeometryUtils.js': geometryUtils,
    react, 'react/jsx-runtime': { jsx, jsxs: jsx }, three: { ...THREE, WebGLRenderer: Renderer },
    '@tauri-apps/api/core': { invoke(command) {
      if (command !== 'get_runtime_session') throw new Error(`Forbidden command: ${command}`);
      return new Promise((resolve, reject) => requests.push({ resolve, reject, command }));
    } },
  };
  const modules = new Map();
  async function load(id) {
    if (modules.has(id)) return modules.get(id);
    let module;
    if (mocks[id] || id.endsWith('.css')) {
      const exports = mocks[id] ?? {};
      module = new vm.SyntheticModule(Object.keys(exports), function() {
        for (const [key, value] of Object.entries(exports)) this.setExport(key, value);
      }, { context, identifier: id });
    } else module = new vm.SourceTextModule(readFileSync(id, 'utf8'), { context, identifier: id });
    modules.set(id, module);
    await module.link((specifier, ref) => load(specifier.startsWith('.')
      ? join(dirname(ref.identifier), specifier + (specifier.endsWith('.css') ? '' : '.js')) : specifier));
    return module;
  }
  async function exported(file, name) {
    const module = await load(join(output, file + '.js')); await module.evaluate(); return module.namespace[name];
  }
  return {
    exported, canvases, observers, requests, surface, raf,
    setReduced(value) { media.matches=value; media.emit('change'); },
    setHidden(value) { document.hidden=value; document.emit('visibilitychange'); },
    tick(time) { const callbacks=[...raf.values()];raf.clear();callbacks.forEach(fn=>fn(time)); },
    failNextContext: () => { failNextContext = true; },
    silenceRestore: () => { restoreMode = 'silent'; },
    flush() {
      for (let i = 0; i < 20; i++) {
        dirty = false; cursor = 0; tree = component(args);
        const effects = pending; pending = [];
        for (const effect of effects) effect.cleanup?.();
        for (const effect of effects) effect.cleanup = effect.setup();
        if (!dirty) return tree;
      }
      throw new Error('Unbounded effect/retry loop');
    },
    mount(fn, props) { component = fn; args = props; return this.flush(); },
    replay() {
      const effects = slots.filter(s => s?.setup);
      for (const effect of effects) effect.cleanup?.();
      for (const effect of effects) effect.cleanup = effect.setup();
      return this.flush();
    },
    unmount() { for (const slot of slots) slot?.cleanup?.(); },
    get tree() { return tree; },
    metrics: () => ({ draws, resizeCalls, live, maxLive, maxRAF, pendingRAF: raf.size,
      resources: resources.size, writes, listeners: document.count() + window.count() + media.count() + canvases.reduce((n,c) => n+c.count(),0) }),
  };
}

export function nodes(tree, predicate) {
  if (!tree || typeof tree !== 'object') return [];
  if (Array.isArray(tree)) return tree.flatMap(node => nodes(node, predicate));
  return [...(predicate(tree) ? [tree] : []), ...nodes(tree.props?.children, predicate)];
}
