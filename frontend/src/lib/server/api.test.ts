import { describe, expect, it } from 'vitest';

import { pagesPath, sitePath } from './api.ts';

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

describe('sitePath', () => {
  it('keeps the path of a redirect to the site', () => {
    expect(sitePath('https://eve-telescope.com')).toBe('/');
    expect(sitePath('https://eve-telescope.com/s/ABC?x=1')).toBe('/s/ABC?x=1');
    expect(sitePath('/eve')).toBe('/eve');
    expect(sitePath(null)).toBe('/');
  });
});
