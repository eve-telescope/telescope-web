// The feature cards of the home page, as the Vue page listed them. The
// icons are lucide paths.

export type Feature = { title: string; description: string; color: string; icon: string };

export const FEATURES: Feature[] = [
  {
    title: 'Local scans',
    description: 'Paste local chat and pilots stream in as they are looked up.',
    color: 'text-eve-cyan',
    icon: '<path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M22 21v-2a4 4 0 0 0-3-3.87"/><path d="M16 3.13a4 4 0 0 1 0 7.75"/>',
  },
  {
    title: 'Danger score',
    description:
      'One 0 to 100 rating per pilot, weighted toward small-gang lethality, with a breakdown on hover.',
    color: 'text-eve-orange',
    icon: '<path d="m12 14 4-4"/><path d="M3.34 19a10 10 0 1 1 17.32 0"/>',
  },
  {
    title: 'Pilot roles',
    description: 'Recons, marauders, cynos, capitals and more, shown as in-game hull brackets.',
    color: 'text-eve-red',
    icon: '<path d="M12 3 3 19h18L12 3Z"/><path d="m12 9-3.5 6h7L12 9Z"/>',
  },
  {
    title: 'D-scan',
    description: 'Ship counts by type and class, sorted by hull size.',
    color: 'text-eve-green',
    icon: '<path d="M19.07 4.93A10 10 0 0 0 6.99 3.34"/><path d="M4 6h.01"/><path d="M2.29 9.62A10 10 0 1 0 21.31 8.35"/><path d="M16.24 7.76A6 6 0 1 0 8.23 16.67"/><path d="M12 18h.01"/><path d="M17.99 11.66A6 6 0 0 1 15.77 16.67"/><circle cx="12" cy="12" r="2"/><path d="m13.41 10.59 5.66-5.66"/>',
  },
  {
    title: 'Intel networks',
    description:
      'Tag pilots, corporations and alliances and share scans with your group in real time.',
    color: 'text-eve-cyan',
    icon: '<circle cx="18" cy="5" r="3"/><circle cx="6" cy="12" r="3"/><circle cx="18" cy="19" r="3"/><path d="m8.59 13.51 6.83 3.98"/><path d="m15.41 6.51-6.82 3.98"/>',
  },
  {
    title: 'Overlay and hotkey',
    description:
      'An always-on-top window and a global hotkey to scan the clipboard while EVE has focus.',
    color: 'text-eve-yellow',
    icon: '<path d="M12.83 2.18a2 2 0 0 0-1.66 0L2.6 6.08a1 1 0 0 0 0 1.83l8.58 3.91a2 2 0 0 0 1.66 0l8.58-3.9a1 1 0 0 0 0-1.83Z"/><path d="m22 17.65-9.17 4.16a2 2 0 0 1-1.66 0L2 17.65"/><path d="m22 12.65-9.17 4.16a2 2 0 0 1-1.66 0L2 12.65"/>',
  },
];

/** The version the home page shows: the latest release, or the first
 * one when GitHub could not be asked. */
export function shownVersion(appVersion: string | null | undefined): string {
  return appVersion ?? 'v0.1.0';
}
