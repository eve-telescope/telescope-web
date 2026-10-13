import type { PageServerLoad } from './$types';

import { loadPage } from '#lib/server/api.ts';
import type { ShareProps } from '#lib/types.ts';

// Each load of the page counts one view (the api increments it), so it
// runs on the server only, once per visit.
export const load: PageServerLoad = (event) => loadPage<ShareProps>(event);
