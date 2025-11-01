//! RO:WHAT   HTTP middleware stack assembly.

use axum::Router;

mod body_caps;
mod classify;
mod corr_id;
mod decompress_guard;
mod policy;
mod slow_loris;

pub fn apply<S>(router: Router<S>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let router = router
        .layer(classify::layer())
        .layer(corr_id::layer())
        .layer(policy::layer()); // non-generic; matches new policy.rs

    let (preflight_len_guard, default_body_limit) = body_caps::layer();
    let router = router
        .layer(preflight_len_guard)
        .layer(default_body_limit)
        .layer(decompress_guard::layer())
        .layer(slow_loris::layer());

    crate::admission::attach(router)
}
