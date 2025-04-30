use std::sync::RwLock;
use std::collections::HashMap;
use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};
use reqwest::{Client, header};
use serde::Deserialize;
use serde_json::Value;
use super::claims::JwtClaims;
use super::config::AuthConfig;

/// JWKS keys from an auth server
#[derive(Debug, Deserialize)]
struct JwksResponse {
    keys: Vec<Jwk>,
}

/// A single JSON Web Key
#[derive(Debug, Deserialize)]
struct Jwk {
    kid: String,
    alg: String,
    #[serde(rename = "use")]
    usage: String,
    n: String,
    e: String,
    kty: String,
}

/// JWT Client for validating tokens and making interservice calls
pub struct JwtClient {
    http_client: Client,
    config: AuthConfig,
    keys_cache: RwLock<HashMap<String, DecodingKey>>,
}

impl JwtClient {
    pub fn new(config: AuthConfig) -> Self {
        let http_client = Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("Failed to build HTTP client");
        
        Self {
            http_client,
            config,
            keys_cache: RwLock::new(HashMap::new()),
        }
    }
    
    /// Fetch JWKS keys from auth provider
    async fn fetch_jwks(&self) -> Result<JwksResponse, Box<dyn std::error::Error + Send + Sync>> {
        let response = self.http_client
            .get(&self.config.jwks_url)
            .send()
            .await?
            .json::<JwksResponse>()
            .await?;
        
        Ok(response)
    }
    
    /// Refresh JWKS keys
    async fn refresh_keys(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let jwks = self.fetch_jwks().await?;
        
        let mut cache = self.keys_cache.write().unwrap();
        cache.clear();
        
        for key in jwks.keys {
            if key.usage == "sig" && key.kty == "RSA" {
                let decoding_key = DecodingKey::from_rsa_components(&key.n, &key.e)
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
                
                cache.insert(key.kid, decoding_key);
            }
        }
        
        Ok(())
    }
    
    /// Validate a JWT token
    pub async fn validate_token(&self, token: &str) -> Result<JwtClaims, Box<dyn std::error::Error + Send + Sync>> {
        // Extract the key ID from the token
        let header = jsonwebtoken::decode_header(token)?;
        let kid = header.kid.ok_or("Token has no key ID")?;
        
        // Check if we have the key cached
        let decoding_key = {
            let cache = self.keys_cache.read().unwrap();
            cache.get(&kid).cloned()
        };
        
        // If not, refresh keys
        let decoding_key = match decoding_key {
            Some(key) => key,
            None => {
                self.refresh_keys().await?;
                let cache = self.keys_cache.read().unwrap();
                cache.get(&kid).cloned().ok_or("Key not found after refresh")?
            }
        };
        
        // Validate the token
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_audience(&[&self.config.audience]);
        validation.set_issuer(&[&self.config.issuer]);
        
        let token_data = decode::<JwtClaims>(token, &decoding_key, &validation)?;
        
        Ok(token_data.claims)
    }
    
    /// Make an authenticated call to another service with the user's token
    pub async fn call_service_with_auth(
        &self, 
        method: reqwest::Method,
        url: &str,
        auth_token: &str,
        body: Option<Value>,
    ) -> Result<reqwest::Response, Box<dyn std::error::Error + Send + Sync>> {
        let mut req = self.http_client
            .request(method, url)
            .header(header::AUTHORIZATION, format!("Bearer {}", auth_token));
        
        if let Some(json_body) = body {
            req = req.json(&json_body);
        }
        
        let response = req.send().await?;
        
        // Check if the request was successful
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("Service call failed: {}", error_text).into());
        }
        
        Ok(response)
    }
} 