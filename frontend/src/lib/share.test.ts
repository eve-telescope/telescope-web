import { describe, expect, it } from 'vitest';

import { deepLink, pilotsText, previewPilots } from './share.ts';

describe('deepLink', () => {
  it('opens the share in the desktop app', () => {
    expect(deepLink('ABCDEFGH')).toBe('telescope://s/ABCDEFGH');
  });
});

describe('previewPilots', () => {
  it('shows every pilot of a short list', () => {
    expect(previewPilots(['A', 'B'])).toEqual({ shown: ['A', 'B'], more: 0 });
  });

  it('shows the first 50 and counts the rest', () => {
    const pilots = Array.from({ length: 53 }, (_, i) => `Pilot ${i}`);
    const preview = previewPilots(pilots);
    expect(preview.shown).toHaveLength(50);
    expect(preview.shown[49]).toBe('Pilot 49');
    expect(preview.more).toBe(3);
  });
});

describe('pilotsText', () => {
  it('puts one pilot per line', () => {
    expect(pilotsText(['A', 'B C', 'D'])).toBe('A\nB C\nD');
  });
});
