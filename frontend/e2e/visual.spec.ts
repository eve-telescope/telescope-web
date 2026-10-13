// Every page against the screenshot of the Laravel page
// (scripts/laravel-reference/capture.mjs), at desktop and phone width,
// within the 3% pixel tolerance of playwright.config.ts. The baselines
// depend on the host's fonts, so CI skips them.
import { expect, test } from '@playwright/test';

import { seedDataset } from './helpers.ts';
import { HIDE_STARS, VIEWPORTS, VISUAL_PAGES } from './visual-pages.ts';

test.skip(Boolean(process.env.CI), 'screenshots are host specific');

for (const visual of VISUAL_PAGES) {
  for (const viewport of VIEWPORTS) {
    test(`${visual.name} looks like Laravel's (${viewport.device})`, async ({ page }) => {
      seedDataset();
      await page.setViewportSize({ width: viewport.width, height: viewport.height });
      await page.goto(visual.path);
      await page.waitForLoadState('networkidle');
      if (visual.waitForText) {
        await page.getByText(visual.waitForText).waitFor({ timeout: 5000 });
      }
      // The demo login exists only on local stacks (DEMO_ENABLED), not
      // on the site the baselines were taken from.
      await page.addStyleTag({
        content: `${HIDE_STARS} [data-testid="demo-login"] { display: none !important; }`,
      });
      await page.evaluate(() => document.fonts.ready);
      await expect(page).toHaveScreenshot(`${visual.name}-${viewport.device}.png`, {
        fullPage: false,
      });
    });
  }
}
