from __future__ import annotations

from dataclasses import dataclass
from typing import Dict

"""
In-process request metrics for ron-app-sdk-py.

RO:WHAT
    Tiny, dependency-free counters for RonClient activity.

RO:WHY
    - Give apps a simple way to introspect SDK behavior during tests/dev.
    - Keep the shape close to other language SDKs without pulling in
      Prometheus or heavy exporters.

RO:INVARIANTS
    - No global mutable state; metrics are per-client instance.
    - Safe to ignore entirely if callers do not need metrics.
"""

__all__ = [
    "RequestMetrics",
]


@dataclass
class RequestMetrics:
    """Simple per-client counters for HTTP activity."""

    total_requests: int = 0
    total_errors: int = 0
    total_timeouts: int = 0
    total_network_errors: int = 0

    def record_success(
        self,
        *,
        method: str,
        path: str,
        status_code: int,
        elapsed_ms: float,
    ) -> None:
        # Parameters are currently unused but kept for future expansion
        # and to mirror other SDKs.
        del method, path, status_code, elapsed_ms
        self.total_requests += 1

    def record_error(
        self,
        *,
        method: str,
        path: str,
        status_code: int,
        elapsed_ms: float,
    ) -> None:
        del method, path, status_code, elapsed_ms
        self.total_requests += 1
        self.total_errors += 1

    def record_timeout(self, *, method: str, path: str) -> None:
        del method, path
        self.total_timeouts += 1

    def record_network_error(self, *, method: str, path: str) -> None:
        del method, path
        self.total_network_errors += 1

    def snapshot(self) -> Dict[str, int]:
        """Return a dict snapshot of current counters (useful for tests)."""
        return {
            "total_requests": self.total_requests,
            "total_errors": self.total_errors,
            "total_timeouts": self.total_timeouts,
            "total_network_errors": self.total_network_errors,
        }
