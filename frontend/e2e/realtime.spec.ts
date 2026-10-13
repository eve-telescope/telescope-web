// The realtime channel as the Tauri desktop clients (v0.2.1 to v0.4.0)
// used it: laravel-echo with pusher-js 8, private channel auth through
// `/broadcasting/auth` with the bearer token, and `listen('IntelEntryCreated')`,
// which only matches the event name `App\Events\IntelEntryCreated`. Runs
// in node against the composed stack, websocket and API through Caddy.
import { expect, test } from '@playwright/test';
import Echo from 'laravel-echo';
import Pusher from 'pusher-js';

import {
  PUSHER_KEY,
  baseUrl,
  createNetwork,
  createToken,
  createUser,
  grant,
  resetDatabase,
} from './helpers.ts';

test.beforeEach(() => resetDatabase());

function echoFor(token: string): Echo<'pusher'> {
  const url = new URL(baseUrl());
  return new Echo({
    broadcaster: 'pusher',
    Pusher,
    key: PUSHER_KEY,
    cluster: 'mt1',
    wsHost: url.hostname,
    wsPort: Number(url.port || (url.protocol === 'https:' ? 443 : 80)),
    wssPort: Number(url.port || 443),
    forceTLS: url.protocol === 'https:',
    enabledTransports: ['ws', 'wss'],
    disableStats: true,
    authEndpoint: `${baseUrl()}/broadcasting/auth`,
    auth: { headers: { Authorization: `Bearer ${token}`, Accept: 'application/json' } },
  });
}

/** Resolves with the payload of the first event with this name. */
function next<T>(channel: ReturnType<Echo<'pusher'>['private']>, event: string): Promise<T> {
  return new Promise((resolve) => {
    channel.listen(event, (payload: T) => resolve(payload));
  });
}

function subscribed(channel: ReturnType<Echo<'pusher'>['private']>): Promise<void> {
  return new Promise((resolve, reject) => {
    channel.subscribed(() => resolve());
    channel.error((error: unknown) => reject(new Error(JSON.stringify(error))));
  });
}

test('echo gets the entry and scan events of a network', async ({ request }) => {
  const owner = createUser('Owner Pilot');
  const network = createNetwork(owner);
  const token = createToken(owner);
  const echo = echoFor(token);
  try {
    const channel = echo.private(`intel-network.${network}`);
    await subscribed(channel);
    const headers = { Accept: 'application/json', Authorization: `Bearer ${token}` };

    const created = next<{ network_id: number; entry_id: number }>(channel, 'IntelEntryCreated');
    const entry = await request.post(`/api/networks/${network}/entries`, {
      headers,
      data: { entity_type: 'character', entity_id: 1234, entity_name: 'Target', label: 'HOSTILE' },
    });
    expect(entry.status()).toBe(201);
    const entryId = (await entry.json()).id as number;
    expect(await created).toEqual({ network_id: network, entry_id: entryId });

    const updated = next(channel, 'IntelEntryUpdated');
    await request.put(`/api/networks/${network}/entries/${entryId}`, {
      headers,
      data: { label: 'FRIENDLY' },
    });
    expect(await updated).toEqual({ network_id: network, entry_id: entryId });

    const deleted = next(channel, 'IntelEntryDeleted');
    await request.delete(`/api/networks/${network}/entries/${entryId}`, { headers });
    expect(await deleted).toEqual({ network_id: network, entry_id: entryId });

    const shared = next<{ network_id: number; scan_id: number }>(channel, 'ScanShared');
    const scan = await request.post(`/api/networks/${network}/scans`, {
      headers,
      data: { scan_type: 'local', raw_text: 'Pilot One' },
    });
    expect(await shared).toEqual({ network_id: network, scan_id: (await scan.json()).id });
  } finally {
    echo.disconnect();
  }
});

test('an outsider cannot subscribe', async () => {
  const owner = createUser('Owner Pilot');
  const outsider = createUser('Outsider Pilot');
  const network = createNetwork(owner);
  const echo = echoFor(createToken(outsider));
  try {
    const channel = echo.private(`intel-network.${network}`);
    await expect(subscribed(channel)).rejects.toThrow(/403/);
  } finally {
    echo.disconnect();
  }
});

test('a viewer hears what a member does', async ({ request }) => {
  const owner = createUser('Owner Pilot');
  const viewer = createUser('Viewer Pilot');
  const network = createNetwork(owner);
  grant(network, viewer, 'viewer');
  const echo = echoFor(createToken(viewer));
  try {
    const channel = echo.private(`intel-network.${network}`);
    await subscribed(channel);
    const created = next(channel, 'IntelEntryCreated');
    await request.post(`/api/networks/${network}/entries`, {
      headers: { Accept: 'application/json', Authorization: `Bearer ${createToken(owner)}` },
      data: { entity_type: 'alliance', entity_id: 99, entity_name: 'Enemies' },
    });
    expect(await created).toEqual({ network_id: network, entry_id: 1 });
  } finally {
    echo.disconnect();
  }
});

test('the preflight of the Tauri webview is answered', async ({ request }) => {
  const response = await request.fetch('/broadcasting/auth', {
    method: 'OPTIONS',
    headers: {
      Origin: 'tauri://localhost',
      'Access-Control-Request-Method': 'POST',
      'Access-Control-Request-Headers': 'authorization, content-type',
    },
  });
  expect(response.status()).toBe(204);
  expect(response.headers()['access-control-allow-origin']).toBe('*');
  expect(response.headers()['access-control-allow-headers']).toBe('authorization, content-type');
});
