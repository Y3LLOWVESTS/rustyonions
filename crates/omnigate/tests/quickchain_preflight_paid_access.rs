//! RO:WHAT — Paid-access QuickChain Phase-0 guards for omnigate.
//! RO:WHY — Paid routes must fail closed and rely on backend wallet/receipt truth.
//! RO:INTERACTS — content_view, site_visit, chat, streams, paid storage route sources.
//! RO:INVARIANTS — no caller/cache bypass; quote is read-only; pay uses svc-wallet; paid storage write is proxy-only.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — prevents caller-supplied paid/unlock claims from becoming authority.
//! RO:TEST — cargo test -p omnigate --test quickchain_preflight_paid_access.

use std::{
    fs,
    path::{Path, PathBuf},
};

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read_rel(path: &str) -> String {
    let full = crate_root().join(path);
    fs::read_to_string(&full).unwrap_or_else(|err| {
        panic!("failed to read {}: {err}", full.display());
    })
}

fn collect_rust_files(root: &Path, out: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(root).unwrap_or_else(|err| {
        panic!("failed to read directory {}: {err}", root.display());
    });

    for entry in entries {
        let path = entry
            .unwrap_or_else(|err| panic!("failed to read directory entry: {err}"))
            .path();

        if path.is_dir() {
            collect_rust_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

fn production_sources() -> Vec<(PathBuf, String)> {
    let mut files = Vec::new();
    collect_rust_files(&crate_root().join("src"), &mut files);
    files
        .into_iter()
        .map(|path| {
            let text = fs::read_to_string(&path)
                .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
            (path, text)
        })
        .collect()
}

#[test]
fn production_source_does_not_contain_paid_or_unlock_bypass_shortcuts() {
    let forbidden_markers = [
        "paid=true",
        "unlock=true",
        "unlocked=true",
        "cache_unlocked",
        "local_unlock",
        "dev_unlock",
        "free_unlock",
        "skip_payment",
        "bypass_payment",
        "trust_client_receipt",
        "trust_cache_receipt",
        "trust_client_paid",
        "trust_cache_paid",
        "force_paid",
        "force_unlock",
    ];

    for (path, source) in production_sources() {
        let lower = source.to_ascii_lowercase();
        for marker in forbidden_markers {
            assert!(
                !lower.contains(marker),
                "{} must not contain paid/unlock bypass marker `{marker}`",
                path.display()
            );
        }
    }
}

#[test]
fn content_view_and_site_visit_pay_routes_name_wallet_front_door_invariant() {
    for rel in [
        "src/routes/v1/content_view.rs",
        "src/routes/v1/site_visit.rs",
    ] {
        let source = read_rel(rel);

        for required in [
            "quote is read-only",
            "pay uses svc-wallet only",
            "no direct ledger mutation",
            "integer minor units only",
            "wallet_receipt",
        ] {
            assert!(
                source.contains(required),
                "{rel} must preserve paid-access invariant `{required}`"
            );
        }
    }
}

#[test]
fn stream_and_chat_paid_paths_fail_closed_on_backend_truth() {
    let streams = read_rel("src/routes/v1/streams.rs");
    for required in [
        "viewer media requires wallet receipt lookup",
        "fail-closed receipt validation",
        "no wallet mutation here",
    ] {
        assert!(
            streams.contains(required),
            "streams route must preserve backend receipt invariant `{required}`"
        );
    }

    let chat = read_rel("src/routes/v1/chat.rs");
    for required in [
        "paid send via svc-wallet",
        "paid send uses svc-wallet only",
        "paid message appears only after backend wallet success",
        "cache never unlocks paid chat",
        "no direct ledger mutation",
    ] {
        assert!(
            chat.contains(required),
            "chat route must preserve paid-send invariant `{required}`"
        );
    }
}

#[test]
fn paid_storage_route_stays_prepare_estimate_or_proxy_only() {
    let paid = read_rel("src/routes/v1/paid.rs");

    for required in [
        "prepare/estimate are read-only",
        "write is proxy-only",
        "no wallet, ledger, accounting, or storage mutation here",
        "wallet receipt verification",
        "capture/release",
    ] {
        assert!(
            paid.contains(required),
            "paid storage route must preserve proxy-only invariant `{required}`"
        );
    }
}
