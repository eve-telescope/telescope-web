// The share page's pieces of logic: the desktop app's deep link, the
// preview of the first 50 pilots and the text the copy button copies.

/** How many pilots the page shows before the `+N more` badge. */
export const PREVIEW_LIMIT = 50;

/** The link that opens a share in the desktop app. */
export function deepLink(code: string): string {
  return `telescope://s/${code}`;
}

/** The pilots the page lists, and how many more there are. */
export function previewPilots(pilots: string[]): { shown: string[]; more: number } {
  return {
    shown: pilots.slice(0, PREVIEW_LIMIT),
    more: Math.max(pilots.length - PREVIEW_LIMIT, 0),
  };
}

/** One pilot per line, as local chat lists them. */
export function pilotsText(pilots: string[]): string {
  return pilots.join('\n');
}
