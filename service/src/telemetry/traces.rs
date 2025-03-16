use error_stack::{Result, ResultExt};
use lib::error::Error;
use opentelemetry::trace::TracerProvider;
use opentelemetry_otlp::{Protocol, WithExportConfig};
use opentelemetry_sdk::runtime;
use tracing::Subscriber;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::Layer;

use super::{get_otlp_resource, TelemetryParams};

pub fn new<S>(
    telemetry_params: TelemetryParams,
) -> Result<
    (
        Box<dyn Layer<S> + Send + Sync>,
        opentelemetry_sdk::trace::TracerProvider,
    ),
    Error,
>
where
    S: Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a> + Send + Sync,
{
    let resource = get_otlp_resource(
        &telemetry_params.service_name.clone(),
        &telemetry_params.service_namespace.clone(),
    );
    let otlp_exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(telemetry_params.grpc_endpoint)
        .with_protocol(Protocol::Grpc)
        .build()
        .change_context(Error::Unknown)?;

    let provider = opentelemetry_sdk::trace::TracerProvider::builder()
        .with_batch_exporter(otlp_exporter, runtime::Tokio)
        .with_resource(resource.clone())
        .build();

    let tracer_name = format!(
        "{}-{}-tracer",
        telemetry_params.service_name, telemetry_params.service_namespace
    );
    let tracer = provider.tracer(tracer_name.clone());

    let tracing_layer = OpenTelemetryLayer::new(tracer);

    Ok((tracing_layer.boxed(), provider))
}
