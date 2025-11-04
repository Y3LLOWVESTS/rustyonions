//! Shared rejections counter (prometheus) to avoid double-register panics.

use prometheus::{IntCounterVec, Opts};
use std::sync::OnceLock;

const NAME: &str = "gateway_rejections_total";

pub fn counter() -> &'static IntCounterVec {
    static CTR: OnceLock<IntCounterVec> = OnceLock::new();
    CTR.get_or_init(|| {
        let vec = IntCounterVec::new(
            Opts::new(NAME, "Gateway rejections by reason"),
            &["reason"],
        )
        .expect("IntCounterVec");
        prometheus::register(Box::new(vec.clone()))
            .expect("register gateway_rejections_total");
        vec
    })
}
