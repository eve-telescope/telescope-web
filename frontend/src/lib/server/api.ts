import { error, redirect } from '@sveltejs/kit';

import { errorMessage } from '#lib/errors.ts';

/** The status codes SvelteKit's `redirect` accepts. */
type RedirectStatus = 300 | 301 | 302 | 303 | 304 | 305 | 306 | 307 | 308;

/**
 * Loads a page's props from the Axum API for an SSR load function. A
 * redirect from the api (the EVE login sending the browser home) becomes
 * the same redirect here, any other non-OK status the error page for
 * that status.
 */
export async function apiGet<T>(fetch: typeof globalThis.fetch, path: string): Promise<T> {
  const response = await fetch(path, { redirect: 'manual' });

  if (response.status >= 300 && response.status < 400) {
    redirect(response.status as RedirectStatus, sitePath(response.headers.get('location')));
  }

  if (!response.ok) {
    error(response.status, errorMessage(response.status));
  }

  return response.json();
}

/** The api only ever redirects to the site itself (`APP_URL`); SvelteKit
 * takes that as its path. */
export function sitePath(location: string | null): string {
  if (!location) {
    return '/';
  }
  const url = new URL(location, 'http://site.invalid');
  return `${url.pathname}${url.search}`;
}

/** The props endpoint of a page URL: `/api/pages` plus its path, the home
 * page at `/api/pages/home`. */
export function pagesPath(url: URL): string {
  const path = url.pathname === '/' ? '/home' : url.pathname;
  return `/api/pages${path}${url.search}`;
}

/** The load of every page: its props from the api. */
export function loadPage<T>({ fetch, url }: { fetch: typeof globalThis.fetch; url: URL }) {
  return apiGet<T>(fetch, pagesPath(url));
}
