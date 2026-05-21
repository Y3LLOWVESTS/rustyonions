#!/usr/bin/env python3
# RO:WHAT — Patch RustyOnions gateway/omnigate for podcast-lite prepare/upload routes.
# RO:WHY — CrabLink podcast page currently gets 404 because backend does not expose podcast routes yet.
# RO:INVARIANTS — gateway-first; omnigate coordinates storage/index only; no wallet/ledger mutation here.
# RO:SECURITY — podcast-lite only; no live stream truth; no cover-art upload; no backend legal ownership proof claim.
# RO:TEST — cargo fmt -p omnigate -p svc-gateway; cargo check -p omnigate --all-targets; cargo check -p svc-gateway --all-targets.

from pathlib import Path

ROOT = Path.cwd()


def load(path: Path) -> str:
    if not path.exists():
        raise SystemExit(f"missing file: {path}")
    return path.read_text()


def save(path: Path, text: str) -> None:
    path.write_text(text)


def replace_once(text: str, old: str, new: str, label: str) -> str:
    if new in text:
        print(f"skip {label}: already patched")
        return text

    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected 1 match, found {count}")

    print(f"patch {label}")
    return text.replace(old, new, 1)


def replace_first_available(text: str, replacements: list[tuple[str, str]], label: str) -> str:
    for old, new in replacements:
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


def insert_after_once(text: str, marker: str, insertion: str, label: str) -> str:
    if insertion.strip() in text:
        print(f"skip {label}: already patched")
        return text

    count = text.count(marker)
    if count != 1:
        raise SystemExit(f"{label}: expected 1 marker, found {count}")

    print(f"patch {label}")
    return text.replace(marker, marker + insertion, 1)


def insert_before_once(text: str, marker: str, insertion: str, label: str) -> str:
    if insertion.strip() in text:
        print(f"skip {label}: already patched")
        return text

    count = text.count(marker)
    if count != 1:
        raise SystemExit(f"{label}: expected 1 marker, found {count}")

    print(f"patch {label}")
    return text.replace(marker, insertion + marker, 1)


# ---------------------------------------------------------------------------
# svc-gateway product proxy routes
# ---------------------------------------------------------------------------

product_path = ROOT / "crates/svc-gateway/src/routes/product.rs"
product = load(product_path)

product = insert_after_once(
    product,
    '/// POST /assets/music\n',
    '/// POST /assets/podcast/prepare\n/// POST /assets/podcast\n',
    "svc-gateway docs",
)

product = insert_after_once(
    product,
    '        .route("/assets/music", post(music_upload))\n',
    '        .route("/assets/podcast/prepare", post(podcast_prepare))\n'
    '        .route("/assets/podcast", post(podcast_upload))\n',
    "svc-gateway router podcast routes",
)

product_podcast_fns = r'''
/// Proxy `POST /assets/podcast/prepare` to `omnigate /v1/assets/podcast/prepare`.
///
/// Gateway does not validate podcast rights, guest permissions, price writes,
/// store bytes, write index pointers, transcode audio, upload cover art, or
/// claim legal proof. It only exposes the public CrabLink route.
pub async fn podcast_prepare(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    proxy_to_omnigate(
        &state,
        Method::POST,
        "/v1/assets/podcast/prepare",
        headers,
        body,
    )
    .await
}

/// Proxy `POST /assets/podcast` to `omnigate /v1/assets/podcast`.
///
/// Gateway only exposes the public `CrabLink` route. Omnigate coordinates the
/// bounded podcast-lite paid storage and manifest/index write.
pub async fn podcast_upload(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    proxy_to_omnigate(&state, Method::POST, "/v1/assets/podcast", headers, body).await
}

'''

product = insert_after_once(
    product,
    '''/// Proxy `POST /assets/music` to `omnigate /v1/assets/music`.
///
/// Gateway only exposes the public `CrabLink` route. Omnigate coordinates the
/// bounded music-lite paid storage and manifest/index write.
pub async fn music_upload(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    proxy_to_omnigate(&state, Method::POST, "/v1/assets/music", headers, body).await
}

''',
    product_podcast_fns,
    "svc-gateway podcast proxy fns",
)

