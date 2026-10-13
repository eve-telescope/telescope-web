/**
 * Browsers that kept a Laravel page open across the switch still navigate
 * with Inertia's XHR (`X-Inertia: true`). Inertia answers a 409 with an
 * `X-Inertia-Location` header by doing a full page load of that URL, so
 * those tabs land on the SvelteKit page instead of breaking.
 */
export function inertiaReload(request: Request, url: URL): Response | null {
  if (request.headers.get('x-inertia') === null) {
    return null;
  }
  return new Response(null, {
    status: 409,
    headers: { 'X-Inertia-Location': url.href },
  });
}
