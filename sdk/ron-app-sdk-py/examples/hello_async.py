from __future__ import annotations

import asyncio

from ron_app_sdk_py import RonClient, ClientConfig


async def main() -> None:
    cfg = ClientConfig(
        base_url="http://localhost:8080",
        allow_insecure_http=True,
        token=None,
    )
    client = RonClient(config=cfg)

    resp = await client.get("/app/hello")
    print("hello async:", resp)

    await client.aclose()


if __name__ == "__main__":
    asyncio.run(main())
