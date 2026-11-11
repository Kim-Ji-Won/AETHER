import test from 'node:test';
import assert from 'node:assert/strict';
import { FIXTURE_INPUT, runFakeProvider } from '../src/fake-provider.ts';

test('fixed input emits ordered progress and exactly one terminal after acknowledgements', async () => {
  const events = [];
  let pending = false;
  await runFakeProvider(FIXTURE_INPUT, async event => {
    assert.equal(pending, false);
    pending = true;
    await new Promise(resolve => setTimeout(resolve, 5));
    events.push(event);
    pending = false;
  });
  assert.deepEqual(events, [
    { type: 'start', value: '' },
    { type: 'progress', value: '25' },
    { type: 'progress', value: '50' },
    { type: 'progress', value: '75' },
    { type: 'terminal', value: 'FAKE_OK' },
  ]);
});

test('unsupported input emits nothing', async () => {
  let calls = 0;
  await assert.rejects(runFakeProvider('unsupported fixture', async () => { calls++; }));
  assert.equal(calls, 0);
});

test('host rejection stops subsequent events and does not emit terminal', async () => {
  const events = [];
  await assert.rejects(runFakeProvider(FIXTURE_INPUT, async event => {
    events.push(event);
    if (event.type === 'progress') throw new Error('Host rejected event');
  }, 0));
  assert.deepEqual(events.map(event => event.type), ['start', 'progress']);
});

test('pause25 cancellation interrupts provider without late progress, repeated ten times', async () => {
  for (let trial = 0; trial < 10; trial++) {
    const controller = new AbortController(); const events = [];
    await assert.rejects(runFakeProvider(FIXTURE_INPUT, async event => {
      events.push(event);
      if (event.value === '25') controller.abort();
    }, 0, { signal: controller.signal, pause25: true }));
    assert.deepEqual(events.map(e => e.value), ['', '25']);
  }
});
test('pre-cancelled provider emits no start', async () => {
  const controller = new AbortController(); controller.abort(); let calls = 0;
  await assert.rejects(runFakeProvider(FIXTURE_INPUT, async () => { calls++; }, 0, { signal: controller.signal }));
  assert.equal(calls, 0);
});
