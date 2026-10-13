import type { PageServerLoad } from './$types';

import { loadPage } from '#lib/server/api.ts';
import type { AuthTokenProps } from '#lib/types.ts';

// The local demo login (DEMO_ENABLED): the EVE callback for the demo
// pilot, without EVE SSO. 404 when it is off.
export const load: PageServerLoad = (event) => loadPage<AuthTokenProps>(event);
