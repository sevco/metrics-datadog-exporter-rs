extern crate core;

use anyhow::Result;
use assert_json_diff::{CompareMode, Config};
use httpmock::Method::POST;
use httpmock::MockServer;
use metrics::counter;
use metrics_datadog_exporter::DataDogBuilder;
use serde_json::{json, Value};
use std::io::Read;

#[tokio::test]
async fn write_to_api_test() -> Result<()> {
    let server = MockServer::start();

    let metrics = DataDogBuilder::default()
        .write_to_stdout(false)
        .write_to_api(true, Some("DUMMY".to_string()))
        .api_host(server.base_url())
        .build()
        .install()?;

    counter!("metric", 1);
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/series")
            .header("Content-Type", "application/json")
            .header("Content-Encoding", "gzip")
            .matches(|req| {
                let body = req.body.clone().unwrap();
                let mut gz = flate2::read::GzDecoder::new(body.as_slice());
                let mut buffer = Vec::new();
                gz.read_to_end(&mut buffer).expect("");

                let j: Value = serde_json::from_slice(buffer.as_slice()).expect("");
                let expected = json!({"series":[{"resources":[{"type": "host", "name": "lambda"}],"metric":"metric","type":1}]});
                assert_json_diff::assert_json_matches!(
                    j,
                    expected,
                    Config::new(CompareMode::Inclusive)
                );
                true
            });
        then.status(200);
    });

    metrics.flush().await?;
    mock.assert();
    Ok(())
}
