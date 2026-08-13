/**
 * Live Windows plugin smoke test against the already-running crimsons-server.
 * Uses %APPDATA%\com.laoy.crimsons\auth.token for WS auth.
 * Optional: CRIMSON_TEST_EMAIL/PASSWORD + VITE_SUPABASE_* for AUTH_SESSION.
 */
import WebSocket from 'ws';
import fs from 'fs';
import path from 'path';

const APPDATA = process.env.APPDATA || '';
const DATA = path.join(APPDATA, 'com.laoy.crimsons');
const token = fs.readFileSync(path.join(DATA, 'auth.token'), 'utf8').trim();
const WS_URL = `ws://127.0.0.1:40510/?token=${encodeURIComponent(token)}`;

const results = [];
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

function record(name, status, detail = '') {
  results.push({ name, status, detail });
  console.log(`[${status === 'PASSED' ? 'PASS' : status === 'SKIPPED' ? 'SKIP' : 'FAIL'}] ${name}${detail ? ' — ' + detail : ''}`);
}

function openSocket(label) {
  return new Promise((resolve, reject) => {
    const inbox = [];
    const ws = new WebSocket(WS_URL, { headers: { Origin: 'http://tauri.localhost' } });
    const timer = setTimeout(() => reject(new Error(`${label} timeout`)), 10000);
    ws.on('open', () => {
      clearTimeout(timer);
      resolve({
        ws,
        inbox,
        send: (o) => ws.send(JSON.stringify(o)),
        clear: () => { inbox.length = 0; },
        waitFor: async (pred, timeout = 10000, name = 'msg') => {
          const start = Date.now();
          while (Date.now() - start < timeout) {
            const hit = inbox.find(pred);
            if (hit) return hit;
            await sleep(80);
          }
          throw new Error(`Timeout ${name}`);
        },
        expect: async (pred, timeout = 2500) => {
          const start = Date.now();
          while (Date.now() - start < timeout) {
            const hit = inbox.find(pred);
            if (hit) return hit;
            await sleep(50);
          }
          return null;
        },
        close: () => { try { ws.close(); } catch {} },
      });
    });
    ws.on('error', (e) => { clearTimeout(timer); reject(e); });
    ws.on('message', (d) => {
      try { inbox.push(JSON.parse(d.toString())); } catch {}
    });
  });
}

async function maybeAuth(app) {
  const url = (process.env.VITE_SUPABASE_URL || '').replace(/\/$/, '');
  const key = process.env.VITE_SUPABASE_ANON_KEY || '';
  const email = process.env.CRIMSON_TEST_EMAIL || '';
  const password = process.env.CRIMSON_TEST_PASSWORD || '';
  if (!url || !key || !email || !password) {
    record('AUTH_SESSION', 'SKIPPED', 'missing VITE_SUPABASE_* or test credentials in env');
    return false;
  }
  const resp = await fetch(`${url}/auth/v1/token?grant_type=password`, {
    method: 'POST',
    headers: { apikey: key, 'Content-Type': 'application/json' },
    body: JSON.stringify({ email, password }),
  });
  const json = await resp.json();
  if (!resp.ok) {
    record('AUTH_SESSION', 'FAILED', json.error_description || json.msg || String(resp.status));
    return false;
  }
  app.send({
    type: 'AUTH_SESSION',
    access_token: json.access_token,
    refresh_token: json.refresh_token || '',
  });
  await sleep(600);
  record('AUTH_SESSION', 'PASSED', json.user?.email || email);
  return true;
}

