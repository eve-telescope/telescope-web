// Records Reverb's Pusher protocol frames from the Laravel reference
// (`run.sh start`, Reverb on ws://127.0.0.1:8319, the app on :8318) into
// tests/fixtures/golden/pusher/<name>.json, for the websocket suites to
// replay against the Rust server byte for byte.
//
//   node scripts/laravel-reference/pusher.mjs [name-prefix]
//
// Reverb pings subscribed connections that were silent for its ping
// interval, on a 60 second timer, and drops them when they are still
// silent at the next tick; record `server_ping` with `run.sh start 3` (a
// 3 second interval) and the rest with the default:
//
//   node scripts/laravel-reference/pusher.mjs server_ping
//
// Steps in a fixture: `connect` (a path), `send` (text; `<auth:...>`
// stands for the signature of the client's socket id and a channel),
// `receive` (text; `<socket_id:x>` stands for client x's socket id),
// `silent` (nothing within the given milliseconds), `closed` (the close
// code and reason, 1006 when the connection dropped without a close
// frame) and `http` (a request to the app, as a desktop client sends it).
import { execFileSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const repo = join(dirname(fileURLToPath(import.meta.url)), '..', '..');
const HTTP = 'http://127.0.0.1:8318';
const WS = 'ws://127.0.0.1:8319';
const KEY = '04pcwy13bvcyjoio6mf6';
const OUT = join(repo, 'tests', 'fixtures', 'golden', 'pusher');
const dataset = JSON.parse(readFileSync(join(repo, 'tests', 'fixtures', 'dataset.json'), 'utf8'));
const tokens = Object.fromEntries(dataset.personal_access_tokens.map((t) => [t.id, t._plain]));

function reset() {
  const sql = execFileSync('python3', ['-I', join(repo, 'scripts', 'laravel-reference', 'record.py'), '--print-reset']);
  execFileSync(
    'docker',
    ['exec', '-i', 'telescope-laravel-mysql', 'mysql', '-uroot', '-ptelescope', '--default-character-set=utf8mb4', 'laravel'],
    { input: sql, stdio: ['pipe', 'ignore', 'ignore'] },
  );
}

class Client {
  constructor(name, path) {
    this.name = name;
    this.queue = [];
    this.waiting = null;
    this.socketId = null;
    this.ws = new WebSocket(WS + path);
    this.ws.onmessage = (message) => this.push({ type: 'text', data: String(message.data) });
    this.ws.onclose = (event) => this.push({ type: 'closed', code: event.code, reason: event.reason });
    this.ws.onerror = () => {};
  }

  push(item) {
    if (this.waiting) {
      const resolve = this.waiting;
      this.waiting = null;
      resolve(item);
    } else {
      this.queue.push(item);
    }
  }

  next(timeout) {
    if (this.queue.length) return Promise.resolve(this.queue.shift());
    return new Promise((resolve) => {
      const timer = setTimeout(() => {
        this.waiting = null;
        resolve({ type: 'silent' });
      }, timeout);
      this.waiting = (item) => {
        clearTimeout(timer);
        resolve(item);
      };
    });
  }
}

class Recording {
  constructor(name) {
    this.name = name;
    this.steps = [];
    this.clients = {};
  }

  normalize(text) {
    let result = text;
    for (const client of Object.values(this.clients)) {
      if (client.socketId) result = result.split(client.socketId).join(`<socket_id:${client.name}>`);
    }
    return result;
  }

  async connect(name, path = `/app/${KEY}?protocol=7&client=js&version=8.4.0&flash=false`) {
    const client = new Client(name, path);
    this.clients[name] = client;
    this.steps.push({ client: name, connect: path });
    const first = await this.receive(name);
    if (first.type === 'text') {
      const frame = JSON.parse(first.data);
      if (frame.event === 'pusher:connection_established') {
        client.socketId = JSON.parse(frame.data).socket_id;
        // Recorded again now that the socket id is known.
        this.steps[this.steps.length - 1] = { client: name, receive: this.normalize(first.data) };
      }
    }
    return client;
  }

  async receive(name, timeout = 3000) {
    const item = await this.clients[name].next(timeout);
    if (item.type === 'text') {
      this.steps.push({ client: name, receive: this.normalize(item.data) });
    } else if (item.type === 'closed') {
      this.steps.push({ client: name, closed: { code: item.code, reason: item.reason } });
    } else {
      this.steps.push({ client: name, silent: timeout });
    }
    return item;
  }

  send(name, text, template = text) {
    this.clients[name].ws.send(text);
    this.steps.push({ client: name, send: this.normalize(template) });
  }

  /** A private channel signature from /broadcasting/auth, as a client gets it. */
  async auth(name, channel, token = 1) {
    const response = await fetch(`${HTTP}/broadcasting/auth`, {
      method: 'POST',
      headers: {
        Accept: 'application/json',
        Authorization: `Bearer ${tokens[token]}`,
        'Content-Type': 'application/x-www-form-urlencoded',
      },
      body: new URLSearchParams({ socket_id: this.clients[name].socketId, channel_name: channel }),
    });
    return response.json();
  }

  async subscribe(name, channel, token = 1) {
    const { auth, channel_data } = await this.auth(name, channel, token);
    const data = channel_data ? { auth, channel_data, channel } : { auth, channel };
    const template = channel_data
      ? { auth: `<auth:${channel}>`, channel_data, channel }
      : { auth: `<auth:${channel}>`, channel };
    this.send(
      name,
      JSON.stringify({ event: 'pusher:subscribe', data }),
      JSON.stringify({ event: 'pusher:subscribe', data: template }),
    );
  }

  async http(method, path, token, body) {
    const response = await fetch(HTTP + path, {
      method,
      headers: {
        Accept: 'application/json',
        'Content-Type': 'application/json',
        Authorization: `Bearer ${tokens[token]}`,
      },
      body: body === undefined ? undefined : JSON.stringify(body),
    });
    this.steps.push({ http: { method, path, token, body: body ?? null }, status: response.status });
  }

  save() {
    for (const client of Object.values(this.clients)) client.ws.close();
    mkdirSync(OUT, { recursive: true });
    writeFileSync(join(OUT, `${this.name}.json`), JSON.stringify({ name: this.name, steps: this.steps }, null, 2) + '\n');
  }
}

const scenarios = {
  async connect(r) {
    await r.connect('a');
    await r.receive('a', 1000);
  },
  async connect_without_query(r) {
    await r.connect('a', `/app/${KEY}`);
  },
  async wrong_key(r) {
    await r.connect('a', '/app/wrongkey?protocol=7&client=js&version=8.4.0&flash=false');
    await r.receive('a');
  },
  async ping(r) {
    await r.connect('a');
    r.send('a', '{"event":"pusher:ping","data":{}}');
    await r.receive('a');
    r.send('a', '{"event":"pusher:ping"}');
    await r.receive('a');
    r.send('a', '{"event":"pusher:pong","data":{}}');
    await r.receive('a', 1000);
  },
  async subscribe_private(r) {
    await r.connect('a');
    await r.subscribe('a', 'private-intel-network.1');
    await r.receive('a');
    await r.receive('a', 1000);
  },
  async subscribe_private_twice(r) {
    await r.connect('a');
    await r.subscribe('a', 'private-intel-network.1');
    await r.receive('a');
    await r.subscribe('a', 'private-intel-network.1');
    await r.receive('a');
  },
  async subscribe_private_bad_auth(r) {
    await r.connect('a');
    r.send('a', '{"event":"pusher:subscribe","data":{"auth":"04pcwy13bvcyjoio6mf6:bad","channel":"private-intel-network.1"}}');
    await r.receive('a');
    await r.receive('a', 1000);
  },
  async subscribe_private_without_auth(r) {
    await r.connect('a');
    r.send('a', '{"event":"pusher:subscribe","data":{"channel":"private-intel-network.1"}}');
    await r.receive('a');
  },
  async subscribe_private_other_socket(r) {
    await r.connect('a');
    await r.connect('b');
    const { auth } = await r.auth('b', 'private-intel-network.1');
    r.send(
      'a',
      JSON.stringify({ event: 'pusher:subscribe', data: { auth, channel: 'private-intel-network.1' } }),
      JSON.stringify({ event: 'pusher:subscribe', data: { auth: '<auth:b:private-intel-network.1>', channel: 'private-intel-network.1' } }),
    );
    await r.receive('a');
  },
  async subscribe_public(r) {
    await r.connect('a');
    r.send('a', '{"event":"pusher:subscribe","data":{"channel":"public-updates"}}');
    await r.receive('a');
  },
  async subscribe_presence(r) {
    await r.connect('a');
    await r.subscribe('a', 'presence-intel-network.1');
    await r.receive('a');
  },
  async subscribe_presence_bad_auth(r) {
    await r.connect('a');
    r.send('a', '{"event":"pusher:subscribe","data":{"auth":"04pcwy13bvcyjoio6mf6:bad","channel_data":"{\\"user_id\\":1}","channel":"presence-intel-network.1"}}');
    await r.receive('a');
  },
  async subscribe_without_channel(r) {
    await r.connect('a');
    r.send('a', '{"event":"pusher:subscribe","data":{}}');
    await r.receive('a');
  },
  async subscribe_string_data(r) {
    await r.connect('a');
    r.send('a', '{"event":"pusher:subscribe","data":"{\\"channel\\":\\"public-updates\\"}"}');
    await r.receive('a');
  },
  async unsubscribe(r) {
    await r.connect('a');
    await r.subscribe('a', 'private-intel-network.1', 2);
    await r.receive('a');
    r.send('a', '{"event":"pusher:unsubscribe","data":{"channel":"private-intel-network.1"}}');
    await r.receive('a', 1000);
    await r.http('POST', '/api/networks/1/entries', 2, { entity_type: 'character', entity_id: 90001010, entity_name: 'New Target' });
    await r.receive('a', 1000);
  },
  async invalid_json(r) {
    await r.connect('a');
    r.send('a', 'not json');
    await r.receive('a');
    r.send('a', '{"event":"pusher:ping","data":{}}');
    await r.receive('a');
  },
  async unknown_event(r) {
    await r.connect('a');
    r.send('a', '{"event":"foo","data":{}}');
    await r.receive('a');
    r.send('a', '{"event":"pusher:foo","data":{}}');
    await r.receive('a');
    r.send('a', '{"data":{}}');
    await r.receive('a');
    r.send('a', '[1,2]');
    await r.receive('a');
  },
  async client_event(r) {
    await r.connect('a');
    await r.connect('b');
    await r.subscribe('a', 'private-intel-network.1', 1);
    await r.receive('a');
    await r.subscribe('b', 'private-intel-network.1', 2);
    await r.receive('b');
    r.send('a', '{"event":"client-typing","data":{"x":1},"channel":"private-intel-network.1"}');
    await r.receive('a', 1000);
    await r.receive('b', 1000);
  },
  async client_event_unsubscribed(r) {
    await r.connect('a');
    r.send('a', '{"event":"client-typing","data":{"x":1},"channel":"private-intel-network.1"}');
    await r.receive('a', 1000);
  },
  async oversized_message(r) {
    await r.connect('a');
    r.send('a', JSON.stringify({ event: 'pusher:ping', data: { pad: 'x'.repeat(20000) } }), '{"event":"pusher:ping","data":{"pad":"<20000 x>"}}');
    await r.receive('a');
    await r.receive('a');
  },
  async entry_events(r) {
    await r.connect('a');
    await r.connect('b');
    await r.subscribe('a', 'private-intel-network.1', 2);
    await r.receive('a');
    await r.subscribe('b', 'private-intel-network.2', 4);
    await r.receive('b');
    await r.http('POST', '/api/networks/1/entries', 2, { entity_type: 'character', entity_id: 90001010, entity_name: 'New Target', color: '#FF3B3B', label: 'HOSTILE' });
    await r.receive('a');
    await r.http('POST', '/api/networks/1/entries', 2, { entity_type: 'character', entity_id: 90001001, entity_name: 'Hostile Hank', label: 'UPSERTED' });
    await r.receive('a');
    await r.http('PUT', '/api/networks/1/entries/7', 2, { label: 'FRIENDLY' });
    await r.receive('a');
    await r.http('DELETE', '/api/networks/1/entries/7', 2);
    await r.receive('a');
    await r.receive('b', 1000);
  },
  async scan_event(r) {
    await r.connect('a');
    await r.subscribe('a', 'private-intel-network.1', 3);
    await r.receive('a');
    await r.http('POST', '/api/networks/1/scans', 2, { scan_type: 'local', raw_text: 'Pilot One\nPilot Two' });
    await r.receive('a');
  },
  async no_event_on_failure(r) {
    await r.connect('a');
    await r.subscribe('a', 'private-intel-network.1', 3);
    await r.receive('a');
    await r.http('POST', '/api/networks/1/entries', 3, { entity_type: 'character', entity_id: 1, entity_name: 'x' });
    await r.http('POST', '/api/networks/1/entries', 2, { entity_type: 'ship' });
    await r.receive('a', 1000);
  },
  async server_ping(r) {
    await r.connect('a');
    await r.subscribe('a', 'private-intel-network.1');
    await r.receive('a');
    const started = Date.now();
    const item = await r.receive('a', 70000);
    r.steps.push({ note: `after ${Math.round((Date.now() - started) / 1000)} s` });
    if (item.type === 'text') {
      const next = await r.receive('a', 70000);
      r.steps.push({ note: `after ${Math.round((Date.now() - started) / 1000)} s` });
      if (next.type === 'text') await r.receive('a', 5000);
    }
  },
  async server_ping_answered(r) {
    await r.connect('a');
    await r.subscribe('a', 'private-intel-network.1');
    await r.receive('a');
    const item = await r.receive('a', 70000);
    if (item.type === 'text') {
      r.send('a', '{"event":"pusher:pong","data":{}}');
      await r.receive('a', 65000);
    }
  },
  async server_ping_unsubscribed(r) {
    await r.connect('a');
    await r.receive('a', 130000);
  },
};

const prefix = process.argv[2] ?? '';
for (const [name, run] of Object.entries(scenarios)) {
  if (!name.startsWith(prefix)) continue;
  if (prefix === '' && name.startsWith('server_ping')) continue;
  reset();
  const recording = new Recording(name);
  await run(recording);
  recording.save();
  console.log(name, recording.steps.length);
}
process.exit(0);
