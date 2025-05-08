use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthConfig {
    pub jwks_url: String,
    pub audience: String,
    pub issuer: String,
    pub api_gateway_url: String,
    pub auth_services_url: String,
}

impl AuthConfig {
    pub fn from_env() -> Self {
        let jwks_url = std::env::var("JWKS_URL")
            .unwrap_or_else(|_| "https://auth.formation.dev/.well-known/jwks.json".to_string());
        
        let audience = std::env::var("AUTH_AUDIENCE")
            .unwrap_or_else(|_| "https://api.formation.dev".to_string());
        
        let issuer = std::env::var("AUTH_ISSUER")
            .unwrap_or_else(|_| "https://auth.formation.dev/".to_string());
        
        let api_gateway_url = std::env::var("API_GATEWAY_URL")
            .unwrap_or_else(|_| "https://api.formation.dev".to_string());
        
        let auth_services_url = std::env::var("AUTH_SERVICES_URL")
            .unwrap_or_else(|_| "https://auth.formation.dev".to_string());
        
        Self {
            jwks_url,
            audience,
            issuer,
            api_gateway_url,
            auth_services_url,
        }
    }
} 