async function main() {
  console.log('=== Live Windows plugin smoke test ===');
  console.log('APPDATA data dir:', DATA);

  // Disk evidence
  const statePath = path.join(DATA, 'spotify_state.json');
  if (fs.existsSync(statePath)) {
    const st = JSON.parse(fs.readFileSync(statePath, 'utf8'));
    record('Spotify disk state present', 'PASSED', `track="${st.track_name || '?'}" playing=${st.is_playing}`);
  } else {
    record('Spotify disk state present', 'FAILED', 'spotify_state.json missing');
  }

  const app = await openSocket('app');
  record('WS connect with auth.token', 'PASSED');

  app.clear();
  app.send({ type: 'GET_VERSION' });
  try {
    const v = await app.waitFor((m) => m.type === 'SERVER_VERSION', 5000, 'SERVER_VERSION');
    record('GET_VERSION', 'PASSED', v.version);
  } catch (e) {
    record('GET_VERSION', 'FAILED', e.message);
  }

  const hb0 = await app.expect((m) => m.type === 'HEARTBEAT_STATUS', 3000);
  record(
    'HEARTBEAT_STATUS',
    hb0 ? 'PASSED' : 'FAILED',
    hb0 ? `server=${hb0.server} lol=${hb0.lol} discord=${hb0.discord}` : 'missing'
  );

  await maybeAuth(app);

  // Enable plugins
  for (const plugin of ['leagueOfLegends', 'spotify', 'discord']) {
    app.send({ type: 'TOGGLE_PLUGIN', plugin, enabled: true });
  }
  await sleep(800);
  const hb1 = await app.expect((m) => m.type === 'HEARTBEAT_STATUS', 3000);
  record(
    'Plugins enabled heartbeat',
    hb1 ? 'PASSED' : 'SKIPPED',
    hb1 ? `lol=${hb1.lol} discord=${hb1.discord}` : 'no hb'
  );

  // --- Spotify live ---
  // Wait briefly for background SPOTIFY_STATE broadcasts after enabling
  let before = await app.expect((m) => m.type === 'SPOTIFY_STATE' && (m.data?.track_name || m.track_name), 6000);
  if (!before) {
    app.clear();
    app.send({ type: 'SPOTIFY_COMMAND', endpoint: 'playpause', payload: {} });
    before = await app.expect((m) => m.type === 'SPOTIFY_STATE', 8000);
  }
  record(
    'Spotify state available',
    before ? 'PASSED' : 'FAILED',
    before
      ? `track="${before.data?.track_name || before.track_name || '?'}" playing=${before.data?.is_playing ?? before.is_playing}`
      : 'no SPOTIFY_STATE (premium/OAuth/device?)'
  );

  const playingBefore = before?.data?.is_playing ?? before?.is_playing ?? false;
  app.clear();
  app.send({ type: 'SPOTIFY_COMMAND', endpoint: 'playpause', payload: {} });
  const afterToggle = await app.expect((m) => {
    if (m.type !== 'SPOTIFY_STATE') return false;
    const p = m.data?.is_playing ?? m.is_playing;
    return p !== playingBefore;
  }, 10000);
  record(
    'Spotify play/pause toggle',
    afterToggle ? 'PASSED' : 'FAILED',
    afterToggle
      ? `playing ${playingBefore} -> ${afterToggle.data?.is_playing ?? afterToggle.is_playing}`
      : 'state did not change'
  );

  // StreamDock keyDown path
  const bridge = await openSocket('bridge');
  bridge.send({ type: 'REGISTER_STREAMDOCK_BRIDGE', port: 40510, uuid: 'live-windows-tester' });
  await sleep(500);
  app.clear();
  const playBefore = (await app.expect((m) => m.type === 'SPOTIFY_STATE', 1500))?.data?.is_playing;
  bridge.send({
    event: 'keyDown',
    context: 'live-spotify-pp',
    action: 'com.laoy.streamdock.spotify.playpause',
    payload: { settings: {} },
  });
  const playAfterMsg = await app.expect((m) => {
    if (m.type !== 'SPOTIFY_STATE') return false;
    if (typeof playBefore !== 'boolean') return true;
    return (m.data?.is_playing ?? m.is_playing) !== playBefore;
  }, 8000);
  record(
    'Spotify StreamDock keyDown playpause',
    playAfterMsg ? 'PASSED' : 'FAILED',
    playAfterMsg
      ? `playing -> ${playAfterMsg.data?.is_playing ?? playAfterMsg.is_playing}`
      : 'no state change'
  );

  // --- Discord live ---
  app.clear();
  app.send({ type: 'DISCORD_COMMAND', endpoint: 'toggleMute', payload: {} });
  const discordState = await app.expect((m) => m.type === 'DISCORD_STATE', 6000);
  record(
    'Discord toggleMute',
    discordState ? 'PASSED' : 'FAILED',
    discordState
      ? `muted=${discordState.data?.is_muted} deaf=${discordState.data?.is_deaf}`
      : 'no DISCORD_STATE (IPC/auth/premium?)'
  );

  bridge.send({
    event: 'keyDown',
    context: 'live-discord-mute',
    action: 'com.laoy.streamdock.discord.togglemute',
    payload: { settings: {} },
  });
  const discord2 = await app.expect((m) => m.type === 'DISCORD_STATE', 6000);
  record(
    'Discord StreamDock keyDown togglemute',
    discord2 ? 'PASSED' : 'FAILED',
    discord2 ? `muted=${discord2.data?.is_muted}` : 'no DISCORD_STATE'
  );

  // --- LoL ---
  app.clear();
  app.send({ type: 'SET_AUTO_ACCEPT', enabled: true });
  try {
    const st = await app.waitFor((m) => m.type === 'AUTO_ACCEPT_STATE', 4000);
    record('LoL SET_AUTO_ACCEPT', 'PASSED', `enabled=${st.enabled}`);
  } catch (e) {
    record('LoL SET_AUTO_ACCEPT', 'FAILED', e.message);
  }

  const hbLol = await app.expect((m) => m.type === 'HEARTBEAT_STATUS' && m.lol === true, 2000);
  if (hbLol) {
    record('LoL LCU connected', 'PASSED', 'heartbeat.lol=true');
  } else {
    // ask for another heartbeat by toggling
    app.clear();
    app.send({ type: 'TOGGLE_PLUGIN', plugin: 'leagueOfLegends', enabled: true });
    const hb = await app.expect((m) => m.type === 'HEARTBEAT_STATUS', 3000);
    record(
      'LoL LCU connected',
      hb?.lol ? 'PASSED' : 'FAILED',
      hb ? `heartbeat.lol=${hb.lol} (League Client must be fully open)` : 'no heartbeat'
    );
  }

  bridge.close();
  app.close();

  console.log('\n=== SUMMARY ===');
  for (const r of results) console.log(`${r.status.padEnd(8)} | ${r.name}${r.detail ? ' | ' + r.detail : ''}`);
  const failed = results.filter((r) => r.status === 'FAILED').length;
  process.exit(failed ? 1 : 0);
}

main().catch((e) => { console.error(e); process.exit(1); });
