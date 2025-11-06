use crate::{error::KmsError, traits::Signer, types::KeyId};
#[cfg(feature = "with-metrics")]
use {crate::telemetry, std::time::Instant};

pub fn sign<K: Signer>(ks: &K, kid: &KeyId, msg: &[u8]) -> Result<Vec<u8>, KmsError> {
    #[cfg(feature = "with-metrics")]
    let start = Instant::now();

    let res = ks.sign(kid, msg);

    #[cfg(feature = "with-metrics")]
    {
        let m = telemetry::metrics();
        match &res {
            Ok(_) => {
                m.ops_total
                    .with_label_values(&["sign", kid.alg.as_str()])
                    .inc();
                m.op_latency_seconds.observe(start.elapsed().as_secs_f64());
            }
            Err(e) => {
                m.failures_total
                    .with_label_values(&["sign", e.kind()])
                    .inc();
                m.op_latency_seconds.observe(start.elapsed().as_secs_f64());
            }
        }
    }
    res
}
