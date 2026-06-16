#!/usr/bin/env bash
# RO:WHAT — Bash-only inventory verifier for checked-in QuickChain vector files.
# RO:WHY — ECON/GOV: vector lifecycle counts must be reviewable without adding another scripting runtime.
# RO:INTERACTS — crates/ron-proto/tests/vectors/quickchain and quickchain_vector_inventory.rs.
# RO:INVARIANTS — counts vector statuses, filename lifecycle suffixes, schema family, locked-hash fields, and no fake repeated b3 hash claims.
# RO:METRICS — prints inventory counts only.
# RO:CONFIG — optional repository root as argv[1].
# RO:SECURITY — produces no roots, checkpoints, receipts, signatures, settlement claims, validators, anchors, bridges, or authority.
# RO:TEST — cargo test -p ron-proto --test quickchain_tooling_boundary; run this script directly.

set -euo pipefail

repo_root="${1:-$(pwd)}"
vector_dir="${repo_root}/crates/ron-proto/tests/vectors/quickchain"

expected_files=36
expected_locked_bytes=31
expected_locked_hash=5
expected_sketch=0
expected_typed_vectors=31

fail() {
  printf 'quickchain vector inventory failed: %s\n' "$*" >&2
  exit 1
}

require_count() {
  label="$1"
  actual="$2"
  expected="$3"

  [ "$actual" = "$expected" ] || fail "${label}: expected ${expected}, got ${actual}"
}

count_files() {
  find "$vector_dir" -type f -name '*.json' | wc -l | tr -d '[:space:]'
}

count_field_value() {
  field="$1"
  value="$2"

  awk -v field="\"${field}\"" -v value="\"${value}\"" '
    index($0, field) && index($0, value) { count += 1 }
    END { print count + 0 }
  ' $(find "$vector_dir" -type f -name '*.json' | sort)
}

extract_string_field() {
  file="$1"
  field="$2"

  sed -n "s/.*\"${field}\"[[:space:]]*:[[:space:]]*\"\([^\"]*\)\".*/\1/p" "$file" | head -n 1
}

require_filename_suffixes() {
  while IFS= read -r file; do
    rel="${file#${vector_dir}/}"

    case "$rel" in
      *_sketch_v1.json)
        grep -q '"status"[[:space:]]*:[[:space:]]*"sketch"' "$file" \
          || fail "${rel}: sketch filename without sketch status"
        ;;
      *_locked_bytes_v1.json)
        grep -q '"status"[[:space:]]*:[[:space:]]*"locked_bytes"' "$file" \
          || fail "${rel}: locked_bytes filename without locked_bytes status"
        ;;
      *_locked_hash_v1.json)
        grep -q '"status"[[:space:]]*:[[:space:]]*"locked_hash"' "$file" \
          || fail "${rel}: locked_hash filename without locked_hash status"
        ;;
      *)
        fail "${rel}: filename must end with _sketch_v1.json, _locked_bytes_v1.json, or _locked_hash_v1.json"
        ;;
    esac
  done < <(find "$vector_dir" -type f -name '*.json' | sort)
}

require_schema_family() {
  while IFS= read -r file; do
    rel="${file#${vector_dir}/}"
    schema="$(extract_string_field "$file" "schema")"

    case "$schema" in
      quickchain.test-vector.v1)
        ;;
      quickchain.hold-compaction-vector.v1)
        ;;
      quickchain.concurrent-hold-replay-vector.v1)
        ;;
      quickchain.receipt-sort-key-vector-set.v1)
        ;;
      quickchain.replay-scenario-vector-set.v1)
        ;;
      quickchain.sort-key-vector-set.v1)
        ;;
      *)
        fail "${rel}: unexpected schema ${schema:-<missing>}"
        ;;
    esac
  done < <(find "$vector_dir" -type f -name '*.json' | sort)
}

require_locked_hash_fields() {
  while IFS= read -r file; do
    rel="${file#${vector_dir}/}"

    case "$rel" in
      *_locked_hash_v1.json)
        grep -q '"canonical_payload_hex"[[:space:]]*:[[:space:]]*"' "$file" \
          || fail "${rel}: locked_hash vector missing canonical_payload_hex"
        grep -q '"preimage_hex"[[:space:]]*:[[:space:]]*"' "$file" \
          || fail "${rel}: locked_hash vector missing preimage_hex"
        grep -q '"expected_b3"[[:space:]]*:[[:space:]]*"b3:[0-9a-f]\{64\}"' "$file" \
          || fail "${rel}: locked_hash vector missing expected_b3 b3:<64 lowercase hex>"
        ;;
    esac
  done < <(find "$vector_dir" -type f -name '*.json' | sort)
}

require_no_placeholder_hashes() {
  while IFS= read -r file; do
    rel="${file#${vector_dir}/}"

    expected_b3="$(extract_string_field "$file" "expected_b3")"
    [ -n "$expected_b3" ] || continue

    hash="${expected_b3#b3:}"
    [ "$hash" != "$expected_b3" ] || fail "${rel}: expected_b3 must start with b3:"

    for nibble in 0 1 2 3 4 5 6 7 8 9 a b c d e f; do
      repeated="$(printf '%064s' '' | tr ' ' "$nibble")"
      [ "$hash" != "$repeated" ] || fail "${rel}: expected_b3 must not be a repeated-nibble placeholder"
    done
  done < <(find "$vector_dir" -type f -name '*.json' | sort)
}

[ -d "$vector_dir" ] || fail "missing vector directory: ${vector_dir}"

file_count="$(count_files)"
locked_bytes_count="$(count_field_value "status" "locked_bytes")"
locked_hash_count="$(count_field_value "status" "locked_hash")"
sketch_count="$(count_field_value "status" "sketch")"
typed_vector_count="$(count_field_value "schema" "quickchain.test-vector.v1")"

require_count "vector file count" "$file_count" "$expected_files"
require_count "locked_bytes count" "$locked_bytes_count" "$expected_locked_bytes"
require_count "locked_hash count" "$locked_hash_count" "$expected_locked_hash"
require_count "sketch count" "$sketch_count" "$expected_sketch"
require_count "quickchain.test-vector.v1 count" "$typed_vector_count" "$expected_typed_vectors"

require_filename_suffixes
require_schema_family
require_locked_hash_fields
require_no_placeholder_hashes

printf 'verified quickchain vector inventory: files=%s locked_bytes=%s locked_hash=%s sketch=%s typed_vectors=%s\n' \
  "$file_count" "$locked_bytes_count" "$locked_hash_count" "$sketch_count" "$typed_vector_count"
