#!/usr/bin/env bash
# check_two_node_admin_truth.sh
#
# RO:WHAT — Runtime truth probe for the two-node CrabLink admin contract.
# RO:WHY  — Proves macronode/service_node and micronode/user_node expose the
#           admin API surfaces svc-admin needs before we expand the build.
# RO:NOTE — No set -e: this script reports all probe failures in one pass.

FAIL=0

MACRO_BASE="${MACRO_BASE:-http://127.0.0.1:8080}"
MICRO_BASE="${MICRO_BASE:-http://127.0.0.1:5310}"
SVC_ADMIN_BASE="${SVC_ADMIN_BASE:-http://127.0.0.1:5300}"

have_jq=0
if command -v jq >/dev/null 2>&1; then
  have_jq=1
fi

say() {
  printf '%s\n' "$*"
}

fail() {
  FAIL=$((FAIL + 1))
  printf '[FAIL] %s\n' "$*" >&2
}

ok() {
  printf '[OK] %s\n' "$*"
}

fetch() {
  local url="$1"
  local out="$2"
  local code

  code="$(curl -sS -m 5 -o "$out" -w "%{http_code}" "$url" 2>/tmp/two-node-curl.err || true)"

  if [ "$code" != "200" ]; then
    fail "$url returned HTTP ${code:-curl-failed}"
    if [ -s /tmp/two-node-curl.err ]; then
      sed 's/^/[curl] /' /tmp/two-node-curl.err >&2
    fi
    return 1
  fi

  ok "$url returned 200"
  return 0
}

json_expect() {
  local file="$1"
  local expr="$2"
  local expected="$3"
  local label="$4"

  if [ "$have_jq" != "1" ]; then
    say "[SKIP] jq unavailable: $label"
    return 0
  fi

  local got
  got="$(jq -r "$expr" "$file" 2>/tmp/two-node-jq.err || true)"

  if [ "$got" != "$expected" ]; then
    fail "$label expected '$expected', got '$got'"
    if [ -s /tmp/two-node-jq.err ]; then
      sed 's/^/[jq] /' /tmp/two-node-jq.err >&2
    fi
    return 1
  fi

  ok "$label = $expected"
  return 0
}

json_bool_expect() {
  local file="$1"
  local expr="$2"
  local expected="$3"
  local label="$4"

  if [ "$have_jq" != "1" ]; then
    say "[SKIP] jq unavailable: $label"
    return 0
  fi

  local got
  got="$(jq -r "$expr" "$file" 2>/tmp/two-node-jq.err || true)"

  if [ "$got" != "$expected" ]; then
    fail "$label expected $expected, got $got"
    if [ -s /tmp/two-node-jq.err ]; then
      sed 's/^/[jq] /' /tmp/two-node-jq.err >&2
    fi
    return 1
  fi

  ok "$label = $expected"
  return 0
}

probe_basic_endpoint() {
  local base="$1"
  local path="$2"
  local label="$3"
  local tmp
  tmp="$(mktemp -t two-node-basic.XXXXXX)"

  if fetch "${base}${path}" "$tmp"; then
    ok "$label reachable"
  fi

  rm -f "$tmp"
}

probe_macro_status() {
  say ""
  say "== Macronode / Service Node status =="
  local tmp
  tmp="$(mktemp -t macro-status.XXXXXX)"

  if fetch "${MACRO_BASE}/api/v1/status" "$tmp"; then
    json_expect "$tmp" '.profile' 'macronode' 'macronode profile'
    json_expect "$tmp" '.node_role' 'service_node' 'macronode node_role'
    json_expect "$tmp" '.node_profile' 'macronode' 'macronode node_profile'
    json_bool_expect "$tmp" '.content_serving_enabled' 'true' 'macronode content_serving_enabled'
    json_bool_expect "$tmp" '.service_quorum_enabled' 'false' 'macronode service_quorum_enabled parked'
    json_bool_expect "$tmp" '.wallet_execution_participant' 'false' 'macronode wallet_execution_participant parked'
    json_expect "$tmp" '.user_ip_publication' 'not_applicable_service_node' 'macronode user_ip_publication'
  fi

  rm -f "$tmp"
}

