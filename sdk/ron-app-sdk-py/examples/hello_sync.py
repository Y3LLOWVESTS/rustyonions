from __future__ import annotations

from ron_app_sdk_py import RonClientSync, ClientConfig


def main() -> None:
    cfg = ClientConfig(
        base_url="http://localhost:8080",
        allow_insecure_http=True,
        token=None,
    )

    client = RonClientSync(config=cfg)
    resp = client.get("/app/hello")
    print("hello sync:", resp)
    client.close()


if __name__ == "__main__":
    main()
