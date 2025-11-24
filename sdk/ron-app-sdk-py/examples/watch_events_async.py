from __future__ import annotations

import asyncio

from ron_app_sdk_py import RonClient, ClientConfig
from ron_app_sdk_py.streaming import sse_event_stream


async def main() -> None:
    cfg = ClientConfig(
        base_url="http://localhost:8080",
        allow_insecure_http=True,
    )
    client = RonClient(config=cfg)

    async for event in sse_event_stream(client._client, "/app/events"):
        print("EVENT:", event.json_data())


if __name__ == "__main__":
    asyncio.run(main())
