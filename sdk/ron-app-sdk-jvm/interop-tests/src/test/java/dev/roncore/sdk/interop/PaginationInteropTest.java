package dev.roncore.sdk.interop;

import org.junit.jupiter.api.Assumptions;
import org.junit.jupiter.api.Test;

/**
 * RO:WHAT —
 *   Placeholder for future pagination interop tests (list endpoints, cursors, etc.).
 *
 * RO:WHY —
 *   We want a dedicated home for pagination behavior that hits a *real* RON-CORE
 *   node (Micronode/Macronode) once list APIs are available.
 *
 * RO:INVARIANTS —
 *   - MUST NOT fail CI today — pagination endpoints are not wired yet.
 *   - Clearly marked as a placeholder so future-you knows where to add real tests.
 */
final class PaginationInteropTest {

    @Test
    void paginationInteropPlaceholder() {
        // Always skip for now; this test is a marker that pagination interop
        // should exist, but there is no stable endpoint yet.
        Assumptions.assumeTrue(
                false,
                () -> "Skipping pagination interop: pagination endpoints not implemented yet"
        );
    }
}
