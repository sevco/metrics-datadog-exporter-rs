use anyhow::Result;
use metrics::{counter, gauge, histogram};
use metrics_datadog_exporter::{
    DataDogBuilder, DataDogMetric, DataDogMetricType, DataDogMetricValue,
};
use std::collections::HashMap;

#[test]
fn collect_test() -> Result<()> {
    let metrics = DataDogBuilder::default().build()?.install()?;
    counter!("this.counter", 123, "tag2" => "value2");
    gauge!("this.gauge", 234.0, "tag3" => "value3");
    histogram!("this.histogram", 345.0, "tag4" => "value5");
    histogram!("this.histogram", 456.0, "tag4" => "value5");
    let collected = metrics
        .collect()
        .into_iter()
        .map(|m| (m.metric.to_string(), m))
        .collect::<HashMap<String, DataDogMetric>>();
    assert_eq!(collected.len(), 3);
    let counter = collected.get("this.counter").unwrap();
    assert_eq!(counter.metric_type, DataDogMetricType::Count);
    assert_eq!(counter.tags, vec!["tag2:value2".to_string()]);
    assert_eq!(counter.points, vec![DataDogMetricValue::Int(123)]);
    let histogram = collected.get("this.histogram").unwrap();
    assert_eq!(histogram.metric_type, DataDogMetricType::Histogram);
    assert_eq!(histogram.points.len(), 2);
    Ok(())
}
