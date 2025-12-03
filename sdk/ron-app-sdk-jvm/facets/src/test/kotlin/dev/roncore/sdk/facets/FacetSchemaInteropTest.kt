package dev.roncore.sdk.facets

import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Test

/**
 * RO:WHAT —
 *   Schema-level sanity tests for facet types.
 *
 * RO:WHY —
 *   Guards the enum wire-format and simple round-trips so that
 *   future changes do not silently break manifest compatibility.
 */
class FacetSchemaInteropTest {

    @Test
    fun `facet kind round-trips via wire value`() {
        FacetKind.entries.forEach { kind ->
            val wire = kind.wire()
            val decoded = FacetKind.fromWire(wire)
            assertEquals(kind, decoded)
        }
    }

    @Test
    fun `facet kind parsing is case-insensitive`() {
        val decoded = FacetKind.fromWire("STATIC")
        assertEquals(FacetKind.STATIC, decoded)
    }
}
