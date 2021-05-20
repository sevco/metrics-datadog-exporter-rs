use crate::handle::DataDogHandle;
use crate::recorder::DataDogRecorder;
use metrics::SetRecorderError;
use std::time::Duration;
use thiserror::Error;
use tokio::task::JoinHandle;

pub mod builder;
pub mod data;
pub mod handle;
pub mod recorder;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Serialization failed: `{0}`")]
    SerializationError(#[from] serde_json::Error),
    #[error("API Request Failed: `{0}`")]
    ApiError(#[from] reqwest::Error),
}

pub type Result<T, E = Error> = core::result::Result<T, E>;

pub struct DataDogMetrics {
    pub recorder: DataDogRecorder,
    pub handle: DataDogHandle,
}

impl DataDogMetrics {
    pub fn install(self) -> Result<DataDogHandle, SetRecorderError> {
        metrics::set_boxed_recorder(Box::new(self.recorder))?;
        Ok(self.handle)
    }

    pub async fn flush(&self) -> Result<()> {
        self.handle.flush().await
    }

    pub fn schedule(&'static self, interval: Duration) -> JoinHandle<()> {
        self.handle.schedule(interval)
    }
}
