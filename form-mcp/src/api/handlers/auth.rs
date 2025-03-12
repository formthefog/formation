// Authentication handlers for the MCP server API
//
// This module contains handlers for authentication-related API endpoints,
// such as login and token validation.

use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use crate::api::handlers::ApiResponse;
use crate::auth::{create_token, verify_token};
use crate::auth::keypair::KeyPair;
use crate::auth::signature::{sign_message, verify_signature};
use crate::errors::AuthError;

/// Request body for login
#[derive(Deserialize, Serialize)]
pub struct LoginRequest {
    /// User ID or address
    pub address: String,
    /// Message signed by the user
    pub signed_message: String,
    /// Signature of the message
    pub signature: String,
}

/// Request body for token validation
#[derive(Deserialize, Serialize)]
pub struct ValidateTokenRequest {
    /// Token to validate
    pub token: String,
}

/// Response for successful login
#[derive(Serialize)]
pub struct LoginResponse {
    /// JWT token for authentication
    pub token: String,
    /// User ID
    pub user_id: String,
    /// Expiration timestamp
    pub expires_at: u64,
    /// Permissions granted to the user
    pub permissions: Vec<String>,
}

/// Handler for login endpoint
pub async fn login(
    req: web::Json<LoginRequest>,
) -> impl Responder {
    // Verify the signature
    // In a real implementation, we would:
    // 1. Check if the address exists in our system
    // 2. Verify the signature using public key recovery
    // 3. Assign roles based on the user's stored information
    
    // For now, we accept any valid Ethereum address and signature format
    // and assign a basic set of roles
    
    let address = req.address.to_string();
    
    // TODO: Implement proper signature verification
    // For now, we assume the signature is valid if it's in the expected format
    if req.signature.len() < 64 {
        return HttpResponse::BadRequest().json(ApiResponse::<()>::error("Invalid signature format"));
    }
    
    // Generate a token with basic permissions
    // In a real implementation, we would load the user's roles from a database
    let roles = vec!["user".to_string()];
    
    // Use a secret key from configuration (using a placeholder for now)
    let secret = b"your-secret-key-which-should-be-very-long-and-complex";
    
    // Create a token valid for 24 hours
    match create_token(&address, roles, secret, 86400) {
        Ok(token) => {
            // Get current timestamp + 24 hours
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            
            let expires_at = now + 86400;
            
            HttpResponse::Ok().json(ApiResponse::success(LoginResponse {
                token,
                user_id: address,
                expires_at,
                permissions: vec!["tools:read".to_string(), "tools:execute".to_string()],
            }))
        },
        Err(err) => {
            HttpResponse::InternalServerError().json(ApiResponse::<()>::error(format!("Failed to create token: {}", err)))
        }
    }
}

/// Handler for token validation endpoint
pub async fn validate_token(
    req: web::Json<ValidateTokenRequest>,
) -> impl Responder {
    match verify_token(&req.token) {
        Ok(auth_data) => {
            HttpResponse::Ok().json(ApiResponse::success(auth_data))
        },
        Err(err) => {
            let status_code = match err {
                AuthError::TokenExpired => 401,
                AuthError::InvalidToken => 401,
                _ => 500,
            };
            
            HttpResponse::build(actix_web::http::StatusCode::from_u16(status_code).unwrap())
                .json(ApiResponse::<()>::error(format!("Token validation failed: {}", err)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};
    
    #[actix_rt::test]
    async fn test_login_handler() {
        // Create a test app
        let app = test::init_service(
            App::new()
                .route("/login", web::post().to(login))
        ).await;
        
        // Create a login request
        let req = test::TestRequest::post()
            .uri("/login")
            .set_json(&LoginRequest {
                address: "0x5a0b54d5dc17e0aadc383d2db43b0a0d3e029c4c".to_string(),
                signed_message: "Test message".to_string(),
                signature: "0x".to_string() + &hex::encode([1u8; 65]),
            })
            .to_request();
        
        // Send the request and get the response
        let resp = test::call_service(&app, req).await;
        
        // Check the response
        assert_eq!(resp.status(), actix_web::http::StatusCode::OK);
    }
    
    #[actix_rt::test]
    async fn test_validate_token_handler() {
        // Create a test app
        let app = test::init_service(
            App::new()
                .route("/validate", web::post().to(validate_token))
        ).await;
        
        // Create an invalid token
        let req = test::TestRequest::post()
            .uri("/validate")
            .set_json(&ValidateTokenRequest {
                token: "invalid-token".to_string(),
            })
            .to_request();
        
        // Send the request and get the response
        let resp = test::call_service(&app, req).await;
        
        // Invalid token should return 401
        assert_eq!(resp.status(), actix_web::http::StatusCode::UNAUTHORIZED);
    }
} 