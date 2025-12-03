package dev.roncore.sdk.facets

/**
 * RO:WHAT —
 *   Rate and concurrency hints for a facet.
 *
 * RO:WHY —
 *   Maps to `[facet.limits]` in TOML; Micronode can use these fields
 *   to enforce fair use and protect itself from overloads.
 *
 * RO:INVARIANTS —
 *   - Nulls are omitted.
 *   - If both fields are null, [facet.limits] is omitted.
 */
data class FacetLimits(
    val maxRps: Int? = null,
    val maxConcurrency: Int? = null
)
