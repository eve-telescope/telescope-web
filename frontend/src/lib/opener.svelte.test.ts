import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { AppOpener, CopyState, FALLBACK_DELAY } from './opener.svelte.ts';

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(() => {
  vi.useRealTimers();
});

describe('AppOpener', () => {
  it('opens the link and offers the download after the delay', () => {
    const opened: string[] = [];
    const opener = new AppOpener((url) => opened.push(url));
    opener.open('telescope://s/ABCDEFGH');
    expect(opened).toEqual(['telescope://s/ABCDEFGH']);
    expect(opener.opening).toBe(true);
    expect(opener.showFallback).toBe(false);
    vi.advanceTimersByTime(FALLBACK_DELAY - 1);
    expect(opener.showFallback).toBe(false);
    vi.advanceTimersByTime(1);
    expect(opener.opening).toBe(false);
    expect(opener.showFallback).toBe(true);
  });
});

describe('CopyState', () => {
  it('shows the copied state for two seconds', async () => {
    const written: string[] = [];
    const state = new CopyState();
    await state.copy('1|token', async (text) => {
      written.push(text);
    });
    expect(written).toEqual(['1|token']);
    expect(state.copied).toBe(true);
    vi.advanceTimersByTime(2000);
    expect(state.copied).toBe(false);
  });
});
