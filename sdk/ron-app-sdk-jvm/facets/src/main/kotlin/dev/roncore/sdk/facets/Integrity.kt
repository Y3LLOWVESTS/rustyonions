package dev.roncore.sdk.facets

/**
 * RO:WHAT —
 *   Integrity info (SRI-style) for static assets.
 *
 * RO:WHY —
 *   Lets Micronode verify that a served asset matches an expected
 *   hash before returning it, reducing the risk of tampering.
 *
 * RO:INVARIANTS —
 *   - `algo` is a free-form string (e.g. "sha256").
 *   - `value` is the hash (usually hex or base64).
 */
data class Integrity(
    val algo: String,
    val value: String
)
