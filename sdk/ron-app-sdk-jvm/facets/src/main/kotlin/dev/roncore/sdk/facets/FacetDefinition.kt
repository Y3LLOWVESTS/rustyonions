package dev.roncore.sdk.facets

/**
 * RO:WHAT —
 *   In-memory representation of a single facet manifest.
 *
 * RO:WHY —
 *   JVM apps can define facets using typed builders, then render them
 *   to TOML via [FacetTomlWriter].
 *
 * RO:INVARIANTS —
 *   - `id` MUST be non-blank.
 *   - `routes` MUST be non-empty.
 *   - Additional invariants are enforced by [FacetTomlWriter].
 */
data class FacetDefinition(
    val id: String,
    val kind: FacetKind = FacetKind.STATIC,
    val security: FacetSecurity? = null,
    val meta: FacetMeta? = null,
    val limits: FacetLimits? = null,
    val routes: List<RouteDefinition> = emptyList()
)
