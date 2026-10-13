import { describe, expect, it } from 'vitest';

import { FEATURES, shownVersion } from './features.ts';

describe('shownVersion', () => {
  it('shows the latest release', () => {
    expect(shownVersion('v0.5.1')).toBe('v0.5.1');
  });

  it('falls back to the first release when GitHub failed', () => {
    expect(shownVersion(null)).toBe('v0.1.0');
    expect(shownVersion(undefined)).toBe('v0.1.0');
  });
});

describe('FEATURES', () => {
  it('lists the six cards in order', () => {
    expect(FEATURES.map((feature) => feature.title)).toEqual([
      'Local scans',
      'Danger score',
      'Pilot roles',
      'D-scan',
      'Intel networks',
      'Overlay and hotkey',
    ]);
  });
});
