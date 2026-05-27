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

    let task = client.endpoint("generate").submit(&request).await?;
    println!("submitted task {}", task.task_id());

    loop {
        let status = task.status().await?;
        println!("status: {status:?}");
        if status.is_terminal() {
            break;
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }

    let result: GenerateResponse = task.json().await?;
    println!("result url: {}", result.url);

    Ok(())
}
