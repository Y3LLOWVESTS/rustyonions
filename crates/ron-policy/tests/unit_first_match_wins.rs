use ron_policy::{PolicyBundle, Rule, RuleCondition, Action};
use ron_policy::engine::eval::{Evaluator, DecisionEffect};
use ron_policy::{parse, Context};
use ron_policy::ctx::clock::SystemClock;

#[test]
fn first_match_wins_and_default_applies() {
    // Bundle: two overlapping allows for GET; no catch-all rule.
    // Expectations:
    //  - GET hits the *first* allow rule and returns immediately ("first").
    //  - POST has no matching rule and falls back to defaults ("default" deny).
    let b = PolicyBundle {
        version: 1,
        defaults: Default::default(), // default_action = None -> deny by default
        meta: Default::default(),
        rules: vec![
            Rule {
                id: "allow-1".into(),
                when: RuleCondition {
                    tenant: None,
                    method: Some("GET".into()),
                    region: None,
                    max_body_bytes: None,
                    require_tags_all: vec![],
                },
                action: Action::Allow,
                obligations: vec![],
                reason: Some("first".into()),
            },
            Rule {
                id: "allow-2".into(),
                when: RuleCondition {
                    tenant: None,
                    method: Some("GET".into()),
                    region: None,
                    max_body_bytes: None,
                    require_tags_all: vec![],
                },
                action: Action::Allow,
                obligations: vec![],
                reason: Some("second".into()),
            },
            // NOTE: No deny-fallback "*" rule here; we want POST to fall through to defaults.
        ],
    };

    parse::validate::validate(&b).unwrap();
    let ev = Evaluator::new(&b).unwrap();

    let getc = Context::builder().tenant("t").method("GET").region("US").build(&SystemClock);
    let postc = Context::builder().tenant("t").method("POST").region("US").build(&SystemClock);

    let d_get = ev.evaluate(&getc).unwrap();
    assert!(matches!(d_get.effect, DecisionEffect::Allow));
    assert_eq!(d_get.reason.as_deref(), Some("first"));

    let d_post = ev.evaluate(&postc).unwrap();
    assert!(matches!(d_post.effect, DecisionEffect::Deny));
    assert_eq!(d_post.reason.as_deref(), Some("default"));
}
