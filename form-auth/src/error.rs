use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

/// Errors that can occur during authentication
#[derive(Error, Debug)]
pub enum AuthError {
    /// Missing signature in the request
    #[error("Missing signature")]
    MissingSignature,
    
    /// Invalid signature format
    #[error("Invalid signature format")]
    InvalidSignatureFormat,
    
    /// Invalid signature
    #[error("Invalid signature")]
    InvalidSignature,
    
    /// Signature verification failed
    #[error("Signature verification failed")]
    SignatureVerificationFailed,
    
    /// Unauthorized public key
    #[error("Unauthorized public key")]
    UnauthorizedPublicKey,
    
    /// Missing data
    #[error("Missing required data: {0}")]
    MissingData(String),
    
    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),
    
    /// Invalid header
    #[error("Invalid header: {0}")]
    InvalidHeader(String),
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Self::MissingSignature => (StatusCode::UNAUTHORIZED, "Signature is required".to_string()),
            Self::InvalidSignatureFormat => (StatusCode::BAD_REQUEST, "Invalid signature format".to_string()),
            Self::InvalidSignature => (StatusCode::UNAUTHORIZED, "Signature verification failed".to_string()),
            Self::SignatureVerificationFailed => (StatusCode::UNAUTHORIZED, "Signature verification failed".to_string()),
            Self::UnauthorizedPublicKey => (StatusCode::UNAUTHORIZED, "Unauthorized public key".to_string()),
            Self::MissingData(ref data) => (StatusCode::BAD_REQUEST, "Missing required data".to_string()),
            Self::Internal(ref err) => (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string()),
            Self::InvalidHeader(ref header) => (StatusCode::BAD_REQUEST, format!("Invalid header: {}", header)),
        };
        
        // Convert string message to Json for consistent response format
        let body = Json(json!({ "error": message }));
        
        (status, body).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use axum::response::{IntoResponse, Response};
    
    #[test]
    fn test_auth_error_display() {
        let errors = vec![
            (AuthError::InvalidSignature, "Invalid signature"),
            (AuthError::MissingSignature, "Missing signature"),
            (AuthError::SignatureVerificationFailed, "Signature verification failed"),
            (AuthError::InvalidHeader("test".to_string()), "Invalid header: test"),
            (AuthError::MissingData("test".to_string()), "Missing required data: test"),
            (AuthError::Internal("test error".to_string()), "Internal error: test error"),
        ];
        
        for (error, expected_message) in errors {
            assert_eq!(error.to_string(), expected_message);
        }
    }
    
    #[test]
    fn test_auth_error_into_response() {
        // Test specific error types map to expected status codes
        let errors_and_codes = vec![
            (AuthError::InvalidSignature, StatusCode::UNAUTHORIZED),
            (AuthError::MissingSignature, StatusCode::UNAUTHORIZED),
            (AuthError::SignatureVerificationFailed, StatusCode::UNAUTHORIZED),
            (AuthError::InvalidHeader("test".to_string()), StatusCode::BAD_REQUEST),
            (AuthError::MissingData("test".to_string()), StatusCode::BAD_REQUEST),
            (AuthError::Internal("test".to_string()), StatusCode::INTERNAL_SERVER_ERROR),
        ];
        
        for (error, expected_code) in errors_and_codes {
            let response: Response = error.into_response();
            assert_eq!(response.status(), expected_code);
        }
    }
    
    #[test]
    fn test_auth_error_json_response() {
        // Test that the response contains JSON with error details
        let error = AuthError::InvalidSignature;
        let response = error.into_response();
        
        // Check status code
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        
        // We can't easily test body contents in these tests since
        // it would require async context to extract the body
        // This would be better tested in integration tests
    }
} 