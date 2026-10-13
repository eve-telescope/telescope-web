import { describe, expect, it } from 'vitest';

import { parseSetCookie } from './cookies.ts';

describe('parseSetCookie', () => {
  it('reads the name, value and attributes the api sets', () => {
    expect(
      parseSetCookie(
        'telescope_session=abc123; HttpOnly; SameSite=Lax; Secure; Path=/; Max-Age=7200',
      ),
    ).toEqual({
      name: 'telescope_session',
      value: 'abc123',
      options: { path: '/', httpOnly: true, sameSite: 'lax', secure: true, maxAge: 7200 },
    });
  });

  it('keeps an equals sign inside the value', () => {
    expect(parseSetCookie('telescope_sso=1|a=b; Path=/')?.value).toBe('1|a=b');
  });

  it('reads a removal', () => {
    expect(parseSetCookie('telescope_session=; Max-Age=0; Path=/')).toEqual({
      name: 'telescope_session',
      value: '',
      options: { path: '/', maxAge: 0 },
    });
  });

  it('ignores a header without a name', () => {
    expect(parseSetCookie('=x')).toBeNull();
    expect(parseSetCookie('nonsense')).toBeNull();
  });
});