probe_micro_status() {
  say ""
  say "== Micronode / User Node status =="
  local tmp
  tmp="$(mktemp -t micro-status.XXXXXX)"

  if fetch "${MICRO_BASE}/api/v1/status" "$tmp"; then
    json_expect "$tmp" '.profile' 'micronode' 'micronode profile'
    json_expect "$tmp" '.node_role' 'user_node' 'micronode node_role'
    json_expect "$tmp" '.node_profile' 'micronode' 'micronode node_profile'
    json_bool_expect "$tmp" '.amnesia_mode' 'true' 'micronode amnesia_mode'
    json_bool_expect "$tmp" '.privacy_mode' 'true' 'micronode privacy_mode'
    json_bool_expect "$tmp" '.public_inbound_enabled' 'false' 'micronode public_inbound_enabled'
    json_bool_expect "$tmp" '.content_serving_enabled' 'false' 'micronode content_serving_enabled'
    json_bool_expect "$tmp" '.service_quorum_enabled' 'false' 'micronode service_quorum_enabled'
    json_bool_expect "$tmp" '.wallet_execution_participant' 'false' 'micronode wallet_execution_participant'
    json_expect "$tmp" '.user_ip_publication' 'forbidden' 'micronode user_ip_publication'
  fi

  rm -f "$tmp"
}

probe_micro_rich_endpoints() {
  say ""
  say "== Micronode rich admin endpoints =="
  probe_basic_endpoint "$MICRO_BASE" "/api/v1/system/summary" "micronode system summary"
  probe_basic_endpoint "$MICRO_BASE" "/api/v1/storage/summary" "micronode storage summary"
  probe_basic_endpoint "$MICRO_BASE" "/metrics" "micronode metrics"
}

probe_svc_admin_registry() {
  say ""
  say "== svc-admin registry view =="
  local tmp
  local err
  local code

  tmp="$(mktemp -t svc-admin-nodes.XXXXXX)"
  err="$(mktemp -t svc-admin-nodes-err.XXXXXX)"

  code="$(curl -sS -m 5 -o "$tmp" -w "%{http_code}" "${SVC_ADMIN_BASE}/api/nodes" 2>"$err" || true)"

  case "$code" in
    200)
      ok "${SVC_ADMIN_BASE}/api/nodes returned 200"
      if [ "$have_jq" = "1" ]; then
        local count
        count="$(jq 'length' "$tmp" 2>/dev/null || echo unknown)"
        say "[INFO] svc-admin /api/nodes count: $count"
        jq . "$tmp" 2>/dev/null || cat "$tmp"
      else
        cat "$tmp"
      fi
      ;;
    401)
      ok "${SVC_ADMIN_BASE}/api/nodes returned 401 anonymous-protected"
      say "[INFO] svc-admin registry API is protected; this is expected unless auth.mode=none or a session/token is provided."
      ;;
    403)
      ok "${SVC_ADMIN_BASE}/api/nodes returned 403 anonymous-forbidden"
      say "[INFO] svc-admin registry API is protected; this is expected unless auth.mode=none or a session/token is provided."
      ;;
    *)
      fail "${SVC_ADMIN_BASE}/api/nodes returned HTTP ${code:-curl-failed}"
      if [ -s "$err" ]; then
        sed 's/^/[curl] /' "$err" >&2
      fi
      ;;
  esac

  rm -f "$tmp" "$err"
}

say "Two-node admin truth probe"
say "MACRO_BASE=${MACRO_BASE}"
say "MICRO_BASE=${MICRO_BASE}"
say "SVC_ADMIN_BASE=${SVC_ADMIN_BASE}"

probe_basic_endpoint "$MACRO_BASE" "/healthz" "macronode healthz"
probe_basic_endpoint "$MACRO_BASE" "/readyz" "macronode readyz"
probe_basic_endpoint "$MICRO_BASE" "/healthz" "micronode healthz"
probe_basic_endpoint "$MICRO_BASE" "/readyz" "micronode readyz"

probe_macro_status
probe_micro_status
probe_micro_rich_endpoints
probe_svc_admin_registry

say ""
if [ "$FAIL" -eq 0 ]; then
  say "✅ two-node admin truth probe passed"
  exit 0
fi

say "❌ two-node admin truth probe failed with $FAIL failure(s)"
exit 1
