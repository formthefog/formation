# Design: Admin Key Storage and Configuration

**Sub-sub-task 3.1.2.1: Define admin key storage and configuration**

## Overview

This document defines the storage and configuration system for admin keys in Formation. Admin keys use the same ECDSA cryptographic system as user keys but are pre-configured at deployment time via environment variables, providing global administrative access to all Formation services.

## Design Principles

### 1. Cryptographic Consistency
- **Same ECDSA System**: Admin keys use identical cryptographic primitives as user keys
- **Address Derivation**: Admin addresses derived from public keys using same Ethereum-style derivation
- **Signature Compatibility**: Admin signatures use existing ECDSA verification infrastructure

### 2. Configuration Simplicity
- **Environment-Based**: Admin keys configured via environment variables at startup
- **Address-Only Storage**: Only store admin addresses, not private keys
- **Multi-Service Support**: Same configuration pattern across all Formation services

### 3. Security Requirements
- **No Private Key Storage**: Services never store admin private keys
- **Address Verification**: Verify admin signatures against configured addresses
- **Audit Trail**: Log all admin key usage with full context

## Admin Key Configuration Schema

### Environment Variable Format

```bash
# Single admin key
FORMATION_ADMIN_KEY="0x742d35cc6bd8c532f1c86e26342c91b73e9dea34"

# Multiple admin keys (comma-separated)
FORMATION_ADMIN_KEYS="0x742d35cc6bd8c532f1c86e26342c91b73e9dea34,0x8ba1f109551bD432803012645Hac136c42138c"

# Optional: Admin key descriptions for audit logging
FORMATION_ADMIN_KEY_DESCRIPTIONS="primary-admin,backup-admin"
```

### Alternative Configuration Methods

```bash
# Environment file (.env)
FORMATION_ADMIN_KEY=0x742d35cc6bd8c532f1c86e26342c91b73e9dea34

# Docker environment
docker run -e FORMATION_ADMIN_KEY=0x742d35cc6bd8c532f1c86e26342c91b73e9dea34 formation/form-state

# Kubernetes secret
apiVersion: v1
kind: Secret
metadata:
  name: formation-admin-keys
data:
  FORMATION_ADMIN_KEY: MHg3NDJkMzVjYzZiZDhjNTMyZjFjODZlMjYzNDJjOTFiNzNlOWRlYTM0
```

## Data Structures

### Admin Configuration Structure

```rust
use alloy_primitives::Address;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Configuration for admin keys
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminConfig {
    /// Set of admin addresses that have global access
    pub admin_addresses: HashSet<Address>,
    
    /// Optional descriptions for audit logging
    pub admin_descriptions: Option<Vec<String>>,
    
    /// Whether admin authentication is enabled
    pub enabled: bool,
    
    /// Configuration source (for diagnostics)
    pub source: ConfigSource,
}

/// Source of admin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConfigSource {
    EnvironmentVariable,
    ConfigFile,
    Database,
    Default,
}

impl AdminConfig {
    /// Create new admin config from environment
    pub fn from_environment() -> Result<Self, AdminConfigError> {
        // Implementation details below
    }
    
    /// Check if an address is an admin
    pub fn is_admin_address(&self, address: &Address) -> bool {
        self.enabled && self.admin_addresses.contains(address)
    }
    
    /// Get description for an admin address
    pub fn get_admin_description(&self, address: &Address) -> Option<&str> {
        if let Some(descriptions) = &self.admin_descriptions {
            let admin_list: Vec<_> = self.admin_addresses.iter().collect();
            for (i, admin_addr) in admin_list.iter().enumerate() {
                if *admin_addr == address {
                    return descriptions.get(i).map(|s| s.as_str());
                }
            }
        }
        None
    }
    
    /// Validate admin configuration
    pub fn validate(&self) -> Result<(), AdminConfigError> {
        if self.admin_addresses.is_empty() && self.enabled {
            return Err(AdminConfigError::NoAdminAddresses);
        }
        
        if let Some(descriptions) = &self.admin_descriptions {
            if descriptions.len() != self.admin_addresses.len() {
                return Err(AdminConfigError::DescriptionMismatch);
            }
        }
        
        Ok(())
    }
}
```

