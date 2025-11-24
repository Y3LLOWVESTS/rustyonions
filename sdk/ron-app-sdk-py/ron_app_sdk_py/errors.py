from __future__ import annotations

from typing import Any, Dict, Optional

from pydantic import BaseModel, ConfigDict, Field

__all__ = [
    "Problem",
    "RonError",
    "RonProblemError",
    "RonNetworkError",
    "RonAuthError",
    "RonTimeoutError",
    "RonConfigError",
    "RonParseError",
]


class Problem(BaseModel):
    """Canonical Problem envelope returned by RON-CORE nodes.

    Schema (from SDK_SCHEMA_IDB):

        {
            "code": "string",
            "kind": "string",
            "message": "string",
            "correlation_id": "string?",
            "details": { ... }
        }
    """

    code: str
    message: str
    kind: str
    correlation_id: Optional[str] = None
    details: Dict[str, Any] = Field(default_factory=dict)

    # Ignore unknown fields rather than blowing up if node adds extras
    model_config = ConfigDict(extra="ignore")


class RonError(Exception):
    """Base class for all SDK errors."""


class RonProblemError(RonError):
    """Raised when the node returns a canonical Problem envelope."""

    def __init__(self, problem: Problem, status_code: int) -> None:
        self.problem = problem
        self.status_code = status_code
        super().__init__(f"{problem.code} ({problem.kind}): {problem.message}")


class RonNetworkError(RonError):
    """Network-level error (DNS/TLS/socket failures)."""


class RonAuthError(RonError):
    """Authentication/authorization error (401/403, invalid caps)."""


class RonTimeoutError(RonError):
    """Request exceeded configured timeout."""


class RonConfigError(RonError):
    """Misconfiguration of client or environment."""


class RonParseError(RonError):
    """Response could not be parsed as expected JSON/Problem."""
