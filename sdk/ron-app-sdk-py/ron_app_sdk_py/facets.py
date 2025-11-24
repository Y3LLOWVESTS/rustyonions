from __future__ import annotations

from typing import Optional, TYPE_CHECKING

from ._types import JsonDict, QueryParams

if TYPE_CHECKING:
    from .client import RonClient  # pragma: no cover

"""
Facet-scoped helpers for ron-app-sdk-py.

RO:WHAT
    Provide a thin, typed wrapper around `RonClient` that prefixes all paths
    with a facet id:

        users = FacetClient(client, "users")
        await users.get("/me")  # -> GET /app/users/me

RO:WHY
    Keep app code from repeating facet segments and "/app" prefixes by hand.

RO:INVARIANTS
    - Facet ids never start with "/" internally (normalized).
    - `path` arguments may be with or without a leading "/".
    - Surface is a thin passthrough over RonClient.get/post/put/delete.
"""

__all__ = [
    "FacetClient",
]


class FacetClient:
    """Facet-scoped client that prefixes all paths with "/app/{facetId}"."""

    def __init__(self, ron: "RonClient", facet_id: str) -> None:
        # Normalize facet id to avoid accidental leading slashes.
        self._ron = ron
        self._facet_id = facet_id.lstrip("/")

    def _build_path(self, path: str) -> str:
        # Normalize path to always have a leading slash.
        if not path:
            suffix = ""
        else:
            suffix = path if path.startswith("/") else f"/{path}"
        return f"/app/{self._facet_id}{suffix}"

    async def get(
        self,
        path: str,
        *,
        query: Optional[QueryParams] = None,
    ) -> JsonDict:
        """Issue a GET against this facet."""
        return await self._ron.get(self._build_path(path), query=query)

    async def post(
        self,
        path: str,
        *,
        json: Optional[JsonDict] = None,
        query: Optional[QueryParams] = None,
        idem: bool = False,
    ) -> JsonDict:
        """Issue a POST against this facet."""
        return await self._ron.post(
            self._build_path(path),
            json=json,
            query=query,
            idem=idem,
        )

    async def put(
        self,
        path: str,
        *,
        json: Optional[JsonDict] = None,
        query: Optional[QueryParams] = None,
        idem: bool = False,
    ) -> JsonDict:
        """Issue a PUT against this facet."""
        return await self._ron.put(
            self._build_path(path),
            json=json,
            query=query,
            idem=idem,
        )

    async def delete(
        self,
        path: str,
        *,
        query: Optional[QueryParams] = None,
    ) -> JsonDict:
        """Issue a DELETE against this facet."""
        return await self._ron.delete(
            self._build_path(path),
            query=query,
        )
