#!/usr/bin/env python3
# RO:WHAT — Add fail-closed guard to omnigate podcast upload after svc-index podcast kind fix.
# RO:WHY — Prevents future .podcast mints from returning a crab URL when manifest/index pointer writes fail.
# RO:INVARIANTS — no fake unlock; no fake payable asset; content_view still requires backend pointer + receipt.
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


def insert_before_once(text: str, marker: str, insertion: str, label: str) -> str:
    if insertion.strip() in text:
        print(f"skip {label}: already patched")
        return text

    count = text.count(marker)
    if count != 1:
        raise SystemExit(f"{label}: expected 1 marker, found {count}")

    print(f"patch {label}")
    return text.replace(marker, insertion + marker, 1)


assets = load(ASSETS)

fail_closed = r'''
    if manifest_write.status != "stored" || manifest_write.manifest_cid.is_none() {
        return problem(
            StatusCode::BAD_GATEWAY,
            "podcast_manifest_write_failed",
            "podcast upload stored audio bytes but failed to store the required manifest object",
            true,
            "podcast_manifest_write_failed",
        );
    }

    if index_pointer.status != "stored" {
        return problem(
            StatusCode::BAD_GATEWAY,
            "podcast_index_pointer_write_failed",
            "podcast upload stored audio bytes but failed to write the required asset manifest pointer",
            true,
            "podcast_index_pointer_write_failed",
        );
    }

'''

assets = insert_before_once(
    assets,
    '    let crab_url = format!("crab://{raw_hash}.podcast");',
    fail_closed,
    "omnigate podcast upload fail-closed pointer gate",
)

save(ASSETS, assets)

print("patched podcast upload fail-closed pointer gate")
