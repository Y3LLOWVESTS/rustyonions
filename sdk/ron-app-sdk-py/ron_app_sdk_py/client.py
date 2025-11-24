"""
Async client for talking to RON-CORE via svc-gateway.

RO:WHAT
    RonClient is the async-first HTTP client used by apps and tests.

RO:WHY
    - Centralize retries, timeouts, auth headers, and Problem parsing.
    - Provide a small, stable surface for polyglot SDKs to mirror.

RO:INVARIANTS
    - Structured errors only (Problem envelopes for 4xx/5xx where possible).
    - No unbounded retries; only idempotent ops get bounded backoff.
    - Response size is capped by max_response_bytes in ClientConfig.
"""

from __future__ import annotations

import asyncio
import logging
import random
import time
import uuid
from typing import Any, AsyncIterator, Dict, Optional

import httpx

from .config import ClientConfig
from .errors import (
    Problem,
    RonAuthError,
    RonConfigError,
    RonNetworkError,
    RonParseError,
    RonProblemError,
    RonTimeoutError,
)
from ._types import JsonDict, QueryParams, TokenProvider
from .logging_ import get_logger, log_request, log_response
from .metrics import RequestMetrics

__all__ = ["RonClient"]

_LOGGER = logging.getLogger(__name__)


class RonClient:
    """Async-first HTTP client for talking to RON-CORE via svc-gateway."""

    def __init__(
        self,
        *,
        config: Optional[ClientConfig] = None,
        base_url: Optional[str] = None,
        token: Optional[str] = None,
        token_provider: Optional[TokenProvider] = None,
        client: Optional[httpx.AsyncClient] = None,
    ) -> None:
        if config is None:
            config = ClientConfig.from_env()

        if base_url is not None:
            config = config.with_overrides(base_url=base_url)

        if token is not None and config.token is None:
            # Import locally so config.py can be imported without pydantic,
            # but we already depend on pydantic so this is cheap.
            from pydantic import SecretStr

            config = config.with_overrides(token=SecretStr(token))

        self._config: ClientConfig = config
        self._token_provider: Optional[TokenProvider] = token_provider
        self._client = client or httpx.AsyncClient(
            base_url=config.base_url,
            timeout=config.to_httpx_timeout(),
            verify=config.httpx_verify_arg(),
        )
        self._semaphore: Optional[asyncio.Semaphore] = (
            asyncio.Semaphore(config.max_concurrency)
            if config.max_concurrency and config.max_concurrency > 0
            else None
        )
        self._closed = False

        # Per-client logger and metrics, kept lightweight and opt-in.
        self._logger = get_logger()
        self._metrics = RequestMetrics()

        if (
            not config.allow_insecure_http
            and self._config.base_url.startswith("http://")
        ):
            raise RonConfigError(
                "Insecure HTTP base_url used without allow_insecure_http=True. "
                "For production nodes, always use HTTPS."
            )

    @classmethod
    def from_env(cls, **kwargs: Any) -> "RonClient":
        """Construct a client using ClientConfig.from_env()."""
        cfg = ClientConfig.from_env()
        return cls(config=cfg, **kwargs)

    @property
    def config(self) -> ClientConfig:
        """Return the current client configuration."""
        return self._config

    @property
    def metrics(self) -> RequestMetrics:
        """Return the per-client metrics object."""
        return self._metrics

    async def aclose(self) -> None:
        """Close underlying connections and best-effort zeroize caps."""
        if self._closed:
            return
        self._closed = True

        await self._client.aclose()

        # best-effort zeroize: drop references to caps/token provider
        if self._config.token is not None:
            self._config = self._config.with_overrides(token=None)
        self._token_provider = None

    def close(self) -> None:
        """Synchronous close helper for convenience."""
        asyncio.run(self.aclose())

    # ------------------------------------------------------------------ #
    # Public request helpers                                             #
    # ------------------------------------------------------------------ #

    async def get(self, path: str, *, query: Optional[QueryParams] = None) -> JsonDict:
        return await self.call("GET", path, query=query)

    async def post(
        self,
        path: str,
        *,
        json: Optional[JsonDict] = None,
        query: Optional[QueryParams] = None,
        idem: bool = False,
    ) -> JsonDict:
        return await self.call("POST", path, json=json, query=query, idem=idem)

    async def put(
        self,
        path: str,
        *,
        json: Optional[JsonDict] = None,
        query: Optional[QueryParams] = None,
        idem: bool = False,
    ) -> JsonDict:
        return await self.call("PUT", path, json=json, query=query, idem=idem)

    async def delete(
        self,
        path: str,
        *,
        query: Optional[QueryParams] = None,
        idem: bool = False,
    ) -> JsonDict:
        return await self.call("DELETE", path, query=query, idem=idem)

    async def call(
        self,
        method: str,
        path: str,
        *,
        json: Optional[JsonDict] = None,
        query: Optional[QueryParams] = None,
        idem: bool = False,
    ) -> JsonDict:
        """Low-level call primitive with basic retries and error mapping."""
        method = method.upper()
        is_idempotent = method in ("GET", "HEAD", "OPTIONS") or idem

        if is_idempotent and self._config.max_retries > 0:
            return await self._request_with_retries(
                method=method,
                path=path,
                json=json,
                query=query,
                idem=idem,
            )

        return await self._request_once(
            method=method,
            path=path,
            json=json,
            query=query,
            idem=idem,
        )

    # ------------------------------------------------------------------ #
    # Internal helpers                                                   #
    # ------------------------------------------------------------------ #

    def _current_token(self) -> Optional[str]:
        token: Optional[str] = None

        if self._token_provider is not None:
            try:
                token = self._token_provider()
            except Exception as exc:  # noqa: BLE001
                _LOGGER.exception("token_provider raised an error: %s", exc)
                raise RonConfigError("token_provider callable raised an error") from exc

        if token is None and self._config.token is not None:
            token = self._config.token.get_secret_value()

        return token

    def _build_headers(self, *, idem: bool) -> Dict[str, str]:
        headers: Dict[str, str] = {}

        token = self._current_token()
        if token:
            headers["Authorization"] = f"Bearer {token}"

        # Correlation ID for tracing; safe to log
        headers["X-Request-Id"] = uuid.uuid4().hex

        if idem:
            headers["Idempotency-Key"] = str(uuid.uuid4())

        return headers

    def _record_metrics_and_log(
        self,
        *,
        method: str,
        path: str,
        headers: Dict[str, str],
        status_code: int,
        elapsed_ms: float,
    ) -> None:
        """Record per-request metrics and emit a log line."""
        request_id = headers.get("X-Request-Id") or headers.get("x-request-id")
        idempotency_key = headers.get("Idempotency-Key") or headers.get(
            "idempotency-key"
        )

        if self._metrics is not None:
            if status_code >= 400:
                self._metrics.record_error(
                    method=method,
                    path=path,
                    status_code=status_code,
                    elapsed_ms=elapsed_ms,
                )
            else:
                self._metrics.record_success(
                    method=method,
                    path=path,
                    status_code=status_code,
                    elapsed_ms=elapsed_ms,
                )

        log_response(
            self._logger,
            method=method,
            path=path,
            status_code=status_code,
            elapsed_ms=elapsed_ms,
            request_id=request_id,
            idempotency_key=idempotency_key,
        )

    async def _request_with_retries(
        self,
        *,
        method: str,
        path: str,
        json: Optional[JsonDict],
        query: Optional[QueryParams],
        idem: bool,
    ) -> JsonDict:
        attempts = self._config.max_retries + 1
        last_error: Optional[Exception] = None

        for attempt in range(1, attempts + 1):
            try:
                return await self._request_once(
                    method=method,
                    path=path,
                    json=json,
                    query=query,
                    idem=idem,
                )
            except (RonTimeoutError, RonNetworkError) as exc:
                last_error = exc
                if attempt >= attempts:
                    raise

                # Jittered exponential backoff
                base = 0.2 * (2 ** (attempt - 1))
                jitter = random.uniform(0.0, 0.2)
                await asyncio.sleep(base + jitter)

        # Should not reach here
        assert last_error is not None
        raise last_error

    async def _request_once(
        self,
        *,
        method: str,
        path: str,
        json: Optional[JsonDict],
        query: Optional[QueryParams],
        idem: bool,
    ) -> JsonDict:
        if self._closed:
            raise RonConfigError("RonClient is closed")

        headers = self._build_headers(idem=idem)

        # Log outbound request (headers are redacted by logging_ helpers)
        log_request(self._logger, method=method, path=path, headers=headers)

        if self._semaphore is None:
            return await self._send_request(
                method=method,
                path=path,
                headers=headers,
                json=json,
                query=query,
            )

        async with self._semaphore:
            return await self._send_request(
                method=method,
                path=path,
                headers=headers,
                json=json,
                query=query,
            )

    async def _send_request(
        self,
        *,
        method: str,
        path: str,
        headers: Dict[str, str],
        json: Optional[JsonDict],
        query: Optional[QueryParams],
    ) -> JsonDict:
        start = time.perf_counter()

        try:
            response = await self._client.request(
                method,
                path,
                headers=headers,
                json=json,
                params=query,
            )
        except httpx.TimeoutException as exc:
            # Timeout is treated as a transient error for metrics purposes.
            if self._metrics is not None:
                self._metrics.record_timeout(method=method, path=path)
            raise RonTimeoutError("Request timed out") from exc
        except httpx.RequestError as exc:
            # Network-level failure (DNS/TLS/socket).
            if self._metrics is not None:
                self._metrics.record_network_error(method=method, path=path)
            raise RonNetworkError(
                "Network error while calling RON-CORE node"
            ) from exc

        elapsed_ms = (time.perf_counter() - start) * 1000.0
        status = response.status_code

        # Response size guard
        if (
            self._config.max_response_bytes is not None
            and len(response.content) > self._config.max_response_bytes
        ):
            if self._metrics is not None:
                self._metrics.record_network_error(method=method, path=path)
            raise RonNetworkError(
                f"Response exceeded max_response_bytes={self._config.max_response_bytes}"
            )

        # From here down, we have a valid response body; update metrics + logs
        self._record_metrics_and_log(
            method=method,
            path=path,
            headers=headers,
            status_code=status,
            elapsed_ms=elapsed_ms,
        )

        if status in (401, 403):
            raise RonAuthError(f"Auth failed with status {status}")

        if status >= 400:
            # Attempt to parse canonical Problem envelope
            try:
                data = response.json()
            except ValueError as exc:  # noqa: TRY003
                raise RonParseError(
                    "Failed to parse error response as JSON"
                ) from exc

            try:
                problem = Problem.model_validate(data)
            except Exception as exc:  # noqa: BLE001
                raise RonParseError("Failed to parse Problem envelope") from exc

            raise RonProblemError(problem, status)

        if status == 204:
            return {}

        try:
            data = response.json()
        except ValueError as exc:
            raise RonParseError("Failed to parse response JSON") from exc

        if not isinstance(data, dict):
            # For now, wrap non-dict JSON so callers still have a dict
            return {"value": data}

        return data

    # ------------------------------------------------------------------ #
    # Streaming (MVP stub)                                               #
    # ------------------------------------------------------------------ #

    async def subscribe(
        self,
        path: str,
        *,
        query: Optional[QueryParams] = None,
    ) -> AsyncIterator[JsonDict]:
        """Placeholder for SSE-style streaming.

        Future work will implement:

            async for event in client.subscribe("/app/events"):
                ...

        For now this is a stub so the surface exists but clearly unsupported.
        """
        raise NotImplementedError("Streaming/subscribe is not implemented yet")
