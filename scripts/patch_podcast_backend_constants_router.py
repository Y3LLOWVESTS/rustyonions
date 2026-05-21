#!/usr/bin/env python3
# RO:WHAT — Repair podcast-lite backend patch after partial script abort.
# RO:WHY — Podcast handlers compile but PODCAST_* schema constants were not saved; router may also not have been saved.
# RO:INVARIANTS — gateway-first; no wallet/ledger mutation from omnigate; podcast-lite only.
# RO:TEST — cargo fmt -p omnigate; cargo check -p omnigate --all-targets.

from pathlib import Path

ROOT = Path.cwd()
ASSETS = ROOT / "crates/omnigate/src/routes/v1/assets.rs"


def load(path: Path) -> str:
    if not path.exists():
        raise SystemExit(f"missing file: {path}")
    return path.read_text()


def save(path: Path, text: str) -> None:
    path.write_text(text)


def insert_after_if_missing(text: str, marker: str, insertion: str, already: str, label: str) -> str:
    if already in text:
        print(f"skip {label}: already present")
        return text

    count = text.count(marker)
    if count != 1:
        raise SystemExit(f"{label}: expected 1 marker, found {count}")

    print(f"patch {label}")
    return text.replace(marker, marker + insertion, 1)


assets = load(ASSETS)

assets = insert_after_if_missing(
    assets,
    'const MUSIC_UPLOAD_SCHEMA: &str = "omnigate.music-asset-upload.v1";\n',
    'const PODCAST_PREPARE_SCHEMA: &str = "omnigate.podcast-asset-prepare.v1";\n'
    'const PODCAST_UPLOAD_SCHEMA: &str = "omnigate.podcast-asset-upload.v1";\n',
    'const PODCAST_PREPARE_SCHEMA: &str = "omnigate.podcast-asset-prepare.v1";',
    "omnigate podcast schema constants",
)

assets = insert_after_if_missing(
    assets,
    '        .route("/music", post(music_upload))\n',
    '        .route("/podcast/prepare", post(podcast_prepare))\n'
    '        .route("/podcast", post(podcast_upload))\n',
    '        .route("/podcast/prepare", post(podcast_prepare))',
    "omnigate podcast router routes",
)

save(ASSETS, assets)

print("patched omnigate podcast constants/router")
