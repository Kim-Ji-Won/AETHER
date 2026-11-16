// WKWebView synthetic producer. No fetch, SDK, storage, credentials or tool execution.
export type Session = [instance: string, legacyEpoch: number, wireEpoch: number, generation: number,
  revision: number, highWater: number, closing: boolean];
export type Run = [run: number, request: number, attempt: number, generation: number, correlation: string,
  state: string, seq: number, terminals: number, text: string, error: string, cleanup: string];
export type View = [Session, Run | null, string | null];
export type Transport = (command: string, payload?: Record<string, unknown>) => Promise<View>;
export type Mode = 'normal' | 'queued' | 'idle' | 'run-timeout' | 'cleanup-timeout';
// Matches the existing Rust diagnostic policy. No provider/product limit is inferred.
export const MAX_INPUT_BYTES = 128;
export function inputProblem(text: string): string | null {
  if (!text.trim()) return '텍스트를 입력하세요. 공백만 보낼 수 없습니다.';
  if (new TextEncoder().encode(text).length > MAX_INPUT_BYTES) return '128 UTF-8 bytes를 초과했습니다. 입력을 줄여 주세요.';
  return null;
}
export function activeRun(view: View | null): boolean {
  const run = view?.[1];
  return !!run && (!['Succeeded', 'Failed', 'Cancelled'].includes(run[5]) || !['Confirmed', 'NotRequired'].includes(run[10]));
}
const namespace = 'agent.text.v1';

export function newer(previous: View | null, next: View): boolean {
  if (!previous) return true;
  // A component binds one host process. A restart requires a fresh bootstrap/client.
  if (previous[0][0] !== next[0][0] || next[0][2] < previous[0][2] || next[0][3] < previous[0][3]
    || next[0][4] <= previous[0][4] || next[0][5] < previous[0][5] || (previous[0][6] && !next[0][6])) return false;
  const before = previous[1], after = next[1];
  if (before && (!after || after[0] < before[0])) return false;
  if (before && after?.[0] === before[0]) {
    if ([1, 2, 3, 4].some(i => after[i] !== before[i])) return false;
    if (after[6] < before[6]) return false;
    if (['Succeeded', 'Failed', 'Cancelled'].includes(before[5])
      && (after[5] !== before[5] || after[6] !== before[6] || after[7] !== before[7] || after[8] !== before[8] || after[9] !== before[9])) return false;
  }
  return true;
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
  private bootstrapping: Promise<View> | null = null;
  submitted: { run: number; text: string } | null = null;
  onChange: () => void = () => {};
  get busy() { return this.starting || this.producer !== null; }
  onView: (view: View) => void = () => {};
  private transport: Transport;
  constructor(transport: Transport) { this.transport = transport; }
  reflect(next: View): View {
    if (newer(this.view, next)) { this.view = next; this.onView(next); this.onChange(); }
    return this.view!; // Rejected responses must not drive producer/cancellation decisions.
  }
  bootstrap(): Promise<View> {
    if (this.bootstrapping) return this.bootstrapping;
    this.bootstrapping = this.transport('agent_bootstrap').then(value => this.reflect(value))
      .finally(() => { this.bootstrapping = null; });
    return this.bootstrapping;
  }
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
  async admit(text = 'P2.1b synthetic fixture'): Promise<View> {
    await this.snapshot();
    const v = this.view!;
    const requestId = v[0][5] + 1;
    const body = this.envelope(v, { ...this.session(v), request_id: requestId, text });
    const response = await this.transport('agent_start', body);
    const admitted = response[1];
    if (response[0][0] !== v[0][0] || response[0][2] !== v[0][2]
      || !admitted || admitted[1] !== requestId) throw new Error('Invalid admission');
    const result = this.reflect(response);
    if (result[1]?.[0] !== admitted[0]) throw new Error('Stale admission');
    this.lastStart = body; this.lastRun = result;
    this.submitted = { run: result[1]![0], text }; this.onChange();
    return result;
  }
  async start(mode: Mode, text = 'P2.1b synthetic fixture') {
    if (this.starting || this.producer) throw new Error('Busy');
    const problem = inputProblem(text);
    if (problem) throw new Error(problem);
    this.starting = true; this.onChange();
    try {
      const view = await this.admit(text);
      if (mode === 'queued') return;
      await this.runtime(view, 'dispatch');
      const abort = new AbortController(); this.abort = abort;
      this.producer = this.produce(view, mode, abort.signal).finally(() => {
        if (this.abort === abort) { this.abort = null; this.producer = null; this.onChange(); }
      });
    } finally { this.starting = false; this.onChange(); }
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
    this.starting = true; this.onChange();
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
    } finally { this.starting = false; this.onChange(); }
  }
}
