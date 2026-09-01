//! Retries must not replay requests that run inference or create tasks.

use std::time::Duration;

use bentoml::prelude::*;
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Builds a client pointed at `server`, with retries left at the default.
fn client(server: &MockServer) -> Client {
    Client::builder()
        .with_base_url(server.uri())
        .build()
        .expect("the mock server's URI parses")
}

#[tokio::test]
async fn a_failing_post_is_not_replayed() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/summarize"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&server)
        .await;

    let client = client(&server);
    let _ = client.endpoint("summarize").call(&json!({})).await;

    // A replayed POST runs inference a second time, or bills twice.
    let requests = server.received_requests().await.unwrap_or_default();
    assert_eq!(
        requests.len(),
        1,
        "a 5xx POST must be attempted exactly once, got {} attempts",
        requests.len()
    );
}

#[tokio::test]
async fn a_failing_task_submit_is_not_replayed() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/generate/submit"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;

    let client = client(&server);
    let _ = client.task("generate").submit(&json!({})).await;

    // A replayed submit leaves a duplicate task queued server-side.
    let requests = server.received_requests().await.unwrap_or_default();
    assert_eq!(
        requests.len(),
        1,
        "a 5xx submit must be attempted exactly once, got {} attempts",
        requests.len()
    );
}

#[tokio::test]
async fn a_failing_get_is_still_retried() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/readyz"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&server)
        .await;

    let client = client(&server);
    let _ = client.is_ready().await;

    // Idempotent reads keep the transient-failure retries: 1 attempt + 3 retries.
    let requests = server.received_requests().await.unwrap_or_default();
    assert_eq!(
        requests.len(),
        4,
        "a 5xx GET should be retried to the configured limit, got {} attempts",
        requests.len()
    );
}

#[tokio::test]
async fn retries_disabled_means_a_single_attempt() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/readyz"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&server)
        .await;

    let client = Client::builder()
        .with_base_url(server.uri())
        .with_max_retries(0u32)
        .build()
        .expect("the mock server's URI parses");
    let _ = client.is_ready().await;

    let requests = server.received_requests().await.unwrap_or_default();
    assert_eq!(requests.len(), 1, "max_retries = 0 must not retry");
}

#[tokio::test]
async fn polling_is_bounded_by_the_deadline() {
    let server = MockServer::start().await;

    // A server that accepts the connection then stalls well past the deadline.
    Mock::given(method("GET"))
        .and(path("/readyz"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(30)))
        .mount(&server)
        .await;

    let client = client(&server);

    let started = std::time::Instant::now();
    let result = client
        .wait_until_ready(Duration::from_millis(300), Duration::from_millis(50))
        .await;
    let elapsed = started.elapsed();

    assert!(result.is_err(), "a stalled poll must time out");
    // Without bounding the request itself, this would hang for the full 30s.
    assert!(
        elapsed < Duration::from_secs(5),
        "polling overran its 300ms deadline by far too much: {elapsed:?}"
    );
}