save(product_path, product)


# ---------------------------------------------------------------------------
# svc-gateway proxy test table
# ---------------------------------------------------------------------------

proxy_test_path = ROOT / "crates/svc-gateway/tests/product_routes_proxy.rs"
proxy_test = load(proxy_test_path)

proxy_test = insert_after_once(
    proxy_test,
    '        ("/assets/music", "/v1/assets/music"),\n',
    '        ("/assets/podcast/prepare", "/v1/assets/podcast/prepare"),\n'
    '        ("/assets/podcast", "/v1/assets/podcast"),\n',
    "svc-gateway proxy test podcast rows",
)

save(proxy_test_path, proxy_test)


# ---------------------------------------------------------------------------
# omnigate assets route
# ---------------------------------------------------------------------------

assets_path = ROOT / "crates/omnigate/src/routes/v1/assets.rs"
assets = load(assets_path)

assets = insert_after_once(
    assets,
    'const MUSIC_UPLOAD_SCHEMA: &str = "omnigate.music-asset-upload.v1";\n',
    'const PODCAST_PREPARE_SCHEMA: &str = "omnigate.podcast-asset-prepare.v1";\n'
    'const PODCAST_UPLOAD_SCHEMA: &str = "omnigate.podcast-asset-upload.v1";\n',
    "omnigate podcast schema constants",
)

assets = insert_after_once(
    assets,
    '        .route("/music", post(music_upload))\n',
    '        .route("/podcast/prepare", post(podcast_prepare))\n'
    '        .route("/podcast", post(podcast_upload))\n',
    "omnigate podcast router",
)

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
    '#[derive(Debug, Clone, Deserialize, Serialize)]\nstruct StreamAssetRequest {',
    podcast_dto,
    "omnigate PodcastAssetPrepareRequest",
)

