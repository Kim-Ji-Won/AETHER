export const FIXTURE_INPUT = '테스트 질문';
export type FakeEvent = { type: 'start' | 'progress' | 'terminal'; value: string };
export type FixtureControl = { signal?: AbortSignal; pause25?: boolean };

function wait(ms: number, signal?: AbortSignal): Promise<void> {
  return new Promise((resolve, reject) => {
    if (signal?.aborted) { reject(new Error('Provider cancelled')); return; }
    const abort = () => { clearTimeout(timer); reject(new Error('Provider cancelled')); };
    const timer = setTimeout(() => { signal?.removeEventListener('abort', abort); resolve(); }, ms);
    signal?.addEventListener('abort', abort, { once: true });
  });
}

// Delays follow host ACKs. pause25 is a bounded synthetic cancellation barrier.
export async function runFakeProvider(
  input: string,
  emit: (event: FakeEvent) => Promise<void>,
  intervalMs = 100,
  control: FixtureControl = {},
) {
  if (input !== FIXTURE_INPUT) throw new Error('Unsupported fixture');
  const send = async (event: FakeEvent) => {
    if (control.signal?.aborted) throw new Error('Provider cancelled');
    await emit(event);
  };
  await send({ type: 'start', value: '' });
  for (const value of ['25', '50', '75']) {
    await wait(intervalMs, control.signal);
    await send({ type: 'progress', value });
    if (value === '25' && control.pause25) {
      await wait(300_000, control.signal);
      throw new Error('Barrier timeout');
    }
  }
  await wait(intervalMs, control.signal);
  await send({ type: 'terminal', value: 'FAKE_OK' });
}
