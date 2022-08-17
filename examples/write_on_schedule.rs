use anyhow::Result;
use log::LevelFilter;
use metrics::{counter, gauge, histogram};
use metrics_datadog_exporter::DataDogBuilder;
use metrics_datadog_exporter::DataDogExporter;
use once_cell::sync::Lazy;
use std::ops::Deref;
use std::time::Duration;
use tokio::time::sleep;

static DD_METRICS: Lazy<DataDogExporter> = Lazy::new(|| {
    DataDogBuilder::default()
        .tags(vec![("tag1".to_string(), "value1".to_string())])
        .write_to_stdout(true)
        .build()
        .unwrap()
        .install()
        .unwrap()
});

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    env_logger::builder()
        .filter_level(LevelFilter::Debug)
        .init();

    let metrics = DD_METRICS.deref();

    counter!("this.counter", 123, "tag2" => "value2");
    gauge!("this.gauge", 234.0, "tag3" => "value3");
    histogram!("this.histogram", 345.0, "tag4" => "value5");
    histogram!("this.histogram", 456.0, "tag4" => "value5");

    metrics.schedule(Duration::from_millis(100));
    sleep(Duration::from_secs(3)).await;
    metrics.flush().await?;
    Ok(())
}
