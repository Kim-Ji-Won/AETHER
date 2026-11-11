export const FIXTURE_INPUT = '테스트 질문';
export type FakeEvent = { type: 'start' | 'progress' | 'terminal'; value: string };

// Each delay starts after the previous event is acknowledged by the host.
// This is a deterministic fixture, not a timing/performance benchmark.
export async function runFakeProvider(
  input: string,
  emit: (event: FakeEvent) => Promise<void>,
  intervalMs = 100,
) {
  if (input !== FIXTURE_INPUT) throw new Error('Unsupported fixture');
  await emit({ type: 'start', value: '' });
  for (const value of ['25', '50', '75']) {
    await new Promise<void>(resolve => setTimeout(resolve, intervalMs));
    await emit({ type: 'progress', value });
  }
  await new Promise<void>(resolve => setTimeout(resolve, intervalMs));
  await emit({ type: 'terminal', value: 'FAKE_OK' });
}
