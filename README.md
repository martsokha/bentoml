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

```rust
use bentoml::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct SummarizeRequest { text: String }

#[derive(Deserialize)]
struct SummarizeResponse { summary: String }

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::builder()
        .base_url("http://localhost:3000")?
        .build()?;

    let resp: SummarizeResponse = client
        .call("summarize", &SummarizeRequest { text: "...".into() })
        .await?;

    println!("{}", resp.summary);
    Ok(())
}
```

See [`examples/`](examples/) for a runnable example.

## Features

| Feature      | Default | Description                                  |
| ------------ | :-----: | -------------------------------------------- |
| `rustls-tls` |    ✓    | HTTPS via Rustls.                            |
| `native-tls` |         | HTTPS via the platform-native TLS stack.     |
| `tracing`    |         | Structured logging over HTTP operations.     |

## License

Licensed under the [MIT License](LICENSE.txt).

[BentoML]: https://www.bentoml.com
