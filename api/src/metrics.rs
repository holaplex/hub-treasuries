#[allow(clippy::wildcard_imports)]
use hub_core::{
    anyhow::{anyhow, Result},
    metrics::*,
};

#[derive(Clone)]
pub struct Metrics {
    pub registry: Registry,
    pub provider: MeterProvider,
    pub sign_duration_ms_bucket: Histogram<i64>,
}

impl Metrics {
    /// Res
    /// # Errors
    pub fn new() -> Result<Self> {
        let registry = Registry::new();
        let exporter = hub_core::metrics::exporter()
            .with_registry(registry.clone())
            .with_namespace("hub_treasuries")
            .build()
            .map_err(|e| anyhow!("Failed to build exporter: {}", e))?;

        let provider = MeterProvider::builder()
            .with_reader(exporter)
            .with_resource(Resource::new(vec![KeyValue::new(
                "service.name",
                "hub-treasuries",
            )]))
            .build();

        let meter = provider.meter("hub-treasuries");

        let sign_duration_ms_bucket = meter
            .i64_histogram("sign.time")
            .with_unit(Unit::new("ms"))
            .with_description("Signing duration time in milliseconds.")
            .init();

        Ok(Self {
            registry,
            provider,
            sign_duration_ms_bucket,
        })
    }
}
