import { describe, expect, it } from 'vitest';

import { isAxumRequest } from './proxy-paths.ts';

describe('isAxumRequest', () => {
  it('sends every mutation to the api', () => {
    for (const method of ['POST', 'PUT', 'PATCH', 'DELETE', 'OPTIONS']) {
      expect(isAxumRequest(method, '/api/networks/1')).toBe(true);
    }
  });

  it('sends the api paths to the api', () => {
    for (const path of [
      '/api/networks',
      '/api/pages/home',
      '/up',
      '/eve',
      '/sanctum/csrf-cookie',
      '/storage/file.txt',
      '/broadcasting/auth',
      '/app/04pcwy13bvcyjoio6mf6',
    ]) {
      expect(isAxumRequest('GET', path), path).toBe(true);
    }
  });

  it('keeps the pages in SvelteKit', () => {
    for (const path of [
      '/',
      '/s/ABCDEFGH',
      '/eve/callback',
      '/demo-login',
      '/_app/x.js',
      '/apps',
    ]) {
      expect(isAxumRequest('GET', path), path).toBe(false);
    }
  });
});
