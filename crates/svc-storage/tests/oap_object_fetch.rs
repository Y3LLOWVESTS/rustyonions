//! RO:WHAT — Phase 9A local OAP OBJ_GET acceptance and rejection tests.
//! RO:WHY — Prove policy-first, bounded, full-digest object flow before listener wiring.
//! RO:INTERACTS — svc_storage::oap_object, MemoryStorage, OAP codec, ron-proto DTOs.
//! RO:INVARIANTS — frame ≤1 MiB; chunk ≤64 KiB; corrupt/wrong b3 rejected.
//! RO:SECURITY — no IP-bearing request fields; policy denial occurs before storage lookup.
//! RO:TEST — cargo test -p svc-storage --test oap_object_fetch.

use std::sync::Arc;

use bytes::Bytes;
use oap::{Flags, Frame, FrameBuilder, OapWriter, ParserState, MAX_FRAME_BYTES, STREAM_CHUNK_SIZE};
use ron_policy::{load_json, B3Id, ModerationPolicy, ModerationReasonCode, PolicyBundle};
use ron_proto::oap::data::Data;
use ron_proto::oap::object::OBJ_GET_APP_PROTO_ID;
use ron_proto::ContentId;
use svc_storage::oap_object::{
    build_obj_get_request, verify_obj_stream, LocalOapObjectService, OapObjectError,
};
use svc_storage::storage::{DynStorage, MemoryStorage};

fn allow_policy() -> PolicyBundle {
    load_json(
        br#"{
            "version": 1,
            "rules": [
                {
                    "id": "allow-private-local-obj-get",
                    "when": {
                        "method": "OBJ_GET",
                        "require_tags_all": ["privacy-aware-local"]
                    },
                    "action": "allow",
                    "reason": "local private object read"
                }
            ]
        }"#,
    )
    .expect("valid allow policy")
}

fn deny_policy() -> PolicyBundle {
    load_json(
        br#"{
            "version": 1,
            "rules": []
        }"#,
    )
    .expect("valid deny-by-default policy")
}

fn cid_for(bytes: &[u8]) -> ContentId {
    format!("b3:{}", blake3::hash(bytes).to_hex())
        .parse()
        .expect("calculated BLAKE3 must be a valid ContentId")
}

fn moderation_id(cid: &ContentId) -> B3Id {
    cid.as_str()
        .parse()
        .expect("canonical ContentId must be a moderation B3Id")
}

fn wire_roundtrip(frame: Frame) -> Frame {
    let mut writer = OapWriter::with_default();

    writer.encode_to_buf(frame).expect("frame must encode");

    let wire = writer.take_buf();
    let mut parser = ParserState::with_default();

    parser.push(&wire).expect("wire bytes must buffer");

    let decoded = parser
        .try_next()
        .expect("wire frame must decode")
        .expect("one complete frame must be present");

    assert_eq!(parser.buffered_len(), 0);

    decoded
}

async fn put_object(store: &DynStorage, cid: &ContentId, bytes: Bytes) {
    store
        .put(cid.as_str(), bytes)
        .await
        .expect("test object put must succeed");
}

#[tokio::test]
async fn happy_obj_get_roundtrips_over_real_oap_codec() {
    let object_len = STREAM_CHUNK_SIZE * 2 + 17;
    let object: Vec<u8> = (0..object_len).map(|index| (index % 251) as u8).collect();

    let cid = cid_for(&object);
    let store: DynStorage = Arc::new(MemoryStorage::default());

    put_object(&store, &cid, Bytes::from(object.clone())).await;

    let service =
        LocalOapObjectService::new(store, allow_policy()).expect("valid local object service");

    let request = build_obj_get_request(cid.clone(), 0xCAFE, 77).expect("valid OBJ_GET request");

    let request = wire_roundtrip(request);

    let responses = service
        .serve_obj_get(request)
        .await
        .expect("OBJ_GET should succeed");

    let responses: Vec<Frame> = responses.into_iter().map(wire_roundtrip).collect();

    assert_eq!(responses.len(), 5);
    assert!(responses[0].header.flags.contains(Flags::START));
    assert!(responses[4].header.flags.contains(Flags::END));

    for frame in &responses[1..4] {
        let data: Data = serde_json::from_slice(frame.payload.as_ref().expect("DATA payload"))
            .expect("valid DATA DTO");

        assert!(data.bytes.len() <= STREAM_CHUNK_SIZE);
    }

    let verified =
        verify_obj_stream(&cid, &responses).expect("full object verification should succeed");

    assert_eq!(verified.as_ref(), object.as_slice());
}

