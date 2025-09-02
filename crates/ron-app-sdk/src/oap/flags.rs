#![forbid(unsafe_code)]

bitflags::bitflags! {
    /// OAP/1 flags (subset needed for Bronze).
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct OapFlags: u16 {
        const REQ      = 1 << 0;
        const RESP     = 1 << 1;
        const EVENT    = 1 << 2;
        const START    = 1 << 3;
        const END      = 1 << 4;
        const ACK_REQ  = 1 << 5;
        const COMP     = 1 << 6;
        const APP_E2E  = 1 << 7;
    }
}
