// Error handling module for the MCP server
//
// This module defines the error types used throughout the MCP server.

use thiserror::Error;
use actix_web::{HttpResponse, ResponseError};
use serde::Serialize;

/// API error response format
#[derive(Serialize)]
pub struct ErrorResponse {
    pub status: String,
    pub message: String,
    pub code: Option<String>,
}

/// Common error types for the MCP server
#[derive(Error, Debug)]
pub enum ServerError {
    #[error("Authentication failed: {0}")]
    Auth(#[from] AuthError),
    
    #[error("Database error: {0}")]
    Database(String),
    
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("Invalid request: {0}")]
    BadRequest(String),
    
    #[error("Not found: {0}")]
    NotFound(String),
    
    #[error("Internal server error: {0}")]
    Internal(String),
    
    #[error("Not implemented: {0}")]
    NotImplemented(String),
    
    #[error("External service error: {0}")]
    ExternalService(String),
}

/// Authentication-specific errors
#[derive(Error, Debug)]
pub enum AuthError {
    #[error("Invalid credentials")]
    InvalidCredentials,
    
    #[error("Signature verification failed")]
    SignatureVerification,
    
    #[error("Token expired")]
    TokenExpired,
    
    #[error("Invalid token format")]
    InvalidToken,
    
    #[error("Missing authentication")]
    MissingAuth,
    
    #[error("Permission denied")]
    PermissionDenied,
    
    #[error("Invalid role: {0}")]
    InvalidRole(String),
    
    #[error("Internal auth error: {0}")]
    Internal(String),
    
    #[error("Not implemented: {0}")]
    NotImplemented(String),
}

/// Tool-specific errors
#[derive(Error, Debug)]
pub enum ToolError {
    #[error("Tool not found: {0}")]
    NotFound(String),
    
    #[error("Tool execution failed: {0}")]
    ExecutionFailed(String),
    
    #[error("Invalid tool parameters: {0}")]
    InvalidParameters(String),
    
    #[error("Tool registration failed: {0}")]
    RegistrationFailed(String),
    
    #[error("Tool operation timed out")]
    Timeout,
}

// Implement ResponseError for ServerError to convert it to HTTP responses
impl ResponseError for ServerError {
    fn error_response(&self) -> HttpResponse {
        match self {
            ServerError::Auth(auth_err) => {
                HttpResponse::Unauthorized().json(ErrorResponse {
                    status: "error".to_string(),
                    message: auth_err.to_string(),
                    code: Some("AUTH_ERROR".to_string()),
                })
            },
            ServerError::BadRequest(msg) => {
                HttpResponse::BadRequest().json(ErrorResponse {
                    status: "error".to_string(),
                    message: msg.clone(),
                    code: Some("BAD_REQUEST".to_string()),
                })
            },
            ServerError::NotFound(msg) => {
                HttpResponse::NotFound().json(ErrorResponse {
                    status: "error".to_string(),
                    message: msg.clone(),
                    code: Some("NOT_FOUND".to_string()),
                })
            },
            ServerError::NotImplemented(msg) => {
                HttpResponse::NotImplemented().json(ErrorResponse {
                    status: "error".to_string(),
                    message: msg.clone(),
                    code: Some("NOT_IMPLEMENTED".to_string()),
                })
            },
            _ => {
                // Internal errors, database errors, etc.
                HttpResponse::InternalServerError().json(ErrorResponse {
                    status: "error".to_string(),
                    message: self.to_string(),
                    code: Some("INTERNAL_ERROR".to_string()),
                })
            }
        }
    }
} 