### Configuration Errors

```rust
/// Errors that can occur during admin configuration
#[derive(Debug, thiserror::Error)]
pub enum AdminConfigError {
    #[error("Invalid admin address format: {0}")]
    InvalidAddressFormat(String),
    
    #[error("Environment variable not found: {0}")]
    EnvironmentVariableNotFound(String),
    
    #[error("No admin addresses configured but admin auth is enabled")]
    NoAdminAddresses,
    
    #[error("Admin descriptions count doesn't match admin addresses count")]
    DescriptionMismatch,
    
    #[error("Duplicate admin address: {0}")]
    DuplicateAddress(String),
    
    #[error("Configuration validation failed: {0}")]
    ValidationFailed(String),
}
```

## Configuration Loading Implementation

### Environment Variable Parsing

```rust
impl AdminConfig {
    /// Load admin configuration from environment variables
    pub fn from_environment() -> Result<Self, AdminConfigError> {
        // Check if admin authentication should be enabled
        let enabled = std::env::var("FORMATION_ADMIN_ENABLED")
            .unwrap_or_else(|_| "true".to_string())
            .parse::<bool>()
            .unwrap_or(true);
        
        if !enabled {
            return Ok(AdminConfig {
                admin_addresses: HashSet::new(),
                admin_descriptions: None,
                enabled: false,
                source: ConfigSource::EnvironmentVariable,
            });
        }
        
        // Try single admin key first
        let admin_addresses = if let Ok(single_key) = std::env::var("FORMATION_ADMIN_KEY") {
            vec![single_key]
        } else if let Ok(multiple_keys) = std::env::var("FORMATION_ADMIN_KEYS") {
            multiple_keys.split(',').map(|s| s.trim().to_string()).collect()
        } else {
            return Err(AdminConfigError::EnvironmentVariableNotFound(
                "FORMATION_ADMIN_KEY or FORMATION_ADMIN_KEYS".to_string()
            ));
        };
        
        // Parse addresses
        let mut parsed_addresses = HashSet::new();
        for addr_str in admin_addresses {
            let cleaned = addr_str.trim();
            
            // Handle both with and without 0x prefix
            let addr_str = if cleaned.starts_with("0x") {
                cleaned
            } else {
                &format!("0x{}", cleaned)
            };
            
            let address = addr_str.parse::<Address>()
                .map_err(|_| AdminConfigError::InvalidAddressFormat(addr_str.to_string()))?;
                
            if !parsed_addresses.insert(address) {
                return Err(AdminConfigError::DuplicateAddress(addr_str.to_string()));
            }
        }
        
        // Parse optional descriptions
        let admin_descriptions = std::env::var("FORMATION_ADMIN_KEY_DESCRIPTIONS")
            .ok()
            .map(|desc_str| {
                desc_str.split(',').map(|s| s.trim().to_string()).collect::<Vec<_>>()
            });
        
        let config = AdminConfig {
            admin_addresses: parsed_addresses,
            admin_descriptions,
            enabled: true,
            source: ConfigSource::EnvironmentVariable,
        };
        
        // Validate before returning
        config.validate()?;
        
        Ok(config)
    }
    
    /// Create default configuration (no admin keys)
    pub fn default() -> Self {
        AdminConfig {
            admin_addresses: HashSet::new(),
            admin_descriptions: None,
            enabled: false,
            source: ConfigSource::Default,
        }
    }
    
    /// Create test configuration with provided addresses
    #[cfg(test)]
    pub fn test_config(addresses: Vec<&str>) -> Result<Self, AdminConfigError> {
        let mut admin_addresses = HashSet::new();
        for addr_str in addresses {
            let address = addr_str.parse::<Address>()
                .map_err(|_| AdminConfigError::InvalidAddressFormat(addr_str.to_string()))?;
            admin_addresses.insert(address);
        }
        
        Ok(AdminConfig {
            admin_addresses,
            admin_descriptions: None,
            enabled: true,
            source: ConfigSource::Default,
        })
    }
}
```

