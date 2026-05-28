//! Calls a `classify` endpoint with a mixed `multipart/form-data` body: a JSON
//! parameter plus an image file part.
//!
//! Mirrors a BentoML service like `classify(self, top_k: int, image: PIL.Image)`,
//! where each parameter becomes its own form field. Run with:
//!
//! ```sh
//! cargo run --example classify
//! ```

use std::fs;

use bentoml::prelude::*;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct Label {
    name: String,
    score: f64,
}

#[derive(Deserialize, Debug)]
struct Classification {
    labels: Vec<Label>,
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

    // Read the image bytes however you like; the crate does no file I/O itself.
    let image = fs::read("image.jpg").unwrap_or_else(|_| {
        eprintln!("image.jpg not found; sending placeholder bytes");
        b"\xff\xd8\xff".to_vec()
    });

    // Each parameter is its own form field: `top_k` is JSON-encoded, `image` is a
    // file part — matching how the service's parameters are named.
    let body = Multipart::new().field("top_k", &3).part(
        "image",
        Part::new(image).file_name("image.jpg").mime("image/jpeg"),
    );

    let response = client.endpoint("classify").call_multipart(body).await?;
    let result: Classification = response.json().await?;

    for label in &result.labels {
        println!("{:20} {:.3}", label.name, label.score);
    }

    Ok(())
}
