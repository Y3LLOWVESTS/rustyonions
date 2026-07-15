#!/usr/bin/env bash
set -euo pipefail

ROOT="$(
  cd "$(dirname "${BASH_SOURCE[0]}")/.." >/dev/null 2>&1
  pwd
)"

CRABLINK_ROOT="${CRABLINK_ROOT:-/Users/mymac/Desktop/crablink}"
TAURI_MANIFEST="$CRABLINK_ROOT/apps/crablink-tauri/src-tauri/Cargo.toml"
TAURI_CONFIG_FILE="$CRABLINK_ROOT/apps/crablink-tauri/src-tauri/tauri.macos.dev-media.conf.json"

fail() {
  printf 'Phase 24 private-beta acceptance failed: %s\n' "$*" >&2
  exit 1
}

[[ -f "$ROOT/Cargo.toml" ]] ||
  fail "RustyOnions workspace Cargo.toml is missing"

[[ -f "$TAURI_MANIFEST" ]] ||
  fail "CrabLink Tauri manifest is missing: $TAURI_MANIFEST"

[[ -f "$TAURI_CONFIG_FILE" ]] ||
  fail "CrabLink Tauri development config is missing: $TAURI_CONFIG_FILE"

printf '%s\n' \
  "== Phase 24 private-beta readiness acceptance ==" \
  "RustyOnions: $ROOT" \
  "CrabLink:    $CRABLINK_ROOT"

cd "$ROOT"

echo
echo "== Phase 24 documentation contracts =="

bash scripts/check-phase24-private-beta-node-runbook.sh
bash scripts/check-phase24-node-runbook-alignment.sh
bash scripts/check-phase24-economic-runbook-alignment.sh
bash scripts/check-phase24-content-privacy-runbook-alignment.sh

echo
echo "== Rust formatting =="

for crate in \
  macronode \
  micronode \
  svc-admin \
  svc-dht \
  svc-storage \
  svc-registry \
  svc-rewarder \
  svc-wallet \
  ron-ledger
do
  cargo fmt -p "$crate" -- --check
done

echo
echo "== User Node behavior =="

cargo test \
  -p micronode \
  --test object_verification \
  --test passive_runtime \
  --test admin_parity \
  --test internal_roc_beta_phase17_epoch_replay \
  -- \
  --nocapture

echo
echo "== Service Node CLI, setup, and operator controls =="

cargo test \
  -p macronode \
  --test crabnode_cli \
  --test crabnode_policy_cli \
  --test crabnode_prune_cli \
  --test crabnode_rewards_cli \
  --test crabnode_setup_cli \
  --test crabnode_persistence_cli \
  --test operator_admin \
  -- \
  --nocapture

echo
echo "== Provider privacy and stale discovery =="

cargo test \
  -p svc-dht \
  --test phase23_provider_discovery_chaos \
  --test privacy_no_ip_leak \
  --test provider_lookup_selection \
  --test provider_selection \
  -- \
  --nocapture

echo
echo "== Content integrity, moderation, amnesia, and pruning =="

cargo test \
  -p svc-storage \
  --test phase23_hostile_content_chaos \
  --test oap_object_fetch \
  --test oap_http_transport \
  --test amnesia_storage \
  --test legacy_http_moderation \
  --test quickchain_preflight_no_direct_mutation \
  -- \
  --nocapture

echo
echo "== Reward binding, eligibility, and Sybil resistance =="

cargo test \
  -p svc-registry \
  --test reward_binding_registry \
  --test reward_binding_request_intake \
  --test reward_payout_guard \
  --test internal_roc_beta_phase18_eligibility_registry \
  --test internal_roc_beta_phase18_eligibility_transition \
  --test internal_roc_beta_phase18_quorum_weighting \
  --test internal_roc_beta_phase18_cross_crate_sybil \
  -- \
  --nocapture

echo
echo "== Deterministic reward planning and dependency failure =="

cargo test \
  -p svc-rewarder \
  --test integration \
  --test quickchain_preflight_no_direct_mutation \
  --test quickchain_preflight_replay_no_double_issue \
  --test internal_roc_beta_phase22_local_reward_loop \
  -- \
  --nocapture

echo
echo "== Wallet quorum, mutation, and duplicate protection =="

cargo test \
  -p svc-wallet \
  --test internal_roc_beta_phase16_quorum_execution \
  --test internal_roc_beta_phase3_approved_payout_execution_boundary \
  --test internal_roc_beta_phase2_paid_action_idempotency \
  --test internal_roc_beta_phase2_receipt_lookup_after_replay \
  -- \
  --nocapture

echo
echo "== Ledger durability, replay, and conservation =="

cargo test \
  -p ron-ledger \
  --features quickchain-preflight \
  --test internal_roc_beta_phase3_approved_payout_replay \
  --test internal_roc_beta_phase2_replay_conservation \
  --test internal_roc_beta_paid_content_replay_label \
  -- \
  --nocapture

echo
echo "== Strict Clippy =="

for crate in \
  macronode \
  micronode \
  svc-admin \
  svc-dht \
  svc-storage \
  svc-registry \
  svc-rewarder \
  svc-wallet
do
  cargo clippy \
    -p "$crate" \
    --all-targets \
    --no-deps \
    -- \
    -D warnings
done

cargo clippy \
  -p ron-ledger \
  --all-targets \
  --features quickchain-preflight \
  --no-deps \
  -- \
  -D warnings

echo
echo "== RustyOnions workspace compilation =="

cargo check --workspace

cargo check \
  -p ron-ledger \
  --features quickchain-preflight

echo
echo "== CrabLink User Node and privacy boundary =="

cd "$CRABLINK_ROOT"

cargo fmt \
  --manifest-path "$TAURI_MANIFEST" \
  -- \
  --check

export TAURI_CONFIG="$(
  cat "$TAURI_CONFIG_FILE"
)"

cargo test \
  --manifest-path "$TAURI_MANIFEST" \
  --test phase23_privacy_relay_unavailable \
  -- \
  --nocapture

cargo test \
  --manifest-path "$TAURI_MANIFEST" \
  --lib \
  user_node_verification::tests \
  -- \
  --nocapture

node scripts/check-crablink-user-node-verification-boundary.mjs

cargo clippy \
  --manifest-path "$TAURI_MANIFEST" \
  --all-targets \
  --no-deps \
  -- \
  -D warnings

cargo check \
  --manifest-path "$TAURI_MANIFEST"

echo
echo "== Phase 24 final park status =="

cd "$ROOT"
bash scripts/check-phase24-private-beta-park-status.sh

echo
echo "Phase 24 private-beta readiness acceptance passed."
echo "Phase 24 is complete, green, and parked."
echo "PHASE24_FINAL_STATUS=GREEN_PARKED"
