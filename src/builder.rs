use crate::handle::DataDogHandle;
use crate::recorder::DataDogRecorder;
use crate::DataDogMetrics;
use metrics::{Key, Label};
use metrics_util::{Handle, NotTracked, Registry};
use reqwest::Client;
use std::sync::Arc;

pub struct DataDogBuilder {
    write_to_stdout: bool,
    write_to_api: bool,
    api_host: String,
    api_key: Option<String>,
    tags: Vec<Label>,
}

impl DataDogBuilder {
    pub fn default() -> Self {
        DataDogBuilder {
            write_to_stdout: true,
            write_to_api: false,
            api_host: "https://api.datadoghq.com/api/v1".to_string(),
            api_key: None,
            tags: vec![],
        }
    }

    pub fn write_to_stdout(self, b: bool) -> DataDogBuilder {
        DataDogBuilder {
            write_to_stdout: b,
            ..self
        }
    }

    pub fn write_to_api(self, b: bool, api_key: Option<String>) -> DataDogBuilder {
        DataDogBuilder {
            write_to_api: b,
            api_key,
            ..self
        }
    }

    pub fn api_host(self, api_host: String) -> DataDogBuilder {
        DataDogBuilder { api_host, ..self }
    }

    pub fn tags(self, tags: Vec<(String, String)>) -> DataDogBuilder {
        DataDogBuilder {
            tags: tags.iter().map(Label::from).collect(),
            ..self
        }
    }

    pub fn build(&self) -> DataDogMetrics {
        let registry = Arc::new(Registry::<Key, Handle, NotTracked<Handle>>::untracked());
        let recorder = DataDogRecorder::new(registry.clone());
        let handle = DataDogHandle::new(
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
        DataDogMetrics { recorder, handle }
    }
}
