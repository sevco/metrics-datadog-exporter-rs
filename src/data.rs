//! Data model
//!
use chrono::Utc;
use itertools::Itertools;
use metrics::{Key, Label};
use metrics_util::AtomicBucket;
use portable_atomic::AtomicU64;
use serde::{Deserialize, Serialize};
use serde_repr::*;
use serde_with::skip_serializing_none;
use std::sync::atomic::Ordering;
use std::sync::Arc;

static LAMBDA_HOSTNAME: &str = "lambda";

/// Metric type
#[derive(Debug, Serialize_repr, Deserialize_repr, Clone, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u8)]
pub enum DataDogMetricType {
    /// Unspecified
    Unspecified = 0,
    /// Counter
    Count,
    /// Gauge
    Gauge,
    /// Rate
    Rate,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialOrd, PartialEq)]
#[serde(untagged)]
/// Metric value
pub enum DataDogMetricValue {
    /// Float
    Float(f64),
    /// Int
    Int(u64),
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialOrd, PartialEq)]
/// DataDog formatted metric
pub struct DataDogMetric {
    /// Metric name
    pub metric_name: String,
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
                DataDogMetricValue::Int(u)
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
        DataDogMetric::from_metric_value(DataDogMetricType::Unspecified, key, values, global_tags)
    }

    fn from_metric_value(
        metric_type: DataDogMetricType,
        key: Key,
        values: Vec<DataDogMetricValue>,
        global_tags: &[Label],
    ) -> Self {
        DataDogMetric {
            metric_name: key.name().to_string(),
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
                metric_name: self.metric_name.to_string(),
                value: v.clone(),
                timestamp: self.timestamp,
                tags: self.tags.clone(),
            })
            .collect()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DataDogMetricLine {
    #[serde(rename = "m")]
    pub metric_name: String,
    #[serde(rename = "v")]
    pub value: DataDogMetricValue,
    #[serde(rename = "e")]
    pub timestamp: i64,
    #[serde(rename = "t")]
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DataDogApiPost {
    pub series: Vec<String>,
}

impl DataDogApiPost {
    pub fn json(&self) -> String {
        format!("{{ \"series\": [ {} ] }}", self.series.join(","))
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DataDogSeriesResource {
    pub name: String,
    #[serde(rename = "type")]
    pub resource_type: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DataDogSeriesPoint {
    #[serde(rename = "type")]
    pub timestamp: i64,
    pub value: DataDogMetricValue,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DataDogSeries {
    pub interval: Option<i64>,
    pub metric: String,
    pub points: Vec<DataDogSeriesPoint>,
    pub tags: Option<Vec<String>>,
    #[serde(rename = "type")]
    pub metric_type: DataDogMetricType,
    pub resources: Option<Vec<DataDogSeriesResource>>,
    pub unit: Option<String>,
}

impl From<DataDogMetric> for DataDogSeries {
    fn from(m: DataDogMetric) -> Self {
        DataDogSeries {
            interval: None,
            unit: None,
            metric: m.metric_name,
            points: m
                .points
                .into_iter()
                .map(|v| DataDogSeriesPoint {
                    timestamp: m.timestamp,
                    value: v,
                })
                .collect_vec(),
            tags: Some(m.tags),
            metric_type: m.metric_type,
            resources: Some(vec![DataDogSeriesResource {
                resource_type: "host".to_string(),
                name: LAMBDA_HOSTNAME.to_string(),
            }]),
        }
    }
}
