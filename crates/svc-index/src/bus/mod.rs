//! RO:WHAT — Bus event stubs (tie to ron-bus later).

pub mod events {
    #[derive(Clone, Debug)]
    pub enum BusEvent {
        ConfigUpdated,
        Shutdown,
    }
}
