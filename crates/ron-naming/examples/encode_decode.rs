use ron_naming::{
    types::{ContentId, Fqdn, NameRecord},
    version::parse_version,
    wire,
};

fn main() {
    let rec = NameRecord {
        name: Fqdn("files.example".into()),
        version: Some(parse_version("1.0.0").unwrap()),
        content: ContentId(
            "b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".into(),
        ),
    };
    let jb = wire::json::to_json_bytes(&rec).unwrap();
    let round: NameRecord = wire::json::from_json_bytes(&jb).unwrap();
    assert_eq!(rec, round);
    println!("{}", String::from_utf8(jb).unwrap());
}
