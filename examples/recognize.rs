//! Calls a batch `recognize` endpoint whose handler takes a single list parameter.
//!
//! Mirrors a handler like `recognize(self, requests: list[NerRequest])`. BentoML
//! keys the input on the parameter name, so the batch goes in a struct with a
//! `requests` field and is sent as `{"requests": [...]}`, not as a bare array. Run
//! with:
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

/// The handler's parameters: here, just `requests`.
#[derive(Serialize)]
struct Recognize<'a> {
    requests: &'a [NerRequest],
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

    let responses: Vec<NerResponse> = client
        .endpoint("recognize")
        .invoke(&Recognize {
            requests: &requests,
        })
        .await?;

    for (request, response) in requests.iter().zip(&responses) {
        println!("{}", request.text);
        for entity in &response.entities {
            println!("  {:20} {}", entity.text, entity.label);
        }
    }

    Ok(())
}
