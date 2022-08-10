use flate2::Compression;
use futures::future::try_join_all;
use itertools::Itertools;
use std::io::Write;
use std::sync::Arc;
use std::time::Duration;

use log::{debug, error};
use metrics::{Key, Label};
use metrics_util::registry::{AtomicStorage, Registry};
use reqwest::Client;
use tokio::spawn;
use tokio::task::JoinHandle;
use tokio_schedule::{every, Job};

use crate::data::{DataDogApiPost, DataDogMetric, DataDogSeries};
use crate::{Error, Result};

const MAX_BODY_BYTES: usize = 5242880;
const CHUNK_BODY_BYTES: usize = ((MAX_BODY_BYTES as f32) * 0.75) as usize;

fn metric_requests(metrics: Vec<DataDogMetric>) -> Result<Vec<DataDogApiPost>> {
    let mut series: Vec<Vec<String>> = vec![];
    let mut current_series: Vec<String> = vec![];
    let mut current_series_size = 0;

    for metric in metrics {
        let serialized = serde_json::to_string(&DataDogSeries::from(metric))?;
        current_series_size += serialized.len();
        current_series.push(serialized);

        if current_series_size > CHUNK_BODY_BYTES {
            series.push(current_series);
            current_series = vec![];
            current_series_size = 0;
        };
    }
    if !current_series.is_empty() {
        series.push(current_series);
    }

    Ok(series
        .into_iter()
        .map(|s| DataDogApiPost { series: s })
        .collect_vec())
}

/// Metric exporter
pub struct DataDogExporter {
    registry: Arc<Registry<Key, AtomicStorage>>,
    write_to_stdout: bool,
    write_to_api: bool,
    api_host: String,
    api_client: Option<Client>,
    api_key: Option<String>,
    tags: Vec<Label>,
}

impl DataDogExporter {
    pub(crate) fn new(
        registry: Arc<Registry<Key, AtomicStorage>>,
        write_to_stdout: bool,
        write_to_api: bool,
        api_host: String,
        api_client: Option<Client>,
        api_key: Option<String>,
        tags: Vec<Label>,
    ) -> Self {
        DataDogExporter {
            registry,
            write_to_stdout,
            write_to_api,
            api_host,
            api_client,
            api_key,
            tags,
        }
    }

    /// Write metrics every [`Duration`]
    pub fn schedule(&'static self, interval: Duration) -> JoinHandle<()> {
        let every = every(interval.as_secs() as u32)
            .seconds()
            .perform(move || async move {
                let result = self.flush().await;
                if let Err(e) = result {
                    error!("Failed to flush metrics: {:?}", e);
                }
            });
        spawn(every)
    }

    /// Collect metrics
    ///
    /// Note: This will clear histogram observations    
    pub fn collect(&self) -> Vec<DataDogMetric> {
        let counters = self
            .registry
            .get_counter_handles()
            .into_iter()
            .group_by(|(k, _)| k.clone())
            .into_iter()
            .map(|(key, values)| {
                DataDogMetric::from_counter(
                    key,
                    values.into_iter().map(|(_, v)| v).collect_vec(),
                    &self.tags,
                )
            })
            .collect_vec();

        let gauges = self
            .registry
            .get_gauge_handles()
            .into_iter()
            .group_by(|(k, _)| k.clone())
            .into_iter()
            .map(|(key, values)| {
                DataDogMetric::from_gauge(
                    key,
                    values.into_iter().map(|(_, v)| v).collect_vec(),
                    &self.tags,
                )
            })
            .collect_vec();

        let histograms = self
            .registry
            .get_histogram_handles()
            .into_iter()
            .group_by(|(k, _)| k.clone())
            .into_iter()
            .map(|(key, values)| {
                DataDogMetric::from_histogram(
                    key,
                    values.into_iter().map(|(_, v)| v).collect_vec(),
                    &self.tags,
                )
            })
            .collect_vec();

        self.registry.clear();

        counters
            .into_iter()
            .chain(gauges.into_iter())
            .chain(histograms.into_iter())
            .collect_vec()
    }

    /// Flush metrics
    pub async fn flush(&self) -> Result<()> {
        let metrics: Vec<DataDogMetric> = self.collect();
        debug!("Flushing {} metrics", metrics.len());

        if self.write_to_stdout {
            self.write_to_stdout(metrics.as_slice())?;
        }

        if self.write_to_api {
            self.write_to_api(metrics).await?;
        }

        Ok(())
    }

    fn write_to_stdout(&self, metrics: &[DataDogMetric]) -> Result<()> {
        for metric in metrics {
            for m in metric.to_metric_lines() {
                println!("{}", serde_json::to_string(&m)?)
            }
        }
        Ok(())
    }

    async fn send_request(&self, request: DataDogApiPost) -> Result<(), Error> {
        debug!(
            "Posting to datadog: {}",
            serde_json::to_string_pretty(&request).unwrap()
        );
        let mut e = flate2::write::GzEncoder::new(Vec::new(), Compression::default());

        e.write_all(request.json().as_bytes())?;

        self.api_client
            .as_ref()
            .unwrap()
            .post(format!("{}/series", self.api_host))
            .header("DD-API-KEY", self.api_key.as_ref().unwrap().to_owned())
            .header("Content-Type", "application/json")
            .header("Content-Encoding", "gzip")
            .body(e.finish()?)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }

    async fn write_to_api(&self, metrics: Vec<DataDogMetric>) -> Result<(), Error> {
        let requests = metric_requests(metrics)?;
        try_join_all(
            requests
                .into_iter()
                .map(|request| self.send_request(request)),
        )
        .await?;

        Ok(())
    }
}

impl Drop for DataDogExporter {
    fn drop(&mut self) {
        fn send(host: String, key: String, metrics: Vec<DataDogMetric>) {
            let requests = metric_requests(metrics).unwrap();
            for request in requests {
                debug!(
                    "Posting to datadog: {}",
                    serde_json::to_string_pretty(&request).unwrap()
                );
                let client = reqwest::blocking::Client::new();
                let response = client
                    .post(format!("{}/series", host))
                    .header("DD-API-KEY", key.to_string())
                    .json(&request)
                    .send();
                if let Err(e) = response {
                    eprintln!("Failed to flush metrics {}", e)
                }
            }
        }

        let metrics = self.collect();
        if self.write_to_stdout {
            if let Err(e) = self.write_to_stdout(metrics.as_slice()) {
                eprintln!("Failed to flush to stdout: {:?}", e)
            };
        }

        if self.write_to_api {
            let host = self.api_host.to_string();
            let api_key = self.api_key.as_ref().unwrap().to_string();
            // reqwest::blocking can't run in existing runtime
            std::thread::spawn(move || send(host, api_key, metrics))
                .join()
                .expect("Failed to join flush thread in drop");
        }
    }
}
