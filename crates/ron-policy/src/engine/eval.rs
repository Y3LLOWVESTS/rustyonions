//! RO:WHAT — Core evaluation logic producing a Decision + Trace.
//!
//! RO:WHY  — Deterministic, explainable allow/deny with reasons and obligations.

use super::{index::RuleIndex, metrics, obligations::ObligationSet};
use crate::{
    errors::Error,
    explain::trace::{DecisionTrace, TraceStep},
    model::{Action, PolicyBundle, Rule},
    Context,
};
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecisionEffect {
    Allow,
    Deny,
}

#[derive(Debug, Clone)]
pub struct Decision {
    pub effect: DecisionEffect,
    pub obligations: ObligationSet,
    pub reason: Option<String>,
    pub trace: DecisionTrace,
}

pub struct Evaluator<'a> {
    bundle: &'a PolicyBundle,
    index: RuleIndex<'a>,
}

impl<'a> Evaluator<'a> {
    /// Construct an evaluator over a validated bundle.
    ///
    /// # Errors
    ///
    /// Currently infallible; reserved for future index build errors.
    pub fn new(bundle: &'a PolicyBundle) -> Result<Self, Error> {
        Ok(Self {
            index: RuleIndex::build(bundle),
            bundle,
        })
    }

    /// Evaluate a decision for `ctx`.
    ///
    /// # Errors
    ///
    /// Returns `Error::Eval` if evaluation cannot complete (reserved; not used today).
    pub fn evaluate(&self, ctx: &Context) -> Result<Decision, Error> {
        metrics::REQUESTS_TOTAL.inc();
        let t0 = Instant::now();

        let mut trace = DecisionTrace::default();
        let mut obligations = ObligationSet::default();

        // Hard guard: body cap from defaults first.
        if let Some(max) = self.bundle.defaults.max_body_bytes {
            if ctx.body_bytes > max {
                metrics::REJECTED_TOTAL
                    .with_label_values(&["body_too_large"])
                    .inc();
                trace
                    .steps
                    .push(TraceStep::note("defaults.max_body_bytes", "exceeded"));
                metrics::EVAL_LATENCY_SECONDS.observe(t0.elapsed().as_secs_f64());
                return Ok(Decision {
                    effect: DecisionEffect::Deny,
                    obligations,
                    reason: Some("body too large (defaults)".into()),
                    trace,
                });
            }
        }

        // Candidate rules restricted by method (and then checked fully).
        // `ctx.method` is already uppercased by the builder; reuse it to avoid alloc.
        let method: &str = &ctx.method;
        for r in self.index.candidates(method) {
            if rule_matches(r, ctx) {
                if matches!(r.action, Action::Deny) {
                    metrics::REJECTED_TOTAL
                        .with_label_values(&["rule_deny"])
                        .inc();
                }
                obligations.extend(&r.obligations);
                trace.steps.push(TraceStep::rule_hit(
                    &r.id,
                    r.reason.as_deref().unwrap_or(""),
                ));
                metrics::EVAL_LATENCY_SECONDS.observe(t0.elapsed().as_secs_f64());
                return Ok(Decision {
                    effect: match r.action {
                        Action::Allow => DecisionEffect::Allow,
                        Action::Deny => DecisionEffect::Deny,
                    },
                    obligations,
                    reason: r.reason.clone(),
                    trace,
                });
            }
            // Miss path (the `if` branch returns on hit).
            trace.steps.push(TraceStep::rule_miss(&r.id));
        }

        // No matches → default action (deny-by-default if unspecified)
        let effect = self.bundle.defaults.default_action.unwrap_or(Action::Deny);

        if matches!(effect, Action::Deny) {
            metrics::REJECTED_TOTAL
                .with_label_values(&["default_deny"])
                .inc();
        }

        metrics::EVAL_LATENCY_SECONDS.observe(t0.elapsed().as_secs_f64());
        Ok(Decision {
            effect: match effect {
                Action::Allow => DecisionEffect::Allow,
                Action::Deny => DecisionEffect::Deny,
            },
            obligations,
            reason: Some("default".into()),
            trace,
        })
    }
}

fn rule_matches(r: &Rule, ctx: &Context) -> bool {
    if let Some(t) = &r.when.tenant {
        if t != "*" && t != &ctx.tenant {
            return false;
        }
    }
    if let Some(m) = &r.when.method {
        if m != "*" && m.to_ascii_uppercase() != ctx.method {
            return false;
        }
    }
    if let Some(g) = &r.when.region {
        if g != "*" && g != &ctx.region {
            return false;
        }
    }
    if let Some(n) = r.when.max_body_bytes {
        if ctx.body_bytes > n {
            return false;
        }
    }
    if !r.when.require_tags_all.is_empty() {
        for tag in &r.when.require_tags_all {
            if !ctx.tags.contains(&tag.to_ascii_lowercase()) {
                return false;
            }
        }
    }
    true
}
