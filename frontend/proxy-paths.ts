// The shared-origin routing contract, mirrored by deploy/Caddyfile: keep
// both in sync when a path changes.

// Paths Axum owns outright: every method proxies to the API. `/api` the
// desktop clients' JSON API, the page props and the health check; `/up`
// the health page; `/sanctum/csrf-cookie` and `/storage` two
// routes older clients may still call; `/broadcasting/auth` the channel signatures; `/app` the
// Pusher protocol websocket.
export const axumPrefixes = ['/api', '/up', '/sanctum', '/storage', '/broadcasting/auth', '/app'];

// `/eve` itself (the redirect to EVE SSO) is Axum's; the callback page
// below it, `/eve/callback`, is SvelteKit's.
const axumExact = new Set(['/eve']);

// Every request that is not a GET or HEAD goes to Axum as well.
export function isAxumRequest(method: string, path: string): boolean {
  if (method !== 'GET' && method !== 'HEAD') {
    return true;
  }
  return (
    axumExact.has(path) ||
    axumPrefixes.some((prefix) => path === prefix || path.startsWith(`${prefix}/`))
  );
}
