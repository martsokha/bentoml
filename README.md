# bentoml

[![Build](https://img.shields.io/github/actions/workflow/status/martsokha/bentoml/build.yml?branch=main&label=build%20%26%20test&style=flat-square)](https://github.com/martsokha/bentoml/actions/workflows/build.yml)
[![Crate](https://img.shields.io/crates/v/bentoml.svg?style=flat-square)](https://crates.io/crates/bentoml)
[![Docs](https://img.shields.io/docsrs/bentoml?style=flat-square)](https://docs.rs/bentoml)

An unofficial async Rust client for [BentoML] services ([GitHub][bentoml-gh]).

BentoML services expose their `@bentoml.api` methods as HTTP `POST` endpoints whose
route is derived from the method name. Because endpoints are defined dynamically
per-service, this crate doesn't generate typed bindings: instead it offers a generic
`call` over `serde` types, plus extension traits for the rest of the HTTP surface.

## Features

- **Generic calls**: invoke any endpoint with `call(route, payload)` over your own
  `serde` request and response types, with no codegen or per-service bindings.
- **Async task queues**: submit `@bentoml.task` jobs and poll status, fetch results,
  retry, or cancel through a `TaskHandle`.
- **File and streaming I/O**: `multipart/form-data` file inputs, raw-binary root
  inputs, binary responses, and chunked streaming endpoints (feature `stream`).
- **Resilient transport**: per-request timeouts and exponential-backoff retries via
  `reqwest-middleware`, bearer-token auth, and a cheap-to-clone `Arc`-backed client.

## Usage

Add the dependency:

```toml
[dependencies]
bentoml = "0.1"
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
        .call(&SummarizeRequest { text: "...".into() })
        .await?;

    println!("{}", resp.summary);
    Ok(())
}
```

A `Client::endpoint(route)` handle names the route once; calls are made on it. See
[`examples/`](examples/) for runnable examples.

## Capabilities

A `Client::endpoint(route)` handle covers the BentoML HTTP surface:

- `call`: the common JSON-in, JSON-out request.
- `call_json` / `call_bytes` / `call_multipart`: send a JSON, raw-binary, or
  `multipart/form-data` body (built with `Multipart`), returning a `Response` you
  read as `.json::<R>()`, `.bytes()`, or `.text()` — so input and output encodings
  are chosen independently.
- `submit`: async task queues (`@bentoml.task`); returns a `TaskHandle` for
  `status` / `get` / `retry` / `cancel`.
- `Streaming` trait: `stream` returns a `ByteStream` of response chunks; decode it
  with `.text()`, `.lines()`, or `.json::<T>()` (feature `stream`).

The `Streaming` trait lives in the prelude. The `Client` itself provides health
checks: `is_ready` / `is_live` and `wait_until_ready`.

These are gated by feature flags:

- `rustls-tls` *(default)*: HTTPS via Rustls.
- `native-tls`: HTTPS via the platform-native TLS stack.
- `stream`: streaming response endpoints (`Streaming`).
- `tracing`: `#[tracing::instrument]` on request methods.

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for release notes and version history.

## License

Licensed under the [MIT License](LICENSE.txt).

[BentoML]: https://www.bentoml.com
[bentoml-gh]: https://github.com/bentoml/BentoML
