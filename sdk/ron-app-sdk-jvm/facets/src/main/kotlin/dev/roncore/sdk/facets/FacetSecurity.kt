package dev.roncore.sdk.facets

/**
 * RO:WHAT —
 *   Security hints for a facet: whether it is public and/or requires auth.
 *
 * RO:WHY —
 *   Maps directly to `[facet.security]` in the facet manifest TOML.
 *
 * RO:INVARIANTS —
 *   - Null values are treated as "unspecified" and omitted from TOML.
 *   - When both flags are null, the [facet.security] table is omitted.
 */
data class FacetSecurity(
    val public: Boolean? = null,
    val requiresAuth: Boolean? = null
)
