use crate::{
    error::KmsError,
    traits::Keystore,
    types::{KeyId, KeyMeta},
};
#[cfg(feature = "with-metrics")]
use {crate::telemetry, std::time::Instant};

pub fn attest<K: Keystore>(ks: &K, kid: &KeyId) -> Result<KeyMeta, KmsError> {
    #[cfg(feature = "with-metrics")]
    let start = Instant::now();

    let res = ks.meta(kid);

    #[cfg(feature = "with-metrics")]
    {
        let m = telemetry::metrics();
        match &res {
            Ok(meta) => {
                m.ops_total
                    .with_label_values(&["attest", meta.alg.as_str()])
                    .inc();
                m.op_latency_seconds.observe(start.elapsed().as_secs_f64());
            }
            Err(e) => {
                m.failures_total
                    .with_label_values(&["attest", e.kind()])
                    .inc();
                m.op_latency_seconds.observe(start.elapsed().as_secs_f64());
            }
        }
    }
    res
}
