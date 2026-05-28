# bentoml

[![Build](https://img.shields.io/github/actions/workflow/status/martsokha/bentoml/build.yml?branch=main&label=build%20%26%20test&style=flat-square)](https://github.com/martsokha/bentoml/actions/workflows/build.yml)
[![Crate](https://img.shields.io/crates/v/bentoml.svg?style=flat-square)](https://crates.io/crates/bentoml)
[![Docs](https://img.shields.io/docsrs/bentoml?style=flat-square)](https://docs.rs/bentoml)

An unofficial async Rust client for [BentoML] services ([GitHub][bentoml-gh]).

BentoML services expose their `@bentoml.api` methods as HTTP `POST` endpoints whose
route is derived from the method name. Because endpoints are defined dynamically
per-service, this crate doesn't generate typed bindings: instead you name a route
with `client.endpoint(route)` (or `client.task(route)` for an `@bentoml.task`) and
call it over your own `serde` types.

## Features

- **Typed calls**: `endpoint(route).invoke(&payload)` over your own `serde` request
  and response types, with no codegen or per-service bindings.
- **Async task queues**: `client.task(route)` submits `@bentoml.task` jobs, then poll
  status, fetch results, retry, or cancel through a `TaskHandle`. The synchronous and
  task surfaces are distinct handle types, so `call` and `submit` can't be mixed.
- **File and streaming I/O**: `multipart/form-data` file inputs, raw-binary root
  inputs, binary responses, and chunked streaming endpoints (feature `stream`).
- **Resilient transport**: exponential-backoff retries via `reqwest-middleware`,
  bearer-token auth, an optional per-request timeout, and a cheap-to-clone
  `Arc`-backed client.

## Usage

Add the dependency:

```toml
[dependencies]
bentoml = "0.4"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
serde = { version = "1", features = ["derive"] }
```

The minimum supported Rust version (MSRV) is **1.91**.

```rust,no_run
use bentoml::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct SummarizeRequest { text: String }

#[derive(Deserialize)]
struct SummarizeResponse { summary: String }

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::builder()
        .with_base_url("http://localhost:3000")
        .build()?;

    let resp: SummarizeResponse = client
        .endpoint("summarize")
        .invoke(&SummarizeRequest { text: "...".into() })
        .await?;

    println!("{}", resp.summary);
    Ok(())
}
```

A handle names the route once; calls are made on it. See [`examples/`](examples/) for
runnable examples.

## Capabilities

The handle's kind mirrors the BentoML decorator, and decides which methods are
available — `call` is not callable on a task handle, nor `submit` on an api handle.

A `Client::endpoint(route)` handle (`@bentoml.api`) covers the synchronous surface:

- `call` / `call_bytes` / `call_multipart`: send a JSON, raw-binary, or
  `multipart/form-data` body (built with `multipart::Multipart`), returning an
  `EndpointReply` you read as `.json::<R>()`, `.bytes()`, `.text()`, or (feature
  `stream`) `.stream()` — so input and output encodings are chosen independently.
- `invoke`: the JSON-in, JSON-out shorthand — `invoke(&p)` deserializes the response
  for you, equivalent to `call(&p).await?.json().await?`.

A `Client::task(route)` handle (`@bentoml.task`) covers the async task surface:

- `submit` / `submit_bytes` / `submit_multipart`: submit a JSON, raw-binary, or
  `multipart/form-data` task input; return a `TaskHandle` for `status`, `wait`,
  `retry`, `cancel`, and a result read as `json::<R>()` / `bytes()` / `text()`.

`EndpointReply::stream()` yields a `ByteStream` of response chunks; decode it with
`.text()`, `.lines()`, or `.json::<T>()`.

The `Client` itself provides health checks: `is_ready` / `is_live` and
`wait_until_ready`.

These are gated by feature flags:

- `rustls-tls` *(default)*: HTTPS via Rustls.
- `native-tls`: HTTPS via the platform-native TLS stack.
- `stream`: response streaming via `EndpointReply::stream`.
- `tracing`: `#[tracing::instrument]` spans on request methods, including any
  `x-request-id` as a `request_id` field.

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for release notes and version history.

## License

Licensed under the [MIT License](LICENSE.txt).

[BentoML]: https://www.bentoml.com
[bentoml-gh]: https://github.com/bentoml/BentoML
