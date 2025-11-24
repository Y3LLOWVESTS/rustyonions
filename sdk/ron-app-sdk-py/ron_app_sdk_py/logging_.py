from __future__ import annotations

import logging
from typing import Mapping, Optional

"""
Lightweight logging helpers for ron-app-sdk-py.

RO:WHAT
    Small helpers that integrate RonClient with the standard `logging` module
    without forcing any global configuration.

RO:WHY
    - Keep request/response logging consistent across SDKs.
    - Centralize header redaction (never log caps/tokens).

RO:INVARIANTS
    - Never log Authorization/capability headers in plaintext.
    - Safe to call even if logging is disabled or minimally configured.
"""

__all__ = [
    "get_logger",
    "log_request",
    "log_response",
]


def get_logger(name: str = "ron_app_sdk_py.client") -> logging.Logger:
    """Return the default logger used by RonClient.

    The caller is responsible for configuring handlers/formatters as needed.
    """
    return logging.getLogger(name)


def _redact_headers(headers: Mapping[str, str]) -> Mapping[str, str]:
    """Return a redacted copy of headers safe for logging.

    Sensitive values like Authorization / caps are replaced with a placeholder.
    """
    redacted: dict[str, str] = {}
    for key, value in headers.items():
        lower = key.lower()
        if lower in ("authorization", "x-ron-cap", "x-api-key"):
            redacted[key] = "***REDACTED***"
        else:
            redacted[key] = value
    return redacted


def log_request(
    logger: logging.Logger,
    *,
    method: str,
    path: str,
    headers: Mapping[str, str],
) -> None:
    """Log an outbound HTTP request at DEBUG level.

    This is intentionally lightweight; if DEBUG is disabled the cost is minimal.
    """
    if not logger.isEnabledFor(logging.DEBUG):
        return

    safe_headers = _redact_headers(headers)
    request_id = safe_headers.get("X-Request-Id") or safe_headers.get("x-request-id")
    idempotency_key = safe_headers.get("Idempotency-Key") or safe_headers.get(
        "idempotency-key"
    )

    logger.debug(
        "ron-client request %s %s",
        method,
        path,
        extra={
            "request_id": request_id,
            "idempotency_key": idempotency_key,
            "headers": safe_headers,
        },
    )


def log_response(
    logger: logging.Logger,
    *,
    method: str,
    path: str,
    status_code: int,
    elapsed_ms: float,
    request_id: Optional[str] = None,
    idempotency_key: Optional[str] = None,
) -> None:
    """Log an HTTP response at INFO/DEBUG level.

    Callers typically log once per completed request (success or error).
    """
    if not logger.isEnabledFor(logging.INFO):
        return

    logger.info(
        "ron-client response %s %s -> %d in %.1fms",
        method,
        path,
        status_code,
        elapsed_ms,
        extra={
            "request_id": request_id,
            "idempotency_key": idempotency_key,
        },
    )
