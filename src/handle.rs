use crate::data::{DataDogApiPost, DataDogMetric, DataDogSeries};
use crate::{Error, Result};
use log::error;
use metrics::{Key, Label};
use metrics_util::{Handle, NotTracked, Registry};
use reqwest::{Client, Response};
use std::sync::Arc;
use std::time::Duration;
use tokio::spawn;
use tokio::task::JoinHandle;
use tokio_schedule::{every, Job};

pub struct DataDogHandle {
    registry: Arc<Registry<Key, Handle, NotTracked<Handle>>>,
    write_to_stdout: bool,
    write_to_api: bool,
    api_host: String,
    api_client: Option<Client>,
    api_key: Option<String>,
    tags: Vec<Label>,
}

impl DataDogHandle {
    pub(crate) fn new(
        registry: Arc<Registry<Key, Handle, NotTracked<Handle>>>,
        write_to_stdout: bool,
        write_to_api: bool,
        api_host: String,
        api_client: Option<Client>,
        api_key: Option<String>,
        tags: Vec<Label>,
    ) -> Self {
        DataDogHandle {
            registry,
            write_to_stdout,
            write_to_api,
            api_host,
            api_client,
            api_key,
            tags,
        }
    }

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

    pub fn collect(&self) -> Vec<DataDogMetric> {
        self.registry
            .get_handles()
            .iter()
            .map(|((kind, key), (_, handle))| {
                DataDogMetric::from_metric(&kind, &key, &handle, &self.tags)
            })
            .collect()
    }

    pub async fn flush(&self) -> Result<()> {
        let metrics: Vec<DataDogMetric> = self.collect();
        log::debug!("Flushing {} metrics", metrics.len());

        if self.write_to_stdout {
            for metric in &metrics {
                self.write_to_stdout(metric)?;
            }
        }

        if self.write_to_api {
            self.write_to_api(&metrics).await?;
        }

        Ok(())
    }

    fn write_to_stdout(&self, metric: &DataDogMetric) -> Result<()> {
        for m in metric.to_metric_lines() {
            println!("{}", serde_json::to_string(&m)?)
        }
        Ok(())
    }

    async fn write_to_api(&self, metrics: &[DataDogMetric]) -> Result<Response, Error> {
        let body = &DataDogApiPost {
            series: metrics
                .iter()
                .map(|m| m.into())
                .collect::<Vec<DataDogSeries>>(),
        };
        self.api_client
            .as_ref()
            .unwrap()
            .post(format!("{}/series", self.api_host))
            .header("DD-API-KEY", self.api_key.as_ref().unwrap())
            .json(&body)
            .send()
            .await?
            .error_for_status()
            .map_err(crate::Error::from)
    }
}
