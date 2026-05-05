//! RO:WHAT — Tests for omnigate WEB3_2 built-in crab page resolver metadata.
//! RO:WHY — Batch 2 keeps reserved product pages visible without enabling unsafe mutation paths.
//! RO:INVARIANTS — built-in pages are read-only metadata; mutating actions require explicit confirmation.

use omnigate::routes::v1::crab::{builtin_page_response, parse_builtin_page_url, BuiltinPageKind};

#[test]
fn parses_reserved_builtin_page_urls() {
    assert_eq!(
        parse_builtin_page_url("crab://site"),
        Some(BuiltinPageKind::Site)
    );
    assert_eq!(
        parse_builtin_page_url("crab://image"),
        Some(BuiltinPageKind::Image)
    );
    assert_eq!(
        parse_builtin_page_url("crab://music"),
        Some(BuiltinPageKind::Music)
    );
    assert_eq!(
        parse_builtin_page_url("crab://article"),
        Some(BuiltinPageKind::Article)
    );
}

#[test]
fn rejects_noncanonical_builtin_page_forms() {
    assert_eq!(parse_builtin_page_url("crab://Site"), None);
    assert_eq!(parse_builtin_page_url("crab://site/"), None);
    assert_eq!(parse_builtin_page_url("crab://site?x=1"), None);
    assert_eq!(parse_builtin_page_url("crab://image.extra"), None);
    assert_eq!(parse_builtin_page_url("crab://dashboard"), None);
    assert_eq!(parse_builtin_page_url("crab://song"), None);
}

#[test]
fn site_builtin_page_contract_points_at_site_routes() {
    let page = builtin_page_response(BuiltinPageKind::Site);

    assert_eq!(page.schema, "omnigate.builtin-page.v1");
    assert_eq!(page.url, "crab://site");
    assert_eq!(page.page_kind, "site");
    assert_eq!(page.status, "active");
    assert!(page.requires_passport);
    assert!(page.requires_wallet);

    let prepare = page
        .actions
        .iter()
        .find(|action| action.id == "site.prepare")
        .expect("site prepare action");
    assert_eq!(prepare.method, "POST");
    assert_eq!(prepare.route, "/sites/prepare");
    assert!(!prepare.mutates);
    assert!(!prepare.requires_confirmation);

    let create = page
        .actions
        .iter()
        .find(|action| action.id == "site.create")
        .expect("site create action");
    assert_eq!(create.method, "POST");
    assert_eq!(create.route, "/sites");
    assert!(create.mutates);
    assert!(create.requires_confirmation);

    assert!(page.fields.iter().any(|field| {
        field.name == "site_name" && field.field_type == "text" && field.required
    }));
}

#[test]
fn image_builtin_page_contract_points_at_image_routes() {
    let page = builtin_page_response(BuiltinPageKind::Image);

    assert_eq!(page.schema, "omnigate.builtin-page.v1");
    assert_eq!(page.url, "crab://image");
    assert_eq!(page.page_kind, "image");
    assert_eq!(page.status, "active");
    assert!(page.requires_passport);
    assert!(page.requires_wallet);

    let prepare = page
        .actions
        .iter()
        .find(|action| action.id == "image.prepare")
        .expect("image prepare action");
    assert_eq!(prepare.method, "POST");
    assert_eq!(prepare.route, "/assets/image/prepare");
    assert!(!prepare.mutates);
    assert!(!prepare.requires_confirmation);

    let create = page
        .actions
        .iter()
        .find(|action| action.id == "image.create")
        .expect("image create action");
    assert_eq!(create.method, "POST");
    assert_eq!(create.route, "/assets/image");
    assert!(create.mutates);
    assert!(create.requires_confirmation);

    let file = page
        .fields
        .iter()
        .find(|field| field.name == "file")
        .expect("image file field");
    assert_eq!(file.field_type, "file");
    assert_eq!(file.accept.as_deref(), Some("image/*"));
    assert!(file.required);
}

#[test]
fn music_builtin_page_is_reserved_but_not_mutating_yet() {
    let page = builtin_page_response(BuiltinPageKind::Music);

    assert_eq!(page.schema, "omnigate.builtin-page.v1");
    assert_eq!(page.url, "crab://music");
    assert_eq!(page.page_kind, "music");
    assert_eq!(page.status, "coming_soon");
    assert!(!page.requires_passport);
    assert!(!page.requires_wallet);
    assert!(page.actions.is_empty());
    assert!(page
        .warnings
        .iter()
        .any(|warning| warning == "music_page_coming_soon"));
}

#[test]
fn article_builtin_page_is_reserved_but_not_mutating_yet() {
    let page = builtin_page_response(BuiltinPageKind::Article);

    assert_eq!(page.schema, "omnigate.builtin-page.v1");
    assert_eq!(page.url, "crab://article");
    assert_eq!(page.page_kind, "article");
    assert_eq!(page.status, "coming_soon");
    assert!(!page.requires_passport);
    assert!(!page.requires_wallet);
    assert!(page.actions.is_empty());
    assert!(page
        .warnings
        .iter()
        .any(|warning| warning == "article_page_coming_soon"));
}
