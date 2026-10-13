// The <head> of every page: the title from the page (with the
// ` | Telescope` suffix), the description, and the OpenGraph and Twitter tags with the
// page's URL without its query.

export const DESCRIPTION = 'Intel tool for EVE Online. Instant pilot lookups and danger ratings.';

export type MetaTag = { name?: string; property?: string; content: string };

export type Head = { title: string; meta: MetaTag[] };

/** `{title} | Telescope`, or `Telescope` alone. */
export function pageTitle(title?: string): string {
  return title ? `${title} | Telescope` : 'Telescope';
}

export function pageHead(url: Pick<URL, 'origin' | 'pathname'>, title?: string): Head {
  const image = `${url.origin}/og-image.png`;
  return {
    title: pageTitle(title),
    meta: [
      { name: 'description', content: DESCRIPTION },
      { property: 'og:type', content: 'website' },
      { property: 'og:site_name', content: 'Telescope' },
      { property: 'og:title', content: 'Telescope' },
      { property: 'og:description', content: DESCRIPTION },
      { property: 'og:url', content: `${url.origin}${url.pathname}` },
      { property: 'og:image', content: image },
      { property: 'og:image:width', content: '1200' },
      { property: 'og:image:height', content: '630' },
      { name: 'twitter:card', content: 'summary_large_image' },
      { name: 'twitter:image', content: image },
    ],
  };
}
