use anyhow::Result;
use httpmock::Method::POST;
use httpmock::MockServer;
use metrics_datadog_exporter::DataDogBuilder;
use metrics_macros::counter;

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
        when.method(POST).path("/series").json_body_partial(
            r#"
            {
                "series": [
                    {
                        "host": "lambda",
                        "metric": "metric",
                        "type": "count"
                    }
                ]
            }
            "#,
        );
        then.status(200);
    });

    metrics.flush().await?;
    mock.assert();
    Ok(())
}
