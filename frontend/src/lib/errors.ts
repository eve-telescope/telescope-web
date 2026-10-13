// The messages of the error pages, the api's list (src/server/errors.rs).
const MESSAGES: Record<number, string> = {
  401: 'Unauthorized',
  402: 'Payment Required',
  403: 'Forbidden',
  404: 'Not Found',
  405: 'Method Not Allowed',
  419: 'Page Expired',
  429: 'Too Many Requests',
  503: 'Service Unavailable',
};

export function errorMessage(status: number): string {
  return MESSAGES[status] ?? 'Server Error';
}
