package dev.roncore.sdk.facets

/**
 * RO:WHAT —
 *   Enumerates supported facet kinds.
 *
 * RO:WHY —
 *   Keeps JVM facet builders aligned with the canonical facet schema
 *   (static docs, echo/test facets, and future proxy facets).
 *
 * RO:INVARIANTS —
 *   - `wire()` returns the lower-case string used in TOML.
 *   - `fromWire` is case-insensitive and throws on unknown values.
 */
enum class FacetKind(
    private val wireValue: String
) {
    STATIC("static"),
    ECHO("echo"),
    PROXY("proxy");

    /**
     * Wire-format value used in TOML (`kind = "static"`).
     */
    fun wire(): String = wireValue

    companion object {
        /**
         * Resolve a facet kind from its wire-format string.
         */
        fun fromWire(value: String): FacetKind =
            entries.firstOrNull { it.wireValue.equals(value, ignoreCase = true) }
                ?: throw IllegalArgumentException("Unknown facet kind: $value")
    }
}
