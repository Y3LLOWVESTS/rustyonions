/**
 * Config resolution for ron-app-sdk-ts.
 *
 * Merges RonOptions with environment-derived defaults (Node) and enforces
 * security invariants like HTTPS-only-by-default.
 */

import type { RonOptions } from './types';
import { SdkConfigError } from './errors';

export function resolveConfig(options: RonOptions): RonOptions {
  const baseUrl =
    options.baseUrl ||
    (typeof process !== 'undefined'
      ? process.env.RON_SDK_GATEWAY_ADDR ?? ''
      : '');

  if (!baseUrl) {
    throw new SdkConfigError('baseUrl is required for Ron client');
  }

  if (!options.allowInsecureHttp && baseUrl.startsWith('http://')) {
    throw new SdkConfigError(
      'Insecure HTTP is disabled; set allowInsecureHttp: true for local dev only',
    );
  }

  return {
    overallTimeoutMs:
      options.overallTimeoutMs ??
      (typeof process !== 'undefined'
        ? Number(process.env.RON_SDK_OVERALL_TIMEOUT_MS || 10000)
        : 10000),
    ...options,
    baseUrl,
  };
}
