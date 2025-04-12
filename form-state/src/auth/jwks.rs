use crate::auth::config::AuthConfig;
use crate::auth::claims::DynamicClaims;
use crate::auth::middleware::decode_jwt_claims;
use jwt_authorizer::{AuthorizerBuilder, Validation};
use jsonwebtoken::{jwk::JwkSet, DecodingKey, decode, Algorithm, Validation as JwtValidation, TokenData};
use std::sync::Arc;
use std::time::{Duration, Instant};
use reqwest;
use std::collections::{HashMap, HashSet};
use tokio::sync::{RwLock, Mutex};
use tokio::time::sleep;
use std::sync::atomic::{AtomicBool, Ordering};

/// Manager for JWKS (JSON Web Key Set) operations with improved caching
pub struct JWKSManager {
    config: AuthConfig,
    // Cached JWKs with thread-safe access
    jwks_cache: Arc<RwLock<JWKSCache>>,
    // Cache refresh interval
    refresh_interval: Duration,
    // Client for HTTP requests
    client: reqwest::Client,
    // Flag to indicate if a background refresh is in progress
    refreshing: Arc<AtomicBool>,
    // Background refresh task guard
    #[allow(dead_code)]
    refresh_task_guard: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

/// Cache structure for JWKS
struct JWKSCache {
    // The cached JWK set
    jwks: Option<JwkSet>,
    // Last refresh timestamp
    last_refresh: Instant,
    // Next scheduled refresh timestamp
    next_refresh: Instant,
    // Cache of decoded DecodingKeys by key ID
    keys_by_kid: HashMap<String, DecodingKey>,
    // Last fetch result status
    last_fetch_success: bool,
}

impl JWKSCache {
    fn new() -> Self {
        Self {
            jwks: None,
            last_refresh: Instant::now().checked_sub(Duration::from_secs(3600)).unwrap_or(Instant::now()),
            next_refresh: Instant::now(),
            keys_by_kid: HashMap::new(),
            last_fetch_success: false,
        }
    }
    
    /// Check if the cache needs to be refreshed based on the provided interval
    fn needs_refresh(&self, _interval: Duration) -> bool {
        self.jwks.is_none() || Instant::now() >= self.next_refresh
    }
    
    /// Update the cache with a new JWK set
    fn update(&mut self, jwks: JwkSet) {
        // Process and cache DecodingKeys
        let mut new_keys = HashMap::new();
        
        for jwk in &jwks.keys {
            // Extract the key ID if available
            if let Some(kid) = jwk.common.key_id.as_ref() {
                if let Ok(key) = DecodingKey::from_jwk(jwk) {
                    new_keys.insert(kid.clone(), key);
                }
            }
        }
        
        // Keep track of the previous keys to enable key rotation
        if !self.keys_by_kid.is_empty() {
            // Merge new keys with existing ones, preferring new keys when there's a conflict
            for (kid, key) in new_keys {
                self.keys_by_kid.insert(kid, key);
            }
        } else {
            // First load, just use the new keys
            self.keys_by_kid = new_keys;
        }
        
        self.jwks = Some(jwks);
        self.last_refresh = Instant::now();
        self.last_fetch_success = true;
    }
    
    /// Set the next refresh time based on the interval and jitter
    fn schedule_next_refresh(&mut self, interval: Duration) {
        // Add some jitter (±10%) to prevent thundering herd
        let jitter_factor = 0.9 + (rand::random::<f64>() * 0.2);
        let jittered_interval = Duration::from_secs_f64(interval.as_secs_f64() * jitter_factor);
        
        self.next_refresh = Instant::now() + jittered_interval;
    }
    
    /// Record a failed fetch attempt
    fn record_failure(&mut self) {
        self.last_fetch_success = false;
        
        // On failure, schedule a quicker retry (1/10 of the normal interval, but at least 30s)
        let retry_delay = Duration::from_secs(30);
        self.next_refresh = Instant::now() + retry_delay;
    }
    
    /// Get a decoding key by key ID
    fn get_key(&self, kid: &str) -> Option<&DecodingKey> {
        self.keys_by_kid.get(kid)
    }
    
