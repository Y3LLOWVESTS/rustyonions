use crate::{error::KmsError, traits::Keystore, types::KeyId};
#[cfg(feature = "with-metrics")]
use {crate::telemetry, std::time::Instant};

pub fn rotate<K: Keystore>(ks: &K, kid: &KeyId) -> Result<KeyId, KmsError> {
    #[cfg(feature = "with-metrics")]
    let start = Instant::now();

    let res = ks.rotate(kid);

    #[cfg(feature = "with-metrics")]
    {
        let m = telemetry::metrics();
        match &res {
            Ok(new_kid) => {
                m.ops_total
                    .with_label_values(&["rotate", new_kid.alg.as_str()])
                    .inc();
                m.op_latency_seconds.observe(start.elapsed().as_secs_f64());
            }
            Err(e) => {
                m.failures_total
                    .with_label_values(&["rotate", e.kind()])
                    .inc();
                m.op_latency_seconds.observe(start.elapsed().as_secs_f64());
            }
        }
    }
    res
}
