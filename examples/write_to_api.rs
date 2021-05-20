use anyhow::Result;
use metrics_datadog_exporter::DataDogBuilder;
use metrics_macros::{counter, gauge, histogram};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let metrics = DataDogBuilder::default()
        .tags(vec![("tag1".to_string(), "value1".to_string())])
        .write_to_stdout(false)
        .write_to_api(true, Some("DD_API_KEY".to_string()))
        .build()
        .install()?;
    counter!("this.counter", 123, "tag2" => "value2");
    gauge!("this.gauge", 234.0, "tag3" => "value3");
    histogram!("this.histogram", 345.0, "tag4" => "value5");
    histogram!("this.histogram", 456.0, "tag4" => "value5");
    metrics.flush().await?;
    Ok(())
}