## Service Integration Pattern

### Configuration Initialization

```rust
/// Service-specific admin configuration initialization
pub struct ServiceAdminConfig {
    pub admin_config: AdminConfig,
    pub service_name: String,
    pub startup_time: std::time::SystemTime,
}

impl ServiceAdminConfig {
    /// Initialize admin configuration for a service
    pub fn initialize(service_name: &str) -> Self {
        let admin_config = AdminConfig::from_environment()
            .unwrap_or_else(|e| {
                log::warn!("Failed to load admin config: {}. Using default (no admin keys)", e);
                AdminConfig::default()
            });
        
        log::info!("Admin configuration for {}: {} admin keys configured, enabled: {}", 
                   service_name, 
                   admin_config.admin_addresses.len(),
                   admin_config.enabled);
        
        if admin_config.enabled {
            for (i, addr) in admin_config.admin_addresses.iter().enumerate() {
                let description = admin_config.admin_descriptions
                    .as_ref()
                    .and_then(|descs| descs.get(i))
                    .map(|s| s.as_str())
                    .unwrap_or("unnamed");
                    
                log::info!("  Admin key {}: 0x{} ({})", i + 1, hex::encode(addr.as_slice()), description);
            }
        }
        
        ServiceAdminConfig {
            admin_config,
            service_name: service_name.to_string(),
            startup_time: std::time::SystemTime::now(),
        }
    }
    
    /// Check if an address is an admin for this service
    pub fn is_admin(&self, address: &Address) -> bool {
        self.admin_config.is_admin_address(address)
    }
    
    /// Log admin usage for audit trail
    pub fn log_admin_usage(&self, address: &Address, endpoint: &str, method: &str) {
        let description = self.admin_config.get_admin_description(address)
            .unwrap_or("unknown");
            
        log::warn!("ADMIN ACCESS: {} used admin key 0x{} ({}) for {} {} on service {}", 
                   description,
                   hex::encode(address.as_slice()), 
                   description,
                   method,
                   endpoint,
                   self.service_name);
    }
}
```

## Configuration File Support (Future Enhancement)

### TOML Configuration Format

```toml
# formation.toml
[admin]
enabled = true

[[admin.keys]]
address = "0x742d35cc6bd8c532f1c86e26342c91b73e9dea34"
description = "Primary Admin"
permissions = ["all"]

[[admin.keys]]
address = "0x8ba1f109551bD432803012645Hac136c42138c"
description = "Backup Admin"
permissions = ["all"]

[admin.audit]
log_level = "warn"
log_format = "json"
```

### Configuration File Loading

```rust
#[cfg(feature = "config-file")]
impl AdminConfig {
    /// Load from TOML configuration file
    pub fn from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self, AdminConfigError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| AdminConfigError::ValidationFailed(format!("File read error: {}", e)))?;
            
        let config: TomlAdminConfig = toml::from_str(&content)
            .map_err(|e| AdminConfigError::ValidationFailed(format!("TOML parse error: {}", e)))?;
            
        config.try_into()
    }
}

#[derive(Deserialize)]
struct TomlAdminConfig {
    admin: TomlAdminSection,
}

#[derive(Deserialize)]
struct TomlAdminSection {
    enabled: bool,
    keys: Vec<TomlAdminKey>,
}

#[derive(Deserialize)]
struct TomlAdminKey {
    address: String,
    description: Option<String>,
    permissions: Option<Vec<String>>,
}
```

## Security Considerations

### Address Validation

```rust
/// Validate admin address format and security properties
pub fn validate_admin_address(address_str: &str) -> Result<Address, AdminConfigError> {
    // Parse the address
    let address = address_str.parse::<Address>()
        .map_err(|_| AdminConfigError::InvalidAddressFormat(address_str.to_string()))?;
    
    // Check for well-known weak addresses
    let weak_addresses = [
        "0x0000000000000000000000000000000000000000", // Zero address
        "0x1111111111111111111111111111111111111111", // Obvious pattern
        "0xffffffffffffffffffffffffffffffffffffffff", // Max address
    ];
    
    let addr_hex = hex::encode(address.as_slice());
    if weak_addresses.iter().any(|&weak| weak == format!("0x{}", addr_hex)) {
        return Err(AdminConfigError::ValidationFailed(
            format!("Weak or invalid admin address: {}", address_str)
        ));
    }
    
    Ok(address)
}
```

