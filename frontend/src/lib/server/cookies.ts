// The api's Set-Cookie headers on SSR loads (the session the EVE login
// starts, the login's state cookie it clears) have to reach the browser.
// SvelteKit only forwards them for same-origin fetches, and the loads go
// to Axum's own origin, so hooks.server.ts copies them over with this
// parser.

export type CookieOptions = {
  path: string;
  domain?: string;
  maxAge?: number;
  expires?: Date;
  httpOnly?: boolean;
  secure?: boolean;
  sameSite?: 'lax' | 'strict' | 'none';
};

export type ParsedCookie = { name: string; value: string; options: CookieOptions };

/** One `Set-Cookie` header value; `null` when it has no name. */
export function parseSetCookie(header: string): ParsedCookie | null {
  const [pair, ...attributes] = header.split(';');
  const separator = pair.indexOf('=');
  if (separator <= 0) {
    return null;
  }
  const name = pair.slice(0, separator).trim();
  const value = pair.slice(separator + 1).trim();
  const options: CookieOptions = { path: '/' };
  for (const attribute of attributes) {
    const [key, ...rest] = attribute.split('=');
    const argument = rest.join('=').trim();
    switch (key.trim().toLowerCase()) {
      case 'path':
        options.path = argument || '/';
        break;
      case 'domain':
        options.domain = argument;
        break;
      case 'max-age':
        options.maxAge = Number(argument);
        break;
      case 'expires':
        options.expires = new Date(argument);
        break;
      case 'httponly':
        options.httpOnly = true;
        break;
      case 'secure':
        options.secure = true;
        break;
      case 'samesite': {
        const sameSite = argument.toLowerCase();
        if (sameSite === 'lax' || sameSite === 'strict' || sameSite === 'none') {
          options.sameSite = sameSite;
        }
        break;
      }
    }
  }
  return { name, value, options };
}
