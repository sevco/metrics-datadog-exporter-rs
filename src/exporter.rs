//! DataDog HTTP API exporter

use flate2::write::GzEncoder;
use flate2::Compression;
use futures::future::try_join_all;
use std::io::Write;
use std::sync::Arc;
use std::time::Duration;

use itertools::Itertools;
use metrics::{Key, Label};
use metrics_util::registry::{AtomicStorage, Registry};
use reqwest::header::CONTENT_ENCODING;
use reqwest::{blocking, Client};
use tokio::spawn;
use tokio::task::JoinHandle;
use tokio_schedule::{every, Job};
use tracing::Level;
use tracing::{debug, enabled, warn};

use crate::builder::DataDogConfig;
use crate::data::{DataDogApiPost, DataDogMetric, DataDogSeries};
use crate::{Error, Result};

// Size constants from https://docs.datadoghq.com/api/latest/metrics/#submit-metrics
const MAX_PAYLOAD_BYTES: usize = 3200000;
const MAX_DECOMPRESSED_PAYLOAD: usize = 62914560;

fn send_blocking(
    metrics: Vec<DataDogMetric>,
    gzip: bool,
    api_host: String,
    api_key: String,
    client: blocking::Client,
) -> Result<(), Error> {
    if !metrics.is_empty() {
        let requests = metric_requests(metrics, gzip)?;
        for request in requests {
            let mut request = client
                .post(format!("{}/series", api_host.to_owned()))
                .header("DD-API-KEY", api_key.to_owned())
                .body(request);
            if gzip {
                request = request.header(CONTENT_ENCODING, "gzip");
            }

            let response = request.send()?.error_for_status()?;
            if enabled!(Level::DEBUG) {
                let status = response.status();
                let message = response.text()?;
                debug!(status = %status, message = %message, "Response from DataDog API")
            }
        }
    };
    Ok(())
}

async fn send_async(
    metrics: Vec<DataDogMetric>,
    gzip: bool,
    api_host: &String,
    api_key: &String,
    client: &Client,
) -> Result<(), Error> {
    if !metrics.is_empty() {
        let requests = metric_requests(metrics, gzip)?;
        let responses = try_join_all(requests.into_iter().map(|request| async {
            let mut request = client
                .post(format!("{}/series", api_host.to_owned()))
                .header("DD-API-KEY", api_key.to_owned())
                .body(request);
            if gzip {
                request = request.header(CONTENT_ENCODING, "gzip");
            }
            let response = request.send().await?.error_for_status()?;
            let status = response.status();
            let message = response.text().await?;
            Ok::<_, reqwest::Error>((status, message))
        }))
        .await?;

        if enabled!(Level::DEBUG) {
            responses.into_iter().for_each(|(status, message)| {
                debug!(status = %status, message = %message, "Response from DataDog API")
            });
        }
    };
    Ok(())
}

fn metric_requests(metrics: Vec<DataDogMetric>, gzip: bool) -> Result<Vec<Vec<u8>>> {
    let series = metrics
        .into_iter()
        .flat_map(DataDogSeries::new)
        .collect_vec();
    if gzip {
        split_and_compress_series(&series)
    } else {
        split_series(&series)
    }
}

fn split_series(series: &[DataDogSeries]) -> Result<Vec<Vec<u8>>> {
    let body = serde_json::to_vec(&DataDogApiPost { series })?;
    if body.len() < MAX_PAYLOAD_BYTES {
        Ok(vec![body])
    } else {
        let (left, right) = series.split_at(series.len() / 2);
        Ok(split_series(left)?
            .into_iter()
            .chain(split_series(right)?.into_iter())
            .collect_vec())
    }
}

fn split_and_compress_series(series: &[DataDogSeries]) -> Result<Vec<Vec<u8>>> {
    fn split(series: &[DataDogSeries]) -> Result<Vec<Vec<u8>>> {
        let (left, right) = series.split_at(series.len() / 2);
        Ok(split_and_compress_series(left)?
            .into_iter()
            .chain(split_and_compress_series(right)?.into_iter())
            .collect_vec())
    }

    let body = serde_json::to_vec(&DataDogApiPost { series })?;
    if body.len() > MAX_DECOMPRESSED_PAYLOAD {
        split(series)
    } else {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&serde_json::to_vec(&DataDogApiPost { series })?)?;
        let compressed = encoder.finish()?;
        if compressed.len() < MAX_PAYLOAD_BYTES {
            Ok(vec![compressed])
        } else {
            split(series)
        }
    }
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
    gzip: bool,
}

impl DataDogExporter {
    pub(crate) fn new(
        registry: Arc<Registry<Key, AtomicStorage>>,
        client: Option<Client>,
        config: DataDogConfig,
    ) -> Self {
        DataDogExporter {
            registry,
            write_to_stdout: config.write_to_stdout,
            write_to_api: config.write_to_api,
            api_host: config.api_host,
            api_client: client,
            api_key: config.api_key,
            tags: config.tags,
            gzip: config.gzip,
        }
    }

    /// Write metrics every [`Duration`]
    pub fn schedule(self, interval: Duration) -> (Arc<Self>, JoinHandle<()>) {
        let exporter = Arc::new(self);
        let scheduled_exporter = exporter.clone();
        let every = every(interval.as_secs() as u32).seconds().perform(move || {
            let exporter = scheduled_exporter.clone();
            async move {
                let result = exporter.clone().flush().await;
                if let Err(e) = result {
                    warn!(error = ?e, "Failed to flush metrics");
                }
            }
        });
        (exporter, spawn(every))
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

    async fn write_to_api(&self, metrics: Vec<DataDogMetric>) -> Result<(), Error> {
        send_async(
            metrics,
            self.gzip,
            &self.api_host,
            self.api_key.as_ref().unwrap(),
            self.api_client.as_ref().unwrap(),
        )
        .await
    }
}

impl Drop for DataDogExporter {
    fn drop(&mut self) {
        let metrics = self.collect();
        if self.write_to_stdout {
            if let Err(e) = self.write_to_stdout(metrics.as_slice()) {
                eprintln!("Failed to flush to stdout: {}", e)
            };
        }

        if self.write_to_api {
            let host = self.api_host.to_string();
            let api_key = self.api_key.as_ref().unwrap().to_string();
            let compression = self.gzip;
            // reqwest::blocking can't run in existing runtime
            let joined = std::thread::spawn(move || {
                send_blocking(
                    metrics,
                    compression,
                    host,
                    api_key,
                    blocking::Client::default(),
                )
            })
            .join();

            match joined {
                Ok(r) => match r {
                    Ok(_) => (),
                    Err(e) => {
                        eprintln!("Failed to flush metrics in drop: {}", e);
                    }
                },
                Err(_) => {
                    eprintln!("Failed to join flush thread in drop");
                }
            };
        }
    }
}
