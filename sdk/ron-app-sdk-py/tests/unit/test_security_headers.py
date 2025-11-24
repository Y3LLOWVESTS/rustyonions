from __future__ import annotations

from typing import Optional

from pydantic import SecretStr

from ron_app_sdk_py import ClientConfig, RonClient


def test_build_headers_includes_auth_and_request_id() -> None:
    cfg = ClientConfig(
        base_url="https://test",
        token=SecretStr("cap-123"),
    )
    client = RonClient(config=cfg)

    try:
        # Access the internal helper to verify header shape
        headers = client._build_headers(idem=False)  # type: ignore[attr-defined]

        # Authorization header derived from token
        assert headers.get("Authorization") == "Bearer cap-123"
        # Correlation header always present
        assert "X-Request-Id" in headers
        assert isinstance(headers["X-Request-Id"], str)
        assert headers["X-Request-Id"]
        # No idempotency key when idem=False
        assert "Idempotency-Key" not in headers
    finally:
        client.close()


def test_build_headers_idempotency_key_and_token_provider_override() -> None:
    def token_provider() -> str:
        return "cap-from-provider"

    cfg = ClientConfig(
        base_url="https://test",
        token=SecretStr("cap-ignored"),
    )
    client = RonClient(config=cfg, token_provider=token_provider)

    try:
        headers = client._build_headers(idem=True)  # type: ignore[attr-defined]

        # Token provider should win over static config token
        assert headers.get("Authorization") == "Bearer cap-from-provider"

        # Idempotency key for idem=True
        idem_key: Optional[str] = headers.get("Idempotency-Key")
        assert idem_key is not None
        assert isinstance(idem_key, str)
        assert idem_key

        # Still has a request id
        assert "X-Request-Id" in headers
    finally:
        client.close()
