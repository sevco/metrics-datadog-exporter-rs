//! Data model
//!
use std::sync::atomic::Ordering;
use std::sync::Arc;

use chrono::Utc;
use itertools::Itertools;
use metrics::atomics::AtomicU64;
use metrics::{Key, Label};
use metrics_util::AtomicBucket;

use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

/// Metric type
#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq, PartialOrd, Ord)]
pub enum DataDogMetricType {
    /// Counter
    #[serde(rename = "count")]
    Count,
    /// Gauge
    #[serde(rename = "gauge")]
    Gauge,
    /// Histogram
    #[serde(rename = "histogram")]
    Histogram,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialOrd, PartialEq)]
#[serde(untagged)]
/// Metric value
pub enum DataDogMetricValue {
    /// Float
    Float(f64),
    /// Unsigned
    Unsigned(u64),
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialOrd, PartialEq)]
/// DataDog formatted metric
pub struct DataDogMetric {
    /// Metric name
    pub metric: String,
    /// Metric type
    pub metric_type: DataDogMetricType,
    /// Metric values
    pub points: Vec<DataDogMetricValue>,
    /// Timestamp
    pub timestamp: i64,
    /// Tags
    pub tags: Vec<String>,
}

impl DataDogMetric {
    pub(crate) fn from_counter(
        key: Key,
        values: Vec<Arc<AtomicU64>>,
        global_tags: &[Label],
    ) -> Self {
        let values = values
            .into_iter()
            .map(|value| {
                let u = value.load(Ordering::Acquire);
                DataDogMetricValue::Unsigned(u)
            })
            .collect_vec();
        DataDogMetric::from_metric_value(DataDogMetricType::Count, key, values, global_tags)
    }

    pub(crate) fn from_gauge(key: Key, values: Vec<Arc<AtomicU64>>, global_tags: &[Label]) -> Self {
        let values = values
            .into_iter()
            .map(|value| {
                let u = f64::from_bits(value.load(Ordering::Acquire));
                DataDogMetricValue::Float(u)
            })
            .collect_vec();
        DataDogMetric::from_metric_value(DataDogMetricType::Gauge, key, values, global_tags)
    }

    pub(crate) fn from_histogram(
        key: Key,
        values: Vec<Arc<AtomicBucket<f64>>>,
        global_tags: &[Label],
    ) -> Self {
        let values = values
            .into_iter()
            .flat_map(|value| value.data().into_iter().map(DataDogMetricValue::Float))
            .collect_vec();
        DataDogMetric::from_metric_value(DataDogMetricType::Histogram, key, values, global_tags)
    }

    fn from_metric_value(
        metric_type: DataDogMetricType,
        key: Key,
        values: Vec<DataDogMetricValue>,
        global_tags: &[Label],
    ) -> Self {
        DataDogMetric {
            metric: key.name().to_string(),
            metric_type,
            points: values,
            timestamp: Utc::now().timestamp(),
            tags: global_tags
                .iter()
                .chain(key.labels())
                .map(|l| format!("{}:{}", l.key(), l.value()))
                .collect(),
        }
    }

    pub(crate) fn to_metric_lines(&self) -> Vec<DataDogMetricLine> {
        self.points
            .iter()
            .map(|v| DataDogMetricLine {
                name: self.metric.to_string(),
                value: v.clone(),
                timestamp: self.timestamp,
                tags: self.tags.clone(),
            })
            .collect()
    }
}

/// StdOut representation of a metric
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DataDogMetricLine {
    /// Metric name
    #[serde(rename = "m")]
    pub name: String,
    /// Metric value
    #[serde(rename = "v")]
    pub value: DataDogMetricValue,
    /// Metric timestamp
    #[serde(rename = "e")]
    pub timestamp: i64,
    /// Metric tags
    #[serde(rename = "t")]
    pub tags: Vec<String>,
}

/// DataDog API Post Body
#[derive(Debug, Serialize, Clone)]
pub struct DataDogApiPost<'a> {
    /// Metric series
    pub series: &'a [DataDogSeries],
}

/// DataDog Metric Series
#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DataDogSeries {
    /// Metric interval
    pub interval: Option<i64>,
    /// Metric name
    pub metric: String,
    /// Metric time series
    pub points: Vec<(i64, DataDogMetricValue)>,
    /// Metric tags
    pub tags: Vec<String>,
    /// Metric type
    #[serde(rename = "type")]
    pub metric_type: DataDogMetricType,
}

impl DataDogSeries {
    /// Create metric series from metric
    pub fn new(m: DataDogMetric) -> Vec<DataDogSeries> {
        m.points
            .chunks(3)
            .map(|points| DataDogSeries {
                interval: None,
                metric: m.metric.to_owned(),
                points: points.iter().map(|v| (m.timestamp, v.to_owned())).collect(),
                tags: m.tags.to_owned(),
                metric_type: m.metric_type.to_owned(),
            })
            .collect_vec()
    }
}
