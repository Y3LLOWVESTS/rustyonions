#!/usr/bin/env python3
# RO:WHAT — Continue podcast-lite backend patch after the StreamAssetRequest marker mismatch.
# RO:WHY — The first patch partially succeeded, then stopped before DTO/handlers/helpers/resolver/content_view patches.
# RO:INVARIANTS — gateway-first; omnigate coordinates storage/index only; no wallet/ledger mutation here.
# RO:SECURITY — podcast-lite only; no live stream truth; no backend legal ownership proof claim.
# RO:TEST — cargo fmt -p omnigate -p svc-gateway; cargo check -p omnigate --all-targets; cargo check -p svc-gateway --all-targets.

from pathlib import Path

ROOT = Path.cwd()


def load(path: Path) -> str:
    if not path.exists():
        raise SystemExit(f"missing file: {path}")
    return path.read_text()


def save(path: Path, text: str) -> None:
    path.write_text(text)


def insert_before_once(text: str, marker: str, insertion: str, label: str) -> str:
    if insertion.strip() in text:
        print(f"skip {label}: already patched")
        return text

    count = text.count(marker)
    if count != 1:
        raise SystemExit(f"{label}: expected 1 marker, found {count}")

    print(f"patch {label}")
    return text.replace(marker, insertion + marker, 1)


def replace_first_available(text: str, replacements: list[tuple[str, str]], label: str) -> str:
    for _old, new in replacements:
        if new in text:
            print(f"skip {label}: already patched")
            return text

    for old, new in replacements:
        count = text.count(old)
        if count == 1:
            print(f"patch {label}")
            return text.replace(old, new, 1)

    details = "; ".join(f"candidate had {text.count(old)} matches" for old, _new in replacements)
    raise SystemExit(f"{label}: no usable replacement candidate found ({details})")


# ---------------------------------------------------------------------------
# omnigate assets route continuation
# ---------------------------------------------------------------------------

assets_path = ROOT / "crates/omnigate/src/routes/v1/assets.rs"
assets = load(assets_path)

podcast_dto = r'''
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct PodcastAssetPrepareRequest {
    bytes: u64,
    #[serde(default)]
    payer_account: Option<String>,
    #[serde(default)]
    owner_passport_subject: Option<String>,
    #[serde(default)]
    content_type: Option<String>,
    #[serde(default)]
    expected_asset_cid: Option<String>,
    #[serde(default)]
    file_name: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    show_title: Option<String>,
    #[serde(default)]
    host_display: Option<String>,
    #[serde(default)]
    guest_display: Option<String>,
    #[serde(default)]
    season_number: Option<String>,
    #[serde(default)]
    episode_number: Option<String>,
    #[serde(default)]
    duration: Option<String>,
    #[serde(default)]
    category: Option<String>,
    #[serde(default)]
    language: Option<String>,
    #[serde(default)]
    explicit_rating: Option<String>,
    #[serde(default)]
    cover_image_crab_url: Option<String>,
    #[serde(default)]
    transcript_crab_url: Option<String>,
    #[serde(default)]
    chapters_crab_url: Option<String>,
    #[serde(default)]
    show_page_crab_url: Option<String>,
    #[serde(default)]
    rights_mode: Option<String>,
    #[serde(default)]
    license_mode: Option<String>,
    #[serde(default)]
    guest_permission_attested: bool,
    #[serde(default)]
    legal_attestation_accepted: bool,
    #[serde(default)]
    client_idempotency_key: Option<String>,
}

'''

assets = insert_before_once(
    assets,
    '#[derive(Debug, Clone, Deserialize, Serialize)]\n#[serde(deny_unknown_fields)]\nstruct StreamAssetRequest {',
    podcast_dto,
    "omnigate PodcastAssetPrepareRequest",
)

