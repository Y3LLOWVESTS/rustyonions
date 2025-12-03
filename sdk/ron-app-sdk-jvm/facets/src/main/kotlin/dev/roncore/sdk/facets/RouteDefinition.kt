package dev.roncore.sdk.facets

/**
 * RO:WHAT —
 *   Per-route configuration within a facet.
 *
 * RO:WHY —
 *   Describes how Micronode should map HTTP requests (method + path)
 *   to either a static file or an upstream route (for future proxy facets).
 *
 * RO:INVARIANTS —
 *   - `path` MUST be an absolute HTTP path (starts with "/").
 *   - For STATIC facets, `file` MUST be non-null.
 *   - For PROXY facets (future), `upstreamPath` MUST be non-null.
 */
data class RouteDefinition(
    val method: String,
    val path: String,
    val file: String? = null,
    val upstreamPath: String? = null,
    val integrity: Integrity? = null
)
