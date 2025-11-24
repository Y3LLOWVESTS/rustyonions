from __future__ import annotations

from datetime import datetime, timezone
import base64

"""
Small codec utilities for ron-app-sdk-py.

RO:WHAT
    Helpers for encoding/decoding common wire-level formats used by RON-CORE:
    - u64 as string in JSON
    - base64url for binary blobs
    - RFC 3339 / ISO-8601 timestamps

RO:WHY
    Keep these details out of client/business logic so apps and tests can share
    a single, well-tested implementation.

RO:INVARIANTS
    - u64 helpers never accept negative values.
    - base64url functions are padding-tolerant on decode.
    - Timestamps are normalized to UTC with a trailing "Z".
"""


__all__ = [
    "u64_to_str",
    "u64_from_str",
    "b64url_encode",
    "b64url_decode",
    "parse_iso8601",
    "format_iso8601",
]


def u64_to_str(value: int) -> str:
    """Encode a Python int as a JSON-friendly u64 string.

    Raises
    ------
    ValueError
        If the value is negative.
    """
    if value < 0:
        raise ValueError("u64 value cannot be negative")
    return str(value)


def u64_from_str(value: str) -> int:
    """Parse a JSON u64 string back into a Python int.

    Raises
    ------
    ValueError
        If the string is empty, not an integer, or represents a negative value.
    """
    if not value:
        raise ValueError("u64 string cannot be empty")

    num = int(value, 10)
    if num < 0:
        raise ValueError("u64 value cannot be negative")
    return num


def b64url_encode(data: bytes) -> str:
    """Encode bytes as URL-safe base64 without padding.

    This matches the usual JWS/JWT-style encoding that omits '=' padding.
    """
    if not isinstance(data, (bytes, bytearray, memoryview)):
        raise TypeError("b64url_encode expects a bytes-like object")

    encoded = base64.urlsafe_b64encode(bytes(data))
    return encoded.rstrip(b"=").decode("ascii")


def b64url_decode(data: str) -> bytes:
    """Decode a URL-safe base64 string (with or without padding)."""
    if not isinstance(data, str):
        raise TypeError("b64url_decode expects a string")

    raw = data.encode("ascii")

    # Pad to multiple of 4 as required by the base64 codec.
    pad_len = (4 - (len(raw) % 4)) % 4
    raw += b"=" * pad_len

    return base64.urlsafe_b64decode(raw)


def parse_iso8601(value: str) -> datetime:
    """Parse a timestamp string into a timezone-aware UTC datetime.

    Accepts the common "Z" suffix (e.g. "2025-01-01T00:00:00Z") and ensures
    the result is timezone-aware in UTC.
    """
    if not isinstance(value, str):
        raise TypeError("parse_iso8601 expects a string")

    # datetime.fromisoformat does not understand "Z", so normalize first.
    normalized = value.replace("Z", "+00:00")
    dt = datetime.fromisoformat(normalized)

    if dt.tzinfo is None:
        dt = dt.replace(tzinfo=timezone.utc)

    return dt.astimezone(timezone.utc)


def format_iso8601(dt: datetime) -> str:
    """Format a datetime as an ISO-8601 string in UTC with a trailing 'Z'."""
    if dt.tzinfo is None:
        dt = dt.replace(tzinfo=timezone.utc)

    dt_utc = dt.astimezone(timezone.utc)
    # Use ISO format then normalize "+00:00" to "Z".
    return dt_utc.isoformat().replace("+00:00", "Z")
