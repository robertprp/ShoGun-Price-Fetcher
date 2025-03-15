use error_stack::{Result, ResultExt};
use lib::error::Error;
use opentelemetry_otlp::{Protocol, WithExportConfig};
use opentelemetry_sdk::{metrics::SdkMeterProvider, runtime};
use std::time::Duration;
use super::{get_otlp_resource, TelemetryParams};

pub fn new(telemetry_params: TelemetryParams) -> Result<SdkMeterProvider, Error> {
    let exporter = opentelemetry_otlp::MetricExporter::builder()
        .with_tonic()
        .with_endpoint(telemetry_params.grpc_endpoint)
        .with_protocol(Protocol::Grpc)
        .with_timeout(Duration::from_secs(3))
        .build()
        .change_context(Error::Unknown)?;

    let reader = opentelemetry_sdk::metrics::PeriodicReader::builder(exporter, runtime::Tokio);

    let reader = reader
        .with_interval(std::time::Duration::from_secs(3))
        .build();

    let resource = get_otlp_resource(&telemetry_params.service_name, &telemetry_params.service_namespace);

    let provider = opentelemetry_sdk::metrics::SdkMeterProvider::builder()
        .with_reader(reader)
        .with_resource(resource)
        .build();

    Ok(provider)
}
