use axum::{routing::post, Json, Router};
use serde::{Deserialize, Serialize};

// Dummy types
#[derive(Debug, Serialize, Deserialize)]
struct Embedding {
    embedding: Vec<f64>,
    index: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct EmbeddingResponse {
    model: String,
    data: Vec<Embedding>,
    usage: Usage,
}

#[derive(Debug, Serialize, Deserialize)]
struct Usage {
    prompt_tokens: usize,
    total_tokens: usize,
}

pub fn router() -> Router {
    Router::new().route("/embeddings", post(create_embedding))
}

// Handler functions with minimal implementations
async fn create_embedding() -> Json<EmbeddingResponse> {
    Json(EmbeddingResponse {
        model: "text-embedding-ada-002".to_string(),
        data: vec![Embedding {
            embedding: vec![0.1, 0.2, 0.3, 0.4, 0.5],
            index: 0,
        }],
        usage: Usage {
            prompt_tokens: 5,
            total_tokens: 5,
        },
    })
}
