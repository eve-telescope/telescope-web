import { describe, expect, it } from 'vitest';

import { DESCRIPTION, pageHead, pageTitle } from './seo.ts';

describe('pageTitle', () => {
  it('adds the site name', () => {
    expect(pageTitle('EVE Online Intel Tool')).toBe('EVE Online Intel Tool | Telescope');
    expect(pageTitle('12 Pilots Shared')).toBe('12 Pilots Shared | Telescope');
  });

  it('is the site name alone without a title', () => {
    expect(pageTitle()).toBe('Telescope');
    expect(pageTitle('')).toBe('Telescope');
  });
});

describe('pageHead', () => {
  it('builds the tags of the Laravel root view', () => {
    const head = pageHead(new URL('https://eve-telescope.com/s/ABCDEFGH?x=1'), '3 Pilots Shared');
    expect(head).toEqual({
      title: '3 Pilots Shared | Telescope',
      meta: [
        { name: 'description', content: DESCRIPTION },
        { property: 'og:type', content: 'website' },
        { property: 'og:site_name', content: 'Telescope' },
        { property: 'og:title', content: 'Telescope' },
        { property: 'og:description', content: DESCRIPTION },
        { property: 'og:url', content: 'https://eve-telescope.com/s/ABCDEFGH' },
        { property: 'og:image', content: 'https://eve-telescope.com/og-image.png' },
        { property: 'og:image:width', content: '1200' },
        { property: 'og:image:height', content: '630' },
        { name: 'twitter:card', content: 'summary_large_image' },
        { name: 'twitter:image', content: 'https://eve-telescope.com/og-image.png' },
      ],
    });
  });
});
