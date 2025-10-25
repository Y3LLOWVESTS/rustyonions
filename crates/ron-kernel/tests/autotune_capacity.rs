#![cfg(feature = "bus_autotune_cap")]
use ron_kernel::autotune_capacity;

#[test]
fn mapping_basic_thresholds() {
    assert_eq!(autotune_capacity(0, None), 64);
    assert_eq!(autotune_capacity(1, None), 64);
    assert_eq!(autotune_capacity(4, None), 64);
    assert_eq!(autotune_capacity(5, None), 128);
    assert_eq!(autotune_capacity(16, None), 128);
    assert_eq!(autotune_capacity(17, None), 256);
    assert_eq!(autotune_capacity(64, None), 256);
}

#[test]
fn override_is_respected() {
    assert_eq!(autotune_capacity(1, Some(128)), 128);
    assert_eq!(autotune_capacity(32, Some(64)), 64);
}

#[test]
fn monotone_in_n_with_default_map() {
    let mut prev = 0;
    for n in 0..200 {
        let cap = autotune_capacity(n, None);
        assert!(cap >= prev);
        prev = cap;
    }
}
