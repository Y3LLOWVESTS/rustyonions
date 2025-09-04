#!/usr/bin/env bash
set -euo pipefail

# Quota/overload demo with parallel sends, per-run deltas, and a classified summary.
# Env knobs:
#   N: total sends (default 40)
#   P: concurrency (default 20)
#   QUOTA_MAILBOX_RPS, QUOTA_TILE_RPS, MAX_INFLIGHT, READY_OVERLOAD_PCT
#   ADDR (overlay TLS, default 127.0.0.1:9443)
#   ADMIN_ADDR (admin HTTP, default 127.0.0.1:9096)
#   TLS_DIR (server cert/key; default testing/tls)

ROOT=${ROOT:-$(pwd)}
TLS_DIR=${TLS_DIR:-testing/tls}
ADDR=${ADDR:-127.0.0.1:9443}
ADMIN_ADDR=${ADMIN_ADDR:-127.0.0.1:9096}

N=${N:-40}
P=${P:-20}

MAX_INFLIGHT=${MAX_INFLIGHT:-2}
QUOTA_MAILBOX_RPS=${QUOTA_MAILBOX_RPS:-2}
QUOTA_TILE_RPS=${QUOTA_TILE_RPS:-2}
READY_OVERLOAD_PCT=${READY_OVERLOAD_PCT:-80}

CA_CRT="$TLS_DIR/ca.crt"
SRV_CRT="$TLS_DIR/server.crt"
SRV_KEY="$TLS_DIR/server.key"

get_port() { printf "%s\n" "$1" | awk -F: '{print $NF}'; }
port_listening() { lsof -n -i TCP:"$1" -sTCP:LISTEN >/dev/null 2>&1; }

require_tls() {
  if [ ! -f "$SRV_CRT" ] || [ ! -f "$SRV_KEY" ]; then
    echo "[quota] ERROR: missing $SRV_CRT or $SRV_KEY"
    echo "Run: bash scripts/dev_tls_setup_mac.sh"
    exit 1
  fi
}

build_all() {
  echo "[quota] building"
  cargo build -p svc-omnigate -p ron-app-sdk --examples
}

start_server_if_free() {
  local oap_port admin_port
  oap_port="$(get_port "$ADDR")"
  admin_port="$(get_port "$ADMIN_ADDR")"

  if port_listening "$oap_port" && port_listening "$admin_port"; then
    echo "[quota] detected existing server on $ADDR / $ADMIN_ADDR — reusing it"
    SVC_PID=""
    return 0
  fi

  echo "[quota] starting svc-omnigate"
  CERT_PEM="$SRV_CRT" \
  KEY_PEM="$SRV_KEY" \
  ADDR="$ADDR" \
  ADMIN_ADDR="$ADMIN_ADDR" \
  MAX_INFLIGHT="$MAX_INFLIGHT" \
  QUOTA_MAILBOX_RPS="$QUOTA_MAILBOX_RPS" \
  QUOTA_TILE_RPS="$QUOTA_TILE_RPS" \
  READY_OVERLOAD_PCT="$READY_OVERLOAD_PCT" \
  target/debug/svc-omnigate &
  SVC_PID=$!
  echo $SVC_PID > "$TLS_DIR/.svc_omnigate.pid"
  trap 'if [ -n "${SVC_PID:-}" ]; then kill $SVC_PID 2>/dev/null || true; fi' EXIT
}

wait_ready() {
  echo "[quota] waiting for /readyz"
  for _ in $(seq 1 50); do
    if curl -fsS "http://$ADMIN_ADDR/readyz" >/dev/null 2>&1; then
      echo "[quota] server is ready"
      return 0
    fi
    sleep 0.2
  done
  echo "[quota] ERROR: /readyz did not come up in time"
  exit 1
}

metrics_fetch() {
  # $1: file to write metrics snapshot into
  curl -fsS "http://$ADMIN_ADDR/metrics" > "$1" || true
}

metric_val() {
  # $1: file; $2: key
  # prints numeric value or 0 if missing
  awk -v key="$2" '$1==key{print $2}' "$1" 2>/dev/null | head -n1 || true
}

send_one() {
  i="$1"
  RUST_LOG=${RUST_LOG:-warn} \
  RON_NODE_URL="https://$ADDR" \
  RON_CA_PEM="${RON_CA_PEM:-$CA_CRT}" \
  target/debug/examples/mailbox_send "topic=a" "text=hi-$i" "idem=quota-$i" 2>&1
}

