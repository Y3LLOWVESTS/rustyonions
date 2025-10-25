use std::future::Future;
use tokio::task;

use crate::events::KernelEvent;
use crate::internal::types::{BoxError, ServiceName};
use crate::metrics::exporter::Metrics;
use crate::Bus;

pub async fn run_once<F, Fut>(
    name: ServiceName,
    metrics: &Metrics,
    bus: &Bus<KernelEvent>,
    work: F,
) -> Result<(), BoxError>
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<(), BoxError>> + Send + 'static,
{
    let join = task::spawn(async move { work().await }).await;

    match join {
        Ok(Ok(())) => {
            metrics.inc_restart(name);
            bus.publish(KernelEvent::ServiceCrashed {
                service: name.to_string(),
            });
            Ok(())
        }
        Ok(Err(e)) => {
            metrics.inc_restart(name);
            bus.publish(KernelEvent::ServiceCrashed {
                service: name.to_string(),
            });
            Err(e)
        }
        Err(_join_err) => {
            metrics.inc_restart(name);
            bus.publish(KernelEvent::ServiceCrashed {
                service: name.to_string(),
            });
            Ok(())
        }
    }
}
