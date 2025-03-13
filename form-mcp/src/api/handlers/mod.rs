// API handlers for the MCP server
//
// This module contains the request handlers for the MCP server API endpoints.
// Handlers process incoming requests and return appropriate responses.

pub mod tools;
pub mod operations;
pub mod auth;

/// Common response structure for API endpoints
#[derive(serde::Serialize)]
pub struct ApiResponse<T> 
where 
    T: serde::Serialize
{
    /// Status of the response (success or error)
    pub status: String,
    /// Response data (if any)
    pub data: Option<T>,
    /// Error message (if any)
    pub message: Option<String>,
}

impl<T> ApiResponse<T> 
where 
    T: serde::Serialize
{
    /// Create a new success response with data
    pub fn success(data: T) -> Self {
        Self {
            status: "success".to_string(),
            data: Some(data),
            message: None,
        }
    }
    
    /// Create a new error response with message
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            status: "error".to_string(),
            data: None,
            message: Some(message.into()),
        }
    }
} 