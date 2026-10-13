// The route contract of the whole stack (Caddy, SvelteKit, Axum) that the
// PHP suite only touched with "GET / returns 200": pages, static files,
// the API and the websocket through the proxy, errors and redirects.
import { readFileSync } from 'node:fs';
import { expect, test } from '@playwright/test';

import { PUSHER_KEY, baseUrl, createShare, resetDatabase } from './helpers.ts';

test.beforeEach(() => resetDatabase());

// tests/Feature/ExampleTest.php "returns a successful response".
test('returns a successful response', async ({ request }) => {
  const response = await request.get('/');
  expect(response.status()).toBe(200);
});

test('redirects trailing slashes on pages', async ({ request }) => {
  // FIXES.md (deliberate changes): Laravel answered the page itself.
  createShare('ABCDEFGH', ['Pilot One']);
  const response = await request.get('/s/ABCDEFGH/', { maxRedirects: 0 });
  expect(response.status()).toBe(308);
  expect(new URL(response.headers()['location'], response.url()).pathname).toBe('/s/ABCDEFGH');
});

test('serves the static files the site had', async ({ request }) => {
  for (const [path, type] of [
    ['/robots.txt', 'text/plain'],
    ['/site.webmanifest', 'application/manifest+json'],
    ['/favicon.ico', 'image/x-icon'],
    ['/favicon-16.png', 'image/png'],
    ['/favicon-32.png', 'image/png'],
    ['/apple-touch-icon.png', 'image/png'],
    ['/icon-192.png', 'image/png'],
    ['/icon-512.png', 'image/png'],
    ['/icon-512-maskable.png', 'image/png'],
    ['/logo.png', 'image/png'],
    ['/app.png', 'image/png'],
    ['/og-image.png', 'image/png'],
  ]) {
    const response = await request.get(path);
    expect(response.status(), path).toBe(200);
    expect(response.headers()['content-type'], path).toContain(type);
    const file = readFileSync(new URL(`../static${path}`, import.meta.url));
    expect(Buffer.compare(await response.body(), file), path).toBe(0);
  }
  expect(await (await request.get('/robots.txt')).text()).toBe('User-agent: *\nDisallow:\n');
});

test('answers the not found page for an unknown page and share', async ({ page }) => {
  for (const path of ['/nope', '/s/NOPE0000']) {
    const response = await page.goto(path);
    expect(response?.status(), path).toBe(404);
    await expect(page).toHaveTitle('Not Found');
    await expect(page.locator('body')).toContainText('404');
  }
});

test('reloads tabs left open on the Laravel site', async ({ request }) => {
  // FIXES.md (deliberate changes).
  const response = await request.get('/', {
    headers: { 'X-Inertia': 'true', 'X-Inertia-Version': 'old' },
    maxRedirects: 0,
  });
  expect(response.status()).toBe(409);
  expect(response.headers()['x-inertia-location']).toBe(`${baseUrl()}/`);
});

test('sends the headers nginx sent', async ({ request }) => {
  const response = await request.get('/');
  expect(response.headers()['x-frame-options']).toBe('SAMEORIGIN');
  expect(response.headers()['x-content-type-options']).toBe('nosniff');
  expect(response.headers()['x-xss-protection']).toBe('1; mode=block');
});

test('serves the health checks', async ({ request }) => {
  const up = await request.get('/up');
  expect(up.status()).toBe(200);
  expect(await up.text()).toContain('Application up');
  expect(await (await request.get('/api/health')).json()).toEqual({
    status: 'ok',
    database: 'ok',
  });
});

test('serves the API through the proxy', async ({ request }) => {
  createShare('ABCDEFGH', ['Pilot One', 'Pilot Two'], 3);
  const response = await request.get('/api/share/abcdefgh', {
    headers: { Accept: 'application/json', Origin: 'tauri://localhost' },
  });
  expect(response.status()).toBe(200);
  expect(response.headers()['access-control-allow-origin']).toBe('*');
  expect(await response.json()).toEqual({
    code: 'ABCDEFGH',
    pilots: ['Pilot One', 'Pilot Two'],
    pilotCount: 2,
  });
  const guest = await request.get('/api/networks/', { headers: { Accept: 'application/json' } });
  expect(guest.status()).toBe(401);
  expect(await guest.json()).toEqual({ message: 'Unauthenticated.' });
});

test('sends the EVE login to EVE SSO', async ({ request }) => {
  const response = await request.get('/eve?desktop', { maxRedirects: 0 });
  expect(response.status()).toBe(302);
  const location = new URL(response.headers()['location']);
  expect(location.origin + location.pathname).toBe(
    'https://login.eveonline.com/v2/oauth/authorize/',
  );
  expect(location.searchParams.get('response_type')).toBe('code');
  expect(location.searchParams.get('state')).toMatch(/^[A-Za-z0-9]{40}$/);
});

test('answers the csrf cookie route without cookies', async ({ request }) => {
  const response = await request.get('/sanctum/csrf-cookie');
  expect(response.status()).toBe(204);
  expect(response.headers()['set-cookie']).toBeUndefined();
});

test('accepts websockets at /app/<key> through the proxy', async () => {
  const url = `${baseUrl().replace(/^http/, 'ws')}/app/${PUSHER_KEY}?protocol=7&client=js&version=8.4.0&flash=false`;
  const socket = new WebSocket(url);
  const first = await new Promise<string>((resolve, reject) => {
    socket.addEventListener('message', (message) => resolve(String(message.data)), {
      once: true,
    });
    socket.addEventListener('error', () => reject(new Error('the websocket failed')), {
      once: true,
    });
  });
  socket.close();
  const frame = JSON.parse(first) as { event: string; data: string };
  expect(frame.event).toBe('pusher:connection_established');
  expect(JSON.parse(frame.data)).toEqual({
    socket_id: expect.stringMatching(/^\d+\.\d+$/),
    activity_timeout: 30,
  });
});
