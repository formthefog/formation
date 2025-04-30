use reqwest::Client;
use serde_json::Value;
use super::{ApiKeyInfo, ApiKeyError};

#[derive(Debug, Clone)]
pub struct ApiKeyClient {
    http_client: Client,
    api_key_service_url: String,
}

impl ApiKeyClient {
    pub fn new(api_key_service_url: String) -> Self {
        let http_client = Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("Failed to build HTTP client");
        
        Self {
            http_client,
            api_key_service_url,
        }
    }
    
    pub fn from_env() -> Self {
        let api_key_service_url = std::env::var("API_KEY_SERVICE_URL")
            .unwrap_or_else(|_| "https://api.formation.dev/api-keys".to_string());
        
        Self::new(api_key_service_url)
    }
    
    /// Validate an API key by checking with the API key service
    pub async fn validate_key(&self, api_key: &str) -> Result<ApiKeyInfo, ApiKeyError> {
        let response = self.http_client
            .get(format!("{}/validate", self.api_key_service_url))
            .header("X-API-Key", api_key)
            .send()
            .await
            .map_err(|_| ApiKeyError::ServiceError)?;
        
        if !response.status().is_success() {
            return match response.status().as_u16() {
                401 => Err(ApiKeyError::NotFound),
                403 => Err(ApiKeyError::InsufficientPermissions),
                429 => Err(ApiKeyError::RateLimitExceeded),
                _ => Err(ApiKeyError::ServiceError),
            };
        }
        
        let key_info = response.json::<ApiKeyInfo>().await
            .map_err(|_| ApiKeyError::ServiceError)?;
        
        Ok(key_info)
    }
    
    /// Make an authenticated call to another service with the API key
    pub async fn call_service_with_api_key(
        &self,
        method: reqwest::Method,
        url: &str,
        api_key: &str,
        body: Option<Value>,
    ) -> Result<reqwest::Response, ApiKeyError> {
        let mut req = self.http_client
            .request(method, url)
            .header("X-API-Key", api_key);
        
        if let Some(json_body) = body {
            req = req.json(&json_body);
        }
        
        let response = req.send().await
            .map_err(|_| ApiKeyError::ServiceError)?;
        
        if !response.status().is_success() {
            return match response.status().as_u16() {
                401 => Err(ApiKeyError::NotFound),
                403 => Err(ApiKeyError::InsufficientPermissions),
                429 => Err(ApiKeyError::RateLimitExceeded),
                _ => Err(ApiKeyError::ServiceError),
            };
        }
        
        Ok(response)
    }
} 