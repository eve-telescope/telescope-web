// The <head> of every page, as the Laravel root view had it
// (resources/views/app.blade.php), now rendered on the server.
import { expect, test } from '@playwright/test';

import { baseUrl, createShare, resetDatabase, seedDemo } from './helpers.ts';

const DESCRIPTION = 'Intel tool for EVE Online. Instant pilot lookups and danger ratings.';

test.beforeEach(() => resetDatabase());

for (const [path, title, prepare] of [
  ['/', 'EVE Online Intel Tool | Telescope', () => {}],
  ['/s/ABCDEFGH', '2 Pilots Shared | Telescope', () => createShare('ABCDEFGH', ['A', 'B'])],
  ['/demo-login?desktop', 'Authentication | Telescope', seedDemo],
] as const) {
  test(`the head of ${path}`, async ({ request }) => {
    prepare();
    const html = await (await request.get(path)).text();
    const url = new URL(path, baseUrl());
    const tag = (pattern: string) => expect(html, pattern).toContain(pattern);
    tag(`<title>${title}</title>`);
    tag(`<meta name="description" content="${DESCRIPTION}"`);
    tag('<meta name="theme-color" content="#0a0a0b"');
    tag('<meta property="og:type" content="website"');
    tag('<meta property="og:site_name" content="Telescope"');
    tag('<meta property="og:title" content="Telescope"');
    tag(`<meta property="og:description" content="${DESCRIPTION}"`);
    tag(`<meta property="og:url" content="${url.origin}${url.pathname}"`);
    tag(`<meta property="og:image" content="${url.origin}/og-image.png"`);
    tag('<meta property="og:image:width" content="1200"');
    tag('<meta property="og:image:height" content="630"');
    tag('<meta name="twitter:card" content="summary_large_image"');
    tag(`<meta name="twitter:image" content="${url.origin}/og-image.png"`);
    tag('<link rel="icon" href="/favicon.ico" sizes="48x48"');
    tag('<link rel="icon" href="/favicon-32.png" sizes="32x32" type="image/png"');
    tag('<link rel="icon" href="/favicon-16.png" sizes="16x16" type="image/png"');
    tag('<link rel="apple-touch-icon" href="/apple-touch-icon.png"');
    tag('<link rel="manifest" href="/site.webmanifest"');
    tag('https://fonts.bunny.net/css?family=inter:400,500,600,700|jetbrains-mono:400,500,600');
    expect(html).toContain('<html lang="en">');
  });
}
