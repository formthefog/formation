use crate::auth::config::AuthConfig;
use crate::auth::claims::DynamicClaims;
use jwt_authorizer::{AuthorizerBuilder, Validation};
use jsonwebtoken::{jwk::JwkSet, DecodingKey, decode, Algorithm, Validation as JwtValidation, TokenData};
use std::sync::Arc;
use std::time::{Duration, Instant};
use reqwest;
use std::collections::{HashMap, HashSet};
use tokio::sync::RwLock;

/// Manager for JWKS (JSON Web Key Set) operations with improved caching
pub struct JWKSManager {
    config: AuthConfig,
    // Cached JWKs with thread-safe access
    jwks_cache: Arc<RwLock<JWKSCache>>,
    // Cache refresh interval
    refresh_interval: Duration,
    // Client for HTTP requests
    client: reqwest::Client,
}

/// Cache structure for JWKS
struct JWKSCache {
    // The cached JWK set
    jwks: Option<JwkSet>,
    // Last refresh timestamp
    last_refresh: Instant,
    // Cache of decoded DecodingKeys by key ID
    keys_by_kid: HashMap<String, DecodingKey>,
}

impl JWKSCache {
    fn new() -> Self {
        Self {
            jwks: None,
            last_refresh: Instant::now().checked_sub(Duration::from_secs(3600)).unwrap_or(Instant::now()),
            keys_by_kid: HashMap::new(),
        }
    }
    
    /// Check if the cache needs to be refreshed based on the provided interval
    fn needs_refresh(&self, interval: Duration) -> bool {
        self.jwks.is_none() || self.last_refresh.elapsed() > interval
    }
    
    /// Update the cache with a new JWK set
    fn update(&mut self, jwks: JwkSet) {
        // Clear existing keys
        self.keys_by_kid.clear();
        
        // Process and cache DecodingKeys
        for jwk in &jwks.keys {
            // Extract the key ID if available
            if let Some(kid) = jwk.common.key_id.as_ref() {
                if let Ok(key) = DecodingKey::from_jwk(jwk) {
                    self.keys_by_kid.insert(kid.clone(), key);
                }
            }
        }
        
        self.jwks = Some(jwks);
        self.last_refresh = Instant::now();
    }
    
    /// Get a decoding key by key ID
    fn get_key(&self, kid: &str) -> Option<&DecodingKey> {
        self.keys_by_kid.get(kid)
    }
}

impl JWKSManager {
    /// Create a new JWKSManager using global configuration
    pub fn new() -> Self {
        Self {
            config: AuthConfig::global().clone(),
            jwks_cache: Arc::new(RwLock::new(JWKSCache::new())),
            refresh_interval: Duration::from_secs(3600), // 1 hour default refresh
            client: reqwest::Client::new(),
        }
    }

    /// Create a new JWKSManager with specific configuration
    pub fn with_config(config: AuthConfig) -> Self {
        Self {
            config,
            jwks_cache: Arc::new(RwLock::new(JWKSCache::new())),
            refresh_interval: Duration::from_secs(3600),
            client: reqwest::Client::new(),
        }
    }
    
    /// Create a new JWKSManager with custom refresh interval
    pub fn with_refresh_interval(config: AuthConfig, refresh_interval: Duration) -> Self {
        Self {
            config,
            jwks_cache: Arc::new(RwLock::new(JWKSCache::new())),
            refresh_interval,
            client: reqwest::Client::new(),
        }
    }
    
    /// Set a custom refresh interval
    pub fn set_refresh_interval(&mut self, interval: Duration) {
        self.refresh_interval = interval;
    }
    
    /// Get the JWKS URL
    pub fn get_jwks_url(&self) -> &str {
        &self.config.jwks_url
    }
    
    /// Get the JWKS, fetching if needed
    pub async fn get_jwks(&self) -> Result<JwkSet, String> {
        // Check if we need to refresh
        let needs_refresh = {
            self.jwks_cache.read().await.needs_refresh(self.refresh_interval)
        };
        
        if needs_refresh {
            self.refresh_keys().await?;
        }
        
        // Return the cached JWKS
        let cache = self.jwks_cache.read().await;
        match &cache.jwks {
            Some(jwks) => Ok(jwks.clone()),
            None => Err("Failed to get JWKS".to_string()),
        }
    }
    
    /// Force refresh of JWKS keys
    pub async fn refresh_keys(&self) -> Result<(), String> {
        log::debug!("Refreshing JWKS from {}", self.config.jwks_url);
        
        // Fetch the JWKS with timeout
        let response = self.client.get(&self.config.jwks_url)
            .timeout(Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| format!("Failed to fetch JWKS: {}", e))?;
            
        if !response.status().is_success() {
            return Err(format!("JWKS request failed with status: {}", response.status()));
        }
        
        let jwks_text = response.text()
            .await
            .map_err(|e| format!("Failed to read JWKS response: {}", e))?;
            
        let jwks: JwkSet = serde_json::from_str(&jwks_text)
            .map_err(|e| format!("Failed to parse JWKS: {}", e))?;
            
        // Update the cache with the new JWKS
        {
            let mut cache = self.jwks_cache.write().await;
            cache.update(jwks);
        }
        
        log::debug!("JWKS refreshed successfully with {} keys", {
            let cache = self.jwks_cache.read().await;
            cache.keys_by_kid.len()
        });
        
        Ok(())
    }
    
