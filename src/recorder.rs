use std::sync::Arc;

use metrics::{Counter, Gauge, Histogram, Key, KeyName, Recorder, Unit};
use metrics_util::Registry;

/// Metric recorder
pub struct DataDogRecorder {
    registry: Arc<Registry>,
}

impl DataDogRecorder {
    pub(crate) fn new(registry: Arc<Registry>) -> Self {
        DataDogRecorder { registry }
    }
}

impl Recorder for DataDogRecorder {
    fn describe_counter(&self, _: KeyName, _: Option<Unit>, _: &'static str) {
        unreachable!("Descriptions are not supported")
    }

    fn describe_gauge(&self, _: KeyName, _: Option<Unit>, _: &'static str) {
        unreachable!("Descriptions are not supported")
    }

    fn describe_histogram(&self, _: KeyName, _: Option<Unit>, _: &'static str) {
        unreachable!("Descriptions are not supported")
    }

    fn register_counter(&self, key: &Key) -> Counter {
        self.registry
            .get_or_create_counter(key, |c| c.clone().into())
    }

    fn register_gauge(&self, key: &Key) -> Gauge {
        self.registry.get_or_create_gauge(key, |c| c.clone().into())
    }

    fn register_histogram(&self, key: &Key) -> Histogram {
        self.registry
            .get_or_create_histogram(key, |c| c.clone().into())
    }

    // fn register_counter(&self, key: &Key, _unit: Option<Unit>, _description: Option<&'static str>) {
    //     self.registry
    //         .op(MetricKind::Counter, key, |_| {}, Handle::counter);
    // }
    //
    // fn register_gauge(&self, key: &Key, _unit: Option<Unit>, _description: Option<&'static str>) {
    //     self.registry
    //         .op(MetricKind::Gauge, key, |_| {}, Handle::gauge);
    // }
    //
    // fn register_histogram(
    //     &self,
    //     key: &Key,
    //     _unit: Option<Unit>,
    //     _description: Option<&'static str>,
    // ) {
    //     self.registry
    //         .op(MetricKind::Histogram, key, |_| {}, Handle::histogram);
    // }
    //
    // fn increment_counter(&self, key: &Key, value: u64) {
    //     self.registry.op(
    //         MetricKind::Counter,
    //         key,
    //         |c| c.increment_counter(value),
    //         Handle::counter,
    //     );
    // }
    //
    // fn update_gauge(&self, key: &Key, value: GaugeValue) {
    //     self.registry.op(
    //         MetricKind::Gauge,
    //         key,
    //         |g| g.update_gauge(value),
    //         Handle::gauge,
    //     );
    // }
    //
    // fn record_histogram(&self, key: &Key, value: f64) {
    //     self.registry.op(
    //         MetricKind::Histogram,
    //         key,
    //         |g| g.record_histogram(value),
    //         Handle::histogram,
    //     );
    // }
}
