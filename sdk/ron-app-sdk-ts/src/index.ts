/**
 * ron-app-sdk-ts
 *
 * Public entrypoint. Re-export the primary Ron client, core types,
 * config helpers, and facet helpers.
 */

// Core types & client
export * from './types';
export * from './client';

// Local error / config helpers (optional use by callers)
export * from './errors';
export { resolveConfig } from './config';

// Facet helpers under a namespaced export to avoid polluting top-level names.
export * as Facets from './facets/manifest';
