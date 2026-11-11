import { runFakeProvider, FIXTURE_INPUT } from './fake-provider.js';

const LIMIT = 512;
let buffer = Buffer.alloc(0);
let waiting;
const queued = [];
let ended = false;
function fail() {
  process.stderr.write('AETHER_SIDECAR_PROTOCOL_ERROR\n');
  process.exit(1);
}
process.stdin.on('data', chunk => {
  // Bound total buffered input, including messages sent while no read is pending.
  if (buffer.length + chunk.length > LIMIT) return fail();
  buffer = Buffer.concat([buffer, chunk]);
  let newline;
  while ((newline = buffer.indexOf(10)) !== -1) {
    const line = buffer.subarray(0, newline).toString('utf8');
    buffer = buffer.subarray(newline + 1);
    let value;
    try { value = JSON.parse(line); } catch { return fail(); }
    if (waiting) { const resolve = waiting; waiting = undefined; resolve(value); }
    else if (queued.length < 1) queued.push(value);
    else return fail();
  }
});
process.stdin.on('end', () => { ended = true; if (waiting) fail(); });
process.stdin.on('error', fail);
process.stdout.on('error', fail);
const read = () => queued.length ? Promise.resolve(queued.shift())
  : ended ? Promise.reject(new Error('EOF')) : new Promise(resolve => { waiting = resolve; });
const send = async value => {
  const line = JSON.stringify(value) + '\n';
  if (Buffer.byteLength(line) > LIMIT) throw new Error('Frame too large');
  await new Promise((resolve, reject) => process.stdout.write(line, error => error ? reject(error) : resolve()));
};
const keysAre = (object, keys) => object && typeof object === 'object' && !Array.isArray(object)
  && Object.keys(object).sort().join(',') === [...keys].sort().join(',');
const id = value => Number.isInteger(value) && value > 0 && value <= 0xffffffff;

try {
  const request = await read();
  if (!keysAre(request, ['version', 'type', 'run_id', 'request_id', 'generation', 'input'])
    || request.version !== 1 || request.type !== 'begin' || request.input !== FIXTURE_INPUT
    || !id(request.run_id) || !id(request.request_id) || !id(request.generation)) throw new Error('Invalid begin');
  await send({ version: 1, type: 'ready', pid: process.pid, node_version: process.version, arch: process.arch });
  let seq = 0;
  await runFakeProvider(request.input, async event => {
    await send({ version: 1, run_id: request.run_id, request_id: request.request_id,
      generation: request.generation, seq: ++seq, ...event });
    const ack = await read();
    if (!keysAre(ack, ['version', 'type', 'run_id', 'seq']) || ack.version !== 1
      || ack.type !== 'ack' || ack.run_id !== request.run_id || ack.seq !== seq) throw new Error('Invalid ACK');
  });
  await send({ version: 1, type: 'done', run_id: request.run_id, seq });
  process.stdin.destroy();
} catch { fail(); }
