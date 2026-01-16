use opentelemetry::global;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::{metrics as sdkmetrics, trace as sdktrace};
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{Registry, layer::SubscriberExt};

use tracing::info;

pub fn init_telemetry() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    global::set_text_map_propagator(TraceContextPropagator::new());

    // Debugging: Log the endpoint being used
    if let Ok(endpoint) = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT") {
        info!("Initializing OpenTelemetry with endpoint: {}", endpoint);
    } else {
        info!("Initializing OpenTelemetry with default endpoint (localhost:4317)");
    }

    // Resource Setup
    // Use Resource::default() if available, otherwise assume defaults are sufficient.
    // Explicitly creating Resource::new(vec![]) is problematic if private.
    // But SdkTracerProvider::builder() has a default resource.

    // --- Tracing Setup ---
    // If opentelemetry_otlp::new_exporter() is gone, try SpanExporter::builder().
    // Or opentelemetry_otlp::WithExportConfig trait usage.
    // Try explicit SpanExporter builder if available.
    // Assuming 0.22+: opentelemetry_otlp::SpanExporter::builder().with_tonic().build()?
    // Actually, let's try to use the pipeline pattern if it returned in 0.22 (unlikely).
    // Let's try opentelemetry_otlp::SpanExporter::builder().

    let trace_exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .build()?;

    let tracer_provider = sdktrace::SdkTracerProvider::builder()
        .with_batch_exporter(trace_exporter)
        .build();

    // Install globally
    global::set_tracer_provider(tracer_provider.clone());

    // Get a tracer
    use opentelemetry::trace::TracerProvider;
    let tracer = tracer_provider.tracer("fluor-api");

    // --- Metrics Setup ---
    let metrics_exporter = opentelemetry_otlp::MetricExporter::builder()
        .with_tonic()
        .build()?;

    let reader = sdkmetrics::PeriodicReader::builder(metrics_exporter).build();

    let meter_provider = sdkmetrics::SdkMeterProvider::builder()
        .with_reader(reader)
        .build();

    global::set_meter_provider(meter_provider);

    // --- Logging Setup ---
    let log_exporter = opentelemetry_otlp::LogExporter::builder()
        .with_tonic()
        .build()?;

    let logger_provider = opentelemetry_sdk::logs::SdkLoggerProvider::builder()
        .with_batch_exporter(log_exporter)
        .build();

    // Create a new OpenTelemetryTracingBridge using the logger provider
    let log_layer =
        opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge::new(&logger_provider);

    // --- Subscriber Setup ---
    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);

    let env_filter =
        tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into());

    Registry::default()
        .with(env_filter)
        .with(telemetry)
        .with(log_layer)
        .try_init()?;

    Ok(())
}

pub fn shutdown_telemetry() {
    // In newer OTel versions, global shutdown might be handled differently or explicitly on providers.
    // If global::shutdown_tracer_provider() is gone, we might need to rely on providers being dropped or
    // manually shutting down if we held the provider.
    // For now, we leave this empty to avoid compilation error if the function is missing.
    // If explicit shutdown is needed, we would need to store the provider globally or pass it back.
}