#[tokio::test]
async fn corrupt_stream_bytes_are_rejected_by_full_digest() {
    let object = b"verified object bytes".to_vec();
    let cid = cid_for(&object);
    let store: DynStorage = Arc::new(MemoryStorage::default());

    put_object(&store, &cid, Bytes::from(object)).await;

    let service =
        LocalOapObjectService::new(store, allow_policy()).expect("valid local object service");

    let request = build_obj_get_request(cid.clone(), 1, 2).expect("valid request");

    let mut responses = service
        .serve_obj_get(request)
        .await
        .expect("valid object stream");

    let data_frame = responses.get_mut(1).expect("one DATA frame must exist");

    let mut data: Data = serde_json::from_slice(data_frame.payload.as_ref().expect("DATA payload"))
        .expect("valid DATA DTO");

    data.bytes[0] ^= 0xFF;

    data_frame.payload = Some(Bytes::from(
        serde_json::to_vec(&data).expect("serialize corrupted DATA"),
    ));

    let err = verify_obj_stream(&cid, &responses)
        .expect_err("corrupt stream must fail digest verification");

    assert!(matches!(err, OapObjectError::DigestMismatch { .. }));
}

#[tokio::test]
async fn storage_bytes_under_wrong_b3_are_rejected_before_serve() {
    let claimed_cid = cid_for(b"claimed bytes");
    let store: DynStorage = Arc::new(MemoryStorage::default());

    put_object(
        &store,
        &claimed_cid,
        Bytes::from_static(b"different actual bytes"),
    )
    .await;

    let service =
        LocalOapObjectService::new(store, allow_policy()).expect("valid local object service");

    let request = build_obj_get_request(claimed_cid, 3, 4).expect("valid request");

    let err = service
        .serve_obj_get(request)
        .await
        .expect_err("wrong b3 mapping must not be served");

    assert!(matches!(err, OapObjectError::DigestMismatch { .. }));
}

#[tokio::test]
async fn oversized_oap_request_frame_is_rejected() {
    let store: DynStorage = Arc::new(MemoryStorage::default());

    let service =
        LocalOapObjectService::new(store, allow_policy()).expect("valid local object service");

    let request = FrameBuilder::request(OBJ_GET_APP_PROTO_ID, 5, 6)
        .start()
        .end()
        .payload(Bytes::from(vec![0_u8; MAX_FRAME_BYTES as usize]))
        .build();

    let err = service
        .serve_obj_get(request)
        .await
        .expect_err("oversized frame must fail");

    assert!(matches!(err, OapObjectError::FrameTooLarge { .. }));
}

#[tokio::test]
async fn oversized_data_chunk_is_rejected() {
    let object = b"small valid object".to_vec();
    let cid = cid_for(&object);
    let store: DynStorage = Arc::new(MemoryStorage::default());

    put_object(&store, &cid, Bytes::from(object)).await;

    let service =
        LocalOapObjectService::new(store, allow_policy()).expect("valid local object service");

    let request = build_obj_get_request(cid.clone(), 7, 8).expect("valid request");

    let mut responses = service
        .serve_obj_get(request)
        .await
        .expect("valid object stream");

    let data_frame = responses.get_mut(1).expect("one DATA frame must exist");

    let mut data: Data = serde_json::from_slice(data_frame.payload.as_ref().expect("DATA payload"))
        .expect("valid DATA DTO");

    data.bytes = vec![0_u8; STREAM_CHUNK_SIZE + 1];

    data_frame.payload = Some(Bytes::from(
        serde_json::to_vec(&data).expect("serialize oversized DATA"),
    ));

    let err = verify_obj_stream(&cid, &responses).expect_err("oversized raw chunk must fail");

    assert!(matches!(err, OapObjectError::ChunkTooLarge { .. }));
}

#[tokio::test]
async fn not_found_and_not_ready_are_distinct() {
    let cid = cid_for(b"missing object");
    let store: DynStorage = Arc::new(MemoryStorage::default());

    let not_ready = LocalOapObjectService::new(store.clone(), allow_policy())
        .expect("valid service")
        .with_ready(false);

    let request = build_obj_get_request(cid.clone(), 9, 10).expect("valid request");

    assert_eq!(
        not_ready
            .serve_obj_get(request)
            .await
            .expect_err("not-ready service must fail"),
        OapObjectError::NotReady
    );

    let ready = LocalOapObjectService::new(store, allow_policy()).expect("valid service");

    let request = build_obj_get_request(cid, 9, 11).expect("valid request");

    assert_eq!(
        ready
            .serve_obj_get(request)
            .await
            .expect_err("missing object must fail"),
        OapObjectError::NotFound
    );
}

