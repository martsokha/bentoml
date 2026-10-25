//! Each JSON input method encodes the body the way BentoML keys it.

use bentoml::prelude::*;
use serde::Serialize;
use serde_json::{Value, json};
use wiremock::matchers::{body_bytes, body_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Builds a client pointed at `server`.
fn client(server: &MockServer) -> Client {
    Client::builder()
        .with_base_url(server.uri())
        .build()
        .expect("the mock server's URI parses")
}

#[tokio::test]
async fn a_single_parameter_is_keyed_on_its_name() {
    let server = MockServer::start().await;

    // def recognize(self, requests: list[NerRequest]) keys its input on `requests`.
    Mock::given(method("POST"))
        .and(path("/recognize"))
        .and(body_json(
            json!({"requests": [{"text": "a"}, {"text": "b"}]}),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([1, 2])))
        .expect(1)
        .mount(&server)
        .await;

    #[derive(Serialize)]
    struct Recognize {
        requests: Vec<Value>,
    }

    let input = Recognize {
        requests: vec![json!({"text": "a"}), json!({"text": "b"})],
    };
    let resp: Value = client(&server)
        .endpoint("recognize")
        .invoke(&input)
        .await
        .expect("the keyed body matches the mock");
    assert_eq!(resp, json!([1, 2]));
}

#[tokio::test]
async fn a_task_input_is_keyed_on_parameter_names() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/generate/submit"))
        .and(body_json(json!({"prompts": ["a", "b"]})))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"task_id": "t-1", "status": "in_progress"})),
        )
        .expect(1)
        .mount(&server)
        .await;

    let task = client(&server)
        .task("generate")
        .submit(&json!({"prompts": ["a", "b"]}))
        .await
        .expect("the keyed body matches the mock");
    assert_eq!(task.task_id(), "t-1");
}

#[tokio::test]
async fn a_root_input_sends_the_value_bare() {
    let server = MockServer::start().await;

    // def recognize(self, requests: list[NerRequest], /) takes the bare value.
    Mock::given(method("POST"))
        .and(path("/recognize"))
        .and(body_json(json!([{"text": "a"}])))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([1])))
        .expect(1)
        .mount(&server)
        .await;

    let resp: Value = client(&server)
        .endpoint("recognize")
        .invoke(Input::root(&[json!({"text": "a"})]))
        .await
        .expect("the bare body matches the mock");
    assert_eq!(resp, json!([1]));
}

#[tokio::test]
async fn a_non_object_input_is_rejected_before_sending() {
    let server = MockServer::start().await;

    let err = client(&server)
        .endpoint("recognize")
        .call(&[json!({"text": "a"})])
        .await
        .expect_err("an array is not an input model");
    assert!(matches!(err, Error::InvalidRequest { .. }), "{err:?}");

    let err = client(&server)
        .task("generate")
        .submit(&"a")
        .await
        .expect_err("a string is not an input model");
    assert!(matches!(err, Error::InvalidRequest { .. }), "{err:?}");

    let requests = server.received_requests().await.unwrap_or_default();
    assert!(requests.is_empty(), "nothing reaches the service");
}

#[tokio::test]
async fn a_bytes_input_is_sent_raw_and_the_reply_decoded() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/embed"))
        .and(body_bytes(b"\xff\xd8".to_vec()))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([0.5])))
        .expect(1)
        .mount(&server)
        .await;

    let resp: Vec<f32> = client(&server)
        .endpoint("embed")
        .invoke(Input::bytes(&b"\xff\xd8"[..]))
        .await
        .expect("the raw body matches the mock");
    assert_eq!(resp, [0.5]);
}

#[tokio::test]
async fn a_per_call_content_type_is_not_duplicated() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/summarize"))
        .and(header("content-type", "application/vnd.custom+json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .expect(1)
        .mount(&server)
        .await;

    let _: Value = client(&server)
        .endpoint("summarize")
        .with_header("content-type", "application/vnd.custom+json")
        .invoke(&json!({"text": "a"}))
        .await
        .expect("the per-call content type is kept");

    let requests = server.received_requests().await.unwrap_or_default();
    let values: Vec<_> = requests[0].headers.get_all("content-type").iter().collect();
    assert_eq!(values.len(), 1, "one content type, got {values:?}");
}

/// Call and submit futures stay `Send`, so they can be spawned.
#[allow(dead_code)]
fn futures_are_send(client: Client) {
    fn send<F: Send>(_: F) {}
    let payload = json!({});
    send(client.endpoint("a").call(&payload));
    send(client.endpoint("a").invoke::<Value>(Input::root(&payload)));
    send(client.task("a").submit(Input::root(&payload)));
}
