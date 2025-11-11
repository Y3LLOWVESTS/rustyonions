//! RO:WHAT — Server-Sent Events for registry stream (heartbeat + commit events).
//! RO:INVARIANTS — Heartbeats at configured interval; slow clients are dropped by broadcast.

use std::{convert::Infallible, time::Duration};

use axum::response::sse::{Event, KeepAlive, Sse};
use axum::{extract::State, response::IntoResponse};
use futures_util::stream::Stream;
use tokio_stream::{
    wrappers::IntervalStream,
    StreamExt, // .filter, .map, .merge
};

use crate::http::routes::AppState;

pub async fn sse_stream(State(st): State<AppState>) -> impl IntoResponse {
    st.metrics.sse_client_connected();

    // subscribe() returns BroadcastStream<Result<Head, RecvError>>
    // 1) Synchronous filter keeps only Ok(..)
    // 2) Map Ok(head) -> SSE "commit" Event
    let commits = st.store.subscribe().filter(|res| res.is_ok()).map(|res| {
        // Safe due to the filter above.
        let head = match res {
            Ok(h) => h,
            Err(_) => unreachable!("filtered out Err by .filter(|r| r.is_ok())"),
        };
        let data = serde_json::to_string(&head).unwrap_or_else(|_| "{}".to_string());
        Ok::<Event, Infallible>(Event::default().event("commit").data(data))
    });

    // Heartbeat keepalive (SSE control frame)
    let keepalive = KeepAlive::new()
        .interval(Duration::from_millis(st.sse_heartbeat_ms))
        .text("heartbeat");

    Sse::new(merge_with_heartbeat(commits, st.sse_heartbeat_ms)).keep_alive(keepalive)
}

// Merge commit stream with periodic heartbeat events.
fn merge_with_heartbeat<S>(
    commits: S,
    heartbeat_ms: u64,
) -> impl Stream<Item = Result<Event, Infallible>> + Send
where
    S: Stream<Item = Result<Event, Infallible>> + Send + 'static,
{
    let heartbeats = {
        let interval =
            IntervalStream::new(tokio::time::interval(Duration::from_millis(heartbeat_ms)));
        interval.map(|_| Ok(Event::default().event("heartbeat").data("1")))
    };

    // `.merge` comes from `tokio_stream::StreamExt`
    commits.merge(heartbeats)
}
