from __future__ import annotations

import os
from typing import Dict

from ron_app_sdk_py import ClientConfig


def _with_env(env: Dict[str, str]) -> None:
    """Helper to patch os.environ in-place for the duration of a test."""
    os.environ.clear()
    os.environ.update(env)


def test_config_env_defaults() -> None:
    _with_env({})
    cfg = ClientConfig.from_env()

    assert cfg.base_url == "http://127.0.0.1:8080"
    assert cfg.overall_timeout_ms == 10_000
    assert cfg.connect_timeout_ms == 3_000
    assert cfg.read_timeout_ms == 7_000
    assert cfg.write_timeout_ms == 5_000
    # No token by default
    assert cfg.token is None


def test_config_env_app_url() -> None:
    _with_env({"RON_APP_URL": "https://example.com/app"})
    cfg = ClientConfig.from_env()

    assert cfg.base_url == "https://example.com/app"


def test_config_env_gateway_wins_over_app_url() -> None:
    _with_env(
        {
            "RON_APP_URL": "https://example.com/app",
            "RON_SDK_GATEWAY_ADDR": "https://gateway.ron-core.local:9443",
        }
    )
    cfg = ClientConfig.from_env()

    # Gateway addr takes precedence when both are set
    assert cfg.base_url == "https://gateway.ron-core.local:9443"


def test_config_env_timeouts_and_token() -> None:
    _with_env(
        {
            "RON_APP_URL": "https://example.com/app",
            "RON_SDK_OVERALL_TIMEOUT_MS": "12345",
            "RON_SDK_CONNECT_TIMEOUT_MS": "1111",
            "RON_SDK_READ_TIMEOUT_MS": "2222",
            "RON_SDK_WRITE_TIMEOUT_MS": "3333",
            "RON_APP_TOKEN": "cap-abc123",
        }
    )
    cfg = ClientConfig.from_env()

    assert cfg.overall_timeout_ms == 12_345
    assert cfg.connect_timeout_ms == 1_111
    assert cfg.read_timeout_ms == 2_222
    assert cfg.write_timeout_ms == 3_333
    assert cfg.token is not None
    assert cfg.token.get_secret_value() == "cap-abc123"
