// Real React server rendering; not a browser/IME/VoiceOver substitute.
import test, { after } from 'node:test';
import assert from 'node:assert/strict';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { createServer } from 'vite';
const server = await createServer({ server: { middlewareMode: true, hmr: false }, appType: 'custom' });
after(() => server.close());
const { ConversationResponse } = await server.ssrLoadModule('/src/ConversationResponse.tsx');
const { CommandDock } = await server.ssrLoadModule('/src/ShellParts.tsx');
const conversation = (state = 'Running', cleanup = 'Pending') => ({
  view: [['host', 0, 1, 1, 2, 1, false], [1, 1, 1, 1, 'corr', state, 1, 0, '<script>alert(1)</script>', '', cleanup], 'lease'],
  submitted: {run:1, text:'<img src=x onerror=alert(1)>'}, connection:'connected', busy:false,
  runtime: {cancel:async()=>{}}, reconnect:async()=>{},
});
test('input and output remain escaped React text; no executable HTML; synthetic label always visible', () => {
  const html=renderToStaticMarkup(React.createElement(ConversationResponse,{conversation:conversation()}));
  assert.ok(html.includes('&lt;script&gt;')); assert.ok(html.includes('&lt;img'));
  assert.ok(!html.includes('<script>')); assert.ok(!html.includes('<img'));
  for(const label of ['SYNTHETIC / LOCAL TEST','AI Provider 미연결','Partial Output','Cancel · 실행 취소']) assert.ok(html.includes(label));
});
test('terminal, pending, lost and uncertain connection are separate from completed response', () => {
  for(const state of ['Queued','Running','CancelRequested','Succeeded','Cancelled','Failed']) {
    const model=conversation(state,'Confirmed');
    const html=renderToStaticMarkup(React.createElement(ConversationResponse,{conversation:model}));
    assert.equal(html.includes('Partial Output'), state!=='Succeeded');
  }
  const model=conversation('Failed','Unconfirmed'); model.view[1][9]='RuntimeLost'; model.connection='disconnected';
  const html=renderToStaticMarkup(React.createElement(ConversationResponse,{conversation:model}));
  for(const label of ['cleanup_unconfirmed','RuntimeLost','Reload','disconnected','연결 다시 확인']) assert.ok(html.includes(label));
});
test('late cleanup confirmation preserves failure history without claiming admission remains blocked', () => {
  const model=conversation('Failed','Unconfirmed'); model.view[1][9]='CleanupUnconfirmed';
  assert.ok(renderToStaticMarkup(React.createElement(ConversationResponse,{conversation:model})).includes('새 실행 차단'));
  model.view[1][10]='Confirmed';
  const html=renderToStaticMarkup(React.createElement(ConversationResponse,{conversation:model}));
  assert.ok(html.includes('failed')); assert.ok(html.includes('CleanupUnconfirmed'));
  assert.ok(!html.includes('새 실행 차단')); assert.ok(html.includes('Partial Output'));
});
test('Dock preserves disabled microphone/attachment, bounds input and exposes accessible local Send', () => {
  const model=conversation('Succeeded','Confirmed');
  const html=renderToStaticMarkup(React.createElement(CommandDock,{draft:'한글 😀',setDraft(){},conversation:model}));
  assert.match(html, /disabled="" aria-label="첨부 준비 중"/);
  assert.match(html, /disabled="" aria-label="음성 입력 준비 중"/);
  assert.match(html, /maxLength="128"/i);
  assert.match(html, /class="send-button" aria-label="Synthetic 메시지 전송"/);
  assert.ok(html.includes('11/128 UTF-8 bytes'));
});