podcast_handlers = r'''
/// Prepare a podcast/audio asset upload.
///
/// This is podcast-specific UX glue over paid storage estimate. It intentionally
/// does not create a wallet hold, store bytes, mint receipts, verify legal
/// ownership, verify guest releases, or mutate ledger/accounting.
pub async fn podcast_prepare(headers: HeaderMap, body: Bytes) -> Response {
    let request = match serde_json::from_slice::<PodcastAssetPrepareRequest>(&body) {
        Ok(request) => request,
        Err(_) => {
            return problem(
                StatusCode::BAD_REQUEST,
                "invalid_podcast_prepare_request",
                "podcast prepare request must be strict JSON",
                false,
                "bad_json",
            );
        }
    };

    if request.bytes == 0 {
        return problem(
            StatusCode::BAD_REQUEST,
            "invalid_podcast_prepare_request",
            "bytes must be greater than zero",
            false,
            "invalid_bytes",
        );
    }

    if let Some(content_type) = &request.content_type {
        if !is_valid_audio_content_type(content_type) {
            return problem(
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "invalid_podcast_content_type",
                "content_type must be an audio/* media type",
                false,
                "invalid_content_type",
            );
        }
    }

    if !request.legal_attestation_accepted {
        return problem(
            StatusCode::BAD_REQUEST,
            "podcast_rights_attestation_required",
            "podcast prepare requires creator rights attestation",
            false,
            "missing_podcast_rights_attestation",
        );
    }

    if !request.guest_permission_attested {
        return problem(
            StatusCode::BAD_REQUEST,
            "podcast_guest_permission_required",
            "podcast prepare requires guest/voice permission attestation",
            false,
            "missing_podcast_guest_permission",
        );
    }

    if let Some(expected_asset_cid) = &request.expected_asset_cid {
        if !is_canonical_b3_cid(expected_asset_cid) {
            return problem(
                StatusCode::BAD_REQUEST,
                "invalid_expected_asset_cid",
                "expected_asset_cid must be canonical b3:<64 lowercase hex>",
                false,
                "invalid_expected_asset_cid",
            );
        }
    }

    let storage_estimate = match fetch_storage_estimate(request.bytes, headers).await {
        Ok(storage_estimate) => storage_estimate,
        Err(response) => return response,
    };

    let action =
        value_string(&storage_estimate, "action").unwrap_or_else(|| DEFAULT_ACTION.to_owned());
    let asset =
        value_string(&storage_estimate, "asset").unwrap_or_else(|| DEFAULT_ASSET.to_owned());

    let Some(amount_minor) = value_string(&storage_estimate, "amount_minor")
        .or_else(|| value_string(&storage_estimate, "amount_minor_units"))
        .or_else(|| value_string(&storage_estimate, "amount"))
    else {
        return problem(
            StatusCode::BAD_GATEWAY,
            "storage_estimate_missing_amount",
            "storage estimate did not include an amount",
            true,
            "storage_estimate_missing_amount",
        );
    };

    let minimum_hold_minor = value_string(&storage_estimate, "minimum_hold_minor")
        .or_else(|| value_string(&storage_estimate, "minimum_hold_minor_units"))
        .unwrap_or_else(|| amount_minor.clone());

    let payer_account = request
        .payer_account
        .clone()
        .unwrap_or_else(|| "acct_dev".to_owned());

    let idempotency_key_hint = request
        .client_idempotency_key
        .clone()
        .or_else(|| Some(format!("prepare:podcast:{action}:{}", request.bytes)));

    let response = json!({
        "schema": PODCAST_PREPARE_SCHEMA,
        "asset_kind": "podcast",
        "action": action,
        "asset": asset,
        "bytes": request.bytes,
        "content_type": request.content_type,
        "expected_asset_cid": request.expected_asset_cid,
        "file_name": request.file_name,
        "owner_passport_subject": request.owner_passport_subject,
        "title": request.title,
        "description": request.description,
        "tags": request.tags,
        "podcast": {
            "show_title": request.show_title,
            "host_display": request.host_display,
            "guest_display": request.guest_display,
            "season_number": request.season_number,
            "episode_number": request.episode_number,
            "duration": request.duration,
            "category": request.category,
            "language": request.language,
            "explicit_rating": request.explicit_rating,
            "cover_image_crab_url": request.cover_image_crab_url,
            "transcript_crab_url": request.transcript_crab_url,
            "chapters_crab_url": request.chapters_crab_url,
            "show_page_crab_url": request.show_page_crab_url,
            "rights_mode": request.rights_mode,
            "license_mode": request.license_mode,
            "guest_permission_attested": request.guest_permission_attested,
            "legal_attestation_accepted": request.legal_attestation_accepted
        },
        "paid_storage": {
            "estimate_path": "/paid/o/estimate",
            "submit_path": "/paid/o",
            "estimate": storage_estimate,
        },
        "estimate": {
            "amount_minor": amount_minor,
            "amount": amount_minor,
            "minimum_hold_minor": minimum_hold_minor,
            "asset": DEFAULT_ASSET,
            "currency": DEFAULT_CURRENCY,
            "action": DEFAULT_ACTION,
            "bytes": request.bytes,
        },
        "wallet_hold": {
            "required": true,
            "from": payer_account,
            "to": "escrow_paid_write",
            "asset": DEFAULT_ASSET,
            "amount_minor": minimum_hold_minor,
            "amount": minimum_hold_minor,
            "idempotency_key_hint": idempotency_key_hint,
            "memo": "CrabLink podcast upload hold"
        },
        "next": {
            "create_hold": "/v1/wallet/hold",
            "submit_upload": "/v1/assets/podcast",
            "requires_paid_headers": true,
            "requires_rights_header": "x-ron-podcast-rights-attested=true",
            "requires_guest_permission_header": "x-ron-podcast-guest-permission-attested=true"
        },
        "accepted_headers": {
            "upload": [
                "content-type",
                "idempotency-key",
                "x-ron-passport",
                "x-ron-wallet-account",
                "x-ron-asset-title",
                "x-ron-asset-description",
                "x-ron-asset-tags",
                "x-ron-podcast-host",
                "x-ron-podcast-show",
                "x-ron-podcast-guest",
                "x-ron-podcast-episode",
                "x-ron-podcast-season",
                "x-ron-podcast-duration",
                "x-ron-podcast-language",
                "x-ron-podcast-category",
                "x-ron-podcast-explicit-rating",
                "x-ron-podcast-cover-image-crab-url",
                "x-ron-podcast-transcript-crab-url",
                "x-ron-podcast-chapters-crab-url",
                "x-ron-podcast-show-page-crab-url",
                "x-ron-podcast-rights-attested",
                "x-ron-podcast-guest-permission-attested",
                "x-ron-permission",
                "x-ron-spend-limit",
                "x-correlation-id",
                "x-request-id"
            ]
        },
        "warnings": [
            "podcast_lite_only_no_transcoding_no_drm_no_cover_art_upload",
            "cover_art_and_transcripts_are_reference_only",
            "legal_attestation_is_creator_confirmation_not_backend_ownership_proof",
            "podcast_lite_is_recorded_audio_not_live_stream_delivery"
        ],
    });

    (StatusCode::OK, Json(response)).into_response()
}

/// Upload bounded audio bytes through paid storage, then write a podcast manifest
/// object and `svc-index` asset→manifest pointer.
///
/// This is intentionally podcast-lite. Storage enforces the paid write. Index
/// owns the pointer. Cover art, transcript, chapters, and show pages are
/// references only and are never uploaded by this route.
pub async fn podcast_upload(headers: HeaderMap, body: Bytes) -> Response {
    let Some(content_type) = grab(&headers, "content-type") else {
        return problem(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "missing_podcast_content_type",
            "podcast upload requires an audio/* content type",
            false,
            "missing_content_type",
        );
    };

    if !is_valid_audio_content_type(&content_type) {
        return problem(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "invalid_podcast_content_type",
            "podcast upload content-type must be audio/*",
            false,
            "invalid_content_type",
        );
    }

    if !truthy_header(&headers, "x-ron-podcast-rights-attested") {
        return problem(
            StatusCode::BAD_REQUEST,
            "podcast_rights_attestation_required",
            "podcast upload requires x-ron-podcast-rights-attested=true",
            false,
            "missing_podcast_rights_attestation",
        );
    }

    if !truthy_header(&headers, "x-ron-podcast-guest-permission-attested") {
        return problem(
            StatusCode::BAD_REQUEST,
            "podcast_guest_permission_required",
            "podcast upload requires x-ron-podcast-guest-permission-attested=true",
            false,
            "missing_podcast_guest_permission",
        );
    }

    let storage_upload = match send_to_storage(
        Method::POST,
        "/paid/o",
        headers.clone(),
        body,
        "storage paid podcast upload upstream unavailable",
    )
    .await
    {
        Ok(storage_upload) => storage_upload,
        Err(response) => return response,
    };

    if !storage_upload.status.is_success() {
        return response_from_upstream(storage_upload);
    }

    let storage_upload_json = match serde_json::from_slice::<Value>(&storage_upload.body) {
        Ok(value) => value,
        Err(_) => {
            return problem(
                StatusCode::BAD_GATEWAY,
                "storage_upload_bad_json",
                "storage paid podcast upload response was not valid JSON",
                true,
                "storage_bad_json",
            );
        }
    };

    let Some(asset_cid) = value_string(&storage_upload_json, "cid") else {
        return problem(
            StatusCode::BAD_GATEWAY,
            "storage_upload_missing_cid",
            "storage paid podcast upload response did not include cid",
            true,
            "storage_missing_cid",
        );
    };

    if !is_canonical_b3_cid(&asset_cid) {
        return problem(
            StatusCode::BAD_GATEWAY,
            "storage_upload_invalid_cid",
            "storage paid podcast upload response included invalid cid",
            true,
            "storage_invalid_cid",
        );
    }

    let owner = owner_from_headers(&headers);
    let payout = PayoutSummary {
        default_action: "content_view",
        recipient_account: owner.wallet_account.clone(),
    };

    let manifest = build_podcast_manifest(&headers, &storage_upload_json, &asset_cid, &owner);
    let manifest_bytes = match serde_json::to_vec(&manifest) {
        Ok(bytes) => Bytes::from(bytes),
        Err(_) => {
            return problem(
                StatusCode::INTERNAL_SERVER_ERROR,
                "manifest_encode_failed",
                "failed to encode generated podcast manifest",
                false,
                "manifest_encode_failed",
            );
        }
    };

    let mut warnings = Vec::new();
    warnings.push("podcast_lite_only_no_transcoding_no_drm".to_owned());
    warnings.push("cover_art_reference_only_no_podcast_page_image_upload".to_owned());
    warnings.push("transcript_reference_only_no_podcast_page_transcript_upload".to_owned());
    warnings.push("legal_attestation_is_creator_confirmation_not_backend_ownership_proof".to_owned());
    warnings.push("podcast_lite_is_recorded_audio_not_live_stream_delivery".to_owned());

    let manifest_write = match store_manifest_object(headers.clone(), manifest_bytes).await {
        Ok(upstream) if upstream.status.is_success() => {
            match serde_json::from_slice::<Value>(&upstream.body)
                .ok()
                .and_then(|value| value_string(&value, "cid"))
                .filter(|cid| is_canonical_b3_cid(cid))
            {
                Some(manifest_cid) => ManifestWriteSummary {
                    status: "stored",
                    manifest_cid: Some(manifest_cid),
                    storage_path: "/o",
                },
                None => {
                    warnings.push("manifest_storage_missing_valid_cid".to_owned());
                    ManifestWriteSummary {
                        status: "failed",
                        manifest_cid: None,
                        storage_path: "/o",
                    }
                }
            }
        }
        Ok(upstream) => {
            warnings.push(format!(
                "manifest_storage_http_{}",
                upstream.status.as_u16()
            ));
            ManifestWriteSummary {
                status: "failed",
                manifest_cid: None,
                storage_path: "/o",
            }
        }
        Err(response) => {
            warnings.push(response_warning(&response, "manifest_storage_failed"));
            ManifestWriteSummary {
                status: "failed",
                manifest_cid: None,
                storage_path: "/o",
            }
        }
    };

    let raw_hash = asset_cid.trim_start_matches("b3:").to_owned();
    let pointer_route = format!("/v1/index/assets/{raw_hash}/manifest");

    let index_pointer = if let Some(manifest_cid) = manifest_write.manifest_cid.as_deref() {
        match put_podcast_index_pointer(&headers, &asset_cid, manifest_cid, &owner).await {
            Ok(upstream) if upstream.status.is_success() => IndexPointerSummary {
                status: "stored",
                route: pointer_route.clone(),
                http_status: Some(upstream.status.as_u16()),
            },
            Ok(upstream) => {
                warnings.push(format!("index_pointer_http_{}", upstream.status.as_u16()));
                IndexPointerSummary {
                    status: "failed",
                    route: pointer_route.clone(),
                    http_status: Some(upstream.status.as_u16()),
                }
            }
            Err(response) => {
                warnings.push(response_warning(&response, "index_pointer_failed"));
                IndexPointerSummary {
                    status: "failed",
                    route: pointer_route.clone(),
                    http_status: None,
                }
            }
        }
    } else {
        warnings.push("index_pointer_skipped_missing_manifest_cid".to_owned());
        IndexPointerSummary {
            status: "skipped",
            route: pointer_route.clone(),
            http_status: None,
        }
    };

    let crab_url = format!("crab://{raw_hash}.podcast");
    let manifest_raw = manifest_write
        .manifest_cid
        .as_ref()
        .map(|manifest_cid| format!("/o/{manifest_cid}"));

    let response = json!({
        "schema": PODCAST_UPLOAD_SCHEMA,
        "asset_kind": "podcast",
        "kind": "podcast",
        "asset_cid": &asset_cid,
        "cid": &asset_cid,
        "crab_url": &crab_url,
        "url": &crab_url,
        "storage_upload": storage_upload_json,
        "manifest": {
            "status": manifest_write.status,
            "manifest_cid": manifest_write.manifest_cid,
            "storage_path": manifest_write.storage_path,
        },
        "index_pointer": index_pointer,
        "owner": owner,
        "payout": payout,
        "links": {
            "raw": format!("/o/b3:{raw_hash}"),
            "crab": &crab_url,
            "http_b3": format!("/v1/b3/{raw_hash}.podcast"),
            "resolve": format!("/v1/crab/resolve?url=crab://{raw_hash}.podcast"),
            "manifest_raw": manifest_raw,
        },
        "warnings": warnings,
    });

    (StatusCode::OK, Json(response)).into_response()
}

'''

