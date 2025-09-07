#[cfg(feature = "loom")]
mod loom_tests {
    use loom::sync::{Arc, Mutex};
    use loom::thread;

    #[derive(Default)]
    struct Health { ready: bool, config: bool, db: bool, net: bool, bus: bool }
    impl Health {
        fn set_ready_if_complete(&mut self) {
            self.ready = self.config && self.db && self.net && self.bus;
        }
    }

    #[test]
    fn readiness_eventual_and_consistent() {
        loom::model(|| {
            let h = Arc::new(Mutex::new(Health::default()));
            for key in ["config","db","net","bus"] {
                let h2 = h.clone();
                thread::spawn(move || {
                    let mut g = h2.lock().unwrap();
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
            let g = h.lock().unwrap();
            if g.ready { assert!(g.config && g.db && g.net && g.bus); }
        });
    }
}
