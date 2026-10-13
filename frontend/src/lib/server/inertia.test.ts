import { describe, expect, it } from 'vitest';

import { inertiaReload } from './inertia.ts';

describe('inertiaReload', () => {
  it('sends tabs left open on the Laravel site to a full reload', () => {
    const url = new URL('https://eve-telescope.com/s/ABCDEFGH?x=1');
    const response = inertiaReload(
      new Request(url, { headers: { 'X-Inertia': 'true', 'X-Inertia-Version': 'abc' } }),
      url,
    );
    expect(response?.status).toBe(409);
    expect(response?.headers.get('X-Inertia-Location')).toBe(
      'https://eve-telescope.com/s/ABCDEFGH?x=1',
    );
  });

  it('leaves ordinary requests alone', () => {
    const url = new URL('https://eve-telescope.com/s/ABCDEFGH');
    expect(inertiaReload(new Request(url), url)).toBeNull();
  });
});
