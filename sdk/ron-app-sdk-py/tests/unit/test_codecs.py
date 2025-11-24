from __future__ import annotations

from datetime import datetime, timezone

from ron_app_sdk_py.codecs import (
    b64url_decode,
    b64url_encode,
    format_iso8601,
    parse_iso8601,
    u64_from_str,
    u64_to_str,
)


def test_u64_roundtrip() -> None:
    value = 12345678901234567890
    s = u64_to_str(value)
    assert s == "12345678901234567890"
    back = u64_from_str(s)
    assert back == value


def test_b64url_roundtrip() -> None:
    payload = b"hello-world"
    encoded = b64url_encode(payload)
    # No padding
    assert "=" not in encoded
    decoded = b64url_decode(encoded)
    assert decoded == payload


def test_iso8601_roundtrip() -> None:
    dt = datetime(2025, 1, 1, 12, 0, 0, tzinfo=timezone.utc)
    s = format_iso8601(dt)
    assert s.endswith("Z")
    parsed = parse_iso8601(s)
    assert parsed == dt
