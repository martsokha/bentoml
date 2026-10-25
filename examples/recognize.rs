//! Calls a batch `recognize` endpoint whose handler takes a single list parameter.
//!
//! Mirrors a handler like `recognize(self, requests: list[NerRequest])`. BentoML
//! keys the input on the parameter name, so the batch is sent as
//! `{"requests": [...]}` with `Input::named`, not as a bare array. Run with:
//!
//! ```sh
//! cargo run --example recognize
//! ```

use bentoml::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct NerRequest {
    text: String,
    threshold: f32,
}

#[derive(Deserialize, Debug)]
struct Entity {
    text: String,
    label: String,
}

#[derive(Deserialize, Debug)]
struct NerResponse {
    entities: Vec<Entity>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::builder()
        .with_base_url("http://localhost:3000")
        .build()?;

    let requests = vec![
        NerRequest {
            text: "BentoML was founded in San Francisco.".to_owned(),
            threshold: 0.3,
        },
        NerRequest {
            text: "Rust is developed by the Rust Foundation.".to_owned(),
            threshold: 0.3,
        },
    ];

    // `requests` names the handler parameter that receives the batch.
    let responses: Vec<NerResponse> = client
        .endpoint("recognize")
        .invoke(Input::named("requests", &requests))
        .await?;

    for (request, response) in requests.iter().zip(&responses) {
        println!("{}", request.text);
        for entity in &response.entities {
            println!("  {:20} {}", entity.text, entity.label);
        }
    }

    Ok(())
}
