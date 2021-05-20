use anyhow::Result;
use log::LevelFilter;
use metrics_datadog_exporter::builder::DataDogBuilder;
use metrics_macros::{counter, gauge, histogram};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    env_logger::builder()
        .filter_level(LevelFilter::Debug)
        .init();
    let metrics = DataDogBuilder::default()
        .tags(vec![("tag1".to_string(), "value1".to_string())])
        .write_to_stdout(true)
        .build()
        .install()?;
    counter!("this.counter", 123, "tag2" => "value2");
    gauge!("this.gauge", 234.0, "tag3" => "value3");
    histogram!("this.histogram", 345.0, "tag4" => "value5");
    histogram!("this.histogram", 456.0, "tag4" => "value5");
    metrics.flush().await?;
    Ok(())
}
