/*!
RO:WHAT — Cross-check between scalar verify_chain and verify_chain_soa.
RO:WHY — GOV/RES: ensure SoA fast path preserves scalar semantics on good and bad chains.
RO:INTERACTS — ron_audit::verify::{verify_chain, verify_chain_soa}; hash::b3_no_self; dto::AuditRecord.
RO:INVARIANTS — both paths must agree on success/failure for the same input chain.
RO:METRICS/LOGS — none; this is test-only.
RO:CONFIG — chain lengths fixed in test.
RO:SECURITY — synthetic records; no real PII or keys.
RO:TEST HOOKS — part of ron-audit unit test suite.
*/

use std::time::{SystemTime, UNIX_EPOCH};

use ron_audit::dto::{ActorRef, AuditKind, ReasonCode, SubjectRef};
use ron_audit::hash::b3_no_self;
use ron_audit::verify::{verify_chain, verify_chain_soa};
use ron_audit::AuditRecord;
use serde_json::json;

fn mk_chain(len: usize) -> Vec<AuditRecord> {
    let ts_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_millis() as u64;

    let mut out = Vec::with_capacity(len);

    // genesis
    let mut genesis = AuditRecord {
        v: 1,
        ts_ms,
        writer_id: "verify-soa@test".to_string(),
        seq: 0,
        stream: "verify_soa".to_string(),
        kind: AuditKind::CapIssued,
        actor: ActorRef::default(),
        subject: SubjectRef::default(),
        reason: ReasonCode("verify-soa-test".to_string()),
        attrs: json!({ "seq": 0u64 }),
        prev: "b3:0".to_string(),
        self_hash: String::new(),
    };
    genesis.self_hash = b3_no_self(&genesis).expect("hash");
    out.push(genesis);

    while out.len() < len {
        let prev = out.last().expect("non-empty");
        let mut rec = AuditRecord {
            v: 1,
            ts_ms,
            writer_id: prev.writer_id.clone(),
            seq: prev.seq + 1,
            stream: prev.stream.clone(),
            kind: AuditKind::IndexWrite,
            actor: ActorRef::default(),
            subject: SubjectRef::default(),
            reason: ReasonCode("verify-soa-test".to_string()),
            attrs: json!({ "seq": prev.seq + 1 }),
            prev: prev.self_hash.clone(),
            self_hash: String::new(),
        };
        rec.self_hash = b3_no_self(&rec).expect("hash");
        out.push(rec);
    }

    out
}

#[test]
fn soa_and_scalar_agree_on_valid_chain() {
    let chain = mk_chain(128);

    // Scalar reference over owned iterator.
    verify_chain(chain.clone().into_iter()).expect("scalar verify");

    // SoA fast path over slice.
    verify_chain_soa(&chain).expect("soa verify");
}

#[test]
fn soa_and_scalar_agree_on_tampered_chain() {
    let mut chain = mk_chain(16);

    // Tamper: break linkage between two records.
    if chain.len() > 2 {
        chain[2].prev = "b3:not-a-real-prev".to_string();
    }

    let scalar = verify_chain(chain.clone().into_iter());
    let soa = verify_chain_soa(&chain);

    assert!(
        scalar.is_err() && soa.is_err(),
        "both scalar and soa verify must fail on tampered chain"
    );
}