assets = insert_before_once(
    assets,
    '/// Prepare a stream descriptor publication.\n',
    podcast_handlers,
    "omnigate podcast prepare/upload handlers",
)

put_podcast = r'''
async fn put_podcast_index_pointer(
    headers: &HeaderMap,
    asset_cid: &str,
    manifest_cid: &str,
    owner: &OwnerSummary,
) -> Result<UpstreamBody, Response> {
    let raw_hash = asset_cid.trim_start_matches("b3:");
    let route = format!("/v1/index/assets/{raw_hash}/manifest");
    let index_base = index_base_url();
    let upstream_url = format!("{}{}", index_base.trim_end_matches('/'), route);

    let body = json!({
        "asset_kind": "podcast",
        "manifest_cid": manifest_cid,
        "owner_passport_subject": owner.passport_subject.clone(),
        "owner_wallet_account": owner.wallet_account.clone(),
        "updated_at_ms": now_ms(),
    });

    let body = match serde_json::to_vec(&body) {
        Ok(body) => body,
        Err(_) => {
            return Err(problem(
                StatusCode::INTERNAL_SERVER_ERROR,
                "index_pointer_encode_failed",
                "failed to encode podcast manifest pointer",
                false,
                "index_pointer_encode_failed",
            ));
        }
    };

    let mut req_builder = HTTP_CLIENT
        .put(upstream_url)
        .header(header::CONTENT_TYPE, "application/json")
        .body(body);

    for (name, value) in headers {
        if should_forward_header(name) {
            req_builder = req_builder.header(name, value);
        }
    }

    let upstream_res = match req_builder.send().await {
        Ok(upstream_res) => upstream_res,
        Err(_) => {
            return Err(problem(
                StatusCode::BAD_GATEWAY,
                "upstream_unavailable",
                "index podcast manifest pointer upstream unavailable",
                true,
                "index_connect",
            ));
        }
    };

    let status = upstream_res.status();
    let headers = upstream_res.headers().clone();
    let body = match upstream_res.bytes().await {
        Ok(body) => body,
        Err(_) => {
            return Err(problem(
                StatusCode::BAD_GATEWAY,
                "upstream_unavailable",
                "index podcast manifest pointer upstream unavailable",
                true,
                "index_read",
            ));
        }
    };

    Ok(UpstreamBody {
        status,
        headers,
        body,
    })
}

'''

