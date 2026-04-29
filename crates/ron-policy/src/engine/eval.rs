//! RO:WHAT — Core evaluation logic producing a `Decision` plus trace.
//!
//! RO:WHY — Deterministic, explainable allow/deny with reasons and obligations.

use std::time::Instant;

use super::{index::RuleIndex, metrics, obligations::ObligationSet};
use crate::{
    errors::Error,
    explain::trace::{DecisionTrace, TraceStep},
    model::{Action, PolicyBundle, Rule},
    Context,
};

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
    /// Returns validation errors if the bundle is malformed.
    pub fn new(bundle: &'a PolicyBundle) -> Result<Self, Error> {
        crate::parse::validate::validate(bundle)?;
        Ok(Self {
            index: RuleIndex::build(bundle),
            bundle,
        })
    }

    /// Evaluate a decision for `ctx`.
    ///
    /// # Errors
    ///
    /// Returns `Error::Eval` if evaluation cannot complete.
    pub fn evaluate(&self, ctx: &Context) -> Result<Decision, Error> {
        metrics::requests_total().inc();
        let t0 = Instant::now();

        let mut trace = DecisionTrace::default();
        let mut obligations = ObligationSet::default();

        if let Some(max) = self.bundle.defaults.max_body_bytes {
            if ctx.body_bytes > max {
                metrics::rejected_total()
                    .with_label_values(&["body_too_large"])
                    .inc();
                trace
                    .steps
                    .push(TraceStep::note("defaults.max_body_bytes", "exceeded"));
                metrics::eval_latency_seconds().observe(t0.elapsed().as_secs_f64());
                return Ok(Decision {
                    effect: DecisionEffect::Deny,
                    obligations,
                    reason: Some("body too large (defaults)".into()),
                    trace,
                });
            }
        }

        let method: &str = &ctx.method;
        for rule in self.index.candidates(method) {
            if rule_matches(rule, ctx) {
                if matches!(rule.action, Action::Deny) {
                    metrics::rejected_total()
                        .with_label_values(&["rule_deny"])
                        .inc();
                }

                obligations.extend(&rule.obligations);
                trace.steps.push(TraceStep::rule_hit(
                    &rule.id,
                    rule.reason.as_deref().unwrap_or(""),
                ));
                metrics::eval_latency_seconds().observe(t0.elapsed().as_secs_f64());

                return Ok(Decision {
                    effect: match rule.action {
                        Action::Allow => DecisionEffect::Allow,
                        Action::Deny => DecisionEffect::Deny,
                    },
                    obligations,
                    reason: rule.reason.clone(),
                    trace,
                });
            }

            trace.steps.push(TraceStep::rule_miss(&rule.id));
        }

        let effect = self.bundle.defaults.default_action.unwrap_or(Action::Deny);

        if matches!(effect, Action::Deny) {
            metrics::rejected_total()
                .with_label_values(&["default_deny"])
                .inc();
        }

        metrics::eval_latency_seconds().observe(t0.elapsed().as_secs_f64());

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

fn rule_matches(rule: &Rule, ctx: &Context) -> bool {
    if let Some(tenant) = &rule.when.tenant {
        if tenant != "*" && tenant != &ctx.tenant {
            return false;
        }
    }

    if let Some(method) = &rule.when.method {
        if method != "*" && method.to_ascii_uppercase() != ctx.method {
            return false;
        }
    }

    if let Some(region) = &rule.when.region {
        if region != "*" && region != &ctx.region {
            return false;
        }
    }

    if let Some(max_body_bytes) = rule.when.max_body_bytes {
        if ctx.body_bytes > max_body_bytes {
            return false;
        }
    }

    if !rule.when.require_tags_all.is_empty()
        && !rule
            .when
            .require_tags_all
            .iter()
            .all(|tag| ctx.tags.contains(&tag.to_ascii_lowercase()))
    {
        return false;
    }

    true
}