### Configuration Security

```rust
/// Security checks for admin configuration
impl AdminConfig {
    /// Perform security validation on admin configuration
    pub fn security_validate(&self) -> Result<(), AdminConfigError> {
        // Check for minimum security requirements
        if self.enabled && self.admin_addresses.is_empty() {
            return Err(AdminConfigError::ValidationFailed(
                "Admin authentication enabled but no admin addresses configured".to_string()
            ));
        }
        
        // Validate each admin address
        for address in &self.admin_addresses {
            let addr_str = format!("0x{}", hex::encode(address.as_slice()));
            validate_admin_address(&addr_str)?;
        }
        
        // Warn about single admin key (recommend multiple for redundancy)
        if self.admin_addresses.len() == 1 {
            log::warn!("Only one admin key configured. Consider adding backup admin keys for redundancy.");
        }
        
        // Limit maximum number of admin keys (prevent config bloat)
        if self.admin_addresses.len() > 10 {
            return Err(AdminConfigError::ValidationFailed(
                "Too many admin keys configured (maximum 10)".to_string()
            ));
        }
        
        Ok(())
    }
}
```

## Cross-Service Configuration Sharing

### Shared Configuration Module

```rust
/// Shared admin configuration that can be used across all Formation services
pub mod shared {
    use super::*;
    use std::sync::{Arc, RwLock};
    use std::sync::OnceLock;
    
    /// Global admin configuration instance
    static GLOBAL_ADMIN_CONFIG: OnceLock<Arc<RwLock<AdminConfig>>> = OnceLock::new();
    
    /// Initialize global admin configuration
    pub fn initialize_global_admin_config() -> Result<(), AdminConfigError> {
        let config = AdminConfig::from_environment()?;
        config.security_validate()?;
        
        GLOBAL_ADMIN_CONFIG.set(Arc::new(RwLock::new(config)))
            .map_err(|_| AdminConfigError::ValidationFailed("Global config already initialized".to_string()))?;
            
        Ok(())
    }
    
    /// Get global admin configuration
    pub fn get_global_admin_config() -> Option<Arc<RwLock<AdminConfig>>> {
        GLOBAL_ADMIN_CONFIG.get().cloned()
    }
    
    /// Check if address is global admin
    pub fn is_global_admin(address: &Address) -> bool {
        if let Some(config_lock) = get_global_admin_config() {
            if let Ok(config) = config_lock.read() {
                return config.is_admin_address(address);
            }
        }
        false
    }
}
```

## Testing Support

### Test Configuration Utilities

```rust
#[cfg(test)]
pub mod test_utils {
    use super::*;
    
    /// Create test admin configuration with known keys
    pub fn create_test_admin_config() -> AdminConfig {
        AdminConfig::test_config(vec![
            "0x742d35cc6bd8c532f1c86e26342c91b73e9dea34",
            "0x8ba1f109551bD432803012645Hac136c42138c"
        ]).unwrap()
    }
    
    /// Set test environment variables
    pub fn setup_test_env() {
        std::env::set_var("FORMATION_ADMIN_KEY", "0x742d35cc6bd8c532f1c86e26342c91b73e9dea34");
        std::env::set_var("FORMATION_ADMIN_KEY_DESCRIPTIONS", "test-admin");
    }
    
    /// Clean up test environment variables
    pub fn cleanup_test_env() {
        std::env::remove_var("FORMATION_ADMIN_KEY");
        std::env::remove_var("FORMATION_ADMIN_KEYS");
        std::env::remove_var("FORMATION_ADMIN_KEY_DESCRIPTIONS");
        std::env::remove_var("FORMATION_ADMIN_ENABLED");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::test_utils::*;
    
    #[test]
    fn test_admin_config_from_single_key() {
        setup_test_env();
        
        let config = AdminConfig::from_environment().unwrap();
        assert!(config.enabled);
        assert_eq!(config.admin_addresses.len(), 1);
        
        cleanup_test_env();
    }
    
    #[test]
    fn test_admin_config_validation() {
        let config = create_test_admin_config();
        assert!(config.validate().is_ok());
        assert!(config.security_validate().is_ok());
    }
    
    #[test]
    fn test_is_admin_address() {
        let config = create_test_admin_config();
        let admin_addr = "0x742d35cc6bd8c532f1c86e26342c91b73e9dea34".parse().unwrap();
        let user_addr = "0x1234567890123456789012345678901234567890".parse().unwrap();
        
        assert!(config.is_admin_address(&admin_addr));
        assert!(!config.is_admin_address(&user_addr));
    }
}
```

