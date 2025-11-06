use crate::{error::KmsError, traits::Verifier, types::KeyId};
#[cfg(feature = "with-metrics")]
use {crate::telemetry, std::time::Instant};

pub fn verify<K: Verifier>(ks: &K, kid: &KeyId, msg: &[u8], sig: &[u8]) -> Result<bool, KmsError> {
    #[cfg(feature = "with-metrics")]
    let start = Instant::now();

    let res = ks.verify(kid, msg, sig);

    #[cfg(feature = "with-metrics")]
    {
        let m = telemetry::metrics();
        match &res {
            Ok(_) => {
                m.ops_total
                    .with_label_values(&["verify", kid.alg.as_str()])
                    .inc();
                m.op_latency_seconds.observe(start.elapsed().as_secs_f64());
            }
            Err(e) => {
                m.failures_total
                    .with_label_values(&["verify", e.kind()])
                    .inc();
                m.op_latency_seconds.observe(start.elapsed().as_secs_f64());
            }
        }
    }
    res
}
