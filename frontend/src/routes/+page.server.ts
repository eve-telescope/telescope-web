import type { PageServerLoad } from './$types';

import { loadPage } from '#lib/server/api.ts';
import type { HomeProps } from '#lib/types.ts';

export const load: PageServerLoad = (event) => loadPage<HomeProps>(event);