assets = insert_before_once(
    assets,
    "async fn put_video_index_pointer(",
    put_podcast,
    "omnigate put_podcast_index_pointer",
)

build_podcast = r'''
fn build_podcast_manifest(
    headers: &HeaderMap,
    storage_upload: &Value,
    asset_cid: &str,
    owner: &OwnerSummary,
) -> Value {
    let title =
        grab(headers, "x-ron-asset-title").unwrap_or_else(|| "Untitled podcast episode".to_owned());
    let description = grab(headers, "x-ron-asset-description");
    let tags = tags_from_headers(headers);
    let content_type = grab(headers, "content-type");

    let cover_image_crab_url = grab(headers, "x-ron-podcast-cover-image-crab-url")
        .or_else(|| grab(headers, "x-ron-cover-image-crab-url"));
    let transcript_crab_url = grab(headers, "x-ron-podcast-transcript-crab-url")
        .or_else(|| grab(headers, "x-ron-transcript-crab-url"));
    let chapters_crab_url = grab(headers, "x-ron-podcast-chapters-crab-url");
    let show_page_crab_url = grab(headers, "x-ron-podcast-show-page-crab-url")
        .or_else(|| grab(headers, "x-ron-show-page-crab-url"));
    let guest_display = grab(headers, "x-ron-podcast-guest")
        .or_else(|| grab(headers, "x-ron-podcast-guests"))
        .or_else(|| grab(headers, "x-ron-podcast-guest-display"));

    let mut root = Map::new();
    root.insert("version".to_owned(), json!(1));
    root.insert("asset_cid".to_owned(), json!(asset_cid));
    root.insert("asset_kind".to_owned(), json!("podcast"));
    root.insert("kind".to_owned(), json!("podcast"));
    root.insert(
        "canonical_crab_url".to_owned(),
        json!(format!("crab://{}.podcast", asset_cid.trim_start_matches("b3:"))),
    );

    if owner.passport_subject.is_some() && owner.wallet_account.is_some() {
        root.insert(
            "owner".to_owned(),
            json!({
                "passport_subject": owner.passport_subject.clone(),
                "wallet_account": owner.wallet_account.clone(),
            }),
        );
    }

    if owner.wallet_account.is_some() {
        root.insert(
            "payout".to_owned(),
            json!({
                "default_action": "content_view",
                "recipient_account": owner.wallet_account.clone(),
                "splits": [
                    {
                        "role": "creator",
                        "account": owner.wallet_account.clone(),
                        "bps": 10_000
                    }
                ]
            }),
        );
    }

    root.insert(
        "metadata".to_owned(),
        json!({
            "title": title,
            "description": description,
            "tags": tags,
            "license": grab(headers, "x-ron-asset-license"),
            "content_type": content_type,
            "media_kind": "podcast",
            "podcast": {
                "show_title": grab(headers, "x-ron-podcast-show"),
                "host_display": grab(headers, "x-ron-podcast-host"),
                "guest_display": guest_display,
                "season_number": grab(headers, "x-ron-podcast-season"),
                "episode_number": grab(headers, "x-ron-podcast-episode"),
                "duration": grab(headers, "x-ron-podcast-duration"),
                "category": grab(headers, "x-ron-podcast-category"),
                "language": grab(headers, "x-ron-podcast-language"),
                "explicit_rating": grab(headers, "x-ron-podcast-explicit-rating"),
                "rights_attested": truthy_header(headers, "x-ron-podcast-rights-attested"),
                "guest_permission_attested": truthy_header(headers, "x-ron-podcast-guest-permission-attested"),
            }
        }),
    );

    root.insert(
        "linked_assets".to_owned(),
        json!({
            "cover_image_crab_url": cover_image_crab_url,
            "cover_image_upload_from_podcast_page": false,
            "transcript_crab_url": transcript_crab_url,
            "chapters_crab_url": chapters_crab_url,
            "show_page_crab_url": show_page_crab_url
        }),
    );

    root.insert(
        "rights_policy".to_owned(),
        json!({
            "rights_mode": grab(headers, "x-ron-podcast-rights-mode"),
            "license_mode": grab(headers, "x-ron-podcast-license-mode"),
            "legal_attestation": {
                "accepted": truthy_header(headers, "x-ron-podcast-rights-attested"),
                "guest_permission_attested": truthy_header(headers, "x-ron-podcast-guest-permission-attested"),
                "backend_verified": false,
                "local_ui_attestation_only": true,
                "proves_legal_ownership": false,
                "proves_guest_release": false
            }
        }),
    );

    root.insert(
        "provenance".to_owned(),
        json!({
            "created_at_ms": now_ms(),
            "source": "omnigate.podcast_upload",
            "parent_cids": [],
        }),
    );

    root.insert(
        "storage".to_owned(),
        json!({
            "available": true,
            "content_type": grab(headers, "content-type"),
            "raw_url": format!("/o/{asset_cid}"),
            "paid": true,
            "mode": "podcast_lite",
        }),
    );

    root.insert(
        "media".to_owned(),
        json!({
            "mode": "podcast_lite",
            "range_streaming": false,
            "transcoding": false,
            "drm": false,
            "live_stream": false,
            "note": "podcast-lite proof path only; live stream/session delivery remains future work"
        }),
    );

    root.insert(
        "truth_boundary".to_owned(),
        json!({
            "backend_uploaded": true,
            "audio_asset_uploaded": true,
            "creates_live_stream_session": false,
            "cover_art_upload_from_podcast_page": false,
            "transcript_upload_from_podcast_page": false,
            "legal_ownership_backend_verified": false,
            "guest_release_backend_verified": false,
            "wallet_mutation_by_omnigate": false
        }),
    );

    let receipts = receipt_refs_from_storage_upload(storage_upload, owner);
    root.insert("receipts".to_owned(), json!(receipts));

    Value::Object(root)
}

'''

