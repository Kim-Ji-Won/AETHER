import test from 'node:test';
import assert from 'node:assert/strict';
import { AgentRuntime, newer } from '../src/agent-runtime.ts';

const view = (revision, state = 'Running') => [['host', 0, 1, 1, revision, 0, false],
  [1, 1, 1, 1, 'corr', state, revision, state === 'Running' ? 0 : 1, 'synthetic', '', 'Confirmed'], 'lease'];
test('U01/U03: snapshots replace whole text and never regress revision or terminal', () => {
  const runtime = new AgentRuntime(async () => { throw new Error('No IPC expected'); });
  const terminal = view(5, 'Succeeded');
  runtime.reflect(terminal);
  runtime.reflect(view(3));
  assert.deepEqual(runtime.view, terminal);
  runtime.reflect(terminal);
  assert.equal(runtime.view[1][8], 'synthetic');
  const foreign = view(99); foreign[0][0] = 'another-host';
  assert.equal(newer(runtime.view, foreign), false);
});
test('U05: uncertain admission is not resent and creates no invented terminal', async () => {
  let starts = 0;
  const runtime = new AgentRuntime(async command => {
    if (command === 'agent_start') { starts++; throw new Error('connection lost'); }
    return view(1);
  });
  await runtime.bootstrap();
  await assert.rejects(runtime.start('normal'));
  assert.equal(starts, 1);
  assert.equal(runtime.view[1][5], 'Running');
});
test('queued cancellation sends no runtime dispatch or fake cleanup ACK', async () => {
  const commands = [];
  const runtime = new AgentRuntime(async command => {
    commands.push(command);
    return view(commands.length, command === 'agent_cancel' ? 'Cancelled' : 'Queued');
  });
  await runtime.bootstrap(); await runtime.start('queued'); await runtime.cancel();
  assert.equal(commands.includes('agent_runtime_input'), false);
});
test('normal producer awaits Completed before release; no duplicate text merge', async () => {
  const actions = [];
  let revision = 0;
  let released;
  const done = new Promise(resolve => { released = resolve; });
  const runtime = new AgentRuntime(async (command, body) => {
    if (command === 'agent_runtime_input') actions.push(body.payload.action === 'frame' ? body.payload.data.type : body.payload.action);
    if (actions.includes('release')) released();
    return view(++revision, actions.includes('release') ? 'Succeeded' : 'Running');
  });
  await runtime.bootstrap(); await runtime.start('normal');
  await done;
  assert.deepEqual(actions, ['dispatch', 'TextDelta', 'TextDelta', 'TextDelta', 'Completed', 'release']);
});
test('cancel drains the in-flight frame before confirming runtime cleanup', async () => {
  let enter, finish;
  const entered = new Promise(resolve => { enter = resolve; });
  const barrier = new Promise(resolve => { finish = resolve; });
  const actions = [];
  let cancelled = false, revision = 0;
  const runtime = new AgentRuntime(async (command, body) => {
    if (command === 'agent_cancel') cancelled = true;
    if (command === 'agent_runtime_input') {
      const action = body.payload.action; actions.push(action);
      if (action === 'frame') { enter(); await barrier; }
    }
    return view(++revision, cancelled ? 'CancelRequested' : 'Running');
  });
  await runtime.bootstrap(); await runtime.start('normal'); await entered;
  const cancellation = runtime.cancel();
  await Promise.resolve();
  assert.equal(actions.includes('release'), false);
  finish(); await cancellation;
  assert.deepEqual(actions, ['dispatch', 'frame', 'release']);
});
