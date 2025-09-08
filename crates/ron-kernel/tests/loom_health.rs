// FILE: crates/ron-kernel/tests/loom_health.rs
#![forbid(unsafe_code)]

#[cfg(feature = "loom")]
mod loom_tests {
    use loom::sync::{Arc, Mutex};
    use loom::thread;

    #[derive(Default)]
    struct Health {
        ready: bool,
        config: bool,
        db: bool,
        net: bool,
        bus: bool,
    }

    impl Health {
        fn set_ready_if_complete(&mut self) {
            self.ready = self.config && self.db && self.net && self.bus;
        }
    }

    /// Acquire a lock without `unwrap()`/`expect()`. If the mutex is poisoned,
    /// recover the inner guard so the model can continue exploring interleavings.
    fn lock_no_panic<T>(m: &Mutex<T>) -> loom::sync::MutexGuard<'_, T> {
        match m.lock() {
            Ok(g) => g,
            Err(poison) => poison.into_inner(),
        }
    }

    #[test]
    fn readiness_eventual_and_consistent() {
        loom::model(|| {
            let h = Arc::new(Mutex::new(Health::default()));

            for key in ["config", "db", "net", "bus"] {
                let h2 = Arc::clone(&h);
                thread::spawn(move || {
                    let mut g = lock_no_panic(&h2);
                    match key {
                        "config" => g.config = true,
                        "db" => g.db = true,
                        "net" => g.net = true,
                        "bus" => g.bus = true,
                        _ => {}
                    }
                    g.set_ready_if_complete();
                });
            }

            let g = lock_no_panic(&h);
            if g.ready {
                assert!(g.config && g.db && g.net && g.bus);
            }
        });
    }
}
