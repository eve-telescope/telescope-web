// The pages the visual comparison shoots, shared by the visual spec and
// scripts/laravel-reference/capture.mjs so both sides take the same
// pictures: tests/fixtures/dataset.json seeded (a fresh seed per shot, as
// the share page counts its view), the stars hidden (they are random),
// the share page after the app did not answer its deep link.

export type VisualPage = { name: string; path: string; waitForText?: string };

export const VISUAL_PAGES: VisualPage[] = [
  { name: 'home', path: '/' },
  { name: 'share', path: '/s/ABCDEFGH', waitForText: 'TRY AGAIN' },
  { name: 'share-many', path: '/s/zyxW9876', waitForText: 'TRY AGAIN' },
  { name: 'share-unicode', path: '/s/Unic0de1', waitForText: 'TRY AGAIN' },
];

export const VIEWPORTS = [
  { device: 'desktop', width: 1280, height: 800 },
  { device: 'phone', width: 390, height: 844 },
];

/** Hides the random stars of both the Vue and the Svelte pages. */
export const HIDE_STARS = '.animate-pulse.rounded-full.bg-white { visibility: hidden !important; }';
