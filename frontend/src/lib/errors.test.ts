import { describe, expect, it } from 'vitest';

import { errorMessage } from './errors.ts';

describe('errorMessage', () => {
  it('names the statuses Laravel had pages for', () => {
    expect(errorMessage(404)).toBe('Not Found');
    expect(errorMessage(403)).toBe('Forbidden');
    expect(errorMessage(419)).toBe('Page Expired');
  });

  it('calls everything else a server error', () => {
    expect(errorMessage(500)).toBe('Server Error');
    expect(errorMessage(501)).toBe('Server Error');
    expect(errorMessage(502)).toBe('Server Error');
  });
});
