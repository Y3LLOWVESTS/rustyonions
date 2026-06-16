#!/usr/bin/env bash
# RO:WHAT — Bash-only verifier for QuickChain locked_hash payload vector framing.
# RO:WHY — ECON/GOV: locked hash vectors must keep reviewed payload/preimage/hash fields without Python tooling.
# RO:INTERACTS — tests/vectors/quickchain/hash_payloads and tests/quickchain_hash_payloads.rs.
# RO:INVARIANTS — verifies status, schema, framing, payload hex, preimage hex, and b3 shape; produces no roots.
# RO:METRICS — prints vector counts and byte totals only.
# RO:CONFIG — optional repository root as argv[1].
# RO:SECURITY — expected_b3 is vector evidence only; no wallet, receipt, settlement, checkpoint, or authority is created.
# RO:TEST — run directly with bash; paired with cargo test -p ron-proto --test quickchain_hash_payloads.

set -euo pipefail

repo_root="${1:-$(pwd)}"
vector_dir="${repo_root}/crates/ron-proto/tests/vectors/quickchain/hash_payloads"

expected_files=5
expected_status="locked_hash"
expected_schema="quickchain.test-vector.v1"
expected_encoding="quickchain.canonical-json.v1"
expected_framing="domain_separator_bytes || 0x00 || canonical_payload_bytes"
expected_algorithm="blake3-256"

fail() {
  printf 'quickchain hash payload verification failed: %s\n' "$*" >&2
  exit 1
}

extract_string_field() {
  file="$1"
  field="$2"

  sed -n "s/.*\"${field}\"[[:space:]]*:[[:space:]]*\"\([^\"]*\)\".*/\1/p" "$file" | head -n 1
}

require_string_field() {
  file="$1"
  rel="$2"
  field="$3"

  value="$(extract_string_field "$file" "$field")"
  [ -n "$value" ] || fail "${rel}: missing string field ${field}"
  printf '%s' "$value"
}

hex_of_ascii() {
  printf '%s' "$1" | od -An -tx1 | tr -d ' \n'
}

is_lower_hex_even() {
  value="$1"
  [ -n "$value" ] || return 1
  [ $(( ${#value} % 2 )) -eq 0 ] || return 1
  case "$value" in
    *[!0123456789abcdef]*) return 1 ;;
    *) return 0 ;;
  esac
}

require_non_placeholder_b3() {
  rel="$1"
  value="$2"

  case "$value" in
    b3:????????????????????????????????????????????????????????????????) ;;
    *) fail "${rel}: expected_b3 must be b3:<64 lowercase hex>" ;;
  esac

  hex="${value#b3:}"
  case "$hex" in
    *[!0123456789abcdef]*) fail "${rel}: expected_b3 must use lowercase hex" ;;
  esac

  for nibble in 0 1 2 3 4 5 6 7 8 9 a b c d e f; do
    repeated="$(printf '%064s' '' | tr ' ' "$nibble")"
    [ "$hex" != "$repeated" ] || fail "${rel}: expected_b3 must not be a repeated-nibble placeholder"
  done
}

[ -d "$vector_dir" ] || fail "missing vector directory: ${vector_dir}"

file_count="$(find "$vector_dir" -type f -name '*.json' | wc -l | tr -d '[:space:]')"
[ "$file_count" = "$expected_files" ] || fail "expected ${expected_files} hash payload vectors, got ${file_count}"

payload_bytes_total=0
preimage_bytes_total=0
optional_hash_checks=0

while IFS= read -r file; do
  rel="${file#${repo_root}/}"

  case "$rel" in
    *_locked_hash_v1.json) ;;
    *) fail "${rel}: locked hash vector filename must end with _locked_hash_v1.json" ;;
  esac

  schema="$(require_string_field "$file" "$rel" "schema")"
  status="$(require_string_field "$file" "$rel" "status")"
  encoding="$(require_string_field "$file" "$rel" "canonical_encoding")"
  framing="$(require_string_field "$file" "$rel" "preimage_framing")"
  algorithm="$(require_string_field "$file" "$rel" "hash_algorithm")"
  domain="$(require_string_field "$file" "$rel" "domain_separator")"
  payload_hex="$(require_string_field "$file" "$rel" "canonical_payload_hex")"
  preimage_hex="$(require_string_field "$file" "$rel" "preimage_hex")"
  expected_b3="$(require_string_field "$file" "$rel" "expected_b3")"

  [ "$schema" = "$expected_schema" ] || fail "${rel}: schema mismatch: ${schema}"
  [ "$status" = "$expected_status" ] || fail "${rel}: status mismatch: ${status}"
  [ "$encoding" = "$expected_encoding" ] || fail "${rel}: canonical_encoding mismatch: ${encoding}"
  [ "$framing" = "$expected_framing" ] || fail "${rel}: preimage_framing mismatch"
  [ "$algorithm" = "$expected_algorithm" ] || fail "${rel}: hash_algorithm mismatch: ${algorithm}"

  is_lower_hex_even "$payload_hex" || fail "${rel}: canonical_payload_hex must be non-empty even lowercase hex"
  is_lower_hex_even "$preimage_hex" || fail "${rel}: preimage_hex must be non-empty even lowercase hex"

  domain_hex="$(hex_of_ascii "$domain")"
  expected_preimage_hex="${domain_hex}00${payload_hex}"

  [ "$preimage_hex" = "$expected_preimage_hex" ] \
    || fail "${rel}: preimage_hex must equal domain_separator_bytes || 0x00 || canonical_payload_bytes"

  require_non_placeholder_b3 "$rel" "$expected_b3"

  payload_bytes=$(( ${#payload_hex} / 2 ))
  preimage_bytes=$(( ${#preimage_hex} / 2 ))
  payload_bytes_total=$(( payload_bytes_total + payload_bytes ))
  preimage_bytes_total=$(( preimage_bytes_total + preimage_bytes ))

  if command -v b3sum >/dev/null 2>&1 && command -v xxd >/dev/null 2>&1; then
    actual_b3="b3:$(printf '%s' "$preimage_hex" | xxd -r -p | b3sum | awk '{print $1}')"
    [ "$actual_b3" = "$expected_b3" ] || fail "${rel}: expected_b3 does not match b3sum over preimage"
    optional_hash_checks=$(( optional_hash_checks + 1 ))
  fi
done < <(find "$vector_dir" -type f -name '*.json' | sort)

[ "$payload_bytes_total" -gt 3000 ] || fail "payload byte total unexpectedly small: ${payload_bytes_total}"
[ "$preimage_bytes_total" -gt 3000 ] || fail "preimage byte total unexpectedly small: ${preimage_bytes_total}"

printf 'verified quickchain hash payload vectors with bash: files=%s payload_bytes=%s preimage_bytes=%s optional_b3sum_checks=%s\n' \
  "$file_count" "$payload_bytes_total" "$preimage_bytes_total" "$optional_hash_checks"