assets = insert_before_once(
    assets,
    "fn build_music_manifest(",
    build_podcast,
    "omnigate build_podcast_manifest",
)

save(assets_path, assets)


# ---------------------------------------------------------------------------
# omnigate resolver: allow .podcast pages
# ---------------------------------------------------------------------------

crab_path = ROOT / "crates/omnigate/src/routes/v1/crab.rs"
crab = load(crab_path)

crab = replace_first_available(
    crab,
    [
        (
            '''        "image"
            | "video"
            | "stream"
            | "music"
            | "song"
            | "article"''',
            '''        "image"
            | "video"
            | "stream"
            | "music"
            | "song"
            | "podcast"
            | "article"''',
        ),
        (
            '''        "image" | "video" | "stream" | "music" | "song" | "article"''',
            '''        "image" | "video" | "stream" | "music" | "song" | "podcast" | "article"''',
        ),
    ],
    "omnigate crab resolver podcast kind",
)

save(crab_path, crab)


# ---------------------------------------------------------------------------
# omnigate content_view: allow podcast quote/pay
# ---------------------------------------------------------------------------

content_view_path = ROOT / "crates/omnigate/src/routes/v1/content_view.rs"
content_view = load(content_view_path)

content_view = replace_first_available(
    content_view,
    [
        (
            '''        "image" | "article" | "post" | "comment" | "video" | "stream" | "music" | "song"''',
            '''        "image" | "article" | "post" | "comment" | "video" | "stream" | "music" | "song" | "podcast"''',
        ),
        (
            '''"image" | "article" | "post" | "comment" | "video" | "stream" | "music" | "song"''',
            '''"image" | "article" | "post" | "comment" | "video" | "stream" | "music" | "song" | "podcast"''',
        ),
    ],
    "omnigate content_view podcast kind",
)

save(content_view_path, content_view)

print("continued podcast backend patch")