## Deployment Examples

### Docker Deployment

```dockerfile
# Dockerfile
FROM rust:1.70 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/form-state /usr/local/bin/
EXPOSE 3004

# Admin key configured via environment variable
ENV FORMATION_ADMIN_KEY=""
CMD ["form-state"]
```

```bash
# Run with admin key
docker run -e FORMATION_ADMIN_KEY=0x742d35cc6bd8c532f1c86e26342c91b73e9dea34 formation/form-state
```

### Kubernetes Deployment

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: form-state
spec:
  replicas: 1
  selector:
    matchLabels:
      app: form-state
  template:
    metadata:
      labels:
        app: form-state
    spec:
      containers:
      - name: form-state
        image: formation/form-state:latest
        ports:
        - containerPort: 3004
        env:
        - name: FORMATION_ADMIN_KEY
          valueFrom:
            secretKeyRef:
              name: formation-admin-keys
              key: admin-key
        - name: FORMATION_ADMIN_KEY_DESCRIPTIONS
          value: "k8s-admin"
---
apiVersion: v1
kind: Secret
metadata:
  name: formation-admin-keys
type: Opaque
data:
  admin-key: MHg3NDJkMzVjYzZiZDhjNTMyZjFjODZlMjYzNDJjOTFiNzNlOWRlYTM0  # base64 encoded
```

### systemd Service

```ini
# /etc/systemd/system/form-state.service
[Unit]
Description=Formation State Service
After=network.target

[Service]
Type=simple
User=formation
WorkingDirectory=/opt/formation
ExecStart=/opt/formation/bin/form-state
Environment=FORMATION_ADMIN_KEY=0x742d35cc6bd8c532f1c86e26342c91b73e9dea34
Environment=FORMATION_ADMIN_KEY_DESCRIPTIONS=systemd-admin
Restart=always

[Install]
WantedBy=multi-user.target
```

## Configuration Migration Strategy

### Version Compatibility

```rust
/// Handle configuration format changes across versions
pub enum ConfigVersion {
    V1, // Single FORMATION_ADMIN_KEY
    V2, // Multiple FORMATION_ADMIN_KEYS with descriptions
    V3, // Future: TOML configuration files
}

impl AdminConfig {
    /// Load configuration with automatic version detection
    pub fn load_with_migration() -> Result<Self, AdminConfigError> {
        // Try V2 format first (multiple keys)
        if std::env::var("FORMATION_ADMIN_KEYS").is_ok() {
            return Self::from_environment();
        }
        
        // Fall back to V1 format (single key)
        if std::env::var("FORMATION_ADMIN_KEY").is_ok() {
            return Self::from_environment();
        }
        
        // No configuration found
        log::info!("No admin configuration found, using default (no admin keys)");
        Ok(Self::default())
    }
}
```

## Conclusion

This admin key storage and configuration design provides:

1. **Cryptographic Consistency**: Uses same ECDSA system as user keys
2. **Simple Configuration**: Environment variable-based setup
3. **Security Focus**: Address-only storage, validation, audit logging
4. **Service Integration**: Shared configuration across all Formation services
5. **Production Ready**: Docker, Kubernetes, systemd deployment support
6. **Testing Support**: Comprehensive test utilities and validation

The design ensures admin keys can be easily configured at deployment time while maintaining security best practices and providing a foundation for the admin bypass authentication system. 