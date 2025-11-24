"""
Internal shared typing helpers for ron-app-sdk-py.

These keep the rest of the package imports light and avoid circular imports.
"""

from __future__ import annotations

from typing import Dict, Mapping, MutableMapping, Protocol, Union

# Generic JSON value used in response bodies
JsonValue = Union[
    str,
    int,
    float,
    bool,
    None,
    "JsonDict",
    "JsonList",
]

JsonDict = Dict[str, JsonValue]
JsonList = list[JsonValue]

# Narrower scalar type used for query parameters
QueryValue = Union[str, int, float, bool, None]
QueryParams = Mapping[str, QueryValue]

MutableHeaders = MutableMapping[str, str]


class TokenProvider(Protocol):
    """Callable that returns an access token string (e.g., macaroon or bearer).

    This can be implemented by the caller to refresh tokens on demand.
    """

    def __call__(self) -> str:  # pragma: no cover - protocol signature
        ...


__all__ = [
    "JsonValue",
    "JsonDict",
    "JsonList",
    "QueryValue",
    "QueryParams",
    "MutableHeaders",
    "TokenProvider",
]
