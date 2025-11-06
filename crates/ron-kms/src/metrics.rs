//! RO:WHAT  Prometheus metrics for ron-kms.
//! RO:INV   Small, stable label sets to avoid cardinality explosions.

#![cfg(feature = "with-metrics")]

use prometheus::{opts, register_histogram, register_int_counter_vec, Histogram, IntCounterVec};

pub struct KmsMetrics {
    /// Operation counts, labeled by operation and algorithm.
    /// op ∈ {create,rotate,sign,verify,attest}
    /// alg ∈ {"ed25519", ...}
    pub ops_total: IntCounterVec,

    /// Failure counts, labeled by operation and error kind.
    /// op ∈ {create,rotate,sign,verify,attest}
    /// kind ∈ {"NoSuchKey","AlgUnavailable","Expired","Entropy","VerifyFailed","CapabilityMissing","Busy","Internal"}
    pub failures_total: IntCounterVec,

    /// Latency histogram (seconds) across ops.
    pub op_latency_seconds: Histogram,
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

        // Buckets: generic powers-of-two-ish, adequate for dev; refine later if needed.
        let op_latency_seconds = register_histogram!(
            "kms_op_latency_seconds",
            "Latency of KMS operations in seconds"
        )
        .expect("register kms_op_latency_seconds");

        Self {
            ops_total,
            failures_total,
            op_latency_seconds,
        }
    }
}
