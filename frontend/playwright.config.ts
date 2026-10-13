// End-to-end browser tests against the running docker stack
// (`docker compose up --build` serves the whole app on :5318). Set
// E2E_BASE_URL to point somewhere else, e.g. a vite dev server proxying a
// local API. The suites seed and inspect the database through the
// telescope-postgres container (e2e/helpers.ts), so they run one at a
// time.
import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: 'e2e',
  fullyParallel: false,
  workers: 1,
  retries: 0,
  reporter: [['list']],
  snapshotPathTemplate: 'e2e/__screenshots__/{testFilePath}/{arg}{ext}',
  expect: {
    toHaveScreenshot: { maxDiffPixelRatio: 0.03, animations: 'disabled' },
  },
  use: {
    baseURL: process.env.E2E_BASE_URL ?? 'http://localhost:5318',
    locale: 'en-US',
    timezoneId: 'UTC',
    screenshot: 'only-on-failure',
    trace: 'retain-on-failure',
  },
  projects: [{ name: 'chromium', use: { browserName: 'chromium' } }],
});
