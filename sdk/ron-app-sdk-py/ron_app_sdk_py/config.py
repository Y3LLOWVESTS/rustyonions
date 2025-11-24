"""
Client configuration for ron-app-sdk-py.

RO:WHAT
    ClientConfig encapsulates base URL, timeouts, retries, and TLS options.

RO:WHY
    - Centralize config logic (env vars, defaults, overrides).
    - Keep RonClient construction simple and explicit.

RO:INVARIANTS
    - Explicit arguments override environment variables.
    - When both RON_SDK_GATEWAY_ADDR and RON_APP_URL are set, the gateway addr wins.
    - Base URL may be HTTP in local dev; HTTPS strongly recommended in prod.
"""

from __future__ import annotations

import dataclasses
import os
from dataclasses import dataclass
from typing import Any, Optional, Union

import httpx
from pydantic import SecretStr

__all__ = ["ClientConfig"]


@dataclass
class ClientConfig:
    """Configuration for RonClient / RonClientSync.

    Times are stored in milliseconds to match env vars; the HTTP layer can
    convert to seconds as needed.
    """

    base_url: str
    verify_tls: bool = True
    tls_ca_path: Optional[str] = None

    overall_timeout_ms: int = 10_000
    connect_timeout_ms: int = 3_000
    read_timeout_ms: int = 7_000
    write_timeout_ms: int = 5_000

    max_retries: int = 3
    max_concurrency: Optional[int] = None

    allow_insecure_http: bool = False
    max_response_bytes: int = 16 * 1024 * 1024

    # PQ stub for future hybrid TLS
    tls_pq_mode: str = "off"

    # Optional capability/macaroon (kept in memory, redacted in repr)
    token: Optional[SecretStr] = None

    @classmethod
    def from_env(cls) -> "ClientConfig":
        """Build config from canonical env vars.

        Precedence:
        * RON_SDK_GATEWAY_ADDR (if set)
        * RON_APP_URL
        * fallback http://127.0.0.1:8080
        """
        env = os.environ

        base_url = (
            env.get("RON_SDK_GATEWAY_ADDR")
            or env.get("RON_APP_URL")
            or "http://127.0.0.1:8080"
        )

        overall_timeout_ms = int(env.get("RON_SDK_OVERALL_TIMEOUT_MS", "10000"))
        connect_timeout_ms = int(env.get("RON_SDK_CONNECT_TIMEOUT_MS", "3000"))
        read_timeout_ms = int(env.get("RON_SDK_READ_TIMEOUT_MS", "7000"))
        write_timeout_ms = int(env.get("RON_SDK_WRITE_TIMEOUT_MS", "5000"))

        token_env = env.get("RON_APP_TOKEN")
        token = SecretStr(token_env) if token_env else None

        return cls(
            base_url=base_url,
            overall_timeout_ms=overall_timeout_ms,
            connect_timeout_ms=connect_timeout_ms,
            read_timeout_ms=read_timeout_ms,
            write_timeout_ms=write_timeout_ms,
            token=token,
        )

    def with_overrides(self, **kwargs: Any) -> "ClientConfig":
        """Return a copy of this config with specific fields overridden."""
        data = dataclasses.asdict(self)
        data.update(kwargs)
        return ClientConfig(**data)

    def to_httpx_timeout(self) -> httpx.Timeout:
        """Build an httpx.Timeout instance from ms fields."""
        return httpx.Timeout(
            timeout=self.overall_timeout_ms / 1000.0,
            connect=self.connect_timeout_ms / 1000.0,
            read=self.read_timeout_ms / 1000.0,
            write=self.write_timeout_ms / 1000.0,
        )

    def httpx_verify_arg(self) -> Union[bool, str]:
        """Return the `verify` argument for httpx (bool or CA path).

        This is kept narrow to match httpx.AsyncClient's type expectations.
        """
        if not self.verify_tls:
            return False
        if self.tls_ca_path:
            return self.tls_ca_path
        return True
