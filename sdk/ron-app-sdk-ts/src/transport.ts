/**
 * Transport layer: wraps fetch() with URL construction, timeouts, and
 * response parsing.
 *
 * This is a scaffold; details will be filled in later.
 */

import type { AppRequest, AppResponse, RonOptions } from './types';

export async function sendRequest<T = unknown>(
  _options: RonOptions,
  _req: AppRequest,
): Promise<AppResponse<T>> {
  // TODO: implement URL building, header injection, timeouts, and problem parsing.
  throw new Error('sendRequest not implemented yet (scaffold)');
}
