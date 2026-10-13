import { describe, expect, it } from 'vitest';

import { pagesPath } from './api.ts';

describe('pagesPath', () => {
  it('maps a page URL to its props endpoint', () => {
    expect(pagesPath(new URL('https://eve-telescope.com/'))).toBe('/api/pages/home');
    expect(pagesPath(new URL('https://eve-telescope.com/s/ABCDEFGH'))).toBe(
      '/api/pages/s/ABCDEFGH',
    );
    expect(pagesPath(new URL('https://eve-telescope.com/eve/callback?code=a&state=b'))).toBe(
      '/api/pages/eve/callback?code=a&state=b',
    );
  });
});
