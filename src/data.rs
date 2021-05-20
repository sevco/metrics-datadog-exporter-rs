use chrono::Utc;
use metrics::{Key, Label};
use metrics_util::{Handle, MetricKind};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

static LAMBDA_HOSTNAME: &str = "lambda";

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq, PartialOrd, Ord)]
pub enum DataDogMetricType {
    #[serde(rename = "count")]
    Count,
    #[serde(rename = "gauge")]
    Gauge,
    #[serde(rename = "histogram")]
    Histogram,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialOrd, PartialEq)]
#[serde(untagged)]
pub enum DataDogMetricValue {
    Float(f64),
    Int(u64),
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialOrd, PartialEq)]
pub struct DataDogMetric {
    pub metric_name: String,
    pub metric_type: DataDogMetricType,
    pub points: Vec<DataDogMetricValue>,
    pub timestamp: i64,
    pub tags: Vec<String>,
}

impl DataDogMetric {
    pub fn from_metric(
        kind: &MetricKind,
        key: &Key,
        handle: &Handle,
        global_tags: &[Label],
    ) -> Self {
        match kind {
            MetricKind::Counter => DataDogMetric::from_metric_value(
                DataDogMetricType::Count,
                key,
                vec![DataDogMetricValue::Int(handle.read_counter())],
                global_tags,
            ),
            MetricKind::Gauge => DataDogMetric::from_metric_value(
                DataDogMetricType::Gauge,
                key,
                vec![DataDogMetricValue::Float(handle.read_gauge())],
                global_tags,
            ),
            MetricKind::Histogram => {
                let mut values = vec![];
                handle.read_histogram_with_clear(|v| values.extend_from_slice(v));
                DataDogMetric::from_metric_value(
                    DataDogMetricType::Histogram,
                    key,
                    values.into_iter().map(DataDogMetricValue::Float).collect(),
                    global_tags,
                )
            }
        }
    }

    pub fn from_metric_value(
        metric_type: DataDogMetricType,
        key: &Key,
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

    pub fn to_metric_lines(&self) -> Vec<DataDogMetricLine> {
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
    pub series: Vec<DataDogSeries>,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DataDogSeries {
    pub host: String,
    pub interval: Option<i64>,
    pub metric: String,
    pub points: Vec<(i64, DataDogMetricValue)>,
    pub tags: Vec<String>,
    #[serde(rename = "type")]
    pub metric_type: DataDogMetricType,
}

impl From<&DataDogMetric> for DataDogSeries {
    fn from(m: &DataDogMetric) -> Self {
        DataDogSeries {
            host: LAMBDA_HOSTNAME.to_string(),
            interval: None,
            metric: m.metric_name.clone(),
            points: m.points.iter().map(|v| (m.timestamp, v.clone())).collect(),
            tags: m.tags.clone(),
            metric_type: m.metric_type.clone(),
        }
    }
}
