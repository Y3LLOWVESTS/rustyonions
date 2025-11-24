from __future__ import annotations

import asyncio

from ron_app_sdk_py import RonClient, ClientConfig, iter_pages


async def main() -> None:
    cfg = ClientConfig(
        base_url="http://localhost:8080",
        allow_insecure_http=True,
    )
    client = RonClient(config=cfg)

    async for page in iter_pages(client, "/app/items", page_size=25):
        print("Page:", page.items)

    await client.aclose()


if __name__ == "__main__":
    asyncio.run(main())