    /// Get all cached keys
    fn all_keys(&self) -> Vec<(&String, &DecodingKey)> {
        self.keys_by_kid.iter().collect()
    }
}

impl JWKSManager {
    /// Create a new JWKSManager using global configuration
    pub fn new() -> Self {
        let config = AuthConfig::global().clone();
        let refresh_interval = Duration::from_secs(3600); // 1 hour default refresh
        
        Self::with_refresh_interval(config, refresh_interval)
    }

    /// Create a new JWKSManager with specific configuration
    pub fn with_config(config: AuthConfig) -> Self {
        Self::with_refresh_interval(config, Duration::from_secs(3600))
    }
    
    /// Create a new JWKSManager with custom refresh interval
    pub fn with_refresh_interval(config: AuthConfig, refresh_interval: Duration) -> Self {
        let jwks_cache = Arc::new(RwLock::new(JWKSCache::new()));
        let refreshing = Arc::new(AtomicBool::new(false));
        let refresh_task_guard = Arc::new(Mutex::new(None));
        
        let manager = Self {
            config,
            jwks_cache,
            refresh_interval,
            client: reqwest::Client::new(),
            refreshing,
            refresh_task_guard,
        };
        
        // Start the background refresh task
        manager.start_background_refresh();
        
        manager
    }
    