    /// Create JWT validation settings from config
    fn create_validation(&self) -> Validation {
        let mut validation = Validation::new();
        
        // Set issuer if configured
        if let Some(iss) = &self.config.issuer {
            validation = validation.iss(&[iss.as_str()]);
        }
        
        // Set audience if configured
        if let Some(aud) = &self.config.audience {
            validation = validation.aud(&[aud.as_str()]);
        }
        
        // Set leeway for time-based validation
        validation = validation.leeway(self.config.leeway);
        
        // Always validate expiration
        validation = validation.exp(true);
        
        // Always validate not-before if present
        validation = validation.nbf(true);
        
        validation
    }
    
    /// Create jwt-authorizer validation settings
    fn create_jwt_validation(&self) -> JwtValidation {
        let mut validation = JwtValidation::new(Algorithm::RS256);
        
        // Set issuer if configured
        if let Some(iss) = &self.config.issuer {
            validation.iss = Some(HashSet::from_iter([iss.clone()]));
        }
        
        // Set audience if configured
        if let Some(aud) = &self.config.audience {
            validation.aud = Some(HashSet::from_iter([aud.clone()]));
        }
        
        // Set leeway for time-based validation
        validation.leeway = self.config.leeway;
        
        // Always validate expiration
        validation.validate_exp = true;
        
        // Always validate not-before if present
        validation.validate_nbf = true;
        
        validation
    }
    
    /// Create a new JWT authorizer
    pub async fn create_authorizer(&self) -> Result<jwt_authorizer::Authorizer<DynamicClaims>, String> {
        let validation = self.create_validation();
        
        // Make sure JWKS is fetched
        self.get_jwks().await?;
        
        // Build the authorizer using the correct constructor method
        AuthorizerBuilder::<DynamicClaims>::from_jwks_url(&self.config.jwks_url)
            .validation(validation)
            .build()
            .await
            .map_err(|e| format!("Failed to create JWT authorizer: {}", e))
    }
    
    /// Validate a JWT token directly using the cached keys
    pub async fn validate_token(&self, token: &str) -> Result<TokenData<DynamicClaims>, String> {
        // Extract the key ID from the token header
        let header = jsonwebtoken::decode_header(token)
            .map_err(|e| format!("Failed to decode token header: {}", e))?;
        
        let kid = header.kid.ok_or_else(|| "Token missing key ID (kid)".to_string())?;
        
        // Make sure JWKS is fetched/refreshed if needed
        if self.jwks_cache.read().await.needs_refresh(self.refresh_interval) {
            self.refresh_keys().await?;
        }
        
        // Get the decoding key for this kid
        let decoding_key = {
            let cache = self.jwks_cache.read().await;
            cache.get_key(&kid)
                .ok_or_else(|| format!("No key found for kid: {}", kid))?
                .clone()
        };
        
        // Create validation parameters
        let validation = self.create_jwt_validation();
        
        // Decode and validate the token
        decode::<DynamicClaims>(token, &decoding_key, &validation)
            .map_err(|e| format!("Token validation failed: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    
    #[test]
    fn test_jwks_manager() {
        // Set up test environment
        let test_url = "https://app.dynamic.xyz/api/v0/sdk/test123/.well-known/jwks";
        env::set_var("DYNAMIC_JWKS_URL", test_url);
        
        // Create a custom config
        let config = AuthConfig::new();
        
        // Create manager with custom config
        let manager = JWKSManager::with_config(config);
        
        // Verify URL is correctly set
        assert_eq!(manager.get_jwks_url(), test_url);
        
        // Clean up
        env::remove_var("DYNAMIC_JWKS_URL");
    }
    
    #[tokio::test]
    async fn test_jwks_refresh_and_validation() {
        // This is a mock test to demonstrate the flow, not an actual HTTP call
        let test_url = "https://app.dynamic.xyz/api/v0/sdk/test123/.well-known/jwks";
        env::set_var("DYNAMIC_JWKS_URL", test_url);
        
        // Create config with shorter refresh interval for testing
        let config = AuthConfig::new();
        let refresh_interval = Duration::from_secs(5); // 5 seconds
        
        // Create manager with custom refresh interval
        let _manager = JWKSManager::with_refresh_interval(config, refresh_interval);
        
        // For real tests, we would:
        // 1. Mock the HTTP client to return a predefined JWKS
        // 2. Call refresh_keys() to prime the cache
        // 3. Verify keys are cached properly
        // 4. Try to validate a token with a known key ID
        // 5. Force cache expiration
        // 6. Verify the cache is refreshed on the next validation attempt
        
        // Clean up
        env::remove_var("DYNAMIC_JWKS_URL");
    }
} 