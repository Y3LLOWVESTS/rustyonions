// Conformance against canonical vectors (scaffold placeholder).
// When implemented, load tests/vectors/oap1/* and assert byte-identical decode/encode.
#[test]
fn conformance_vectors_exist() {
    assert!(std::path::Path::new("tests/vectors/oap1").exists());
}
