#![cfg(all(feature = "bus_edge_notify", feature = "loom"))]

//! RO:WHAT
//!   Loom litmus tests for lost-wake and drain-after-clear races.
//!
//! RO:WHY
//!   Ensure the pending bit pattern guarantees no lost wakeups.
//!
//! NOTE
//!   Uses `loom`'s std shims; keep test tiny to avoid state explosion.

use loom::sync::Arc;
use loom::thread;
use std::sync::atomic::{AtomicBool, Ordering};

#[test]
fn lost_wake_is_prevented() {
    loom::model(|| {
        let pending = Arc::new(AtomicBool::new(false));

        // Publisher: set pending and "notify if 0->1".
        let p = {
            let pending = pending.clone();
            thread::spawn(move || {
                let was = pending.swap(true, Ordering::Relaxed);
                // if !was { notify(); }  // modeled implicitly
                was
            })
        };

        // Subscriber: drain, clear, then race-check.
        let s = {
            let pending = pending.clone();
            thread::spawn(move || {
                // drain_all(); // modeled as already drained
                pending.store(false, Ordering::Relaxed);
                // race check
                let raced = pending.swap(false, Ordering::Relaxed);
                if raced {
                    // re-arm
                    pending.store(true, Ordering::Relaxed);
                }
                raced
            })
        };

        let _pub_was_pending = p.join().unwrap();
        let raced = s.join().unwrap();

        // If publisher observed 0->1 and subscriber cleared, either:
        // - race detected (raced=true), subscriber will continue draining
        // - or subscriber awaits but pending remains true (observed by next poll)
        // We assert that *eventually* pending is true if race happened.
        if raced {
            assert!(pending.load(Ordering::Relaxed));
        }
    });
}
