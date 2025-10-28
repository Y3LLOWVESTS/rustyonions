use ron_naming::normalize::normalize_fqdn_ascii;

#[test]
fn idempotent_ascii() {
    let a = normalize_fqdn_ascii("EXAMPLE.COM").unwrap();
    let b = normalize_fqdn_ascii(&a.0 .0).unwrap();
    assert_eq!(a, b);
}

#[test]
fn unicode_maps_to_ascii() {
    let a = normalize_fqdn_ascii("café.example").unwrap();
    assert_eq!(a.0 .0, "xn--caf-dma.example");
}
