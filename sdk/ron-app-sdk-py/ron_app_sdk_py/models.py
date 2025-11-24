from __future__ import annotations

from typing import Any, Dict, Optional
import json
from pydantic import BaseModel, Field

"""
Shared DTOs / models for ron-app-sdk-py.

RO:WHAT
    Typed representations for common app-plane responses that show up in
    examples and high-level helpers (hello, health, events).

RO:WHY
    Keep SDK examples and app code strongly typed without forcing callers
    to hand-roll Pydantic models for the most common shapes.

RO:INVARIANTS
    - Models are backwards-compatible with the TS SDK DTOs.
    - All fields are optional-tolerant where the server might omit them.
"""


__all__ = [
    "HelloResponse",
    "HealthzResponse",
    "ReadyzResponse",
    "Event",
]


class HelloResponse(BaseModel):
    """Simple hello-world response used in examples and smoke tests."""

    message: str = Field(..., description="Human-readable greeting message.")


class HealthzResponse(BaseModel):
    """Shape for /healthz responses exposed by app facets."""

    ok: bool = Field(..., description="Whether the service considers itself healthy.")
    service: str = Field(
        "app",
        description="Service name (usually 'app' or facet identifier).",
    )


class ReadyzResponse(BaseModel):
    """Shape for /readyz responses exposed by app facets."""

    ready: bool = Field(..., description="Whether the service is ready to serve traffic.")
    reason: Optional[str] = Field(
        None,
        description="Optional human-readable reason when not ready.",
    )


class Event(BaseModel):
    """Generic SSE/streaming event envelope.

    This maps closely to the standard EventSource semantics:

    - `id`: event id for at-most-once / resume behavior.
    - `event`: logical event type (e.g. "message", "state-change").
    - `data`: raw event payload as a single string (possibly multi-line).
    - `retry`: optional server hint for reconnection delay (milliseconds).
    """

    id: Optional[str] = None
    event: Optional[str] = None
    data: str
    retry: Optional[int] = None

    def json_data(self) -> Any:
        """Best-effort JSON parse of `data`.

        Returns the parsed JSON value on success; otherwise returns the raw
        string unchanged. This keeps callers from having to wrap try/except
        around every event when most payloads are JSON-encoded.
        """
        try:
            return json.loads(self.data)
        except Exception:
            return self.data

    @property
    def dict_data(self) -> Dict[str, Any]:
        """Convenience accessor for dict-shaped payloads.

        If the underlying data is not a JSON object, this returns an empty
        dict rather than raising.
        """
        parsed = self.json_data()
        if isinstance(parsed, dict):
            return parsed
        return {}
