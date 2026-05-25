//! Consumes a streaming endpoint chunk by chunk.
//!
//! Run with:
//!
//! ```sh
//! cargo run --example stream --features stream
//! ```

use bentoml::prelude::*;
use serde::Serialize;
use tokio_stream::StreamExt;

#[derive(Serialize)]
struct ChatRequest {
    prompt: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::builder()
        .with_base_url("http://localhost:3000")
        .build()?;

    let request = ChatRequest {
        prompt: "Tell me about BentoML.".to_owned(),
    };

    let mut stream = client.endpoint("chat").stream(&request).await?;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        print!("{}", String::from_utf8_lossy(&chunk));
    }
    println!();

    Ok(())
}
