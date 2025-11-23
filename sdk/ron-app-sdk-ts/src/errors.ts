/**
 * SDK-local error types for misconfiguration or catastrophic local failures.
 * Server-originated problems are represented as RonProblem inside AppResponse.
 */

export class SdkConfigError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'SdkConfigError';
  }
}

export class LocalNetworkError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'LocalNetworkError';
  }
}
