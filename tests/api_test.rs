use anyhow::Result;
use assert_json_diff::{assert_json_matches_no_panic, CompareMode, Config};
use httpmock::Method::POST;
use httpmock::MockServer;
use metrics::histogram;
use metrics_datadog_exporter::data::DataDogSeries;
use metrics_datadog_exporter::DataDogBuilder;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::Read;

#[derive(Debug, Clone, Deserialize, Serialize)]
struct DataDogPost {
    pub series: Vec<DataDogSeries>,
}

#[tokio::test]
// Can only test one at a time
//#[ignore]
async fn write_to_api_compressed_test() -> Result<()> {
    let server = MockServer::start();

    let metrics = DataDogBuilder::default()
        .write_to_stdout(false)
        .write_to_api(true, Some("DUMMY".to_string()))
        .api_host(server.base_url())
        .build()?
        .install()?;

    for i in 0..10 {
        histogram!("metric", i as f64);
    }
    let mock = server.mock(|when, then| {
        when.method(POST).path("/series").matches(|req| {
            let body = req.body.clone().unwrap();
            let mut gz = flate2::read::GzDecoder::new(body.as_slice());
            let mut buffer = Vec::new();
            if let Err(e) = gz.read_to_end(&mut buffer) {
                eprintln!("Invalid payload: {}", e);
                return false;
            }
            let expected = json!({"series":[{"metric":"metric","type":"histogram","tags":[]}]});
            let j: Value = serde_json::from_slice(buffer.as_slice()).expect("");
            if assert_json_matches_no_panic(&j, &expected, Config::new(CompareMode::Inclusive))
                .is_err()
            {
                return false;
            }
            let expected: DataDogPost = serde_json::from_slice(buffer.as_slice()).expect("");
            expected.series.len() == 4
        });

        then.status(200);
    });

    metrics.flush().await?;
    mock.assert_hits(1);
    Ok(())
}

#[tokio::test]
// Can only test one at a time
#[ignore]
async fn write_to_api_uncompressed_test() -> Result<()> {
    let server = MockServer::start();

    let metrics = DataDogBuilder::default()
        .write_to_stdout(false)
        .write_to_api(true, Some("DUMMY".to_string()))
        .api_host(server.base_url())
        .gzip(false)
        .build()?
        .install()?;

    for i in 0..10 {
        histogram!("metric", i as f64);
    }
    let mock = server.mock(|when, then| {
        when.method(POST).path("/series").matches(|req| {
            let body = req.body.clone().unwrap();
            println!("{}", String::from_utf8_lossy(&body));
            let expected = json!({"series":[{"metric":"metric","type":"histogram","tags":[]}]});
            let j: Value = serde_json::from_slice(body.as_slice()).expect("");
            if assert_json_matches_no_panic(&j, &expected, Config::new(CompareMode::Inclusive))
                .is_err()
            {
                return false;
            }
            let expected: DataDogPost = serde_json::from_slice(body.as_slice()).expect("");
            expected.series.len() == 4
        });

        then.status(200);
    });

    metrics.flush().await?;
    mock.assert();
    Ok(())
}
