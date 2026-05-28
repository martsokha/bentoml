//! Submits a long-running task and polls it to completion.
//!
//! Run with:
//!
//! ```sh
//! cargo run --example task
//! ```

use std::time::Duration;

use bentoml::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct GenerateRequest {
    prompt: String,
}

#[derive(Deserialize)]
struct GenerateResponse {
    url: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::builder()
        .with_base_url("http://localhost:3000")
        .build()?;

    let request = GenerateRequest {
        prompt: "a bento box, watercolor".to_owned(),
    };

    let task = client.task("generate").submit(&request).await?;
    println!("submitted task {}", task.task_id());

    // Poll until the task reaches a terminal state, sleeping 2s between checks.
    let status = task
        .wait(
            Duration::from_secs(300),
            Duration::from_secs(2),
            tokio::time::sleep,
        )
        .await?;
    println!("status: {status:?}");

    let result: GenerateResponse = task.json().await?;
    println!("result url: {}", result.url);

    Ok(())
}
