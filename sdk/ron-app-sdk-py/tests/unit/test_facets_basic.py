from __future__ import annotations

import asyncio
from typing import Any, Dict, Optional

from ron_app_sdk_py.facets import FacetClient
from ron_app_sdk_py._types import JsonDict, QueryParams


class _FakeRonClient:
    def __init__(self) -> None:
        self.calls: list[Dict[str, Any]] = []

    async def get(self, path: str, *, query: Optional[QueryParams] = None) -> JsonDict:
        self.calls.append({"method": "GET", "path": path, "query": query})
        return {"ok": True}

    async def post(
        self,
        path: str,
        *,
        json: Any = None,
        query: Optional[QueryParams] = None,
        idem: bool = False,
    ) -> JsonDict:
        self.calls.append(
            {
                "method": "POST",
                "path": path,
                "query": query,
                "json": json,
                "idem": idem,
            }
        )
        return {"ok": True}

    async def delete(
        self,
        path: str,
        *,
        query: Optional[QueryParams] = None,
    ) -> JsonDict:
        self.calls.append({"method": "DELETE", "path": path, "query": query})
        return {"ok": True}


def test_facet_client_prefixes_paths() -> None:
    async def _run() -> None:
        ron = _FakeRonClient()
        users = FacetClient(ron, "users")

        await users.get("/me")
        await users.post("create", json={"name": "alice"}, idem=True)
        await users.delete("old")

        assert len(ron.calls) == 3

        assert ron.calls[0]["method"] == "GET"
        assert ron.calls[0]["path"] == "/app/users/me"

        assert ron.calls[1]["method"] == "POST"
        assert ron.calls[1]["path"] == "/app/users/create"
        assert ron.calls[1]["json"] == {"name": "alice"}
        assert ron.calls[1]["idem"] is True

        assert ron.calls[2]["method"] == "DELETE"
        assert ron.calls[2]["path"] == "/app/users/old"

    asyncio.run(_run())
