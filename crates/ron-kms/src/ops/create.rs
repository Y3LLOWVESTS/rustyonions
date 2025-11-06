use crate::{error::KmsError, traits::Keystore, types::KeyId};
#[cfg(feature = "with-metrics")]
use {crate::telemetry, std::time::Instant};

pub fn ed25519<K: Keystore>(ks: &K, tenant: &str, purpose: &str) -> Result<KeyId, KmsError> {
    #[cfg(feature = "with-metrics")]
    let start = Instant::now();

    let res = ks.create_ed25519(tenant, purpose);

    #[cfg(feature = "with-metrics")]
    {
        let m = telemetry::metrics();
        match &res {
            Ok(kid) => {
                m.ops_total
                    .with_label_values(&["create", kid.alg.as_str()])
                    .inc();
                m.op_latency_seconds.observe(start.elapsed().as_secs_f64());
            }
            Err(e) => {
                m.failures_total
                    .with_label_values(&["create", e.kind()])
                    .inc();
                m.op_latency_seconds.observe(start.elapsed().as_secs_f64());
            }
        }
    }
    res
}
