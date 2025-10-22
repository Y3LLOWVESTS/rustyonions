//! Two-phase backoff helper for publisher slow paths (bench/SoA use).
//! Default is ultra-light: short spin (16) then yield. Tunable via env:
//!   RON_PUB_SPIN   => u32 spins (max 256; default 16)
//!   RON_PUB_YIELD  => "0" disables yield, anything else enables (default ON)

use std::cell::Cell;
use std::sync::OnceLock;
use std::thread;

#[derive(Copy, Clone)]
pub struct TwoPhaseBackoff {
    spins_left: Cell<u32>,
    yield_enabled: bool,
}

impl TwoPhaseBackoff {
    #[inline]
    pub fn new() -> Self {
        static SPINS: OnceLock<u32> = OnceLock::new();
        static YIELD: OnceLock<bool> = OnceLock::new();

        let spins = *SPINS.get_or_init(|| {
            std::env::var("RON_PUB_SPIN")
                .ok()
                .and_then(|v| v.parse::<u32>().ok())
                .filter(|&n| n <= 256)
                .unwrap_or(16)
        });
        let yield_enabled = *YIELD.get_or_init(|| {
            std::env::var("RON_PUB_YIELD").map(|v| v != "0").unwrap_or(true)
        });

        Self { spins_left: Cell::new(spins), yield_enabled }
    }

    /// Call when publish cannot immediately progress (slot full/lagged).
    #[inline]
    pub fn tick(&self) {
        let left = self.spins_left.get();
        if left > 0 {
            std::hint::spin_loop();
            self.spins_left.set(left - 1);
        } else if self.yield_enabled {
            thread::yield_now();
        } else {
            std::hint::spin_loop();
        }
    }

    #[inline]
    pub fn reset(&self) {
        self.spins_left.set(16);
    }
}
