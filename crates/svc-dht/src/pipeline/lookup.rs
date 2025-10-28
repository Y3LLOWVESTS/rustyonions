//! RO:WHAT — Lookup FSM: fanout (α) → hedge (β) → converge, under a deadline & hop budget
//! RO:WHY — Tail control & budget adherence; Concerns: PERF/RES
//! RO:INTERACTS — provider::Store (local for MVP); later: transport/kad over ron-transport
//! RO:INVARIANTS — no lock held across .await; limiter bounds total leg concurrency

use super::{deadlines::DeadlineBudget, hedging::race_hedged, rate_limit::Limiter};
use crate::provider::Store;
use anyhow::{anyhow, Result};
use std::{sync::Arc, time::Duration};
use tokio::time::Instant;

#[derive(Clone, Debug)]
pub struct LookupRequest {
    pub cid: String,
    pub alpha: usize,
    pub beta: usize,
    pub hop_budget: usize,
    pub deadline: Duration,
    /// Stagger between hedge legs (β) — small to control tail.
    pub hedge_stagger: Duration,
    /// Per-leg minimum budget (clamped by remaining deadline).
    pub min_leg_budget: Duration,
}

#[derive(Clone, Debug)]
pub struct LookupResult {
    pub providers: Vec<String>,
    pub hops: u32,
    pub elapsed: Duration,
}

pub struct LookupCtx {
    store: Arc<Store>,
    limiter: Limiter,
}

impl LookupCtx {
    pub fn new(store: Arc<Store>, max_concurrent_legs: usize) -> Self {
        Self { store, limiter: Limiter::new(max_concurrent_legs) }
    }

    /// Run a lookup under α/β/hedge/deadline/hop_budget. In this MVP, legs query the local
    /// provider store (network-free), but we still exercise hedging and budgets.
    pub async fn run(&self, req: LookupRequest) -> Result<LookupResult> {
        if req.alpha == 0 {
            return Err(anyhow!("alpha must be > 0"));
        }
        if req.hop_budget == 0 {
            return Err(anyhow!("hop budget must be > 0"));
        }

        let budget = DeadlineBudget::new(req.deadline);
        let started = Instant::now();

        // Compose leg runner. Each leg simulates a "hop" by counting attempt number.
        let cid = req.cid.clone();
        let store = self.store.clone();
        let limiter = self.limiter.clone();

        // Effective leg budget: honor remaining global deadline, but not below min_leg_budget.
        let leg_budget = budget.remaining().max(req.min_leg_budget);

        // In the local MVP there is **no artificial jitter** inside legs.
        // Hedging still races futures; whichever returns first wins.
        let beta = req.beta;
        let stagger = req.hedge_stagger;

        let result = race_hedged::<_, _, _, HedgeErr>(beta, stagger, leg_budget, move |leg_idx| {
            let cid = cid.clone();
            let store = store.clone();
            let limiter = limiter.clone();
            async move {
                let _permit = limiter.acquire().await;
                let providers = store.get_live(&cid);
                if providers.is_empty() {
                    Err(HedgeErr) // in a networked version we'd query peers here
                } else {
                    Ok((providers, leg_idx as u32 + 1)) // hops ~ legs tried until success
                }
            }
        })
        .await;

        match result {
            Ok((providers, hops)) => {
                Ok(LookupResult { providers, hops, elapsed: started.elapsed() })
            }
            Err(_) => Err(anyhow!("lookup failed or timed out")),
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct HedgeErr;
