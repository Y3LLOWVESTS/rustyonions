use ron_proto::{manifest::EntryRef, ContentId};
use serde_json as json;

#[test]
fn entryref_kind_defaults_to_blob() {
    let cid: ContentId = "b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
        .parse()
        .unwrap();

    // kind omitted on purpose (should default to "blob")
    let value = json::json!({
        "id": cid.to_string(),
        "size": 1234
    });

    let parsed: EntryRef = json::from_value(value).unwrap();

    // Serialize back and ensure "blob" is present
    let round = json::to_value(&parsed).unwrap();
    assert_eq!(round.get("kind").unwrap().as_str().unwrap(), "blob");
}
