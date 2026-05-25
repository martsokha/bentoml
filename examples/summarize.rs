//! Calls a `summarize` endpoint on a locally running BentoML service.
//!
//! Run with:
//!
//! ```sh
//! cargo run --example summarize
//! ```

use bentoml::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct SummarizeRequest {
    text: String,
}

#[derive(Deserialize)]
struct SummarizeResponse {
    summary: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::builder()
        .with_base_url("http://localhost:3000")
        .build()?;

    if !client.is_ready().await? {
        eprintln!("service is not ready; is it running on :3000?");
        return Ok(());
    }

    let request = SummarizeRequest {
        text: "BentoML is a framework for serving machine learning models.".to_owned(),
    };

    let response: SummarizeResponse = client.call("summarize", &request).await?;
    println!("summary: {}", response.summary);

    Ok(())
}
