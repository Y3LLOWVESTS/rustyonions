//! RO:WHAT — Bitflags for OAP/1 `flags,u16` header field.
//! RO:WHY — Explicit bit meanings for interop; avoids magic numbers.
//! RO:INTERACTS — Used by header/frame/codec; callers set semantics via these bits.
//! RO:INVARIANTS — Bits stable under semver; RESERVED kept for forward-compat.

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct Flags: u16 {
        const REQ     = 1 << 0;
        const RESP    = 1 << 1;
        const EVENT   = 1 << 2;
        const START   = 1 << 3;
        const END     = 1 << 4;
        const ACK_REQ = 1 << 5;
        const COMP    = 1 << 6; // payload compressed (zstd) with bounded inflate
        const APP_E2E = 1 << 7; // opaque app-layer E2E crypto; platform does not inspect
        // reserve upper bits for future use
    }
}
