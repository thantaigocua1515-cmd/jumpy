// Jumpy web multiplayer relay + matchmaker (WebSocket).
//
// Speaks the same matchmaking flow as bones_framework's matchmaker protocol
// (RequestMatchmaking -> Accepted -> Success with seed + player_idx, then
// proxied messages between players), but over WebSocket instead of QUIC/UDP
// so browsers (the wasm build) can use it. The game itself still runs its
// deterministic GGRS netcode on top; this server only introduces players to
// each other and forwards their messages.
//
// Deploy: Railway (or any Node host). Listens on process.env.PORT.
// Health: GET / -> JSON status. WebSocket endpoint: same port, path /ws
// (any path works; /ws is the documented one).

'use strict';

const http = require('http');
const crypto = require('crypto');
const { WebSocketServer } = require('ws');

const PORT = process.env.PORT || 8931;
const MAX_MESSAGE_BYTES = 1 << 20; // 1 MiB guard, GGRS packets are tiny

const server = http.createServer((req, res) => {
  if (req.url === '/' || req.url === '/health') {
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({
      service: 'jumpy-relay',
      ok: true,
      clients: clients.size,
      waitingRooms: waiting.size,
      activeMatches: matches.size,
    }));
    return;
  }
  res.writeHead(404, { 'Content-Type': 'text/plain' });
  res.end('not found');
});

const wss = new WebSocketServer({ server, maxPayload: MAX_MESSAGE_BYTES });

/** All connected sockets: ws -> { id, state, roomKey, matchId, playerIdx } */
const clients = new Map();
/** Matchmaking queues: roomKey -> array of ws waiting */
const waiting = new Map();
/** Active matches: matchId -> array of ws indexed by player_idx */
const matches = new Map();

let nextClientId = 1;

function send(ws, obj) {
  if (ws.readyState === ws.OPEN) ws.send(JSON.stringify(obj));
}

function roomKeyOf(msg) {
  return [msg.game_id || 'jumpy', msg.player_count || 2, msg.match_data || ''].join('|');
}

function randomSeed() {
  // 64-bit seed as a decimal string (JSON numbers can't hold u64 exactly).
  return crypto.randomBytes(8).readBigUInt64BE(0).toString(10);
}

function startMatch(roomKey, players) {
  const matchId = crypto.randomUUID();
  const seed = randomSeed();
  matches.set(matchId, players);
  players.forEach((ws, idx) => {
    const info = clients.get(ws);
    if (info) { info.state = 'matched'; info.matchId = matchId; info.playerIdx = idx; }
    send(ws, { type: 'success', seed, player_idx: idx, player_count: players.length });
  });
  console.log(`[match] ${matchId} started (${players.length} players, room=${roomKey}, seed=${seed})`);
}

function leaveQueue(ws) {
  const info = clients.get(ws);
  if (!info || info.state !== 'waiting') return;
  const q = waiting.get(info.roomKey) || [];
  const i = q.indexOf(ws);
  if (i >= 0) q.splice(i, 1);
  if (q.length === 0) waiting.delete(info.roomKey);
  info.state = 'idle'; info.roomKey = null;
}

function leaveMatch(ws, notifyPeers) {
  const info = clients.get(ws);
  if (!info || info.state !== 'matched') return;
  const players = matches.get(info.matchId) || [];
  if (notifyPeers) {
    players.forEach((peer) => {
      if (peer !== ws) send(peer, { type: 'peer_disconnected', player_idx: info.playerIdx });
    });
  }
  matches.delete(info.matchId);
  info.state = 'idle'; info.matchId = null; info.playerIdx = null;
}

wss.on('connection', (ws) => {
  const id = nextClientId++;
  clients.set(ws, { id, state: 'idle', roomKey: null, matchId: null, playerIdx: null });
  console.log(`[conn] client ${id} connected (${clients.size} online)`);

  ws.on('message', (raw) => {
    let msg;
    try { msg = JSON.parse(raw.toString()); } catch { return; }
    const info = clients.get(ws);
    if (!info || typeof msg.type !== 'string') return;

    switch (msg.type) {
      case 'request_matchmaking': {
        if (info.state !== 'idle') return;
        const playerCount = Math.max(2, Math.min(4, msg.player_count | 0 || 2));
        const key = roomKeyOf({ ...msg, player_count: playerCount });
        info.state = 'waiting'; info.roomKey = key;
        if (!waiting.has(key)) waiting.set(key, []);
        const q = waiting.get(key);
        q.push(ws);
        send(ws, { type: 'accepted' });
        console.log(`[queue] client ${id} waiting in ${key} (${q.length}/${playerCount})`);
        if (q.length >= playerCount) {
          const players = q.splice(0, playerCount);
          if (q.length === 0) waiting.delete(key);
          startMatch(key, players);
        }
        break;
      }
      case 'stop_matchmaking': {
        leaveQueue(ws);
        break;
      }
      case 'proxy': {
        if (info.state !== 'matched') return;
        const players = matches.get(info.matchId) || [];
        const deliver = (peer, idx) => {
          if (peer && peer !== ws) {
            send(peer, { type: 'proxy_msg', from: info.playerIdx, reliable: msg.reliable !== false, data: msg.data });
          }
        };
        if (msg.to === 'all' || msg.to === undefined) players.forEach(deliver);
        else deliver(players[msg.to | 0], msg.to | 0);
        break;
      }
      case 'ping': {
        send(ws, { type: 'pong', t: msg.t });
        break;
      }
    }
  });

  const cleanup = () => {
    leaveQueue(ws);
    leaveMatch(ws, true);
    clients.delete(ws);
    console.log(`[conn] client ${id} disconnected (${clients.size} online)`);
  };
  ws.on('close', cleanup);
  ws.on('error', cleanup);
});

// Keepalive so proxies (Railway) don't drop idle sockets.
const heartbeat = setInterval(() => {
  wss.clients.forEach((ws) => { try { ws.ping(); } catch { /* already gone */ } });
}, 25000);
heartbeat.unref();

server.listen(PORT, () => {
  console.log(`jumpy-relay listening on port ${PORT}`);
});
