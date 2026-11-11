// Test-only fault peer. Not copied into bundle resources; FAULT is prepended by Rust tests.
let buffer = '';
let run;
let seq = 0;
const send = value => process.stdout.write(JSON.stringify(value) + '\n');
const frame = (n, type, value) => ({ version: 1, run_id: run.run_id, request_id: run.request_id,
  generation: run.generation, seq: n, type, value });
function next() {
  seq++;
  const events = [['start', ''], ['progress', '25'], ['progress', '50'], ['progress', '75'], ['terminal', 'FAKE_OK']];
  if (seq <= 5) send(frame(seq, ...events[seq - 1]));
  else {
    send({ version: 1, type: 'done', run_id: run.run_id, seq: FAULT === 'bad_done' ? 4 : 5 });
    if (FAULT === 'extra') send({ type: 'unexpected' });
    process.stdin.destroy();
  }
}
process.stdin.on('data', chunk => {
  buffer += chunk.toString();
  let end;
  while ((end = buffer.indexOf('\n')) >= 0) {
    const value = JSON.parse(buffer.slice(0, end)); buffer = buffer.slice(end + 1);
    if (!run) {
      run = value;
      if (FAULT === 'timeout') { setInterval(() => {}, 1000); return; }
      if (FAULT === 'eof') { process.exit(0); }
      if (FAULT === 'json') { process.stdout.write('{bad}\n'); return; }
      if (FAULT === 'utf8') { process.stdout.write(Buffer.from([255, 10])); return; }
      if (FAULT === 'no_lf') { process.stdout.write('{}'); process.stdin.destroy(); return; }
      if (FAULT === 'oversize') { process.stdout.write('x'.repeat(513) + '\n'); return; }
      send({ version: 1, type: 'ready', pid: process.pid, node_version: process.version, arch: process.arch });
      if (FAULT === 'unknown') { send(frame(1, 'unknown', '')); return; }
      if (FAULT === 'wrong_id') { send({ ...frame(1, 'start', ''), run_id: run.run_id + 1 }); return; }
      if (FAULT === 'generation') { send({ ...frame(1, 'start', ''), generation: 0 }); return; }
      next();
    } else if (value.type === 'ack') {
      if (seq === 2 && FAULT === 'duplicate') { send(frame(2, 'progress', '25')); return; }
      if (seq === 2 && FAULT === 'conflict') { send(frame(2, 'progress', '50')); return; }
      next();
    } else { process.exit(1); }
  }
});
