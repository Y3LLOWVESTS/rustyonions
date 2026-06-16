#!/usr/bin/env bash
set -euo pipefail

CARGO="${CARGO:-cargo}"
PKG="omnigate"

echo "[omnigate quickchain] fmt"
"$CARGO" fmt -p "$PKG"

echo "[omnigate quickchain] focused preflight tests"
"$CARGO" test -p "$PKG" --test quickchain_preflight_boundary
"$CARGO" test -p "$PKG" --test quickchain_preflight_docs
"$CARGO" test -p "$PKG" --test quickchain_preflight_no_fake_receipts
"$CARGO" test -p "$PKG" --test quickchain_preflight_paid_access
"$CARGO" test -p "$PKG" --test quickchain_preflight_cache_boundary
"$CARGO" test -p "$PKG" --test quickchain_preflight_transport_authority

echo "[omnigate quickchain] paid/product route regressions"
"$CARGO" test -p "$PKG" --test content_view
"$CARGO" test -p "$PKG" --test site_visit
"$CARGO" test -p "$PKG" --test streams
"$CARGO" test -p "$PKG" --test chat_routes
"$CARGO" test -p "$PKG" --test paid_storage_estimate_proxy
"$CARGO" test -p "$PKG" --test paid_storage_prepare
"$CARGO" test -p "$PKG" --test paid_storage_write_proxy

echo "[omnigate quickchain] clippy"
"$CARGO" clippy -p "$PKG" --all-targets --no-deps -- -D warnings

echo "omnigate QuickChain preflight gate passed"