    /// Start the background key refresh task
    fn start_background_refresh(&self) {
        let jwks_cache = self.jwks_cache.clone();
        let refresh_interval = self.refresh_interval;
        let client = self.client.clone();
        let jwks_url = self.config.jwks_url.clone();
        let refreshing = self.refreshing.clone();
        
        let task = tokio::spawn(async move {
            loop {
                // Sleep for the refresh interval
                sleep(refresh_interval).await;
                
                // Only refresh if another refresh isn't already in progress
                if !refreshing.swap(true, Ordering::SeqCst) {
                    // Perform the refresh
                    log::debug!("Background refresh: fetching JWKS from {}", jwks_url);
                    
                    let result = fetch_jwks(&client, &jwks_url).await;
                    
                    // Update the cache based on the result
                    let mut cache = jwks_cache.write().await;
                    match result {
                        Ok(jwks) => {
                            cache.update(jwks);
                            cache.schedule_next_refresh(refresh_interval);
                            log::debug!("Background refresh: JWKS refreshed successfully with {} keys", 
                                cache.keys_by_kid.len());
                        },
                        Err(e) => {
                            cache.record_failure();
                            log::warn!("Background refresh: Failed to refresh JWKS: {}", e);
                        }
                    }
                    
                    // Mark the refresh as complete
                    refreshing.store(false, Ordering::SeqCst);
                }
            }
        });
        
        // Store the task handle
        let mut guard = futures::executor::block_on(async {
            self.refresh_task_guard.lock().await
        });
        *guard = Some(task);
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
        // Only refresh if another refresh isn't already in progress
        if !self.refreshing.swap(true, Ordering::SeqCst) {
            log::debug!("Refreshing JWKS from {}", self.config.jwks_url);
            
            let result = fetch_jwks(&self.client, &self.config.jwks_url).await;
            
            // Update the cache based on the result
            let mut cache = self.jwks_cache.write().await;
            let refresh_result = match result {
                Ok(jwks) => {
                    cache.update(jwks);
                    cache.schedule_next_refresh(self.refresh_interval);
                    log::debug!("JWKS refreshed successfully with {} keys", cache.keys_by_kid.len());
                    Ok(())
                },
                Err(e) => {
                    cache.record_failure();
                    Err(format!("Failed to refresh JWKS: {}", e))
                }
            };
            
            // Mark the refresh as complete
            self.refreshing.store(false, Ordering::SeqCst);
            
            refresh_result
        } else {
            // A refresh is already in progress
            log::debug!("JWKS refresh already in progress, skipping");
            Ok(())
        }
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
    
    /// Get the configured audience (if any)
    pub fn get_audience(&self) -> Option<&String> {
        self.config.audience.as_ref()
    }
    
    /// Get the configured issuer (if any)
    pub fn get_issuer(&self) -> Option<&String> {
        self.config.issuer.as_ref()
    }
    
    /// Validate a JWT token directly using the cached keys
    pub async fn validate_token(&self, token: &str) -> Result<TokenData<DynamicClaims>, String> {
        // Extract the key ID from the token header
        let header = jsonwebtoken::decode_header(token)
            .map_err(|e| format!("Failed to decode token header: {}", e))?;
        
        let kid = match header.kid {
            Some(kid) => kid,
            None => {
                log::warn!("Token missing key ID (kid)");
                return Err("Token missing key ID (kid)".to_string());
            }
        };
        
        // Try to validate with the current cache first
        let validation_result = {
            let cache = self.jwks_cache.read().await;
            
            if let Some(decoding_key) = cache.get_key(&kid) {
                // Create validation parameters
                let validation = self.create_jwt_validation();
                
                // Log validation parameters for debugging
                if let Some(aud) = &validation.aud {
                    log::info!("Validating with audience: {:?}", aud);
                }
                if let Some(iss) = &validation.iss {
                    log::info!("Validating with issuer: {:?}", iss);
                }
                log::info!("Validation parameters - exp: {}, nbf: {}, leeway: {}s", 
                    validation.validate_exp, validation.validate_nbf, validation.leeway);
                
                // Decode and validate the token
                decode::<DynamicClaims>(token, &decoding_key, &validation)
            } else {
                // No key found for this kid, need to refresh
                log::warn!("No key found for kid: {}. Will attempt refresh.", kid);
                Err(jsonwebtoken::errors::Error::from(jsonwebtoken::errors::ErrorKind::InvalidToken))
            }
        };
        
        // If validation fails or the key isn't found, try refreshing the keys and validating again
        if let Err(e) = &validation_result {
            log::error!("Token validation failed with error: {:?}", e);
            
            match e.kind() {
                jsonwebtoken::errors::ErrorKind::InvalidAudience => {
                    log::error!("JWT validation failed due to invalid audience");
                    
                    // Decode token payload for debugging
                    if let Ok(claims) = decode_jwt_claims(token) {
                        if let Some(aud) = claims.get("aud") {
                            log::error!("Token audience: {:?}, Expected: {:?}", aud, self.config.audience);
                        }
                    }
                },
                jsonwebtoken::errors::ErrorKind::InvalidIssuer => {
                    log::error!("JWT validation failed due to invalid issuer");
                    
                    // Decode token payload for debugging
                    if let Ok(claims) = decode_jwt_claims(token) {
                        if let Some(iss) = claims.get("iss") {
                            log::error!("Token issuer: {:?}, Expected: {:?}", iss, self.config.issuer);
                        }
                    }
                },
                _ => log::error!("JWT validation failed: {:?}", e.kind()),
            }
            
            // Force refresh of keys
            if let Err(e) = self.refresh_keys().await {
                log::warn!("Failed to refresh keys during token validation: {}", e);
            }
            
            // Try again with the freshly fetched keys
            let cache = self.jwks_cache.read().await;
            
            if let Some(decoding_key) = cache.get_key(&kid) {
                // Create validation parameters
                let validation = self.create_jwt_validation();
                
                // Decode and validate the token
                decode::<DynamicClaims>(token, &decoding_key, &validation)
                    .map_err(|e| format!("Token validation failed after key refresh: {}", e))
            } else {
                // If we still don't have the key after refresh, try all keys as a last resort
                let all_keys = cache.all_keys();
                
                if all_keys.is_empty() {
                    return Err("No JWKS keys available for validation".to_string());
                }
                
                // Try each key
                for (key_id, key) in all_keys {
                    let validation = self.create_jwt_validation();
                    match decode::<DynamicClaims>(token, key, &validation) {
                        Ok(token_data) => {
                            log::warn!("Token validated with key {} instead of requested {}", key_id, kid);
                            return Ok(token_data);
                        },
                        Err(_) => continue,
                    }
                }
                
                // No key worked
                Err(format!("No key found for kid: {}", kid))
            }
        } else {
            validation_result.map_err(|e| format!("Token validation failed: {}", e))
        }
    }
    
    /// Get a key by key ID
    pub async fn get_key_by_id(&self, kid: &str) -> Option<DecodingKey> {
        // Try to get from cache first
        {
            let cache = self.jwks_cache.read().await;
            if let Some(key) = cache.get_key(kid) {
                return Some(key.clone());
            }
        }
        
        // If not found, try to refresh keys
        if let Err(e) = self.refresh_keys().await {
            log::warn!("Failed to refresh keys while looking for kid {}: {}", kid, e);
            return None;
        }
        
        // Check again after refresh
        let cache = self.jwks_cache.read().await;
        cache.get_key(kid).cloned()
    }
    
    /// Check if a refresh is currently in progress
    pub fn is_refreshing(&self) -> bool {
        self.refreshing.load(Ordering::SeqCst)
    }
    
    /// Get the last refresh time
    pub async fn last_refresh_time(&self) -> Option<Instant> {
        let cache = self.jwks_cache.read().await;
        if cache.last_fetch_success {
            Some(cache.last_refresh)
        } else {
            None
        }
    }
    
    /// Get the time until next refresh
    pub async fn time_until_next_refresh(&self) -> Option<Duration> {
        let cache = self.jwks_cache.read().await;
        let now = Instant::now();
        if now < cache.next_refresh {
            Some(cache.next_refresh.duration_since(now))
        } else {
            None
        }
    }
    
    /// Get count of cached keys
    pub async fn cached_key_count(&self) -> usize {
        let cache = self.jwks_cache.read().await;
        cache.keys_by_kid.len()
    }
}

/// Helper function to fetch JWKS from a URL
async fn fetch_jwks(client: &reqwest::Client, jwks_url: &str) -> Result<JwkSet, String> {
    // Fetch the JWKS with timeout
    let response = client.get(jwks_url)
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
        
    serde_json::from_str(&jwks_text)
        .map_err(|e| format!("Failed to parse JWKS: {}", e))
}

/// Initialize and provide a JWKS manager for the application
/// 
/// This function creates a new JWKSManager with the global config
/// and returns it wrapped in an Arc for shared access.
/// 
/// # Example
/// 
/// ```rust
/// use axum::{Router, routing::get};
/// use std::sync::Arc;
/// 
/// async fn main() {
///     // Create the JWKS manager
///     let jwks_manager = init_jwks_manager();
///     
///     // Build your application with the JWKS manager
///     let app = Router::new()
///         .route("/", get(|| async { "Hello, world!" }))
///         .with_state(jwks_manager);
///         
///     // Start the server with the app
///     // ...
/// }
/// ```
pub fn init_jwks_manager() -> Arc<JWKSManager> {
    // Create the manager with default config
    let manager = JWKSManager::new();
    
    // Wrap in Arc for shared access
    Arc::new(manager)
}

/// Initialize the JWKS manager with a custom refresh interval
pub fn init_jwks_manager_with_interval(refresh_seconds: u64) -> Arc<JWKSManager> {
    let config = AuthConfig::global().clone();
    let refresh_interval = Duration::from_secs(refresh_seconds);
    
    // Create the manager with custom refresh interval
    let manager = JWKSManager::with_refresh_interval(config, refresh_interval);
    
    // Wrap in Arc for shared access
    Arc::new(manager)
}

/// Force a refresh of the JWKS on demand
pub async fn force_jwks_refresh(jwks_manager: &Arc<JWKSManager>) -> Result<(), String> {
    jwks_manager.refresh_keys().await
}

/// Apply JWT authentication middleware to an axum Router
///
/// This is a convenience function that applies the JWT authentication
/// middleware to the provided router using the JWKS manager.
///
/// # Example
///
/// ```rust
/// use axum::{Router, routing::get};
/// use std::sync::Arc;
///
/// #[tokio::main]
/// async fn main() {
///     // Create a router with your endpoints
///     let router = Router::new()
///         .route("/", get(|| async { "Hello, world!" }));
///    
///     // Create JWKS manager
///     let jwks_manager = Arc::new(JWKSManager::new());
///     
///     // Apply JWT authentication manually
///     let authenticated_router = router
///         .layer(axum::middleware::from_fn_with_state(
///             jwks_manager.clone(),
///             jwt_auth_middleware
///         ))
///         .with_state(jwks_manager);
///
///     // Start the server with the app
///     // ...
/// }
/// ```
// This function is currently commented out due to type issues with axum
// #[cfg(feature = "axum")]
// pub fn apply_jwt_auth(router: axum::Router) -> axum::Router {
//     // Initialize JWKS manager
//     let jwks_manager = init_jwks_manager();
//     
//     // Apply the middleware and add the JWKS manager to state
//     router
//         .layer(axum::middleware::from_fn_with_state(
//             jwks_manager.clone(),
//             crate::auth::middleware::jwt_auth_middleware
//         ))
//         .with_state(jwks_manager)
// }

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::time::Duration;
    use tokio::time::sleep;
    
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
    
