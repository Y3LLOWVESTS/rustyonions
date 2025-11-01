use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use axum::{
    body::Body,
    extract::State,
    http::{HeaderMap, HeaderName, HeaderValue, Request, StatusCode},
    middleware::{from_fn_with_state, Next},
    response::{IntoResponse, Response},
    Router,
};
use serde::Serialize;

const HEADER_PRIORITY: &str = "x-omnigate-priority";

#[derive(Clone)]
struct Gate {
    hard: usize,
    headroom: usize,
    in_flight: Arc<AtomicUsize>,
}

impl Gate {
    fn new(hard: usize, headroom: usize) -> Self {
        Self {
            hard,
            headroom,
            in_flight: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn limit_for(&self, headers: &HeaderMap) -> usize {
        match headers
            .get(HEADER_PRIORITY)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("bulk")
        {
            "interactive" => self.hard + self.headroom,
            _ => self.hard,
        }
    }

    fn try_enter(&self, headers: &HeaderMap) -> bool {
        let cap = self.limit_for(headers);
        loop {
            let cur = self.in_flight.load(Ordering::Relaxed);
            if cur >= cap {
                return false;
            }
            if self
                .in_flight
                .compare_exchange(cur, cur + 1, Ordering::AcqRel, Ordering::Relaxed)
                .is_ok()
            {
                return true;
            }
        }
    }

    fn leave(&self) {
        self.in_flight.fetch_sub(1, Ordering::AcqRel);
    }
}

#[derive(Serialize)]
struct ErrorBody<'a> {
    reason: &'a str,
    message: &'a str,
}

async fn fairness_guard(
    State(gate): State<Arc<Gate>>,
    req: Request<Body>,
    next: Next,
) -> Response {
    if gate.try_enter(req.headers()) {
        // We admitted this request; reflect "not saturated" best-effort.
        // (Authoritative gating still happens in ReadyPolicy; this keeps the gauge honest.)
        crate::metrics::gates::READY_QUEUE_SATURATED.set(0);

        // RAII guard to decrement in_flight when response completes.
        struct Guard(Arc<Gate>);
        impl Drop for Guard {
            fn drop(&mut self) {
                self.0.leave();
            }
        }
        let cap = gate.limit_for(req.headers());
        let _guard = Guard(gate.clone());

        let mut resp = next.run(req).await;
        let _ = resp.headers_mut().insert(
            HeaderName::from_static("x-omnigate-cap"),
            HeaderValue::from_str(&cap.to_string()).unwrap_or(HeaderValue::from_static("0")),
        );
        resp
    } else {
        // Shed: mark queue saturation and count a drop event.
        crate::metrics::gates::READY_QUEUE_SATURATED.set(1);
        crate::metrics::FAIR_Q_EVENTS_TOTAL
            .with_label_values(&["dropped"])
            .inc();

        (
            StatusCode::SERVICE_UNAVAILABLE,
            axum::Json(ErrorBody {
                reason: "overloaded",
                message: "server is shedding load; please retry",
            }),
        )
            .into_response()
    }
}

/// Attach the fair-queue guard as a middleware layer to the given Router.
/// Avoids generic `Layer` return types so Axum can infer everything cleanly.
pub fn attach<S>(router: Router<S>) -> Router<S>
where
    S: Clone + Send + 'static,
{
    let gate = Arc::new(Gate::new(256, 32));
    router.layer(from_fn_with_state(gate, fairness_guard))
}
