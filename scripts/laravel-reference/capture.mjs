// Screenshots of the Laravel pages, the baselines of
// frontend/e2e/visual.spec.ts: the pages of frontend/e2e/visual-pages.ts
// at desktop and phone width, with tests/fixtures/dataset.json seeded
// before every shot.
//
//   scripts/laravel-reference/run.sh start
//   cd frontend && node ../scripts/laravel-reference/capture.mjs
import { execFileSync } from 'node:child_process';
import { mkdirSync } from 'node:fs';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';

import { HIDE_STARS, VIEWPORTS, VISUAL_PAGES } from '../../frontend/e2e/visual-pages.ts';

// Playwright from the frontend's dependencies; nothing is installed here.
const { chromium } = createRequire(new URL('../../frontend/package.json', import.meta.url))(
  'playwright',
);

const BASE = 'http://127.0.0.1:8318';
const OUT = new URL('../../frontend/e2e/__screenshots__/visual.spec.ts/', import.meta.url);
mkdirSync(OUT, { recursive: true });

function reset() {
  const record = fileURLToPath(new URL('./record.py', import.meta.url));
  const sql = execFileSync('python3', ['-I', record, '--print-reset']);
  execFileSync(
    'docker',
    ['exec', '-i', 'telescope-laravel-mysql', 'mysql', '-uroot', '-ptelescope', '--default-character-set=utf8mb4', 'laravel'],
    { input: sql, stdio: ['pipe', 'ignore', 'ignore'] },
  );
}

const browser = await chromium.launch();
for (const visual of VISUAL_PAGES) {
  for (const viewport of VIEWPORTS) {
    reset();
    const page = await browser.newPage({ viewport: { width: viewport.width, height: viewport.height } });
    await page.goto(BASE + visual.path);
    await page.waitForLoadState('networkidle');
    if (visual.waitForText) {
      await page.getByText(visual.waitForText).waitFor({ timeout: 5000 });
    }
    await page.addStyleTag({ content: HIDE_STARS });
    await page.evaluate(() => document.fonts.ready);
    await page.screenshot({
      path: fileURLToPath(new URL(`${visual.name}-${viewport.device}.png`, OUT)),
      animations: 'disabled',
    });
    await page.close();
    console.log(visual.name, viewport.device);
  }
}
await browser.close();