burst_parallel() {
  echo "[quota] parallel burst N=$N P=$P (429s if quotas low; 503s if inflight tight)"
  workdir="$(mktemp -d)"
  okf="$workdir/ok.txt"
  q429f="$workdir/429.txt"
  o503f="$workdir/503.txt"
  othf="$workdir/other.txt"
  : > "$okf"; : > "$q429f"; : > "$o503f"; : > "$othf"

  sem=$(mktemp -u)
  mkfifo "$sem"
  exec 3<>"$sem"
  rm "$sem"
  for _ in $(seq 1 "$P"); do echo >&3; done

  for i in $(seq 1 "$N"); do
    read -r -u 3
    {
      out=$(send_one "$i" || true)
      if printf "%s" "$out" | grep -q 'msg_id:'; then
        printf "%s\n" "$out" >> "$okf"
      elif printf "%s" "$out" | grep -Eiq '(^|[^0-9])429([^0-9]|$)|over_quota'; then
        printf "%s\n" "$out" >> "$q429f"
      elif printf "%s" "$out" | grep -Eiq '(^|[^0-9])503([^0-9]|$)|overload|not_ready'; then
        printf "%s\n" "$out" >> "$o503f"
      else
        printf "%s\n" "$out" >> "$othf"
      fi
      echo >&3
    } &
  done
  wait || true
  exec 3>&-

  okc=$(wc -l < "$okf" 2>/dev/null || echo 0)
  rc429=$(wc -l < "$q429f" 2>/dev/null || echo 0)
  rc503=$(wc -l < "$o503f" 2>/dev/null || echo 0)
  othc=$(wc -l < "$othf" 2>/dev/null || echo 0)

  echo "[quota] summary:"
  printf "  ok=%5d  429(quota)=%5d  503(overload)=%5d  other=%5d\n" "$okc" "$rc429" "$rc503" "$othc"

  rm -rf "$workdir"
}

metrics_report() {
  before="$1"
  after="$2"

  r_before=$(metric_val "$before" requests_total);      r_after=$(metric_val "$after" requests_total)
  bo_before=$(metric_val "$before" bytes_out_total);    bo_after=$(metric_val "$after" bytes_out_total)
  bi_before=$(metric_val "$before" bytes_in_total);     bi_after=$(metric_val "$after" bytes_in_total)
  ol_before=$(metric_val "$before" rejected_overload_total); ol_after=$(metric_val "$after" rejected_overload_total)
  nf_before=$(metric_val "$before" rejected_not_found_total); nf_after=$(metric_val "$after" rejected_not_found_total)
  tl_before=$(metric_val "$before" rejected_too_large_total); tl_after=$(metric_val "$after" rejected_too_large_total)
  inflight=$(metric_val "$after" inflight_current)

  delta() { echo $(( $2 - $1 )); }

  echo "[quota] /metrics deltas (this run):"
  printf "  requests_total           +%d (was %s → %s)\n"    "$(delta "$r_before" "$r_after")" "${r_before:-0}" "${r_after:-0}"
  printf "  rejected_overload_total  +%d (was %s → %s)\n"    "$(delta "$ol_before" "$ol_after")" "${ol_before:-0}" "${ol_after:-0}"
  printf "  rejected_not_found_total +%d (was %s → %s)\n"    "$(delta "$nf_before" "$nf_after")" "${nf_before:-0}" "${nf_after:-0}"
  printf "  rejected_too_large_total +%d (was %s → %s)\n"    "$(delta "$tl_before" "$tl_after")" "${tl_before:-0}" "${tl_after:-0}"
  printf "  bytes_out_total          +%d (was %s → %s)\n"    "$(delta "${bo_before:-0}" "${bo_after:-0}")" "${bo_before:-0}" "${bo_after:-0}"
  printf "  bytes_in_total           +%d (was %s → %s)\n"    "$(delta "${bi_before:-0}" "${bi_after:-0}")" "${bi_before:-0}" "${bi_after:-0}"
  printf "  inflight_current          %s\n"                   "${inflight:-0}"
}

require_tls
build_all
start_server_if_free
wait_ready

# Baseline metrics
m_before="$(mktemp)"
metrics_fetch "$m_before"

# Run the hammer
burst_parallel

# After metrics
m_after="$(mktemp)"
metrics_fetch "$m_after"

# Report deltas
metrics_report "$m_before" "$m_after"

echo "[quota] done"