podcast_handlers = r'''
/// Prepare a podcast-lite audio publication.
///
/// This is read-only pricing/policy preparation. It does not create a CID,
/// store audio bytes, mutate wallets, write index pointers, verify legal
/// rights, transcode, claim DRM, or create a live stream session.
pub async fn podcast_prepare(
    headers: HeaderMap,
    Json(req): Json<PodcastAssetPrepareRequest>,
) -> Response {
    if req.bytes == 0 {
        return problem(
            StatusCode::BAD_REQUEST,
            "bad_request",
            "podcast bytes must be greater than zero",
            false,
            "empty_podcast_asset",
        );
    }

    let content_type = req
        .content_type
        .as_deref()
        .unwrap_or("application/octet-stream");

    if !is_valid_audio_content_type(content_type) {
        return problem(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "unsupported_media_type",
            "podcast upload requires an audio content-type",
            false,
            "unsupported_podcast_content_type",
        );
    }

    if !req.legal_attestation_accepted {
        return problem(
            StatusCode::BAD_REQUEST,
            "bad_request",
            "podcast rights attestation is required before prepare",
            false,
            "podcast_rights_attestation_required",
        );
    }

    if !req.guest_permission_attested {
        return problem(
            StatusCode::BAD_REQUEST,
            "bad_request",
            "podcast guest/voice permission attestation is required before prepare",
            false,
            "podcast_guest_permission_required",
        );
    }

    let expected_asset_cid = match req.expected_asset_cid.as_deref() {
        Some(cid) if is_canonical_b3_cid(cid) => Some(cid.to_owned()),
        Some(_) => {
            return problem(
                StatusCode::BAD_REQUEST,
                "bad_request",
                "expected_asset_cid must be canonical b3:<64 lowercase hex>",
                false,
                "invalid_expected_asset_cid",
            );
        }
        None => None,
    };

    let payer = req
        .payer_account
        .clone()
        .or_else(|| grab(&headers, "x-ron-wallet-account"))
        .unwrap_or_else(|| "acct_dev".to_string());

    let owner = resolve_owner_context(&headers, req.owner_passport_subject.as_deref(), Some(&payer));
    let estimate = match paid_storage_estimate(req.bytes, Some(content_type)).await {
        Ok(estimate) => estimate,
        Err(resp) => return resp,
    };

    let idempotency = req
        .client_idempotency_key
        .clone()
        .or_else(|| grab(&headers, "idempotency-key"))
        .unwrap_or_else(|| format!("podcast-prepare-{}", now_ms()));

    (
        StatusCode::OK,
        Json(json!({
            "schema": PODCAST_PREPARE_SCHEMA,
            "asset_kind": "podcast",
            "content_type": content_type,
            "bytes": req.bytes,
            "expected_asset_cid": expected_asset_cid,
            "file_name": req.file_name,
            "title": req.title,
            "description": req.description,
            "tags": req.tags,
            "podcast": {
                "show_title": req.show_title,
                "host_display": req.host_display,
                "guest_display": req.guest_display,
                "season_number": req.season_number,
                "episode_number": req.episode_number,
                "duration": req.duration,
                "category": req.category,
                "language": req.language,
                "explicit_rating": req.explicit_rating,
                "cover_image_crab_url": req.cover_image_crab_url,
                "transcript_crab_url": req.transcript_crab_url,
                "chapters_crab_url": req.chapters_crab_url,
                "show_page_crab_url": req.show_page_crab_url,
                "rights_mode": req.rights_mode,
                "license_mode": req.license_mode,
                "guest_permission_attested": req.guest_permission_attested,
                "legal_attestation_accepted": req.legal_attestation_accepted,
            },
            "paid_storage": estimate.raw,
            "estimate": {
                "amount_minor": estimate.amount_minor,
                "amount": estimate.amount_minor,
                "asset": DEFAULT_ASSET,
                "currency": DEFAULT_CURRENCY,
                "action": DEFAULT_ACTION,
                "bytes": req.bytes,
            },
            "wallet_hold": {
                "required": true,
                "from": payer,
                "to": estimate.escrow_account,
                "asset": DEFAULT_ASSET,
                "amount_minor": estimate.amount_minor,
                "amount": estimate.amount_minor,
                "idempotency_key_hint": idempotency,
                "memo": "CrabLink podcast upload hold"
            },
            "owner": owner,
            "next": {
                "create_hold": "/v1/wallet/hold",
                "submit_upload": "/v1/assets/podcast",
                "requires_paid_headers": true,
                "requires_rights_header": "x-ron-podcast-rights-attested=true",
                "requires_guest_permission_header": "x-ron-podcast-guest-permission-attested=true"
            },
            "warnings": [
                "podcast_lite_only_no_transcoding_no_drm_no_cover_art_upload",
                "cover_art_and_transcripts_are_reference_only",
                "legal_attestation_is_creator_confirmation_not_backend_ownership_proof",
                "podcast_lite_is_recorded_audio_not_live_stream_delivery"
            ]
        })),
    )
        .into_response()
}

/// Upload a podcast-lite audio file with paid proof headers.
///
/// Omnigate coordinates paid storage, manifest storage, and index pointer writes.
/// It does not mutate wallet truth, verify legal rights, transcode, create live
/// stream sessions, or upload cover art.
pub async fn podcast_upload(headers: HeaderMap, body: Bytes) -> Response {
    let content_type = grab(&headers, header::CONTENT_TYPE.as_str())
        .unwrap_or_else(|| "application/octet-stream".to_string());

    if !is_valid_audio_content_type(&content_type) {
        return problem(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "unsupported_media_type",
            "podcast upload requires an audio content-type",
            false,
            "unsupported_podcast_content_type",
        );
    }

    if !truthy_header(&headers, "x-ron-podcast-rights-attested") {
        return problem(
            StatusCode::BAD_REQUEST,
            "bad_request",
            "podcast upload requires x-ron-podcast-rights-attested=true",
            false,
            "podcast_rights_header_required",
        );
    }

    if !truthy_header(&headers, "x-ron-podcast-guest-permission-attested") {
        return problem(
            StatusCode::BAD_REQUEST,
            "bad_request",
            "podcast upload requires x-ron-podcast-guest-permission-attested=true",
            false,
            "podcast_guest_permission_header_required",
        );
    }

    let paid_proof = match PaidProof::from_headers(&headers) {
        Ok(proof) => proof,
        Err(resp) => return resp,
    };

    let owner = resolve_owner_context(
        &headers,
        grab(&headers, "x-ron-owner-passport-subject").as_deref(),
        paid_proof.from.as_deref(),
    );

    let title = grab(&headers, "x-ron-asset-title")
        .or_else(|| grab(&headers, "x-ron-podcast-title"))
        .unwrap_or_else(|| "Untitled podcast episode".to_string());
    let description = grab(&headers, "x-ron-asset-description");
    let tags = normalize_tags(grab(&headers, "x-ron-asset-tags").as_deref());

    let storage = match put_paid_object(&headers, body.clone(), Some(&content_type)).await {
        Ok(storage) => storage,
        Err(resp) => return resp,
    };

    let asset_cid = storage.cid.clone();
    let raw_url = storage.raw_url.clone();

    let manifest = build_podcast_manifest(
        &asset_cid,
        &raw_url,
        &content_type,
        body.len() as u64,
        &title,
        description.as_deref(),
        tags,
        &headers,
        &paid_proof,
        &owner,
    );

    let manifest_summary = match put_manifest_object(&headers, &manifest).await {
        Ok(summary) => summary,
        Err(resp) => return resp,
    };

    let index_summary = match put_podcast_index_pointer(
        &headers,
        &asset_cid,
        &manifest_summary.cid,
        &title,
        &paid_proof,
        &owner,
    )
    .await
    {
        Ok(summary) => summary,
        Err(resp) => return resp,
    };

    let hash = asset_cid.trim_start_matches("b3:");
    let crab_url = format!("crab://{}.podcast", hash);

    (
        StatusCode::CREATED,
        Json(json!({
            "schema": PODCAST_UPLOAD_SCHEMA,
            "asset_kind": "podcast",
            "kind": "podcast",
            "asset_cid": asset_cid,
            "cid": asset_cid,
            "manifest_cid": manifest_summary.cid,
            "crab_url": crab_url,
            "url": crab_url,
            "content_type": content_type,
            "bytes": body.len(),
            "storage": {
                "raw_url": raw_url,
                "paid": true,
                "status": storage.status,
                "response": storage.raw
            },
            "manifest": manifest,
            "manifest_storage": {
                "status": manifest_summary.status,
                "response": manifest_summary.raw
            },
            "index": {
                "status": index_summary.status,
                "response": index_summary.raw
            },
            "paid_proof": paid_proof,
            "owner": owner,
            "warnings": [
                "podcast_lite_no_transcoding_no_drm_no_cover_art_upload",
                "cover_image_crab_url_is_reference_only",
                "transcript_crab_url_is_reference_only",
                "paid_proof_is_wallet_hold_display_data_not_legal_ownership_proof",
                "podcast_lite_is_recorded_audio_not_live_stream_delivery"
            ]
        })),
    )
        .into_response()
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
    title: &str,
    paid_proof: &PaidProof,
    owner: &OwnerContext,
) -> Result<IndexWriteSummary, Response> {
    let index_base = downstream_index_base_url();
    let endpoint = format!("{}/v1/assets/{}", index_base, asset_cid.trim_start_matches("b3:"));

    let mut request = HTTP_CLIENT
        .put(endpoint)
        .header(header::CONTENT_TYPE, "application/json");

    if let Some(auth) = headers.get(header::AUTHORIZATION) {
        request = request.header(header::AUTHORIZATION, auth.clone());
    }

    if let Some(correlation) = headers.get("x-correlation-id") {
        request = request.header("x-correlation-id", correlation.clone());
    }

    let body = json!({
        "schema": "ron.index.asset-pointer.v1",
        "asset_cid": asset_cid,
        "manifest_cid": manifest_cid,
        "asset_kind": "podcast",
        "kind": "podcast",
        "title": title,
        "owner": {
            "passport_subject": owner.passport_subject.clone(),
            "wallet_account": owner.wallet_account.clone(),
            "display": owner.display.clone()
        },
        "economics": {
            "paid": true,
            "asset": paid_proof.asset.clone().unwrap_or_else(|| DEFAULT_ASSET.to_string()),
            "amount_minor": paid_proof.amount_minor.clone(),
            "wallet_txid": paid_proof.txid.clone(),
            "receipt_hash": paid_proof.receipt_hash.clone(),
            "payout_account": owner.wallet_account.clone().or_else(|| paid_proof.from.clone())
        },
        "media": {
            "mode": "podcast_lite",
            "range_streaming": false,
            "transcoding": false,
            "drm": false,
            "live_stream": false
        },
        "updated_at_ms": now_ms()
    });

    let response = request.json(&body).send().await.map_err(|_| {
        problem(
            StatusCode::BAD_GATEWAY,
            "bad_gateway",
            "failed to write podcast index pointer",
            true,
            "index_pointer_write_failed",
        )
    })?;

    let status = response.status();
    let raw = response
        .json::<Value>()
        .await
        .unwrap_or_else(|_| json!({ "status": status.as_u16() }));

    if !status.is_success() {
        return Err((
            StatusCode::BAD_GATEWAY,
            Json(json!({
                "code": "bad_gateway",
                "message": "svc-index rejected podcast asset pointer",
                "retryable": true,
                "reason": "index_pointer_rejected",
                "status": status.as_u16(),
                "downstream": raw
            })),
        )
            .into_response());
    }

    Ok(IndexWriteSummary {
        status: "stored",
        raw,
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
    asset_cid: &str,
    raw_url: &str,
    content_type: &str,
    bytes: u64,
    title: &str,
    description: Option<&str>,
    tags: Vec<String>,
    headers: &HeaderMap,
    paid_proof: &PaidProof,
    owner: &OwnerContext,
) -> Value {
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

    json!({
        "schema": "ron.asset.manifest.v1",
        "asset_kind": "podcast",
        "kind": "podcast",
        "asset_cid": asset_cid,
        "canonical_cid": asset_cid,
        "canonical_crab_url": format!("crab://{}.podcast", asset_cid.trim_start_matches("b3:")),
        "metadata": {
            "title": title,
            "description": description,
            "tags": tags,
            "content_type": content_type,
            "bytes": bytes,
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
                "guest_permission_attested": truthy_header(headers, "x-ron-podcast-guest-permission-attested")
            }
        },
        "linked_assets": {
            "cover_image_crab_url": cover_image_crab_url,
            "cover_image_upload_from_podcast_page": false,
            "transcript_crab_url": transcript_crab_url,
            "chapters_crab_url": chapters_crab_url,
            "show_page_crab_url": show_page_crab_url
        },
        "owner": owner,
        "rights_policy": {
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
        },
        "storage": {
            "raw_url": raw_url,
            "paid": true,
            "content_type": content_type,
            "bytes": bytes
        },
        "media": {
            "mode": "podcast_lite",
            "range_streaming": false,
            "transcoding": false,
            "drm": false,
            "live_stream": false,
            "bounded_mvp": true
        },
        "economics": {
            "paid_storage": true,
            "paid_access": true,
            "proof": paid_proof
        },
        "truth_boundary": {
            "backend_uploaded": true,
            "audio_asset_uploaded": true,
            "creates_live_stream_session": false,
            "cover_art_upload_from_podcast_page": false,
            "transcript_upload_from_podcast_page": false,
            "legal_ownership_backend_verified": false,
            "guest_release_backend_verified": false,
            "wallet_mutation_by_omnigate": false
        },
        "created_at_ms": now_ms()
    })
}

'''

assets = insert_before_once(
    assets,
    "fn build_stream_descriptor(",
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
# omnigate content_view: allow podcast_view quote/pay
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

print("patched podcast backend routes")
