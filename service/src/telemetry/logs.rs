use std::time::Duration;
use error_stack::{Result, ResultExt};
use lib::error::Error;
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_otlp::{Protocol, WithExportConfig};
use opentelemetry_sdk::logs::{Logger, LoggerProvider};
use opentelemetry_sdk::runtime;
use tracing::{Subscriber};
use tracing_subscriber::filter::Filtered;
use tracing_subscriber::fmt::writer::MakeWriterExt;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::{registry::LookupSpan, Layer};

use super::{get_otlp_resource, TelemetryParams};

pub fn new<S, R>(
    telemetry_params: TelemetryParams,
) -> Result<
    (
        Box<dyn Layer<S> + Send + Sync>, // stdout layer
        Box<dyn Layer<R> + Send + Sync>, // otel layer
        LoggerProvider
    ),
    Error,
>
where
    S: Subscriber + for<'a> LookupSpan<'a> + Send + Sync,
    R: Subscriber + for<'a> LookupSpan<'a> + Send + Sync + 'static,
{
    let exporter = opentelemetry_otlp::LogExporter::builder()
        .with_tonic()
        .with_endpoint(telemetry_params.grpc_endpoint)
        .with_protocol(Protocol::Grpc)
        .with_timeout(Duration::from_secs(3))
        .build()
        .change_context(Error::Unknown)?;

    let resource = get_otlp_resource(&telemetry_params.service_name, &telemetry_params.service_namespace);
    let provider: LoggerProvider = LoggerProvider::builder()
        .with_resource(resource)
        .with_batch_exporter(exporter, runtime::Tokio)
        .build();

    let logging_level = telemetry_params.log_level.to_string();
    let filter_otel = EnvFilter::new(&logging_level)
        .add_directive("hyper=off".parse().unwrap())
        .add_directive("opentelemetry=off".parse().unwrap())
        .add_directive("tonic=off".parse().unwrap())
        .add_directive("h2=off".parse().unwrap())
        .add_directive("reqwest=off".parse().unwrap());

    let otel_tracing_bridge = OpenTelemetryTracingBridge::new(&provider);
    let otel_layer: Filtered<OpenTelemetryTracingBridge<LoggerProvider, Logger>, EnvFilter, R> =
        otel_tracing_bridge.with_filter(filter_otel);

    let stdout_log_layer = tracing_subscriber::fmt::layer()
        .compact()
        .with_ansi(true)
        .with_target(true)
        .with_writer(std::io::stdout.with_max_level(telemetry_params.log_level));

    Ok((stdout_log_layer.boxed(), otel_layer.boxed(), provider))
}
