use axum::{
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use form_state::auth::ecdsa::{ecdsa_auth_middleware, SignatureError};
use serde_json::json;

/// Wrapper middleware that converts SignatureError to proper axum responses
pub async fn auth_middleware(request: Request<axum::body::Body>, next: Next) -> Response {
    match ecdsa_auth_middleware(request, next).await {
        Ok(response) => response,
        Err(signature_error) => {
            // Convert SignatureError to a proper axum response
            let (status, message) = match signature_error {
                SignatureError::MissingSignature => (StatusCode::UNAUTHORIZED, "Missing signature"),
                SignatureError::InvalidSignature => (StatusCode::UNAUTHORIZED, "Invalid signature"),
                SignatureError::InvalidMessage => {
                    (StatusCode::BAD_REQUEST, "Invalid message format")
                }
                SignatureError::RecoveryFailed => {
                    (StatusCode::UNAUTHORIZED, "Failed to recover public key")
                }
                SignatureError::InvalidFormat => {
                    (StatusCode::BAD_REQUEST, "Invalid signature format")
                }
            };

            let body = Json(json!({
                "error": message,
            }));

            (status, body).into_response()
        }
    }
}
