package dev.roncore.sdk.facets

import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertThrows
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test

/**
 * RO:WHAT —
 *   Unit tests for [FacetTomlWriter].
 *
 * RO:WHY —
 *   Ensure JVM facet manifests render into TOML that matches the
 *   SDK schema expectations and basic invariants.
 */
class FacetTomlWriterTest {

    @Test
    fun `writes minimal static facet`() {
        val facet = FacetDefinition(
            id = "docs",
            kind = FacetKind.STATIC,
            routes = listOf(
                RouteDefinition(
                    method = "GET",
                    path = "/hello",
                    file = "facets/docs/hello.txt"
                )
            )
        )

        val toml = FacetTomlWriter.write(facet).trim()

        val expected = """
            [facet]
            id = "docs"
            kind = "static"

            [[route]]
            method = "GET"
            path = "/hello"
            file = "facets/docs/hello.txt"
        """.trimIndent().trim()

        assertEquals(expected, toml)
    }

    @Test
    fun `validates route path must start with slash`() {
        val facet = FacetDefinition(
            id = "docs",
            routes = listOf(
                RouteDefinition(
                    method = "GET",
                    path = "relative", // invalid
                    file = "facets/docs/hello.txt"
                )
            )
        )

        assertThrows(IllegalArgumentException::class.java) {
            FacetTomlWriter.write(facet)
        }
    }

    @Test
    fun `emits optional sections when provided`() {
        val facet = FacetDefinition(
            id = "docs",
            kind = FacetKind.STATIC,
            security = FacetSecurity(public = false, requiresAuth = true),
            meta = FacetMeta(
                description = "Docs facet",
                owner = "docs-team",
                version = "1.0.0"
            ),
            limits = FacetLimits(
                maxRps = 100,
                maxConcurrency = 10
            ),
            routes = listOf(
                RouteDefinition(
                    method = "GET",
                    path = "/hello",
                    file = "facets/docs/hello.txt",
                    integrity = Integrity(
                        algo = "sha256",
                        value = "abc123"
                    )
                )
            )
        )

        val toml = FacetTomlWriter.write(facet)

        listOf(
            "[facet]",
            """id = "docs"""",
            """kind = "static"""",
            "[facet.security]",
            "public = false",
            "requires_auth = true",
            "[facet.meta]",
            """description = "Docs facet"""",
            """owner = "docs-team"""",
            """version = "1.0.0"""",
            "[facet.limits]",
            "max_rps = 100",
            "max_concurrency = 10",
            "[[route]]",
            """method = "GET"""",
            """path = "/hello"""",
            """file = "facets/docs/hello.txt"""",
            """integrity = { algo = "sha256", value = "abc123" }"""
        ).forEach { snippet ->
            assertTrue(
                toml.contains(snippet),
                "Expected TOML to contain: $snippet\nActual:\n$toml"
            )
        }
    }
}
