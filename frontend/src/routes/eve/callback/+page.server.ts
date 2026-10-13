import type { PageServerLoad } from './$types';

import { loadPage } from '#lib/server/api.ts';
import type { AuthTokenProps } from '#lib/types.ts';

// The EVE SSO callback: the api checks the login and either sends the
// browser home (website sign-in) or answers the desktop token this page
// shows.
export const load: PageServerLoad = (event) => loadPage<AuthTokenProps>(event);