#[tokio::test]
async fn deny_policy_runs_before_storage_lookup() {
    let missing_cid = cid_for(b"policy-first object");
    let store: DynStorage = Arc::new(MemoryStorage::default());

    let service =
        LocalOapObjectService::new(store, deny_policy()).expect("valid deny-by-default service");

    let request = build_obj_get_request(missing_cid, 12, 13).expect("valid request");

    let err = service
        .serve_obj_get(request)
        .await
        .expect_err("policy must deny before missing storage is consulted");

    assert!(matches!(err, OapObjectError::PolicyDenied { .. }));
}

#[tokio::test]
async fn moderation_refusal_states_run_before_storage_lookup() {
    let missing_cid = cid_for(b"moderation-before-storage object");
    let reasons = [
        ModerationReasonCode::GlobalDeny,
        ModerationReasonCode::OwnerTombstone,
        ModerationReasonCode::LocalBlock,
        ModerationReasonCode::Quarantined,
    ];

    for (index, reason) in reasons.into_iter().enumerate() {
        let store: DynStorage = Arc::new(MemoryStorage::default());

        let mut moderation = ModerationPolicy::default();
        let object = moderation_id(&missing_cid);

        let inserted = match reason {
            ModerationReasonCode::GlobalDeny => moderation.insert_global_deny(object),
            ModerationReasonCode::OwnerTombstone => moderation.insert_owner_tombstone(object),
            ModerationReasonCode::LocalBlock => moderation.insert_local_block(object),
            ModerationReasonCode::Quarantined => moderation.insert_quarantine(object),
            ModerationReasonCode::NoRule | ModerationReasonCode::LocalAllow => {
                unreachable!("test only covers refusal reasons")
            }
        };

        assert!(inserted);

        let service = LocalOapObjectService::new(store, allow_policy())
            .expect("valid local object service")
            .with_moderation_policy(moderation);

        let corr_id = u64::try_from(index).expect("small test index") + 100;

        let request =
            build_obj_get_request(missing_cid.clone(), 14, corr_id).expect("valid request");

        let err = service
            .serve_obj_get(request)
            .await
            .expect_err("moderation refusal must occur before missing storage");

        assert_eq!(err, OapObjectError::ModerationDenied { reason });
    }
}

#[tokio::test]
async fn local_allow_moderation_serves_verified_object() {
    let object = b"explicitly locally allowed object";
    let cid = cid_for(object);
    let store: DynStorage = Arc::new(MemoryStorage::default());

    put_object(&store, &cid, Bytes::from_static(object)).await;

    let mut moderation = ModerationPolicy::default();
    assert!(moderation.insert_local_allow(moderation_id(&cid)));

    let service = LocalOapObjectService::new(store, allow_policy())
        .expect("valid local object service")
        .with_moderation_policy(moderation);

    let request = build_obj_get_request(cid.clone(), 15, 200).expect("valid request");

    let responses = service
        .serve_obj_get(request)
        .await
        .expect("locally allowed object should serve");

    let verified = verify_obj_stream(&cid, &responses)
        .expect("served object must retain full digest verification");

    assert_eq!(verified.as_ref(), object);
}

#[tokio::test]
async fn local_allow_cannot_bypass_local_block_at_serve_gate() {
    let object = b"allow and block conflict object";
    let cid = cid_for(object);
    let store: DynStorage = Arc::new(MemoryStorage::default());

    put_object(&store, &cid, Bytes::from_static(object)).await;

    let moderation_object = moderation_id(&cid);
    let mut moderation = ModerationPolicy::default();

    assert!(moderation.insert_local_allow(moderation_object.clone(),));
    assert!(moderation.insert_local_block(moderation_object));

    let service = LocalOapObjectService::new(store, allow_policy())
        .expect("valid local object service")
        .with_moderation_policy(moderation);

    let request = build_obj_get_request(cid, 16, 300).expect("valid request");

    assert_eq!(
        service
            .serve_obj_get(request)
            .await
            .expect_err("local block must override local allow at serve time",),
        OapObjectError::ModerationDenied {
            reason: ModerationReasonCode::LocalBlock,
        }
    );
}
