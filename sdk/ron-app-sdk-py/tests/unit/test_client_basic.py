from __future__ import annotations

import asyncio

import httpx
import pytest

from ron_app_sdk_py import (
    ClientConfig,
    Problem,
    RonAuthError,
    RonClient,
    RonNetworkError,
    RonProblemError,
    RonTimeoutError,
)


def test_client_basic_get_success() -> None:
    async def _run() -> None:
        async def handler(request: httpx.Request) -> httpx.Response:
            assert request.method == "GET"
            assert request.url.path == "/app/hello"
            # Basic "hello" payload
            return httpx.Response(200, json={"message": "hello from test"})

        transport = httpx.MockTransport(handler)
        async_client = httpx.AsyncClient(transport=transport, base_url="https://test")

        cfg = ClientConfig(base_url="https://test")
        client = RonClient(config=cfg, client=async_client)

        try:
            resp = await client.get("/app/hello")
            assert resp == {"message": "hello from test"}
        finally:
            await client.aclose()

    asyncio.run(_run())


def test_client_maps_problem_response_to_ron_problem_error() -> None:
    async def _run() -> None:
        async def handler(request: httpx.Request) -> httpx.Response:
            problem = {
                "code": "internal_error",
                "kind": "internal",
                "message": "Boom",
                "correlation_id": "req-xyz",
                "details": {"foo": "bar"},
            }
            return httpx.Response(500, json=problem)

        transport = httpx.MockTransport(handler)
        async_client = httpx.AsyncClient(transport=transport, base_url="https://test")

        cfg = ClientConfig(base_url="https://test")
        client = RonClient(config=cfg, client=async_client)

        try:
            with pytest.raises(RonProblemError) as excinfo:
                await client.get("/app/hello")

            err = excinfo.value
            assert err.status_code == 500
            assert isinstance(err.problem, Problem)
            assert err.problem.code == "internal_error"
            assert err.problem.details == {"foo": "bar"}
        finally:
            await client.aclose()

    asyncio.run(_run())


def test_client_maps_401_403_to_auth_error() -> None:
    async def _run() -> None:
        async def handler(request: httpx.Request) -> httpx.Response:
            return httpx.Response(401, json={"message": "nope"})

        transport = httpx.MockTransport(handler)
        async_client = httpx.AsyncClient(transport=transport, base_url="https://test")

        cfg = ClientConfig(base_url="https://test")
        client = RonClient(config=cfg, client=async_client)

        try:
            with pytest.raises(RonAuthError):
                await client.get("/app/hello")
        finally:
            await client.aclose()

    asyncio.run(_run())


def test_client_maps_timeout_and_network_errors() -> None:
    async def _run() -> None:
        async def handler_timeout(request: httpx.Request) -> httpx.Response:
            raise httpx.TimeoutException("timeout", request=request)

        async def handler_network(request: httpx.Request) -> httpx.Response:
            raise httpx.RequestError("boom", request=request)

        cfg = ClientConfig(base_url="https://test")

        # Timeout transport
        transport_timeout = httpx.MockTransport(handler_timeout)
        async_client_timeout = httpx.AsyncClient(
            transport=transport_timeout,
            base_url="https://test",
        )
        client_timeout = RonClient(config=cfg, client=async_client_timeout)

        try:
            with pytest.raises(RonTimeoutError):
                await client_timeout.get("/app/hello")
        finally:
            await client_timeout.aclose()

        # Network error transport
        transport_net = httpx.MockTransport(handler_network)
        async_client_net = httpx.AsyncClient(
            transport=transport_net,
            base_url="https://test",
        )
        client_net = RonClient(config=cfg, client=async_client_net)

        try:
            with pytest.raises(RonNetworkError):
                await client_net.get("/app/hello")
        finally:
            await client_net.aclose()

    asyncio.run(_run())
