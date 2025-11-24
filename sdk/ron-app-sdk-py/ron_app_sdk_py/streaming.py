from __future__ import annotations

import asyncio
import random
from typing import AsyncIterator, Awaitable, Callable, Mapping, Optional, TYPE_CHECKING

import httpx

from .models import Event

if TYPE_CHECKING:
    # Imported only for typing to avoid a hard import cycle.
    from .client import RonClient  # pragma: no cover

"""
Streaming (SSE-style) helpers for ron-app-sdk-py.

RO:WHAT
    Utilities for consuming server-sent events (SSE) / streaming responses
    from RON-CORE app-plane endpoints.

RO:WHY
    - Keep SSE framing and reconnect behavior in one place.
    - Allow app code to work with typed `Event` objects instead of raw lines.

RO:INVARIANTS
    - At-most-once delivery from the perspective of the user callback.
    - Reconnects use exponential backoff + jitter when enabled.
    - Caller controls shutdown via an asyncio.Event.
"""


__all__ = [
    "sse_event_stream",
    "subscribe",
]


async def sse_event_stream(
    client: httpx.AsyncClient,
    path: str,
    *,
    headers: Optional[Mapping[str, str]] = None,
    last_event_id: Optional[str] = None,
) -> AsyncIterator[Event]:
    """Yield `Event` objects from an SSE endpoint.

    Parameters
    ----------
    client:
        An `httpx.AsyncClient` instance to use for the streaming request.
    path:
        Request path (e.g. "/app/events").
    headers:
        Optional additional headers. "Accept: text/event-stream" is added
        automatically if missing.
    last_event_id:
        Optional last event id for resume semantics.
    """
    req_headers: dict[str, str] = dict(headers or {})
    req_headers.setdefault("accept", "text/event-stream")
    if last_event_id:
        req_headers["last-event-id"] = last_event_id

    async with client.stream("GET", path, headers=req_headers, timeout=None) as resp:
        resp.raise_for_status()

        field_id: Optional[str] = None
        field_event: Optional[str] = None
        data_lines: list[str] = []
        retry_ms: Optional[int] = None

        async for raw_line in resp.aiter_lines():
            # SSE uses empty line as event delimiter.
            if raw_line == "":
                if data_lines:
                    data = "\n".join(data_lines)
                    yield Event(
                        id=field_id,
                        event=field_event,
                        data=data,
                        retry=retry_ms,
                    )
                # Reset buffers for next event.
                field_id = None
                field_event = None
                data_lines = []
                retry_ms = None
                continue

            if raw_line.startswith(":"):
                # Comment / heartbeat; ignore.
                continue

            # Field: value
            if ":" in raw_line:
                field, value = raw_line.split(":", 1)
                # Strip the single leading space from the value if present.
                if value.startswith(" "):
                    value = value[1:]
            else:
                field = raw_line
                value = ""

            if field == "id":
                field_id = value
            elif field == "event":
                field_event = value
            elif field == "data":
                data_lines.append(value)
            elif field == "retry":
                try:
                    retry_ms = int(value)
                except ValueError:
                    retry_ms = None

        # End-of-stream flush in case server didn't terminate with a blank line.
        if data_lines:
            data = "\n".join(data_lines)
            yield Event(
                id=field_id,
                event=field_event,
                data=data,
                retry=retry_ms,
            )


async def subscribe(
    ron: "RonClient",
    path: str,
    callback: Callable[[Event], Awaitable[None]],
    *,
    stop_event: Optional[asyncio.Event] = None,
    max_retries: Optional[int] = 5,
    initial_backoff: float = 0.5,
    max_backoff: float = 10.0,
) -> None:
    """Subscribe to an SSE endpoint and dispatch events to a callback.

    This helper uses the underlying httpx client held by `RonClient` and
    automatically reconnects with exponential backoff + jitter.
    """
    last_event_id: Optional[str] = None
    attempt = 0

    while True:
        if stop_event is not None and stop_event.is_set():
            return

        try:
            # Use the underlying httpx client to share connection pools.
            httpx_client = ron._client
            async for event in sse_event_stream(
                httpx_client,
                path,
                last_event_id=last_event_id,
            ):
                attempt = 0
                if event.id is not None:
                    last_event_id = event.id
                await callback(event)
        except Exception:
            attempt += 1
            if max_retries is not None and attempt > max_retries:
                raise

            delay = min(initial_backoff * (2 ** (attempt - 1)), max_backoff)
            # Add +/- 50% jitter.
            jitter = random.uniform(0.5 * delay, 1.5 * delay)
            await asyncio.sleep(jitter)
            continue
