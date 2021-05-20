use metrics::{GaugeValue, Key, Recorder, SetRecorderError, Unit};
use metrics_util::{Handle, MetricKind, NotTracked, Registry};
use std::sync::Arc;

pub struct DataDogRecorder {
    registry: Arc<Registry<Key, Handle, NotTracked<Handle>>>,
}

impl DataDogRecorder {
    pub(crate) fn new(registry: Arc<Registry<Key, Handle, NotTracked<Handle>>>) -> Self {
        DataDogRecorder { registry }
    }

    pub fn install(self) -> Result<(), SetRecorderError> {
        log::info!("Setting DataDog metric recorder");
        metrics::set_boxed_recorder(Box::new(self))?;
        Ok(())
    }
}

impl Recorder for DataDogRecorder {
    fn register_counter(&self, key: &Key, _unit: Option<Unit>, _description: Option<&'static str>) {
        self.registry
            .op(MetricKind::Counter, key, |_| {}, Handle::counter);
    }

    fn register_gauge(&self, key: &Key, _unit: Option<Unit>, _description: Option<&'static str>) {
        self.registry
            .op(MetricKind::Gauge, key, |_| {}, Handle::gauge);
    }

    fn register_histogram(
        &self,
        key: &Key,
        _unit: Option<Unit>,
        _description: Option<&'static str>,
    ) {
        self.registry
            .op(MetricKind::Histogram, key, |_| {}, Handle::histogram);
    }

    fn increment_counter(&self, key: &Key, value: u64) {
        self.registry.op(
            MetricKind::Counter,
            key,
            |c| c.increment_counter(value),
            Handle::counter,
        );
    }

    fn update_gauge(&self, key: &Key, value: GaugeValue) {
        self.registry.op(
            MetricKind::Gauge,
            key,
            |g| g.update_gauge(value),
            Handle::gauge,
        );
    }

    fn record_histogram(&self, key: &Key, value: f64) {
        self.registry.op(
            MetricKind::Histogram,
            key,
            |g| g.record_histogram(value),
            Handle::histogram,
        );
    }
}
