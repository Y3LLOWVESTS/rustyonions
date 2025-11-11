//! HTTP surface (routers, middleware, helpers).
pub mod routes;
pub mod sse;

pub mod middleware {
    pub mod auth;
    pub mod corr_id;
    pub mod limits;
    pub mod metrics;
    pub mod timeouts;
}

pub mod responses;
