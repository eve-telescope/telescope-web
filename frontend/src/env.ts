import { defineEnvVars } from '@sveltejs/kit/env';

/** Where SSR reaches Axum when AXUM_URL is unset: the API's default BIND_ADDR. */
const DEFAULT_AXUM_URL = 'http://127.0.0.1:3000';

export const variables = defineEnvVars({
  AXUM_URL: { schema: (input) => input || DEFAULT_AXUM_URL },
});
