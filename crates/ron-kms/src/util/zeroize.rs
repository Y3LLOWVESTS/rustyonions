// RO:WHAT Helpers to wipe secrets explicitly when using Vec/Box.
use zeroize::Zeroize;

pub fn wipe_vec(v: &mut Vec<u8>) {
    v.zeroize();
}
