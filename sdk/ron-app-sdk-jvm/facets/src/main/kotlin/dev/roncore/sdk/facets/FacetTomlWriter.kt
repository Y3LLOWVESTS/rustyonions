package dev.roncore.sdk.facets

/**
 * RO:WHAT —
 *   Serializes [FacetDefinition] into canonical facet TOML.
 *
 * RO:WHY —
 *   Gives JVM callers a safe, typed way to produce manifests
 *   that Micronode can ingest without hand-writing TOML.
 *
 * RO:INVARIANTS —
 *   - Enforces basic schema rules (id non-blank, absolute paths).
 *   - Static facets require `file` on each route.
 *   - Optional sections (security/meta/limits) are omitted when empty.
 */
object FacetTomlWriter {

    /**
     * Render a facet definition as TOML.
     */
    fun write(facet: FacetDefinition): String {
        validate(facet)

        val sb = StringBuilder()

        // [facet]
        sb.appendLine("[facet]")
        sb.appendLine("""id = "${escape(facet.id)}"""")
        sb.appendLine("""kind = "${facet.kind.wire()}"""")
        sb.appendLine()

        // [facet.security]
        facet.security?.let { sec ->
            if (sec.public != null || sec.requiresAuth != null) {
                sb.appendLine("[facet.security]")
                sec.public?.let { sb.appendLine("public = $it") }
                sec.requiresAuth?.let { sb.appendLine("requires_auth = $it") }
                sb.appendLine()
            }
        }

        // [facet.meta]
        facet.meta?.let { meta ->
            if (!meta.description.isNullOrBlank() ||
                !meta.owner.isNullOrBlank() ||
                !meta.version.isNullOrBlank()
            ) {
                sb.appendLine("[facet.meta]")
                meta.description?.let { sb.appendLine("""description = "${escape(it)}"""") }
                meta.owner?.let { sb.appendLine("""owner = "${escape(it)}"""") }
                meta.version?.let { sb.appendLine("""version = "${escape(it)}"""") }
                sb.appendLine()
            }
        }

        // [facet.limits]
        facet.limits?.let { limits ->
            if (limits.maxRps != null || limits.maxConcurrency != null) {
                sb.appendLine("[facet.limits]")
                limits.maxRps?.let { sb.appendLine("max_rps = $it") }
                limits.maxConcurrency?.let { sb.appendLine("max_concurrency = $it") }
                sb.appendLine()
            }
        }

        // [[route]] entries
        facet.routes.forEach { route ->
            sb.appendLine("[[route]]")
            sb.appendLine("""method = "${route.method.uppercase()}"""")
            sb.appendLine("""path = "${escape(normalizePath(route.path))}"""")

            route.file?.let {
                sb.appendLine("""file = "${escape(it)}"""")
            }

            route.upstreamPath?.let {
                sb.appendLine("""upstream_path = "${escape(normalizePath(it))}"""")
            }

            route.integrity?.let { integrity ->
                sb.appendLine(
                    """integrity = { algo = "${escape(integrity.algo)}", value = "${escape(integrity.value)}" }"""
                )
            }

            sb.appendLine()
        }

        return sb.toString()
    }

    private fun validate(facet: FacetDefinition) {
        require(facet.id.isNotBlank()) { "Facet id must not be blank" }
        require(facet.routes.isNotEmpty()) { "Facet must define at least one route" }

        facet.routes.forEach { route ->
            require(route.path.isNotBlank()) { "Route path must not be blank" }
            require(route.path.startsWith("/")) {
                "Route path must start with '/': ${route.path}"
            }

            if (facet.kind == FacetKind.STATIC) {
                require(!route.file.isNullOrBlank()) {
                    "Static facets require 'file' for each route (id=${facet.id})"
                }
            }
        }
    }

    private fun normalizePath(path: String): String =
        if (path.startsWith("/")) path else "/$path"

    private fun escape(value: String): String =
        buildString {
            value.forEach { ch ->
                when (ch) {
                    '\\' -> append("\\\\")
                    '"' -> append("\\\"")
                    '\n' -> append("\\n")
                    '\r' -> append("\\r")
                    '\t' -> append("\\t")
                    else -> append(ch)
                }
            }
        }
}
