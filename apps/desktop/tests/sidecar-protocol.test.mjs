import test from 'node:test';
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';
const node = fileURLToPath(new URL('../src-tauri/sidecar/generated/node', import.meta.url));
const entry = fileURLToPath(new URL('../src-tauri/sidecar/generated/entry.mjs', import.meta.url));
const begin = { version: 1, type: 'begin', run_id: 1, request_id: 1, generation: 1, input: '테스트 질문', mode: 'normal' };
function peer(initial, corruptAck = false) {
  return new Promise((resolve, reject) => {
    const child = spawn(node, [entry], { env: {}, stdio: ['pipe', 'pipe', 'pipe'] });
    let buffer = ''; const events = []; let diagnostic = ''; let timedOut = false;
    const timeout = setTimeout(() => { timedOut = true; child.kill(); }, 3000);
    child.on('error', reject);
    child.stdin.on('error', () => {});
    child.stderr.on('data', chunk => { diagnostic += chunk.toString(); });
    child.stdout.on('data', chunk => {
      buffer += chunk.toString(); let end;
      while ((end = buffer.indexOf('\n')) >= 0) {
        const frame = JSON.parse(buffer.slice(0, end)); buffer = buffer.slice(end + 1);
        if (['start', 'progress', 'terminal'].includes(frame.type)) {
          events.push(frame);
          const ack = { version: 1, type: 'ack', run_id: 1, seq: frame.seq };
          child.stdin.write(JSON.stringify(corruptAck ? corruptAck(ack) : ack) + '\n');
        }
      }
    });
    child.on('close', (code, signal) => {
      clearTimeout(timeout); resolve({ code, signal, events, diagnostic, timedOut });
    });
    if (initial !== undefined) child.stdin.end(initial);
    else child.stdin.write(JSON.stringify(begin) + '\n');
  });
}
test('real Node rejects malformed begin/UTF-8/framing and recovers with fresh peer', async () => {
  const cases = [Buffer.from('{bad}\n'), Buffer.from([255,10]), Buffer.from('{}'), Buffer.alloc(514, 120),
    Buffer.from(''), Buffer.from(JSON.stringify({...begin, extra:true})+'\n'),
    Buffer.from(JSON.stringify({...begin, version:2})+'\n')];
  for (const bytes of cases) {
    for (let n=0;n<5;n++) {
      const bad=await peer(bytes); assert.equal(bad.timedOut,false); assert.equal(bad.code,1);
      assert.equal(bad.events.length,0); assert.match(bad.diagnostic,/AETHER_SIDECAR_PROTOCOL_ERROR/);
    }
    const fresh=await peer(); assert.equal(fresh.code,0); assert.equal(fresh.events.length,5);
    assert.equal(fresh.events.at(-1).value,'FAKE_OK');
  }
});
test('real Node rejects invalid ACK, emits no subsequent progress, and fresh peers work', async () => {
  for (const corrupt of [a=>({...a,version:2}),a=>({...a,run_id:2}),a=>({...a,seq:2}),
    a=>({...a,type:'done'}),a=>({...a,extra:true}),a=>({version:a.version,type:a.type,seq:a.seq})]) {
    for (let n=0;n<5;n++) {
      const bad=await peer(undefined,corrupt); assert.equal(bad.timedOut,false); assert.equal(bad.code,1);
      assert.deepEqual(bad.events.map(e=>e.type),['start']);
    }
    const fresh=await peer(); assert.equal(fresh.code,0); assert.equal(fresh.events.length,5);
  }
});

test('host pipe loss interrupts the real Node long60 timer before next progress', async () => {
  for (let trial = 0; trial < 5; trial++) {
    const outcome = await new Promise((resolve, reject) => {
      const child = spawn(node, [entry], { env: {}, stdio: ['pipe', 'pipe', 'pipe'] });
      const events = []; let buffer = ''; let timedOut = false;
      const timeout = setTimeout(() => { timedOut = true; child.kill(); }, 2000);
      child.on('error', reject); child.stdin.on('error', () => {}); child.stderr.resume();
      child.stdout.on('data', chunk => {
        buffer += chunk; let end;
        while ((end = buffer.indexOf('\n')) >= 0) {
          const frame = JSON.parse(buffer.slice(0, end)); buffer = buffer.slice(end + 1);
          if (frame.type === 'start') {
            events.push(frame);
            child.stdin.write(JSON.stringify({version:1,type:'ack',run_id:1,seq:1})+'\n');
            // Host-side pipe closure, not a renderer mock or child kill.
            setTimeout(() => child.stdin.end(), 50);
          } else if (['progress','terminal'].includes(frame.type)) events.push(frame);
        }
      });
      child.on('close', code => { clearTimeout(timeout); resolve({code,timedOut,events}); });
      child.stdin.write(JSON.stringify({...begin,mode:'long60'})+'\n');
    });
    assert.equal(outcome.timedOut,false); assert.equal(outcome.code,1);
    assert.deepEqual(outcome.events.map(e=>e.type),['start']);
  }
});
