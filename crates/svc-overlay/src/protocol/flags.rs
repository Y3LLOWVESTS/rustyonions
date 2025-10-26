//! RO:WHAT — OAP/1 capability flags used during handshake.
//! RO:WHY  — Keep overlay features discoverable and future-proof.

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Caps: u32 {
        const NONE        = 0;
        const GOSSIP_V1   = 1 << 0;
        const RESERVED_1  = 1 << 1;
        // Future: const PQ_HYBRID = 1 << 8;  // negotiated at overlay level; transport does TLS/PQ.
    }
}

impl Default for Caps {
    fn default() -> Self {
        Caps::GOSSIP_V1
    }
}
