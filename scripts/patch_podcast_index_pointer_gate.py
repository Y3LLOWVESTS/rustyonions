#!/usr/bin/env python3
# RO:WHAT — Fix podcast asset manifest pointer acceptance and fail-closed podcast upload.
# RO:WHY — .podcast minting can return a crab URL while svc-index rejects the podcast pointer, causing content_view asset_pointer_not_found.
# RO:INVARIANTS — no fake unlock; no silent spend; content_view still requires backend manifest pointer + receipt.
# RO:TEST — cargo fmt -p svc-index -p omnigate; cargo test -p svc-index --test http_contract; cargo check -p svc-index --all-targets; cargo check -p omnigate --all-targets.

from pathlib import Path

ROOT = Path.cwd()


def load(path: Path) -> str:
    if not path.exists():
        raise SystemExit(f"missing file: {path}")
    return path.read_text()


def save(path: Path, text: str) -> None:
    path.write_text(text)


def replace_once(text: str, old: str, new: str, label: str) -> str:
    if new in text:
        print(f"skip {label}: already patched")
        return text

    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected 1 match, found {count}")

    print(f"patch {label}")
    return text.replace(old, new, 1)


def insert_before_once(text: str, marker: str, insertion: str, label: str) -> str:
    if insertion.strip() in text:
        print(f"skip {label}: already patched")
        return text

    count = text.count(marker)
    if count != 1:
        raise SystemExit(f"{label}: expected 1 marker, found {count}")

    print(f"patch {label}")
    return text.replace(marker, insertion + marker, 1)


# ---------------------------------------------------------------------------
# 1) svc-index must accept podcast as a pointer asset_kind.
# ---------------------------------------------------------------------------

types_path = ROOT / "crates/svc-index/src/types.rs"
types = load(types_path)

types = replace_once(
    types,
    '''            | "music"
            | "song"
            | "article"''',
    '''            | "music"
            | "song"
            | "podcast"
            | "article"''',
    "svc-index normalize_asset_kind accepts podcast",
)

save(types_path, types)


# ---------------------------------------------------------------------------
# 2) svc-index HTTP contract test should lock podcast acceptance.
# ---------------------------------------------------------------------------

http_contract_path = ROOT / "crates/svc-index/tests/http_contract.rs"
http_contract = load(http_contract_path)

http_contract = replace_once(
    http_contract,
    '''    assert_eq!(normalize_asset_kind("music").unwrap(), "music");
    assert_eq!(normalize_asset_kind("post").unwrap(), "post");''',
    '''    assert_eq!(normalize_asset_kind("music").unwrap(), "music");
    assert_eq!(normalize_asset_kind("podcast").unwrap(), "podcast");
    assert_eq!(normalize_asset_kind("post").unwrap(), "post");''',
    "svc-index http_contract podcast validator assertion",
)

save(http_contract_path, http_contract)


# ---------------------------------------------------------------------------
# 3) omnigate content_view target parser must accept .podcast.
# ---------------------------------------------------------------------------

content_view_path = ROOT / "crates/omnigate/src/routes/v1/content_view.rs"
content_view = load(content_view_path)

content_view = replace_once(
    content_view,
    '''        "image" | "article" | "post" | "comment" | "video" | "stream" | "music" | "song"''',
    '''        "image" | "article" | "post" | "comment" | "video" | "stream" | "music" | "song" | "podcast"''',
    "omnigate content_view accepts podcast kind",
)

save(content_view_path, content_view)


# ---------------------------------------------------------------------------
# 4) podcast upload should fail closed if manifest or index pointer storage fails.
#    This prevents future “minted but cannot pay/listen” assets.
# ---------------------------------------------------------------------------

assets_path = ROOT / "crates/omnigate/src/routes/v1/assets.rs"
assets = load(assets_path)

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

save(assets_path, assets)

print("patched podcast index pointer gate")
