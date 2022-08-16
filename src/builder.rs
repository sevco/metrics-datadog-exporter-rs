use std::sync::Arc;
use std::time::Duration;

use metrics::Label;
use metrics_util::registry::{AtomicStorage, Registry};
use reqwest::Client;

use crate::exporter::DataDogExporter;
use crate::recorder::DataDogRecorder;
use crate::{DataDogHandle, Error};

pub struct DataDogConfig {
    pub write_to_stdout: bool,
    pub write_to_api: bool,
    pub api_host: String,
    pub api_key: Option<String>,
    pub tags: Vec<Label>,
    pub client_timeout: Option<Duration>,
    pub gzip: bool,
}

/// Builder for creating/installing a DataDog recorder/exporter
pub struct DataDogBuilder {
    write_to_stdout: bool,
    write_to_api: bool,
    api_host: String,
    api_key: Option<String>,
    tags: Vec<Label>,
    client_timeout: Option<Duration>,
    gzip: bool,
}

impl DataDogBuilder {
    /// Creates a new [`DataDogBuilder`]
    pub fn default() -> Self {
        DataDogBuilder {
            write_to_stdout: true,
            write_to_api: false,
            api_host: "https://api.datadoghq.com/api/v1".to_string(),
            api_key: None,
            tags: vec![],
            client_timeout: None,
            gzip: true,
        }
    }

    /// Write metrics to stdout in DataDog JSON format
    #[must_use]
    pub fn write_to_stdout(self, b: bool) -> DataDogBuilder {
        DataDogBuilder {
            write_to_stdout: b,
            ..self
        }
    }

    /// Write metrics to DataDog API
    #[must_use]
    pub fn write_to_api(self, b: bool, api_key: Option<String>) -> DataDogBuilder {
        DataDogBuilder {
            write_to_api: b,
            api_key,
            ..self
        }
    }

    /// Set DataDog API host
    #[must_use]
    pub fn api_host(self, api_host: String) -> DataDogBuilder {
        DataDogBuilder { api_host, ..self }
    }

    /// Set tags to send with metrics
    #[must_use]
    pub fn tags(self, tags: Vec<(String, String)>) -> DataDogBuilder {
        DataDogBuilder {
            tags: tags.iter().map(Label::from).collect(),
            ..self
        }
    }

    /// Set client timeout
    pub fn client_timeout(self, timeout: Duration) -> DataDogBuilder {
        DataDogBuilder {
            client_timeout: Some(timeout),
            ..self
        }
    }

    /// Set compression
    pub fn gzip(self, gzip: bool) -> DataDogBuilder {
        DataDogBuilder { gzip, ..self }
    }

    /// Build [`DataDogHandle`]
    pub fn build(self) -> Result<DataDogHandle, Error> {
        let registry = Arc::new(Registry::new(AtomicStorage));
        let recorder = DataDogRecorder::new(registry.clone());
        let client = if self.write_to_api {
            let mut c = Client::builder();
            if let Some(timeout) = self.client_timeout {
                c = c.timeout(timeout);
            }
            Some(c.build()?)
        } else {
            None
        };
        let config = DataDogConfig {
            write_to_stdout: self.write_to_stdout,
            write_to_api: self.write_to_api,
            api_host: self.api_host,
            api_key: self.api_key,
            tags: self.tags,
            client_timeout: self.client_timeout,
            gzip: self.gzip,
        };
        let handle = DataDogExporter::new(registry, client, config);
        Ok(DataDogHandle { recorder, handle })
    }
}
