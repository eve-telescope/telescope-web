import type { HandleFetch } from '@sveltejs/kit/hooks';
import { AXUM_URL } from '$app/env/private';

import { parseSetCookie } from '#lib/server/cookies.ts';

// Server-side loads reach Axum directly; the browser goes through the
// shared-origin proxy instead. Axum is another origin to SvelteKit, so
// the visitor's cookies (the session, the EVE login's state) are passed
// along by hand, and the cookies Axum sets come back the same way.
export const handleFetch: HandleFetch = async ({ event, request, fetch }) => {
  const url = new URL(request.url);
  if (url.origin !== event.url.origin || !url.pathname.startsWith('/api')) {
    return fetch(request);
  }

  const proxied = new Request(new URL(url.pathname + url.search, AXUM_URL), request);
  const cookie = event.request.headers.get('cookie');
  if (cookie) {
    proxied.headers.set('cookie', cookie);
  }
  // The api decides about Secure cookies by the host the visitor used.
  proxied.headers.set('host', event.url.host);
  proxied.headers.set('x-forwarded-proto', event.url.protocol.replace(':', ''));

  const response = await fetch(proxied);
  for (const header of response.headers.getSetCookie()) {
    const parsed = parseSetCookie(header);
    if (parsed) {
      event.cookies.set(parsed.name, parsed.value, { ...parsed.options, encode: (value) => value });
    }
  }
  return response;
};
