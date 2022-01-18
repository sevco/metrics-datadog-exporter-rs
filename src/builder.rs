use std::sync::Arc;

use metrics::Label;
use metrics_util::Registry;
use reqwest::Client;

use crate::exporter::DataDogExporter;
use crate::recorder::DataDogRecorder;
use crate::DataDogHandle;

/// Builder for creating/installing a DataDog recorder/exporter
pub struct DataDogBuilder {
    write_to_stdout: bool,
    write_to_api: bool,
    api_host: String,
    api_key: Option<String>,
    tags: Vec<Label>,
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

    /// Build [`DataDogHandle`]
    pub fn build(&self) -> DataDogHandle {
        let registry = Arc::new(Registry::new());
        let recorder = DataDogRecorder::new(registry.clone());
        let handle = DataDogExporter::new(
            registry,
            self.write_to_stdout,
            self.write_to_api,
            self.api_host.clone(),
            if self.write_to_api {
                Some(Client::default())
            } else {
                None
            },
            self.api_key.clone(),
            self.tags.clone(),
        );
        DataDogHandle { recorder, handle }
    }
}
