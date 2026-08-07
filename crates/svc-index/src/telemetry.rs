//! RO:WHAT — OpenTelemetry glue for the optional svc-index OTLP feature.
//! RO:WHY — OpenTelemetry 0.24 moved SDK and runtime types into opentelemetry_sdk.
//! RO:CONFIG — OTEL_EXPORTER_OTLP_ENDPOINT and standard OTLP environment variables.
//! RO:INVARIANTS — optional observability only; no request, storage, wallet, ledger, receipt, or settlement authority.
//! RO:TEST — compile with svc-index all features and run focused Phase 6B1 tests.

// FINAL_BETA_PHASE6B1_OTEL_COMPATIBILITY_V1
// FINAL_BETA_PHASE6B1_OTEL_COMPATIBILITY_V2

#[cfg(feature = "otel")]
pub mod otel {
    use opentelemetry::{
        trace::TracerProvider as _,
        KeyValue,
    };
    use opentelemetry_sdk::{
        runtime::Tokio,
        trace as sdktrace,
        Resource,
    };
    use tracing_subscriber::{
        layer::SubscriberExt,
        Registry,
    };

    pub fn init(
        service_name: &str,
    ) {
        let resource =
            Resource::new(
                vec![
                    KeyValue::new(
                        "service.name",
                        service_name.to_owned(),
                    ),
                ],
            );

        let provider =
            opentelemetry_otlp::new_pipeline()
                .tracing()
                .with_exporter(
                    opentelemetry_otlp::
                        new_exporter()
                        .tonic(),
                )
                .with_trace_config(
                    sdktrace::Config::default()
                        .with_resource(
                            resource,
                        ),
                )
                .install_batch(
                    Tokio,
                )
                .expect(
                    "install OTLP tracer provider",
                );

        let tracer =
            provider.tracer(
                service_name.to_owned(),
            );

        let _ =
            opentelemetry::global::
                set_tracer_provider(
                    provider,
                );

        let telemetry_layer =
            tracing_opentelemetry::layer()
                .with_tracer(
                    tracer,
                );

        let subscriber =
            Registry::default()
                .with(
                    telemetry_layer,
                );

        tracing::subscriber::
            set_global_default(
                subscriber,
            )
            .ok();
    }
}
