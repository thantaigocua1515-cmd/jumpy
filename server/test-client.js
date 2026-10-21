// Automated smoke test for the jumpy-relay server.
// Starts the server on a test port, connects two WebSocket clients,
// verifies: accepted -> success (same seed, player_idx 0/1) -> proxy
// delivery both ways -> peer_disconnected on leave.
// Usage: node test-client.js   (expects deps installed: npm install)

'use strict';

process.env.PORT = '18741';
require('./server.js');

const WebSocket = require('ws');

const URL = 'ws://127.0.0.1:18741/ws';
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

function makeClient(name) {
  const ws = new WebSocket(URL);
  const c = { name, ws, messages: [], success: null };
  ws.on('message', (raw) => {
    const msg = JSON.parse(raw.toString());
    c.messages.push(msg);
    if (msg.type === 'success') c.success = msg;
  });
  return new Promise((resolve, reject) => {
    ws.on('open', () => resolve(c));
    ws.on('error', reject);
  });
}

function waitFor(client, type, timeoutMs = 3000) {
  return new Promise((resolve, reject) => {
    const t0 = Date.now();
    const iv = setInterval(() => {
      const hit = client.messages.find((m) => m.type === type);
      if (hit) { clearInterval(iv); resolve(hit); }
      else if (Date.now() - t0 > timeoutMs) { clearInterval(iv); reject(new Error(`${client.name}: timeout waiting for "${type}"`)); }
    }, 20);
  });
}

(async () => {
  let failures = 0;
  const check = (label, cond) => { console.log((cond ? 'PASS' : 'FAIL'), label); if (!cond) failures++; };

  const a = await makeClient('A');
  const b = await makeClient('B');

  a.ws.send(JSON.stringify({ type: 'request_matchmaking', game_id: 'jumpy', player_count: 2, match_data: '' }));
  await waitFor(a, 'accepted');
  check('A accepted into queue', true);

  b.ws.send(JSON.stringify({ type: 'request_matchmaking', game_id: 'jumpy', player_count: 2, match_data: '' }));
  const [sa, sb] = await Promise.all([waitFor(a, 'success'), waitFor(b, 'success')]);
  check('both clients matched', !!sa && !!sb);
  check('same seed for both players', sa.seed === sb.seed && typeof sa.seed === 'string' && sa.seed.length >= 10);
  check('player_idx are 0 and 1', [sa.player_idx, sb.player_idx].sort().join(',') === '0,1');

  const payload = Buffer.from('hello-ggrs-frame').toString('base64');
  a.ws.send(JSON.stringify({ type: 'proxy', to: 'all', reliable: true, data: payload }));
  const got = await waitFor(b, 'proxy_msg');
  check('proxy A->B delivered with data intact', got.data === payload && got.from === sa.player_idx);

  b.ws.send(JSON.stringify({ type: 'proxy', to: sa.player_idx, reliable: true, data: payload }));
  const gotBack = await waitFor(a, 'proxy_msg');
  check('proxy B->A (targeted) delivered', gotBack.data === payload && gotBack.from === sb.player_idx);

  a.ws.close();
  const left = await waitFor(b, 'peer_disconnected');
  check('peer_disconnected notified when A leaves', left.player_idx === sa.player_idx);

  b.ws.close();
  await sleep(100);
  console.log(failures === 0 ? 'ALL SERVER TESTS PASS' : failures + ' FAILURES');
  process.exit(failures ? 1 : 0);
})().catch((e) => { console.error('ERROR', e.message); process.exit(1); });
