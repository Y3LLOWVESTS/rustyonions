//! RO:WHAT — OpenTelemetry glue (optional feature).
//! RO:WHY  — Trace export to OTLP if enabled.
//! RO:CONFIG — OTEL_EXPORTER_OTLP_ENDPOINT, etc.
//! RO:TEST — manual in perf/chaos workflows.

#[cfg(feature = "otel")]
pub mod otel {
    use opentelemetry::sdk::trace as sdktrace;
    use opentelemetry::KeyValue;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::Registry;

    pub fn init(service_name: &str) {
        let tracer = opentelemetry_otlp::new_pipeline()
            .tracing()
            .with_trace_config(
                sdktrace::config().with_resource(opentelemetry::sdk::Resource::new(vec![
                    KeyValue::new("service.name", service_name.to_string()),
                ])),
            )
            .install_batch(opentelemetry::runtime::Tokio)
            .expect("install otlp");

        let telem = tracing_opentelemetry::layer().with_tracer(tracer);
        let subscriber = Registry::default().with(telem);
        tracing::subscriber::set_global_default(subscriber).ok();
    }
}
