#!/usr/bin/env bash
# Unified drift clamp & hygiene. Safe even if some tools aren't installed.
# Usage:
#   scripts/check.sh          # full run (best effort)
#   scripts/check.sh --fast   # skip heavy steps (udeps, audit)

set -euo pipefail

FAST=0
if [[ "${1:-}" == "--fast" ]]; then
  FAST=1
fi

have() { command -v "$1" >/dev/null 2>&1; }
say() { echo "[check] $*"; }

say "fmt"
cargo fmt --all

# --- Hakari: init (one-time), generate, verify deps ---------------------------
if have cargo-hakari; then
  # One-time: ensure a workspace-hack crate exists (named 'workspace-hack')
  if [[ ! -f "workspace-hack/Cargo.toml" ]]; then
    say "hakari init (creating workspace-hack crate)"
    cargo hakari init workspace-hack
  fi

  say "hakari generate (--diff to ensure up-to-date)"
  cargo hakari generate --diff

  say "hakari manage-deps (--dry-run to ensure all crates depend on workspace-hack)"
  cargo hakari manage-deps --dry-run

  # Optional: detect feature-split bugs
  say "hakari verify"
  cargo hakari verify || {
    echo "[i] hakari verify found multi-feature builds; investigate above."
  }
else
  echo "[i] cargo-hakari not installed; skipping (ok)."
fi
# Docs for these commands: generate / manage-deps in the official guide.  # See docs.rs usage. :contentReference[oaicite:1]{index=1}

# --- cargo-deny ---------------------------------------------------------------
say "cargo-deny (licenses/bans/advisories/sources)"
if have cargo-deny; then
  cargo deny check
else
  echo "[i] cargo-deny not installed; skipping (ok)."
fi

# --- clippy -------------------------------------------------------------------
say "clippy (deny warnings)"
cargo clippy --workspace --all-targets -- -D warnings

# --- heavy steps --------------------------------------------------------------
if [[ $FAST -eq 0 ]]; then
  say "udeps (unused dependencies)"
  if have cargo-udeps; then
    cargo udeps --workspace || {
      echo "[i] udeps found issues; review above. Continuing."
    }
  else
    echo "[i] cargo-udeps not installed; skipping (ok)."
  fi

  say "audit (vuln advisories)"
  if have cargo-audit; then
    cargo audit || {
      echo "[i] audit flagged advisories; review above. Continuing."
    }
  else
    echo "[i] cargo-audit not installed; skipping (ok)."
  fi
else
  echo "[i] --fast: skipping udeps and audit."
fi

# --- duplicates report --------------------------------------------------------
say "duplicates (cargo tree -d)"
cargo tree -d || true

echo "[ok] checks complete"
