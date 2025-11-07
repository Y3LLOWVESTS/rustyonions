//! RO:WHAT  Prometheus metrics for ron-kms.
//! RO:INV   Small, stable label sets to avoid cardinality explosions.

#![cfg(feature = "with-metrics")]

use prometheus::{opts, register_histogram, register_int_counter_vec, Histogram, IntCounterVec};
use std::time::Duration;

pub struct KmsMetrics {
    /// Operation counts, labeled by operation and algorithm.
    /// op ∈ {create,rotate,sign,verify,attest,verify_batch}
    /// alg ∈ {"ed25519", ...}
    pub ops_total: IntCounterVec,

    /// Failure counts, labeled by operation and error kind.
    /// op ∈ {create,rotate,sign,verify,attest,verify_batch}
    /// kind ∈ {"NoSuchKey","AlgUnavailable","Expired","Entropy","VerifyFailed","CapabilityMissing","Busy","Internal"}
    pub failures_total: IntCounterVec,

    /// Latency histogram (seconds) across ops.
    pub op_latency_seconds: Histogram,

    /// Batch size histogram for verify_batch; default buckets are fine for ops visibility.
    pub batch_len: Histogram,
}

impl KmsMetrics {
    /// Registers all metrics in the default Prometheus registry.
    #[must_use]
    pub fn register() -> Self {
        let ops_total = register_int_counter_vec!(
            opts!(
                "kms_ops_total",
                "Total successful KMS operations by type and algorithm"
            ),
            &["op", "alg"]
        )
        .expect("register kms_ops_total");

        let failures_total = register_int_counter_vec!(
            opts!(
                "kms_failures_total",
                "Total failed KMS operations by type and error kind"
            ),
            &["op", "kind"]
        )
        .expect("register kms_failures_total");

        // Buckets: using crate defaults is acceptable; we only need broad latency trends.
        let op_latency_seconds = register_histogram!(
            "kms_op_latency_seconds",
            "Latency of KMS operations in seconds"
        )
        .expect("register kms_op_latency_seconds");

        // Batch size distribution for verify_batch; helpful in production to tune batching policy.
        let batch_len =
            register_histogram!("kms_batch_len", "Observed batch sizes for verify_batch")
                .expect("register kms_batch_len");

        Self {
            ops_total,
            failures_total,
            op_latency_seconds,
            batch_len,
        }
    }

    /// Record a successful op with optional batch length.
    pub fn observe(&self, op: &str, alg: &str, batch: Option<usize>, dur: Duration) {
        self.ops_total.with_label_values(&[op, alg]).inc();
        if let Some(n) = batch {
            self.batch_len.observe(n as f64);
        }
        self.op_latency_seconds.observe(dur.as_secs_f64());
    }

    /// Record a failure for `op` with `kind` (e.g., "busy", "bad_sig").
    pub fn fail(&self, op: &str, kind: &str) {
        self.failures_total.with_label_values(&[op, kind]).inc();
    }
}
