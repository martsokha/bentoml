# bentoml

An unofficial async Rust client for [BentoML] services.

BentoML services expose their `@bentoml.api` methods as HTTP `POST` endpoints whose
route is derived from the method name. Because endpoints are defined dynamically
per-service, this crate doesn't generate typed bindings — instead it offers a generic
`call` over [`serde`] types: you describe the request and response shapes, and the
client handles serialization, transport, and error mapping.

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
        .call("summarize", &SummarizeRequest { text: "...".into() })
        .await?;

    println!("{}", resp.summary);
    Ok(())
}
```

See [`examples/`](examples/) for runnable examples.

## Capabilities

Beyond the generic `call`, the client implements a set of extension traits (all in
the [prelude]) covering the BentoML HTTP surface:

- [`Readiness`] — `is_ready` / `is_live` health checks and `wait_until_ready`.
- [`Tasks`] — async task queues (`@bentoml.task`): `submit` returns a `TaskHandle`
  for `status` / `get` / `retry` / `cancel`.
- [`Files`] — `multipart/form-data` file inputs, raw-binary root inputs, and binary
  responses.
- [`Streaming`] (feature `stream`) — `stream` returns a `Stream` of response chunks.

## Features

| Feature      | Default | Description                                  |
| ------------ | :-----: | -------------------------------------------- |
| `rustls-tls` |    ✓    | HTTPS via Rustls.                            |
| `native-tls` |         | HTTPS via the platform-native TLS stack.     |
| `stream`     |         | Streaming response endpoints (`Streaming`).  |
| `tracing`    |         | Structured logging over HTTP operations.     |

[prelude]: https://docs.rs/bentoml/latest/bentoml/prelude/index.html
[`Readiness`]: https://docs.rs/bentoml/latest/bentoml/service/trait.Readiness.html
[`Tasks`]: https://docs.rs/bentoml/latest/bentoml/service/trait.Tasks.html
[`Files`]: https://docs.rs/bentoml/latest/bentoml/service/trait.Files.html
[`Streaming`]: https://docs.rs/bentoml/latest/bentoml/service/trait.Streaming.html

## License

Licensed under the [MIT License](LICENSE.txt).

[BentoML]: https://www.bentoml.com
