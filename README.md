# metrics-datadog-exporter

![Crates.io](https://img.shields.io/crates/v/metrics-datadog-exporter)
![docs.rs](https://docs.rs/metrics-datadog-exporter/badge.svg)
![GitHub Workflow Status](https://img.shields.io/github/workflow/status/sevco/metrics-datadog-exporter-rs/CI)

### Metrics reporter for https://github.com/metrics-rs/metrics that writes to DataDog.

## Usage

### Writing to stdout

```rust
#[tokio::main]
async fn main() {
    let exporter = DataDogBuilder::default()
        .tags(vec![
            "tag1".to_string(),
            "val1".to_string()
        ])
        .build()
        .install()
        .unwrap();
    exporter.flush.await()?;
}
```

### Writing to API
```rust
#[tokio::main]
async fn main() {
    let exporter = DataDogBuilder::default()
        .write_to_stdout(false)
        .write_to_api(true, Some("DD_API_KEY".to_string()))
        .tags(vec![
            "tag1".to_string(),
            "val1".to_string()
        ])
        .build()
        .install()
        .unwrap();
    exporter.flush.await()?;
}
```

### Writing on a schedule
```rust
#[tokio::main]
async fn main() {
    let exporter = DataDogBuilder::default()
        .write_to_stdout(false)
        .write_to_api(true, Some("DD_API_KEY".to_string()))
        .tags(vec![
            "tag1".to_string(),
            "val1".to_string()
        ])
        .build()
        .install()
        .unwrap();
    let (_exporter, _scheduled) = expoerter.schedule(Duration::from_secs(10));
}
```