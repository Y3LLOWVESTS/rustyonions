package dev.roncore.sdk.facets

/**
 * RO:WHAT —
 *   Optional descriptive metadata for a facet.
 *
 * RO:WHY —
 *   Provides human-friendly context for operators and tools:
 *   description, owner, and version.
 *
 * RO:INVARIANTS —
 *   - Null fields are omitted from TOML.
 *   - If all fields are null, [facet.meta] is omitted.
 */
data class FacetMeta(
    val description: String? = null,
    val owner: String? = null,
    val version: String? = null
)
