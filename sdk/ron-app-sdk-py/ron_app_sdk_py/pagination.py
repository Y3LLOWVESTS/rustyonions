"""
Pagination helpers for ron-app-sdk-py.

Provides a generic Page[T] container plus an async iterator helper `iter_pages`
that walks a paginated collection endpoint until exhaustion.

This is intentionally minimal for the MVP and aligned with SDK_SCHEMA_IDB:

- The server returns JSON objects of the form:
  {
      "items": [...],
      "next_cursor": "opaque-string-or-null"
  }

- We surface:
  - `Page.items`: the list of items for that page.
  - `Page.next_cursor`: cursor to pass back to the server for the next page.
  - `Page.raw`: the full raw JSON response, in case callers need extra fields.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import AsyncIterator, Generic, List, Optional, TypeVar, TYPE_CHECKING

from ._types import JsonDict, JsonValue, QueryParams, QueryValue

if TYPE_CHECKING:
    # Imported only for typing; avoids a hard runtime dependency cycle.
    from .client import RonClient

T = TypeVar("T")


@dataclass
class Page(Generic[T]):
    """Generic page of results returned from a paginated endpoint."""

    items: List[T]
    next_cursor: Optional[str] = None
    raw: Optional[JsonDict] = None


__all__ = ["Page", "iter_pages"]


async def iter_pages(
    client: "RonClient",
    path: str,
    *,
    page_size: int = 100,
    query: Optional[QueryParams] = None,
) -> AsyncIterator[Page[JsonDict]]:
    """Iterate over all pages for a paginated endpoint.

    Parameters
    ----------
    client:
        A configured `RonClient` instance.
    path:
        The app-plane path, e.g. "/app/items".
    page_size:
        Page size hint; controls the `limit`/`page_size` query parameter.
    query:
        Optional base query mapping. This will be shallow-copied and extended
        with pagination keys.

    Yields
    ------
    Page[JsonDict]
        A page of JSON objects. The exact item schema is app-dependent.

    Notes
    -----
    This helper assumes a cursor-based contract:

    - The server returns an object with:
      - "items": list
      - "next_cursor": string or null/absent

    - When "next_cursor" is falsy/missing, iteration stops.
    """
    cursor: Optional[str] = None

    while True:
        # Start from the caller's query and add pagination hints.
        params: dict[str, QueryValue] = dict(query or {})
        # Allow either "limit" or "page_size" depending on how the app facet
        # implements pagination. We send both; the server can ignore one.
        params.setdefault("limit", page_size)
        params.setdefault("page_size", page_size)

        if cursor is not None:
            params["cursor"] = cursor

        # We expect `client.get` to return a parsed JSON dict.
        resp: JsonDict = await client.get(path, query=params)

        # Defensive parsing in case the server omits fields.
        raw_items = resp.get("items") or []
        if not isinstance(raw_items, list):
            # Normalize to list to avoid surprising callers.
            raw_items = [raw_items]

        items: list[JsonDict] = []
        for it in raw_items:
            if isinstance(it, dict):
                items.append(it)  # already a JSON object
            else:
                # Wrap scalar/other values in an object for consistency.
                items.append({"value": it})

        next_cursor_value: JsonValue | None = resp.get("next_cursor")
        next_cursor = str(next_cursor_value) if next_cursor_value else None

        page = Page[JsonDict](items=items, next_cursor=next_cursor, raw=resp)
        yield page

        if not next_cursor:
            break

        cursor = next_cursor
