// Common KMS types (scaffold)
#[allow(dead_code)]
pub struct KeyId(pub String);

#[allow(dead_code)]
pub enum Alg {
    Ed25519,
    X25519,
    MlKem,
    MlDsa,
    SlhDsa,
}
