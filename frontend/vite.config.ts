import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vitest/config';
import { playwright } from '@vitest/browser-playwright';
import adapter from '@sveltejs/adapter-node';
import { sveltekit } from '@sveltejs/kit/vite';

import { isAxumRequest } from './proxy-paths.ts';

// Dev-only shared origin (Caddy plays this role in docker and production):
// backend paths and mutations go to Axum, page GETs stay in SvelteKit.
// Never point the browser at Axum's port directly. The SSR fetch rewrite
// in hooks.server.ts reads the same variable.
const AXUM_DEV_URL = process.env.AXUM_URL ?? 'http://127.0.0.1:3000';

// File events do not cross Docker's bind mount on every host;
// VITE_POLL_WATCH (set by docker-compose) switches the watcher to polling.
const watch = process.env.VITE_POLL_WATCH ? { usePolling: true, interval: 300 } : undefined;

export default defineConfig({
  server: {
    watch,
    proxy: {
      // Every path, but only what proxy-paths.ts gives to Axum: the
      // bypass hands everything else back to SvelteKit.
      '/': {
        target: AXUM_DEV_URL,
        // The Pusher protocol websocket at /app/<key>.
        ws: true,
        bypass: (request) => {
          const path = (request.url ?? '/').split('?')[0];
          return isAxumRequest(request.method ?? 'GET', path) ? undefined : request.url;
        },
      },
    },
  },
  plugins: [
    tailwindcss(),
    sveltekit({
      compilerOptions: {
        // Force runes mode for the project, except for libraries. Can be removed in svelte 6.
        runes: ({ filename }) =>
          filename.split(/[/\\]/).includes('node_modules') ? undefined : true,
      },
      adapter: adapter(),
    }),
  ],
  test: {
    expect: { requireAssertions: true },
    projects: [
      {
        extends: './vite.config.ts',
        test: {
          name: 'client',
          setupFiles: ['./vitest-browser-setup.ts'],
          browser: {
            enabled: true,
            provider: playwright(),
            instances: [{ browser: 'chromium', headless: true }],
          },
          include: ['src/**/*.svelte.{test,spec}.{js,ts}'],
          exclude: ['src/lib/server/**'],
        },
      },
      {
        extends: './vite.config.ts',
        test: {
          name: 'server',
          environment: 'node',
          include: ['src/**/*.{test,spec}.{js,ts}', '*.test.ts'],
          exclude: ['src/**/*.svelte.{test,spec}.{js,ts}'],
        },
      },
    ],
  },
});
