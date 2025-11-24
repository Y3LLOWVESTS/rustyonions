"""
Synchronous wrapper for RonClient.

RO:WHAT
    RonClientSync wraps the async RonClient with blocking helpers.

RO:WHY
    - Make it easy to call nodes from small scripts / CLIs without async.
    - Keep the surface parallel to the async client for DX.

RO:INVARIANTS
    - Uses asyncio.run per call (simple, not tuned for high QPS).
    - For services, prefer the async RonClient directly.
"""

from __future__ import annotations

import asyncio
from typing import Optional

from ._types import JsonDict, QueryParams
from .client import RonClient

__all__ = ["RonClientSync"]


class RonClientSync:
    """Simple synchronous wrapper around :class:`RonClient`."""

    def __init__(
        self,
        *,
        base_url: Optional[str] = None,
        token: Optional[str] = None,
    ) -> None:
        self._client = RonClient(base_url=base_url, token=token)

    @classmethod
    def from_env(cls) -> "RonClientSync":
        """Construct a sync client using env-based defaults."""
        # RonClient() with no explicit config delegates to ClientConfig.from_env(),
        # so this keeps semantics aligned with the async client.
        return cls()

    def close(self) -> None:
        asyncio.run(self._client.aclose())

    def get(self, path: str, *, query: Optional[QueryParams] = None) -> JsonDict:
        return asyncio.run(self._client.get(path, query=query))

    def post(
        self,
        path: str,
        *,
        json: Optional[JsonDict] = None,
        query: Optional[QueryParams] = None,
        idem: bool = False,
    ) -> JsonDict:
        return asyncio.run(
            self._client.post(path, json=json, query=query, idem=idem)
        )

    def put(
        self,
        path: str,
        *,
        json: Optional[JsonDict] = None,
        query: Optional[QueryParams] = None,
        idem: bool = False,
    ) -> JsonDict:
        return asyncio.run(
            self._client.put(path, json=json, query=query, idem=idem)
        )

    def delete(
        self,
        path: str,
        *,
        query: Optional[QueryParams] = None,
        idem: bool = False,
    ) -> JsonDict:
        return asyncio.run(
            self._client.delete(path, query=query, idem=idem)
        )

    def call(
        self,
        method: str,
        path: str,
        *,
        json: Optional[JsonDict] = None,
        query: Optional[QueryParams] = None,
        idem: bool = False,
    ) -> JsonDict:
        return asyncio.run(
            self._client.call(
                method=method,
                path=path,
                json=json,
                query=query,
                idem=idem,
            )
        )
