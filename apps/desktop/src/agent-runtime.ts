// WKWebView synthetic producer. No fetch, SDK, storage, credentials or tool execution.
export type Session = [instance: string, legacyEpoch: number, wireEpoch: number, generation: number,
  revision: number, highWater: number, closing: boolean];
export type Run = [run: number, request: number, attempt: number, generation: number, correlation: string,
  state: string, seq: number, terminals: number, text: string, error: string, cleanup: string];
export type View = [Session, Run | null, string | null];
export type Transport = (command: string, payload?: Record<string, unknown>) => Promise<View>;
export type Mode = 'normal' | 'queued' | 'idle' | 'run-timeout' | 'cleanup-timeout';
const namespace = 'agent.text.v1';

export function newer(previous: View | null, next: View): boolean {
  if (!previous) return true;
  // A component binds one host process. A restart requires a fresh bootstrap/client.
  return previous[0][0] === next[0][0] && next[0][2] >= previous[0][2] && next[0][4] >= previous[0][4];
}
function pause(ms: number, signal: AbortSignal): Promise<void> {
  return new Promise(resolve => {
    if (signal.aborted) { resolve(); return; }
    const done = () => { clearTimeout(timer); signal.removeEventListener('abort', done); resolve(); };
    const timer = setTimeout(done, ms);
    signal.addEventListener('abort', done, { once: true });
  });
}
export class AgentRuntime {
  view: View | null = null;
  private abort: AbortController | null = null;
  private producer: Promise<void> | null = null;
  private refreshing = false;
  private starting = false;
  private lastStart: Record<string, unknown> | null = null;
  private lastRun: View | null = null;
  onView: (view: View) => void = () => {};
  private transport: Transport;
  constructor(transport: Transport) { this.transport = transport; }
  reflect(next: View): View {
    if (newer(this.view, next)) { this.view = next; this.onView(next); }
    return next;
  }
  async bootstrap() { return this.reflect(await this.transport('agent_bootstrap')); }
  envelope(view: View, payload: object) { return { instance: view[0][0], epoch: view[0][1], payload }; }
  session(view = this.view!) { return { namespace, host_instance_id: view[0][0], ui_context_epoch: view[0][2] }; }
  async snapshot() {
    if (!this.view) throw new Error('Session unavailable');
    return this.reflect(await this.transport('agent_snapshot', this.envelope(this.view, this.session())));
  }
  async refresh() {
    if (this.refreshing || !this.view) return;
    this.refreshing = true;
    try {
      const value = await this.snapshot();
      if (value[0][6] || (value[1] && value[1][5] !== 'Running')) this.abort?.abort();
    } finally { this.refreshing = false; }
  }
  async runtime(view: View, action: string, data: object = {}, lease = view[2]) {
    return this.reflect(await this.transport('agent_runtime_input', this.envelope(view, { lease, action, data })));
  }
  frame(view: View, seq: number, type: string, payload: object) {
    const r = view[1]!;
    return { namespace, host_instance_id: view[0][0], run_id: r[0], request_id: r[1], attempt_id: r[2],
      session_generation: r[3], correlation_id: r[4], source_kind: 'synthetic', source_seq: seq, type, payload };
  }
  async admit(): Promise<View> {
    await this.snapshot();
    const v = this.view!;
    const body = this.envelope(v, { ...this.session(v), request_id: v[0][5] + 1, text: 'P2.1b synthetic fixture' });
    const result = this.reflect(await this.transport('agent_start', body));
    this.lastStart = body; this.lastRun = result;
    return result;
  }
  async start(mode: Mode) {
    if (this.starting || this.producer) throw new Error('Busy');
    this.starting = true;
    try {
      const view = await this.admit();
      if (mode === 'queued') return;
      await this.runtime(view, 'dispatch');
      const abort = new AbortController(); this.abort = abort;
      this.producer = this.produce(view, mode, abort.signal).finally(() => {
        if (this.abort === abort) { this.abort = null; this.producer = null; }
      });
    } finally { this.starting = false; }
  }
  private async produce(view: View, mode: Mode, signal: AbortSignal) {
    let seq = 0;
    try {
      if (mode === 'idle') await pause(60_000, signal);
      else if (mode === 'run-timeout') {
        for (let i = 0; i < 30 && !signal.aborted; i++) {
          await pause(1_000, signal);
          if (!signal.aborted) await this.runtime(view, 'frame', this.frame(view, ++seq, 'TextDelta', { text: '·' }));
        }
      } else {
        for (const text of ['Synthetic ', 'text ', '완료.']) {
          await pause(250, signal);
          if (signal.aborted) return;
          await this.runtime(view, 'frame', this.frame(view, ++seq, 'TextDelta', { text }));
        }
        if (!signal.aborted) await this.runtime(view, 'frame', this.frame(view, ++seq, 'Completed', {}));
      }
    } catch { /* Host rejection is observed through snapshot; never invent a terminal. */ }
    finally {
      // This finally runs only after timer disposal and the last awaited IPC settles.
      // Deliberately missing ACK fixture retains unconfirmed cleanup until explicit release.
      if (mode !== 'cleanup-timeout') {
        try { await this.runtime(view, 'release'); } catch { /* Revoked context/lease: native owner resolves loss. */ }
      }
      try { await this.snapshot(); } catch { /* Closing/replaced connection. No automatic resubmit. */ }
    }
  }
  async cancel() {
    const v = this.view; const r = v?.[1];
    if (!v || !r) return;
    const result = this.reflect(await this.transport('agent_cancel', this.envelope(v, { ...this.session(v),
      run_id: r[0], request_id: r[1], attempt_id: r[2], session_generation: r[3] })));
    if (['CancelRequested', 'Cancelled', 'Failed'].includes(result[1]![5])) this.abort?.abort();
    if (this.producer) await this.producer;
  }
  async release() {
    if (this.producer) throw new Error('Producer still active');
    if (this.lastRun) await this.runtime(this.lastRun, 'release');
  }
  async duplicateStart() {
    if (!this.lastStart) throw new Error('No fixture');
    return this.transport('agent_start', this.lastStart);
  }
  async oldSession(instance: string) {
    const v = this.view!;
    return this.transport('agent_snapshot', { instance, epoch: v[0][1], payload: { ...this.session(v), host_instance_id: instance } });
  }
  async probes(): Promise<string[]> {
    if (this.producer || this.starting) throw new Error('Busy');
    this.starting = true;
    const results: string[] = [];
    const rejected = async (name: string, action: () => Promise<unknown>, expected: string) => {
      try { await action(); throw new Error(`FAIL ${name}: accepted`); }
      catch (e) { if (String(e) !== expected) throw new Error(`FAIL ${name}`); results.push(`${name}: PASS (${expected})`); }
    };
    try {
      const v = await this.admit(); await this.runtime(v, 'dispatch');
      await rejected('duplicate Start', () => this.duplicateStart(), 'DuplicateRequest');
      const before = await this.snapshot();
      await rejected('wrong sender lease', () => this.runtime(v, 'frame', this.frame(v, 1, 'Completed', {}), 'forged'), 'InvalidIdentity');
      const wrong = { ...this.frame(v, 1, 'Completed', {}), attempt_id: v[1]![2] + 1 };
      await rejected('wrong attempt identity', () => this.runtime(v, 'frame', wrong), 'InvalidIdentity');
      if ((await this.snapshot())[0][4] !== before[0][4]) throw new Error('FAIL unauthorized mutation');
      await this.runtime(v, 'frame', this.frame(v, 1, 'TextDelta', { text: '<script>approved=true</script>' }));
      await rejected('duplicate Event', () => this.runtime(v, 'frame', this.frame(v, 1, 'TextDelta', { text: '<script>approved=true</script>' })), 'DuplicateEvent');
      await this.cancel(); await this.cancel();
      await rejected('late Delta', () => this.runtime(v, 'frame', this.frame(v, 2, 'TextDelta', { text: 'late' })), 'AlreadyTerminal');
      await rejected('late Completion', () => this.runtime(v, 'frame', this.frame(v, 3, 'Completed', {})), 'AlreadyTerminal');
      await this.runtime(v, 'release');
      await this.cancel();
      const final = await this.snapshot();
      if (final[1]?.[5] !== 'Cancelled' || final[1][7] !== 1 || final[1][10] !== 'Confirmed') throw new Error('FAIL cancel cleanup');
      results.push('duplicate Cancel / terminal immutability: PASS');
      const gap = await this.admit(); await this.runtime(gap, 'dispatch');
      await rejected('out-of-order source sequence', () => this.runtime(gap, 'frame', this.frame(gap, 2, 'Completed', {})), 'OutOfOrder');
      await this.runtime(gap, 'release');
      if ((await this.snapshot())[1]?.[9] !== 'ProtocolError') throw new Error('FAIL protocol terminal');
      const unsupported = await this.admit(); await this.runtime(unsupported, 'dispatch');
      await this.runtime(unsupported, 'frame', this.frame(unsupported, 1, 'Failed', { code: 'UNSUPPORTED_CONTENT' }));
      await this.runtime(unsupported, 'release');
      const failed = await this.snapshot();
      if (failed[1]?.[9] !== 'UnsupportedContent' || failed[1][7] !== 1 || failed[1][10] !== 'Confirmed') throw new Error('FAIL unsupported content');
      results.push('unsupported content / cleanup: PASS');
      return results;
    } finally { this.starting = false; }
  }
}
