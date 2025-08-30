use kameo::Actor;
use tokio::sync::mpsc;

pub struct Supervisor {
    bus: mpsc::Sender<KernelEvent>,
    services: Vec<Box<dyn Actor>>,
}

impl Supervisor {
    pub fn new(bus: mpsc::Sender<KernelEvent>) -> Self {
        Supervisor {
            bus,
            services: Vec::new(),
        }
    }

    // Example of how an in-process actor is created instead of spawning a child process.
    pub fn add_service<T: Actor + 'static>(&mut self, service: T) {
        self.services.push(Box::new(service));
    }

    // Supervise and restart logic for in-process actors
    pub async fn supervise_services(&mut self) {
        for service in &self.services {
            service.start().await;
        }
    }
}
