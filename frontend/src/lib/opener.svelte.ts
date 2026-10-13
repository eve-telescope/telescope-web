// Opening a deep link in the desktop app. The browser cannot tell whether
// the app took the link, so after 2.5 seconds the page offers the
// download instead (the share page's fallback).

/** How long the page waits before offering the download. */
export const FALLBACK_DELAY = 2500;

export class AppOpener {
  opening = $state(false);
  showFallback = $state(false);

  #navigate: (url: string) => void;

  constructor(navigate: (url: string) => void) {
    this.#navigate = navigate;
  }

  open(link: string): void {
    this.opening = true;
    this.#navigate(link);
    setTimeout(() => {
      this.showFallback = true;
      this.opening = false;
    }, FALLBACK_DELAY);
  }
}

/** A copy button's "COPIED!" state for two seconds. */
export class CopyState {
  copied = $state(false);

  async copy(text: string, write: (text: string) => Promise<void>): Promise<void> {
    await write(text);
    this.copied = true;
    setTimeout(() => {
      this.copied = false;
    }, 2000);
  }
}
