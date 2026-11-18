// Pure UI policy/controller contracts. Actual WKWebView evidence is recorded separately.
import test from 'node:test';
import assert from 'node:assert/strict';
import { AgentRuntime, inputProblem, newer } from '../src/agent-runtime.ts';
import { canSubmit, submitKey, displayState, safeError } from '../src/conversation-ui.ts';

const initial = () => [['host', 0, 1, 1, 1, 0, false], null, null];
const run = (state = 'Running', cleanup = 'Pending') => [['host', 0, 1, 1, 2, 1, false],
  [1, 1, 1, 1, 'corr', state, 1, 0, '<script>local only</script>', '', cleanup], 'lease'];
test('input is bounded by UTF-8 bytes including Korean and emoji, not UTF-16 length', () => {
  for (const text of ['', ' \n\t', '\u3000']) assert.ok(inputProblem(text));
  for (const text of ['a'.repeat(128), '한'.repeat(42) + 'ab', '😀'.repeat(32), '한글\nemoji 😀']) assert.equal(inputProblem(text), null);
  for (const text of ['a'.repeat(129), '한'.repeat(43), '😀'.repeat(32) + 'a']) assert.ok(inputProblem(text));
});
test('Enter submits once; Shift+Enter, IME, Safari 229/end ordering and repeat do not', () => {
  const enter = { key: 'Enter', shiftKey: false, isComposing: false, keyCode: 13, repeat: false };
  assert.equal(submitKey(enter, false, 51), true);
  for (const patch of [{key:'a'}, {shiftKey:true}, {isComposing:true}, {keyCode:229}, {repeat:true}]) assert.equal(submitKey({...enter,...patch}, false, 100), false);
  assert.equal(submitKey(enter, true, 100), false);
  assert.equal(submitKey(enter, false, 0), false);
});
test('submission requires connection, valid input, idle controller and confirmed Host cleanup', () => {
  assert.equal(canSubmit(initial(), 'connected', false, 'hello'), true);
  for (const state of ['Queued','Running','CancelRequested']) assert.equal(canSubmit(run(state), 'connected', false, 'x'), false);
  for (const state of ['Succeeded','Failed','Cancelled']) {
    for (const cleanup of ['Pending','Unconfirmed']) assert.equal(canSubmit(run(state, cleanup), 'connected', false, 'x'), false);
    for (const cleanup of ['Confirmed','NotRequired']) assert.equal(canSubmit(run(state, cleanup), 'connected', false, 'x'), true);
  }
  for (const connection of ['disconnected','reconnecting']) assert.equal(canSubmit(initial(), connection, false, 'x'), false);
  assert.equal(canSubmit(initial(), 'connected', true, 'x'), false);
  assert.equal(canSubmit(initial(), 'connected', false, ' '), false);
  const closing=initial(); closing[0][6]=true;
  assert.equal(canSubmit(closing, 'connected', false, 'x'), false);
});
test('projection rejects stale/duplicate revision, old identity/sequence and terminal rewriting', () => {
  const previous=run('Succeeded','Pending'); previous[1][7]=1;
  const next=structuredClone(previous); next[0][4]++; next[1][10]='Confirmed';
  assert.equal(newer(previous,next),true);
  for (const mutate of [v=>v[0][4]--, v=>v[0][0]='other', v=>v[0][2]=0,
    v=>v[0][3]=0, v=>v[0][5]=0, v=>v[1]=null, v=>v[1][2]++, v=>v[1][6]=0,
    v=>v[1][5]='Running', v=>v[1][8]='forged', v=>v[1][9]='forged']) {
    const bad=structuredClone(next); mutate(bad); assert.equal(newer(previous,bad),false);
  }
  const client=new AgentRuntime(async()=>next); client.reflect(next);
  assert.equal(client.reflect(previous),next);
});
test('one bootstrap and one Start across simultaneous consumers; original input reaches Host', async () => {
  let bootstraps=0, starts=0, unblock;
  const barrier=new Promise(resolve=>{unblock=resolve;});
  const client=new AgentRuntime(async(command, body)=>{
    if(command==='agent_bootstrap') {bootstraps++; await barrier; return initial();}
    if(command==='agent_start') {starts++; assert.equal(body.payload.text,'한글 😀'); await barrier; return run('Queued','NotRequired');}
    return initial();
  });
  const a=client.bootstrap(), b=client.bootstrap(); assert.equal(a,b); unblock(); await a;
  const first=client.start('queued','한글 😀'); await assert.rejects(client.start('queued','duplicate')); await first;
  assert.equal(bootstraps,1); assert.equal(starts,1); assert.equal(client.submitted.text,'한글 😀');
});
test('invalid or stale Start response never dispatches another run', async () => {
  let dispatches=0;
  const client=new AgentRuntime(async(command)=>{
    if(command==='agent_start') { const stale=run(); stale[1][1]=99; return stale; }
    if(command==='agent_runtime_input') dispatches++;
    return initial();
  });
  await client.bootstrap(); await assert.rejects(client.start('normal','x'));
  assert.equal(dispatches,0); assert.equal(client.submitted,null);
});
test('status and safe errors distinguish partial/failure/cleanup without echoing unknown error text', () => {
  assert.match(displayState(run('CancelRequested')), /cancel_requested/);
  assert.match(displayState(run('Failed','Unconfirmed')), /cleanup_unconfirmed/);
  assert.match(safeError('RuntimeLost'), /Reload/);
  assert.equal(safeError('<script>secret</script>').includes('secret'),false);
});
