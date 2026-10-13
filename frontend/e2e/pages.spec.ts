// The three pages in a browser: the landing page, a share link and the
// desktop login's token page (through the demo login).
import { expect, test } from '@playwright/test';

import {
  assertSee,
  createShare,
  resetDatabase,
  seedDemo,
  shareViews,
  trackJavascriptErrors,
} from './helpers.ts';

test.beforeEach(() => resetDatabase());

test('the landing page shows the app and its downloads', async ({ page }) => {
  const done = trackJavascriptErrors(page);
  await page.goto('/');
  await expect(page).toHaveTitle('EVE Online Intel Tool | Telescope');
  await expect(page.getByRole('heading', { level: 1 })).toHaveText('Telescope');
  for (const feature of [
    'Local scans',
    'Danger score',
    'Pilot roles',
    'D-scan',
    'Intel networks',
    'Overlay and hotkey',
  ]) {
    await assertSee(page, feature);
  }
  for (const platform of ['macOS', 'Windows', 'Linux']) {
    await expect(page.getByRole('link', { name: new RegExp(platform) })).toHaveAttribute(
      'href',
      'https://github.com/eve-telescope/telescope-app/releases/latest',
    );
  }
  await expect(page.locator('body')).toContainText(/v\d+\.\d+\.\d+ • Open Source/);
  // The stack runs with the demo login.
  await expect(page.getByTestId('demo-login')).toBeVisible();
  // The starfield is placed in the browser.
  await expect(page.locator('[data-starfield] > div')).toHaveCount(100);
  done();
});

test('a share link lists the pilots and counts the visit once', async ({ page }) => {
  const done = trackJavascriptErrors(page);
  createShare('ABCDEFGH', ['Pilot One', 'Pilot Two', 'Pilot Three'], 3);
  await page.goto('/s/abcdefgh');
  await expect(page).toHaveTitle('3 Pilots Shared | Telescope');
  await assertSee(page, 'SHARED INTEL SCAN');
  await assertSee(page, '3 pilots in this scan');
  for (const pilot of ['Pilot One', 'Pilot Two', 'Pilot Three']) {
    await assertSee(page, pilot);
  }
  await expect(page.getByTestId('views')).toHaveText('4');
  await expect(page.locator('code')).toHaveText('ABCDEFGH');
  expect(shareViews('ABCDEFGH')).toBe(4);
  // The app did not answer the deep link: the page offers the download.
  await expect(page.getByRole('button', { name: 'TRY AGAIN' })).toBeVisible({ timeout: 5000 });
  await expect(page.getByRole('link', { name: 'DOWNLOAD APP' })).toHaveAttribute('href', '/');
  await page.reload();
  await expect(page.getByTestId('views')).toHaveText('5');
  expect(shareViews('ABCDEFGH')).toBe(5);
  done();
});

test('a long share shows the first 50 pilots', async ({ page }) => {
  createShare(
    'zyxW9876',
    Array.from({ length: 60 }, (_, i) => `Pilot ${String(i).padStart(3, '0')}`),
  );
  await page.goto('/s/zyxW9876');
  await expect(page).toHaveTitle('60 Pilots Shared | Telescope');
  await assertSee(page, 'Pilot 049');
  await expect(page.locator('body')).not.toContainText('Pilot 050');
  await assertSee(page, '+10 more');
});

test('copies the pilot names', async ({ page, context }) => {
  await context.grantPermissions(['clipboard-read', 'clipboard-write']);
  createShare('ABCDEFGH', ['Pilot One', 'Pilot Two']);
  await page.goto('/s/ABCDEFGH');
  await page.getByRole('button', { name: 'COPY PILOT NAMES' }).click();
  await expect(page.getByRole('button', { name: 'COPIED!' })).toBeVisible();
  expect(await page.evaluate(() => navigator.clipboard.readText())).toBe('Pilot One\nPilot Two');
});

test('the demo login shows a desktop token that works', async ({ page, context, request }) => {
  await context.grantPermissions(['clipboard-read', 'clipboard-write']);
  seedDemo();
  const done = trackJavascriptErrors(page);
  await page.goto('/demo-login?desktop');
  await expect(page).toHaveTitle('Authentication | Telescope');
  await assertSee(page, 'Authenticated as Demo Pilot');
  const token = (await page.locator('code').textContent())?.trim() ?? '';
  expect(token).toMatch(/^\d+\|[A-Za-z0-9]{48}$/);
  await page.getByRole('button', { name: /COPY/ }).click();
  await expect(page.getByText('COPIED!')).toBeVisible();
  expect(await page.evaluate(() => navigator.clipboard.readText())).toBe(token);
  const me = await request.get('/api/user', {
    headers: { Accept: 'application/json', Authorization: `Bearer ${token}` },
  });
  expect(me.status()).toBe(200);
  expect((await me.json()).character_name).toBe('Demo Pilot');
  const networks = await request.get('/api/networks', {
    headers: { Accept: 'application/json', Authorization: `Bearer ${token}` },
  });
  expect((await networks.json()).length).toBe(2);
  done();
});

test('the website demo login signs in and goes home', async ({ page, context }) => {
  seedDemo();
  await page.goto('/demo-login');
  await expect(page).toHaveURL(/\/$/);
  const cookies = await context.cookies();
  const session = cookies.find((cookie) => cookie.name === 'telescope_session');
  expect(session?.httpOnly).toBe(true);
  expect(session?.sameSite).toBe('Lax');
});
