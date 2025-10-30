//! Helper to load test vectors.

use ron_policy::{load_json, PolicyBundle};

pub fn load_vector(name: &str) -> PolicyBundle {
    let path = format!("tests/vectors/{}", name);
    let bytes = std::fs::read(path).expect("read vector");
    load_json(&bytes).expect("parse bundle")
}
