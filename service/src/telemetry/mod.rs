mod logs;
pub mod metrics;
pub mod traces;

use error_stack::{Result, ResultExt};
use lib::error::Error;
use opentelemetry::metrics::MeterProvider;
use opentelemetry::trace::TracerProvider;
use opentelemetry::{global, KeyValue};
use opentelemetry_sdk::logs::LoggerProvider;
use opentelemetry_sdk::metrics::SdkMeterProvider;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::Resource;
use tracing::info;
use tracing::subscriber::set_global_default;
use std::sync::{Arc, OnceLock};
use tracing_opentelemetry::MetricsLayer;
use tracing_subscriber::filter::Targets;
use tracing_subscriber::layer::SubscriberExt;

use crate::config::ConfigService;

static METER_PROVIDER: OnceLock<SdkMeterProvider> = OnceLock::new();
static LOGGER_PROVIDER: OnceLock<LoggerProvider> = OnceLock::new();

pub fn init(
    config_service: &ConfigService,
    service_name: String,
    log_level: tracing::Level,
) -> Result<(), Error> {
    let target = Targets::new()
        .with_target("grafana_shogun", log_level)
        .with_target("service", log_level);
    
    let telemetry_params = TelemetryParams::new(&config_service.clone(), &service_name, Some(log_level));

    let meter_provider = metrics::new(telemetry_params.clone())?;
    let (std_layer, otlp_layer, logger_provider) =
        logs::new(telemetry_params.clone())?;
    let (trace_layer, trace_provider) = traces::new(telemetry_params.clone())?;

    // Add panics to tracing
    init_panic_hook();
    
    let registry = tracing_subscriber::Registry::default()
        .with(otlp_layer) // Logs layer
        .with(MetricsLayer::new(meter_provider.clone())) // Metrics layer
        .with(std_layer) // Standard output layer
        .with(target) // Target filter
        .with(trace_layer);
    
    // Discussion: https://github.com/open-telemetry/opentelemetry-rust/discussions/1605
    METER_PROVIDER.set(meter_provider.clone()).expect(
        "This is not meant to be used more than once in the same process",
    );
    LOGGER_PROVIDER.set(logger_provider.clone()).expect(
        "This is not meant to be used more than once in the same process",
    );

    global::set_text_map_propagator(TraceContextPropagator::new()); // Extra data for traces
    global::set_meter_provider(meter_provider);
    global::set_tracer_provider(trace_provider);
    
    // Initiazlie global subscriber
    set_global_default(registry)
        .change_context(Error::Unknown)?;
    
    Ok(())
}

fn shutdown_tracer_provider() {
    global::shutdown_tracer_provider();
}

fn shutdown_meter_provider() {
    if let Some(meter_provider) = METER_PROVIDER.get() {
        meter_provider
            .shutdown()
            .expect("Failed to shutdown meter provider");
    }
}

fn shutdown_logger_provider() {
    if let Some(logger_provider) = LOGGER_PROVIDER.get() {
        logger_provider
            .shutdown()
            .expect("Failed to shutdown logger provider");
    }
}

pub async fn shutdown() -> Result<(), Error> {
    info!("Shutting down all telemetry! Good bye...!");
    let shutdown_handler = tokio::task::spawn_blocking(|| {
        shutdown_logger_provider();
        shutdown_tracer_provider();
        shutdown_meter_provider();
    }).await;
    
    if let Err(e) = shutdown_handler {
        tracing::error!(reason = ?e, "Failed to shutdown telemetry");
        std::process::exit(1);
    }
    
    Ok(())
}

pub fn get_tracer(name: String) -> opentelemetry::global::BoxedTracer {
    global::tracer_provider().tracer(name)
}

pub fn get_meter_provider() -> Arc<dyn MeterProvider + Send + Sync> {
    global::meter_provider()
}

fn init_panic_hook() {
    let prev_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let payload = panic_info.payload();

        #[allow(clippy::manual_map)]
        let payload = if let Some(s) = payload.downcast_ref::<&str>() {
            Some(&**s)
        } else if let Some(s) = payload.downcast_ref::<String>() {
            Some(s.as_str())
        } else {
            None
        };

        tracing::error!(
            panic.payload = payload,
            panic.location = panic_info.location().map(|l| l.to_string()),
            panic.backtrace = tracing::field::display(std::backtrace::Backtrace::capture()),
            "A panic occurred"
        );
        
        // TODO: We should send here an event to a channel in order to flush logs

        prev_hook(panic_info);
    }));
}

/// Creates a new OpenTelemetry resource with the given service name and namespace.
///
/// # Arguments
///
/// * `service_name` - The name of the service.
/// * `service_namespace` - The namespace of the service (e.g., the environment the instance is running in).
///
/// # Example
/// ```rust
/// let resource_indexer = get_otlp_resource("indexer", "stage");
/// let resource_graphql = get_otlp_resource("graphql", "stage");
/// ```
pub(crate) fn get_otlp_resource<'a>(service_name: &'a str, service_namespace: &'a str) -> Resource {
    // Use OpenTelemetry's semantic conventions for standard attribute keys.
    let service_attr = KeyValue::new("service.name", service_name.to_owned());
    let service_namespace_attr = KeyValue::new("service.namespace", service_namespace.to_owned());

    Resource::new(vec![service_attr, service_namespace_attr])
}

#[derive(Debug, Clone)]
pub struct TelemetryParams {
    pub service_name: String,
    pub service_namespace: String,
    pub log_level: tracing::Level,
    pub grpc_endpoint: String,
    pub http_endpoint: String,
}

impl TelemetryParams {
    pub fn new(config_service: &ConfigService, service_name: &str, log_level: Option<tracing::Level>) -> Self {
        Self {
            service_name: service_name.to_owned(),
            service_namespace: config_service.environment.name.clone(),
            log_level: log_level.unwrap_or(tracing::Level::INFO),
            grpc_endpoint: config_service.environment.otlp_grpc_endpoint.clone(),
            http_endpoint: config_service.environment.otlp_http_endpoint.clone(),
        }
    }
}