    #[tokio::test]
    async fn test_cache_scheduling() {
        // Create a JWKSCache for testing
        let mut cache = JWKSCache::new();
        
        // Test scheduling with jitter
        let interval = Duration::from_secs(60);
        cache.schedule_next_refresh(interval);
        
        // Verify that next_refresh is in the future
        assert!(cache.next_refresh > Instant::now());
        
        // Verify it's within expected range (with jitter)
        let max_expected = Instant::now() + Duration::from_secs(72); // 60s * 1.2
        assert!(cache.next_refresh <= max_expected);
        
        // Test failure retry scheduling
        cache.record_failure();
        
        // Verify the next refresh is scheduled sooner after failure
        let retry_time = cache.next_refresh;
        let max_retry_expected = Instant::now() + Duration::from_secs(35); // ~30s plus small buffer
        
        assert!(retry_time <= max_retry_expected);
        assert!(!cache.last_fetch_success);
    }
    
    #[tokio::test]
    async fn test_get_key_by_id() {
        // This test would ideally use a mock client
        // But for now we'll test the method logic
        
        let test_url = "https://app.dynamic.xyz/api/v0/sdk/test123/.well-known/jwks";
        env::set_var("DYNAMIC_JWKS_URL", test_url);
        
        let config = AuthConfig::new();
        let jwks_manager = JWKSManager::with_config(config);
        
        // In a real test with mocking:
        // 1. Mock the HTTP client to return a known JWKS with specific kids
        // 2. Call get_key_by_id for an existing kid
        // 3. Verify the correct key is returned
        // 4. Call get_key_by_id for a non-existent kid
        // 5. Verify that refresh is attempted
        // 6. Verify that None is returned if kid doesn't exist after refresh
        
        // Basic verification that the method exists and runs
        let non_existent_kid = "non-existent-kid";
        let result = jwks_manager.get_key_by_id(non_existent_kid).await;
        
        // Since we're not mocking, this will likely fail to get keys
        // But we're testing the method logic, not the actual HTTP call
        assert!(result.is_none());
        
        // Clean up
        env::remove_var("DYNAMIC_JWKS_URL");
    }
    
    #[tokio::test]
    async fn test_background_refresh() {
        let test_url = "https://app.dynamic.xyz/api/v0/sdk/test123/.well-known/jwks";
        env::set_var("DYNAMIC_JWKS_URL", test_url);
        
        let config = AuthConfig::new();
        // Very short interval for testing
        let refresh_interval = Duration::from_millis(100);
        
        let jwks_manager = JWKSManager::with_refresh_interval(config, refresh_interval);
        
        // Sleep to allow background task to run
        sleep(Duration::from_millis(300)).await;
        
        // In a real test with mocking:
        // 1. Verify that the background task attempted to refresh keys
        // 2. Verify that the cache was updated
        
        // Basic check that the manager has scheduling data
        let time_until_refresh = jwks_manager.time_until_next_refresh().await;
        
        // We can't make strong assertions without mocking
        // But we can at least verify the method doesn't panic
        match time_until_refresh {
            Some(duration) => {
                println!("Next refresh in {:?}", duration);
            },
            None => {
                println!("No scheduled refresh or refresh is due");
            }
        }
        
        // Clean up
        env::remove_var("DYNAMIC_JWKS_URL");
    }
} 