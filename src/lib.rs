#![warn(missing_docs)]

//! Exports any metrics to DataDog

use metrics::SetRecorderError;
use std::io;
use std::time::Duration;
use thiserror::Error;
use tokio::task::JoinHandle;

mod builder;
pub use crate::builder::DataDogBuilder;
pub mod data;
pub use crate::data::DataDogMetric;
pub use crate::data::DataDogMetricType;
pub use crate::data::DataDogMetricValue;
pub use metrics;
pub mod exporter;
pub use crate::exporter::DataDogExporter;
mod recorder;
pub use crate::recorder::DataDogRecorder;

/// Error handling metrics
#[derive(Error, Debug)]
pub enum Error {
    /// Error when serializing metric to JSON
    #[error("Serialization failed: `{0}`")]
    SerializationError(#[from] serde_json::Error),
    /// Error when interacting with DataDog API
    #[error("API Request Failed: `{0}`")]
    ApiError(#[from] reqwest::Error),
    /// Error compressing or decompressing
    #[error("IO error: `{0}`")]
    IOError(#[from] io::Error),
}

/// [`Ok`] or [`enum@Error`]
pub type Result<T, E = Error> = core::result::Result<T, E>;

/// Handle to metrics
pub struct DataDogHandle {
    /// Metric recorder
    pub recorder: DataDogRecorder,
    /// Metric exporter
    pub handle: DataDogExporter,
}

impl DataDogHandle {
    /// Install [`DataDogRecorder`] and return [`DataDogExporter`]
    pub fn install(self) -> Result<DataDogExporter, SetRecorderError> {
        metrics::set_boxed_recorder(Box::new(self.recorder))?;
        Ok(self.handle)
    }

    /// Flush metrics
    pub async fn flush(&self) -> Result<()> {
        self.handle.flush().await
    }

    /// Write metrics every [`Duration`]
    pub fn schedule(&'static self, interval: Duration) -> JoinHandle<()> {
        self.handle.schedule(interval)
    }
}
