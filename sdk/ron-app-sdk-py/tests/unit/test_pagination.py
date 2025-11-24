from __future__ import annotations

import asyncio
from typing import Any, List

import pytest

from ron_app_sdk_py.pagination import Page, iter_pages
from ron_app_sdk_py._types import JsonDict, QueryParams


class FakeClient:
    """Minimal fake client with a `get` method that matches RonClient's surface."""

    def __init__(self, pages: List[JsonDict]) -> None:
        self._pages = pages
        self.calls: int = 0
        self.received_cursors: List[str | None] = []

    async def get(self, path: str, *, query: QueryParams | None = None) -> JsonDict:
        self.calls += 1

        cursor = None
        if query is not None:
            cursor_val: Any | None = query.get("cursor")
            cursor = str(cursor_val) if cursor_val is not None else None

        self.received_cursors.append(cursor)

        # Simple cursor == index scheme for tests
        if cursor is None:
            index = 0
        else:
            index = int(cursor)

        if index >= len(self._pages):
            # Simulate empty final page
            return {"items": [], "next_cursor": None}

        page = dict(self._pages[index])
        # Make next_cursor point to the next index, or None if done.
        if index + 1 < len(self._pages):
            page.setdefault("next_cursor", str(index + 1))
        else:
            page.setdefault("next_cursor", None)
        return page


def test_iter_pages_walks_all_pages() -> None:
    async def _run() -> None:
        pages = [
            {"items": [{"id": 1}, {"id": 2}]},
            {"items": [{"id": 3}]},
        ]
        client = FakeClient(pages)

        collected: list[Page[JsonDict]] = []
        async for page in iter_pages(client, "/app/items", page_size=2):
            collected.append(page)

        # We should have seen both pages
        assert len(collected) == 2
        all_ids = [item["id"] for page in collected for item in page.items]
        assert all_ids == [1, 2, 3]

        # FakeClient should have seen correct cursor progression: [None, "1"]
        assert client.calls == 2
        assert client.received_cursors == [None, "1"]

    asyncio.run(_run())
