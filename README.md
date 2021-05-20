# metrics-datadog-exporter

![Crates.io](https://img.shields.io/crates/v/metrics-datadog-exporter)
![GitHub Workflow Status](https://img.shields.io/github/workflow/status/sevco/metrics-datadog-exporter-rs/CI)

### Metrics reporter for https://github.com/metrics-rs/metrics that writes to DataDog.

## Usage

### Writing to stdout

```rust
#[tokio::main]
async fn main() {
    let reporter = DataDogBuilder::default()
        .tags(vec![
            "tag1".to_string(),
            "val1".to_string()
        ])
        .build()
        .install()
        .unwrap();
    reporter.flush.await()?;
}
```

### Writing to API
```rust
#[tokio::main]
async fn main() {
    let reporter = DataDogBuilder::default()
        .write_to_stdout(false)
        .write_to_api(true, Some("DD_API_KEY".to_string()))
        .tags(vec![
            "tag1".to_string(),
            "val1".to_string()
        ])
        .build()
        .install()
        .unwrap();
    reporter.flush.await()?;
}
```

### Writing on as schedule
```rust
use once_cell::sync::Lazy;

static DD_METRICS: Lazy<DataDogHandle> = Lazy::new(|| {
    DataDogBuilder::default()
        .tags(vec![
            "tag1".to_string(),
            "val1".to_string()
        ])
        .build()
        .install()
        .unwrap();
});

#[tokio::main]
async fn main() {
    let reporter = DD_METRICS.deref();
    reporter.schedule(Duration::from_secs(10));
}
```