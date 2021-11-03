use std::sync::Arc;
use std::time::Duration;

use log::error;
use metrics::{Key, Label};
use metrics_util::{Handle, NotTracked, Registry};
use reqwest::{Client, Response};
use tokio::spawn;
use tokio::task::JoinHandle;
use tokio_schedule::{every, Job};

use crate::data::{DataDogApiPost, DataDogMetric, DataDogSeries};
use crate::{Error, Result};

fn metric_body(metrics: &[DataDogMetric]) -> DataDogApiPost {
    DataDogApiPost {
        series: metrics
            .iter()
            .map(|m| m.into())
            .collect::<Vec<DataDogSeries>>(),
    }
}

/// Metric exporter
pub struct DataDogExporter {
    registry: Arc<Registry<Key, Handle, NotTracked<Handle>>>,
    write_to_stdout: bool,
    write_to_api: bool,
    api_host: String,
    api_client: Option<Client>,
    api_key: Option<String>,
    tags: Vec<Label>,
}

impl DataDogExporter {
    pub(crate) fn new(
        registry: Arc<Registry<Key, Handle, NotTracked<Handle>>>,
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
                    error!("Failed to flush metrics: {}", e);
                }
            });
        spawn(every)
    }

    /// Collect metrics
    ///
    /// Note: This will clear histogram observations    
    pub fn collect(&self) -> Vec<DataDogMetric> {
        self.registry
            .get_handles()
            .iter()
            .map(|((kind, key), (_, handle))| {
                DataDogMetric::from_metric(kind, key, handle, &self.tags)
            })
            .collect()
    }

    /// Flush metrics
    pub async fn flush(&self) -> Result<()> {
        let metrics: Vec<DataDogMetric> = self.collect();
        log::debug!("Flushing {} metrics", metrics.len());

        if self.write_to_stdout {
            self.write_to_stdout(metrics.as_slice())?;
        }

        if self.write_to_api {
            self.write_to_api(&metrics).await?;
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

    async fn write_to_api(&self, metrics: &[DataDogMetric]) -> Result<Response, Error> {
        let body = metric_body(metrics);
        self.api_client
            .as_ref()
            .unwrap()
            .post(format!("{}/series", self.api_host))
            .header("DD-API-KEY", self.api_key.as_ref().unwrap().to_owned())
            .json(&body)
            .send()
            .await?
            .error_for_status()
            .map_err(crate::Error::from)
    }
}

impl Drop for DataDogExporter {
    fn drop(&mut self) {
        fn send(host: String, key: String, metrics: Vec<DataDogMetric>) {
            let body = metric_body(metrics.as_slice());
            let client = reqwest::blocking::Client::new();
            let response = client
                .post(format!("{}/series", host))
                .header("DD-API-KEY", key)
                .json(&body)
                .send();
            if let Err(e) = response {
                eprintln!("Failed to flush metrics {}", e)
            }
        }

        let metrics = self.collect();
        if self.write_to_stdout {
            if let Err(e) = self.write_to_stdout(metrics.as_slice()) {
                eprintln!("Failed to flush to stdout: {}", e)
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
