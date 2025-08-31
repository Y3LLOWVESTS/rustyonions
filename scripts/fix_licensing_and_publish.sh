#!/usr/bin/env bash
set -euo pipefail

# Add/normalize:
#   license = "MIT OR Apache-2.0"
#   publish = false
# for all manifests under crates/, tools/, experiments/

ensure_string() { # file key value
  local f="$1" key="$2" val="$3"
  if grep -Eq "^[[:space:]]*$key[[:space:]]*=" "$f"; then
    sed -i '' -E "s|^([[:space:]]*$key[[:space:]]*=[[:space:]]*).*$|\1\"$val\"|" "$f"
  else
    awk -v k="$key" -v v="$val" '
      BEGIN{done=0}
      {print}
      /^\[package\][[:space:]]*$/ && done==0 { print k " = \"" v "\""; done=1 }
    ' "$f" > "$f.tmp" && mv "$f.tmp" "$f"
  fi
}

ensure_bool() { # file key true|false
  local f="$1" key="$2" val="$3"
  if grep -Eq "^[[:space:]]*$key[[:space:]]*=" "$f"; then
    sed -i '' -E "s|^([[:space:]]*$key[[:space:]]*=[[:space:]]*).*$|\1$val|" "$f"
  else
    awk -v k="$key" -v v="$val" '
      BEGIN{done=0}
      {print}
      /^\[package\][[:space:]]*$/ && done==0 { print k " = " v; done=1 }
    ' "$f" > "$f.tmp" && mv "$f.tmp" "$f"
  fi
}

patch_one() {
  local f="$1"
  ensure_string "$f" license "MIT OR Apache-2.0"
  ensure_bool   "$f" publish false
  echo "patched: $f"
}

export -f ensure_string ensure_bool patch_one

find crates tools experiments -type f -name Cargo.toml -print0 \
  | xargs -0 -I{} bash -c 'patch_one "$@"' _ {}
