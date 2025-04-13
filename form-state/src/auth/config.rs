use std::env;
use std::sync::OnceLock;

/// Default JWKS URL format for Dynamic Auth
const DEFAULT_JWKS_URL: &str = "https://app.dynamic.xyz/api/v0/sdk/ENV_PLACEHOLDER/.well-known/jwks";

/// Environment variable key for the JWKS URL
const ENV_JWKS_URL: &str = "DYNAMIC_JWKS_URL";

/// Environment variable key for the issuer (optional)
const ENV_JWT_ISSUER: &str = "DYNAMIC_JWT_ISSUER";

/// Environment variable key for the audience (optional) 
const ENV_JWT_AUDIENCE: &str = "DYNAMIC_JWT_AUDIENCE";

/// Environment variable key for JWT token leeway in seconds (optional)
const ENV_JWT_LEEWAY: &str = "DYNAMIC_JWT_LEEWAY";

/// Default leeway in seconds for token validation (default: 60s)
const DEFAULT_LEEWAY: u64 = 60;

/// Represents authentication configuration loaded from environment variables
#[derive(Debug, Clone)]
pub struct AuthConfig {
    /// JWKS URL for Dynamic Auth
    pub jwks_url: String,
    
    /// Expected JWT issuer (optional)
    pub issuer: Option<String>,
    
    /// Expected JWT audience (optional)
    pub audience: Option<String>,
    
    /// Leeway in seconds for token validation
    pub leeway: u64,
}

/// Singleton instance of the AuthConfig
static AUTH_CONFIG: OnceLock<AuthConfig> = OnceLock::new();

impl AuthConfig {
    /// Create a new AuthConfig by loading values from environment variables
    pub fn new() -> Self {
        let jwks_url = env::var(ENV_JWKS_URL).unwrap_or_else(|_| {
            log::warn!("{} environment variable not set, using placeholder URL", ENV_JWKS_URL);
            DEFAULT_JWKS_URL.to_string()
        });
        
        let issuer = env::var(ENV_JWT_ISSUER).ok();
        let audience = env::var(ENV_JWT_AUDIENCE).ok();
        
        let leeway = env::var(ENV_JWT_LEEWAY)
            .ok()
            .and_then(|val| val.parse::<u64>().ok())
            .unwrap_or(DEFAULT_LEEWAY);
            
        Self {
            jwks_url,
            issuer,
            audience,
            leeway,
        }
    }
    
    /// Validate the configuration
    pub fn validate(&self) -> Result<(), String> {
        // Validate JWKS URL
        if self.jwks_url == DEFAULT_JWKS_URL || self.jwks_url.contains("ENV_PLACEHOLDER") {
            return Err(format!("{} environment variable not set correctly. Please set it to your Dynamic Auth JWKS URL", ENV_JWKS_URL));
        }
        
        if !self.jwks_url.starts_with("http") || !self.jwks_url.contains("jwks") {
            return Err(format!("Invalid JWKS URL format: {}", self.jwks_url));
        }
        
        Ok(())
    }
    
    /// Get the global instance of AuthConfig
    pub fn global() -> &'static AuthConfig {
        AUTH_CONFIG.get_or_init(|| {
            let config = AuthConfig::new();
            // Log warning if validation fails but don't stop the application
            if let Err(e) = config.validate() {
                log::warn!("Auth configuration validation failed: {}", e);
            }
            config
        })
    }
    
    /// Get the JWKS URL from the config
    pub fn get_jwks_url(&self) -> &str {
        &self.jwks_url
    }
    
    /// Initialize the JWKS URL by validating it
    /// 
    /// Returns an error if the URL is invalid or uses the placeholder
    pub fn init_jwks_url(&self) -> Result<&str, String> {
        self.validate()?;
        Ok(&self.jwks_url)
    }
}

/// Get the JWKS URL from environment variable or use a default value (Legacy function)
/// 
/// The Dynamic Auth JWKS URL should be provided as an environment variable:
/// DYNAMIC_JWKS_URL=https://app.dynamic.xyz/api/v0/sdk/<YOUR_DYNAMIC_ENV_ID>/.well-known/jwks
/// 
/// If not set, a placeholder URL is returned that will need to be updated with a valid env ID.
#[deprecated(since = "0.1.0", note = "use AuthConfig::global().get_jwks_url() instead")]
pub fn get_jwks_url() -> String {
    AuthConfig::global().jwks_url.clone()
}

/// Initialize the JWKS URL by validating it (Legacy function)
/// 
/// Returns an error if the URL is invalid or uses the placeholder
#[deprecated(since = "0.1.0", note = "use AuthConfig::global().init_jwks_url() instead")]
pub fn init_jwks_url() -> Result<String, String> {
    match AuthConfig::global().validate() {
        Ok(_) => Ok(AuthConfig::global().jwks_url.clone()),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn setup() {
        env::remove_var(ENV_JWKS_URL);
        env::remove_var(ENV_JWT_ISSUER);
        env::remove_var(ENV_JWT_AUDIENCE);
        env::remove_var(ENV_JWT_LEEWAY);
    }
    
    #[test]
    fn test_default_config() {
        setup();
        
        let config = AuthConfig::new();
        assert_eq!(config.jwks_url, DEFAULT_JWKS_URL);
        assert_eq!(config.issuer, None);
        assert_eq!(config.audience, None);
        assert_eq!(config.leeway, DEFAULT_LEEWAY);
    }
    
    #[test]
    fn test_custom_config() {
        setup();
        
        let test_url = "https://app.dynamic.xyz/api/v0/sdk/test123/.well-known/jwks";
        let test_issuer = "test-issuer";
        let test_audience = "test-audience";
        let test_leeway = 120;
        
        env::set_var(ENV_JWKS_URL, test_url);
        env::set_var(ENV_JWT_ISSUER, test_issuer);
        env::set_var(ENV_JWT_AUDIENCE, test_audience);
        env::set_var(ENV_JWT_LEEWAY, test_leeway.to_string());
        
        let config = AuthConfig::new();
        assert_eq!(config.jwks_url, test_url);
        assert_eq!(config.issuer, Some(test_issuer.to_string()));
        assert_eq!(config.audience, Some(test_audience.to_string()));
        assert_eq!(config.leeway, test_leeway);
        
        // Clean up
        setup();
    }
    
    #[test]
    fn test_validation() {
        setup();
        
        // Invalid config (default URL) should fail validation
        let config = AuthConfig::new();
        assert!(config.validate().is_err());
        
        // Valid config should pass validation
        let test_url = "https://app.dynamic.xyz/api/v0/sdk/test123/.well-known/jwks";
        env::set_var(ENV_JWKS_URL, test_url);
        let config = AuthConfig::new();
        assert!(config.validate().is_ok());
        
        // Clean up
        setup();
    }
    
    #[test]
    fn test_jwks_url_default() {
        setup();
        
        // Check that we get the default URL
        let config = AuthConfig::new();
        assert_eq!(config.jwks_url, DEFAULT_JWKS_URL);
    }
    
    #[test]
    fn test_jwks_url_from_env() {
        setup();
        
        // Set test value
        let test_url = "https://app.dynamic.xyz/api/v0/sdk/test123/.well-known/jwks";
        env::set_var(ENV_JWKS_URL, test_url);
        
        // Check that we get the value from the environment
        let config = AuthConfig::new();
        assert_eq!(config.jwks_url, test_url);
        
        setup();
    }
} 