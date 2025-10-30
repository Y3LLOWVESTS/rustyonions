//! RO:WHAT — Structured trace steps for explain/debug/audit.

#[derive(Debug, Clone, Default)]
pub struct DecisionTrace {
    pub steps: Vec<TraceStep>,
}

#[derive(Debug, Clone)]
pub enum TraceStep {
    Note { key: String, msg: String },
    RuleHit { id: String, reason: String },
    RuleMiss { id: String },
}

impl DecisionTrace {
    pub fn note(key: impl Into<String>, msg: impl Into<String>) -> Self {
        let mut d = Self::default();
        d.steps.push(TraceStep::Note {
            key: key.into(),
            msg: msg.into(),
        });
        d
    }
}

impl TraceStep {
    pub fn note(key: impl Into<String>, msg: impl Into<String>) -> Self {
        Self::Note {
            key: key.into(),
            msg: msg.into(),
        }
    }
    pub fn rule_hit(id: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::RuleHit {
            id: id.into(),
            reason: reason.into(),
        }
    }
    pub fn rule_miss(id: impl Into<String>) -> Self {
        Self::RuleMiss { id: id.into() }
    }
}
