// See https://svelte.dev/docs/kit/types#app.d.ts
// for information about these interfaces
import type { SharedProps } from '#lib/types.ts';

declare global {
  namespace App {
    // interface Error {}
    // interface Locals {}
    interface PageData extends Partial<SharedProps> {}
    // interface PageState {}
    // interface Platform {}
  }
}
