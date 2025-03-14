// form-fuzzing/src/harness/dns.rs
//! Test harness for DNS management and zone operations

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::generators::dns::{DNSRecord, DNSRecordType, DNSZone};
use crate::instrumentation::fault_injection;
use crate::instrumentation::sanitizer;

/// Result of a DNS operation
#[derive(Debug, Clone, PartialEq)]
pub enum DNSOperationResult {
    /// Operation succeeded
    Success,
    /// Authentication failed
    AuthenticationFailed,
    /// Permission denied
    PermissionDenied,
    /// Zone not found
    ZoneNotFound,
    /// Record not found
    RecordNotFound,
    /// Invalid input
    InvalidInput(String),
    /// Rate limited
    RateLimited,
    /// Internal error
    InternalError(String),
    /// Timeout
    Timeout,
    /// Certificate Error
    CertificateError(String),
}

/// Certificate validation method
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationMethod {
    /// HTTP validation (HTTP-01)
    HTTP,
    /// DNS validation (DNS-01)
    DNS,
    /// Email validation
    Email,
}

/// Certificate type (wildcard or standard)
#[derive(Debug, Clone, PartialEq)]
pub enum CertificateType {
    /// Standard certificate for a specific domain
    Standard,
    /// Wildcard certificate (*.domain.com)
    Wildcard,
}

/// SSL certificate information
#[derive(Debug, Clone)]
pub struct Certificate {
    /// Domain the certificate is for
    pub domain: String,
    /// Whether this is a wildcard certificate
    pub certificate_type: CertificateType,
    /// When the certificate was issued
    pub issued_at: u64,
    /// When the certificate expires
    pub expires_at: u64,
    /// Validation method used
    pub validation_method: ValidationMethod,
    /// Status of the certificate
    pub status: CertificateStatus,
}

/// Certificate status
#[derive(Debug, Clone, PartialEq)]
pub enum CertificateStatus {
    /// Certificate is pending issuance
    Pending,
    /// Certificate is active
    Active,
    /// Certificate is expired
    Expired,
    /// Certificate was revoked
    Revoked,
    /// Certificate issuance failed
    Failed(String),
}

/// Mock DNS zone manager for testing
pub struct MockDNSManager {
    /// Zones managed by this manager
    zones: Arc<Mutex<HashMap<String, DNSZone>>>,
    /// User permissions
    permissions: Arc<Mutex<HashMap<String, Vec<String>>>>,
    /// Rate limiting counters
    rate_limits: Arc<Mutex<HashMap<String, usize>>>,
    /// Maximum number of zones per user
    max_zones_per_user: usize,
    /// Maximum number of records per zone
    max_records_per_zone: usize,
    /// Maximum number of operations per minute
    max_ops_per_minute: usize,
    /// Simulated operation latency
    operation_latency: Duration,
    /// Failure rate for simulating random failures
    failure_rate: f64,
    /// Certificates
    certificates: Arc<Mutex<HashMap<String, Certificate>>>,
}

impl MockDNSManager {
    /// Create a new mock DNS manager
    pub fn new() -> Self {
        Self {
            zones: Arc::new(Mutex::new(HashMap::new())),
            permissions: Arc::new(Mutex::new(HashMap::new())),
            rate_limits: Arc::new(Mutex::new(HashMap::new())),
            max_zones_per_user: 10,
            max_records_per_zone: 100,
            max_ops_per_minute: 60,
            operation_latency: Duration::from_millis(50),
            failure_rate: 0.05,
            certificates: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Set the maximum number of zones per user
    pub fn set_max_zones_per_user(&mut self, max: usize) {
        self.max_zones_per_user = max;
    }

    /// Set the maximum number of records per zone
    pub fn set_max_records_per_zone(&mut self, max: usize) {
        self.max_records_per_zone = max;
    }

    /// Set the maximum number of operations per minute
    pub fn set_max_ops_per_minute(&mut self, max: usize) {
        self.max_ops_per_minute = max;
    }

    /// Set the simulated operation latency
    pub fn set_operation_latency(&mut self, latency: Duration) {
        self.operation_latency = latency;
    }

    /// Set the failure rate for simulating random failures
    pub fn set_failure_rate(&mut self, rate: f64) {
        self.failure_rate = rate;
    }

    /// Grant permission to a user for a zone
    pub fn grant_permission(&self, user_id: &str, zone_name: &str) {
        let mut permissions = self.permissions.lock().unwrap();
        let user_permissions = permissions.entry(user_id.to_string()).or_insert_with(Vec::new);
        if !user_permissions.contains(&zone_name.to_string()) {
            user_permissions.push(zone_name.to_string());
        }
    }

    /// Check if a user has permission for a zone
    fn has_permission(&self, user_id: &str, zone_name: &str) -> bool {
        let permissions = self.permissions.lock().unwrap();
        if let Some(user_permissions) = permissions.get(user_id) {
            user_permissions.contains(&zone_name.to_string())
        } else {
            false
        }
    }

    /// Check rate limits for a user
    fn check_rate_limit(&self, user_id: &str) -> bool {
        let mut rate_limits = self.rate_limits.lock().unwrap();
        let count = rate_limits.entry(user_id.to_string()).or_insert(0);
        *count += 1;
        *count <= self.max_ops_per_minute
    }

    /// Create a new DNS zone
    pub fn create_zone(&self, user_id: &str, zone: DNSZone) -> DNSOperationResult {
        // Inject faults if configured
        if fault_injection::should_inject_fault("dns_create_zone") {
            return DNSOperationResult::InternalError("Injected fault".to_string());
        }

        // Simulate operation latency
        std::thread::sleep(self.operation_latency);

        // Check rate limits
        if !self.check_rate_limit(user_id) {
            return DNSOperationResult::RateLimited;
        }

        // Check if user has reached maximum number of zones
        let permissions = self.permissions.lock().unwrap();
        if let Some(user_permissions) = permissions.get(user_id) {
            if user_permissions.len() >= self.max_zones_per_user {
                return DNSOperationResult::InvalidInput("Maximum number of zones reached".to_string());
            }
        }
        drop(permissions);

        // Validate zone
        if zone.name.is_empty() {
            return DNSOperationResult::InvalidInput("Zone name cannot be empty".to_string());
        }

        if zone.records.len() > self.max_records_per_zone {
            return DNSOperationResult::InvalidInput("Too many records in zone".to_string());
        }

        // Create zone
        let mut zones = self.zones.lock().unwrap();
        if zones.contains_key(&zone.name) {
            return DNSOperationResult::InvalidInput("Zone already exists".to_string());
        }
        zones.insert(zone.name.clone(), zone);

        // Grant permission to the user
        drop(zones);
        self.grant_permission(user_id, &zone.name);

        DNSOperationResult::Success
    }

    /// Delete a DNS zone
    pub fn delete_zone(&self, user_id: &str, zone_name: &str) -> DNSOperationResult {
        // Inject faults if configured
        if fault_injection::should_inject_fault("dns_delete_zone") {
            return DNSOperationResult::InternalError("Injected fault".to_string());
        }

        // Simulate operation latency
        std::thread::sleep(self.operation_latency);

        // Check rate limits
        if !self.check_rate_limit(user_id) {
            return DNSOperationResult::RateLimited;
        }

        // Check permissions
        if !self.has_permission(user_id, zone_name) {
            return DNSOperationResult::PermissionDenied;
        }

        // Delete zone
        let mut zones = self.zones.lock().unwrap();
        if !zones.contains_key(zone_name) {
            return DNSOperationResult::ZoneNotFound;
        }
        zones.remove(zone_name);

        DNSOperationResult::Success
    }

    /// Add a record to a DNS zone
    pub fn add_record(&self, user_id: &str, zone_name: &str, record: DNSRecord) -> DNSOperationResult {
        // Inject faults if configured
        if fault_injection::should_inject_fault("dns_add_record") {
            return DNSOperationResult::InternalError("Injected fault".to_string());
        }

        // Simulate operation latency
        std::thread::sleep(self.operation_latency);

        // Check rate limits
        if !self.check_rate_limit(user_id) {
            return DNSOperationResult::RateLimited;
        }

        // Check permissions
        if !self.has_permission(user_id, zone_name) {
            return DNSOperationResult::PermissionDenied;
        }

        // Add record
        let mut zones = self.zones.lock().unwrap();
        if let Some(zone) = zones.get_mut(zone_name) {
            if zone.records.len() >= self.max_records_per_zone {
                return DNSOperationResult::InvalidInput("Maximum number of records reached".to_string());
            }
            zone.records.push(record);
            DNSOperationResult::Success
        } else {
            DNSOperationResult::ZoneNotFound
        }
    }

    /// Delete a record from a DNS zone
    pub fn delete_record(&self, user_id: &str, zone_name: &str, domain: &str, record_type: DNSRecordType) -> DNSOperationResult {
        // Inject faults if configured
        if fault_injection::should_inject_fault("dns_delete_record") {
            return DNSOperationResult::InternalError("Injected fault".to_string());
        }

        // Simulate operation latency
        std::thread::sleep(self.operation_latency);

        // Check rate limits
        if !self.check_rate_limit(user_id) {
            return DNSOperationResult::RateLimited;
        }

        // Check permissions
        if !self.has_permission(user_id, zone_name) {
            return DNSOperationResult::PermissionDenied;
        }

        // Delete record
        let mut zones = self.zones.lock().unwrap();
        if let Some(zone) = zones.get_mut(zone_name) {
            let initial_len = zone.records.len();
            zone.records.retain(|r| !(r.domain == domain && r.record_type == record_type));
            if zone.records.len() == initial_len {
                return DNSOperationResult::RecordNotFound;
            }
            DNSOperationResult::Success
        } else {
            DNSOperationResult::ZoneNotFound
        }
    }

    /// Update a record in a DNS zone
    pub fn update_record(&self, user_id: &str, zone_name: &str, domain: &str, 
                         record_type: DNSRecordType, new_record: DNSRecord) -> DNSOperationResult {
        // Inject faults if configured
        if fault_injection::should_inject_fault("dns_update_record") {
            return DNSOperationResult::InternalError("Injected fault".to_string());
        }

        // Simulate operation latency
        std::thread::sleep(self.operation_latency);

        // Check rate limits
        if !self.check_rate_limit(user_id) {
            return DNSOperationResult::RateLimited;
        }

        // Check permissions
        if !self.has_permission(user_id, zone_name) {
            return DNSOperationResult::PermissionDenied;
        }

        // Update record
        let mut zones = self.zones.lock().unwrap();
        if let Some(zone) = zones.get_mut(zone_name) {
            let mut found = false;
            for record in &mut zone.records {
                if record.domain == domain && record.record_type == record_type {
                    *record = new_record;
                    found = true;
                    break;
                }
            }
            if !found {
                return DNSOperationResult::RecordNotFound;
            }
            DNSOperationResult::Success
        } else {
            DNSOperationResult::ZoneNotFound
        }
    }

    /// Get all zones for a user
    pub fn get_zones(&self, user_id: &str) -> Vec<String> {
        let permissions = self.permissions.lock().unwrap();
        if let Some(user_permissions) = permissions.get(user_id) {
            user_permissions.clone()
        } else {
            Vec::new()
        }
    }

    /// Get a zone by name
    pub fn get_zone(&self, user_id: &str, zone_name: &str) -> Option<DNSZone> {
        // Check permissions
        if !self.has_permission(user_id, zone_name) {
            return None;
        }

        // Get zone
        let zones = self.zones.lock().unwrap();
        zones.get(zone_name).cloned()
    }

    /// Request a certificate for a domain
    pub fn request_certificate(&self, user_id: &str, domain: &str, cert_type: CertificateType, validation_method: ValidationMethod) -> DNSOperationResult {
        // Inject faults if configured
        if fault_injection::should_inject_fault("dns_request_certificate") {
            return DNSOperationResult::InternalError("Injected fault".to_string());
        }

        // Simulate operation latency
        std::thread::sleep(self.operation_latency);

        // Check rate limits
        if !self.check_rate_limit(user_id) {
            return DNSOperationResult::RateLimited;
        }

        // For wildcard certificates, require DNS validation
        if cert_type == CertificateType::Wildcard && validation_method != ValidationMethod::DNS {
            return DNSOperationResult::CertificateError("Wildcard certificates require DNS validation".to_string());
        }

        // Check if domain is associated with any of the zones the user has permission for
        let domain_zone = domain.split('.').skip(1).collect::<Vec<&str>>().join(".");
        if !self.has_permission(user_id, &domain_zone) {
            return DNSOperationResult::PermissionDenied;
        }

        // Check if certificate already exists
        let mut certificates = self.certificates.lock().unwrap();
        if certificates.contains_key(domain) {
            return DNSOperationResult::InvalidInput("Certificate already exists for this domain".to_string());
        }

        // Create new certificate
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        let cert = Certificate {
            domain: domain.to_string(),
            certificate_type: cert_type,
            issued_at: now,
            expires_at: now + 90 * 24 * 60 * 60, // 90 days
            validation_method,
            status: CertificateStatus::Pending,
        };

        certificates.insert(domain.to_string(), cert);
        
        DNSOperationResult::Success
    }

    /// Verify domain ownership for certificate issuance
    pub fn verify_certificate(&self, user_id: &str, domain: &str) -> DNSOperationResult {
        // Inject faults if configured
        if fault_injection::should_inject_fault("dns_verify_certificate") {
            return DNSOperationResult::InternalError("Injected fault".to_string());
        }

        // Simulate operation latency
        std::thread::sleep(self.operation_latency);

        // Check rate limits
        if !self.check_rate_limit(user_id) {
            return DNSOperationResult::RateLimited;
        }

        // Check if certificate exists
        let mut certificates = self.certificates.lock().unwrap();
        let cert = match certificates.get_mut(domain) {
            Some(cert) => cert,
            None => return DNSOperationResult::CertificateError("No pending certificate found for this domain".to_string()),
        };

        // Only pending certificates can be verified
        if cert.status != CertificateStatus::Pending {
            return DNSOperationResult::CertificateError("Certificate is not in pending state".to_string());
        }

        // Simulate validation process
        let mut rng = rand::thread_rng();
        if rng.gen_bool(0.9) { // 90% success rate
            cert.status = CertificateStatus::Active;
            DNSOperationResult::Success
        } else {
            cert.status = CertificateStatus::Failed("Validation failed".to_string());
            DNSOperationResult::CertificateError("Failed to verify domain ownership".to_string())
        }
    }

    /// Renew a certificate
    pub fn renew_certificate(&self, user_id: &str, domain: &str) -> DNSOperationResult {
        // Inject faults if configured
        if fault_injection::should_inject_fault("dns_renew_certificate") {
            return DNSOperationResult::InternalError("Injected fault".to_string());
        }

        // Simulate operation latency
        std::thread::sleep(self.operation_latency);

        // Check rate limits
        if !self.check_rate_limit(user_id) {
            return DNSOperationResult::RateLimited;
        }

        // Check if certificate exists
        let mut certificates = self.certificates.lock().unwrap();
        let cert = match certificates.get_mut(domain) {
            Some(cert) => cert,
            None => return DNSOperationResult::CertificateError("No certificate found for this domain".to_string()),
        };

        // Only active certificates can be renewed
        if cert.status != CertificateStatus::Active {
            return DNSOperationResult::CertificateError("Certificate is not active".to_string());
        }

        // Update certificate
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        cert.issued_at = now;
        cert.expires_at = now + 90 * 24 * 60 * 60; // 90 days
        
        DNSOperationResult::Success
    }

    /// Revoke a certificate
    pub fn revoke_certificate(&self, user_id: &str, domain: &str) -> DNSOperationResult {
        // Inject faults if configured
        if fault_injection::should_inject_fault("dns_revoke_certificate") {
            return DNSOperationResult::InternalError("Injected fault".to_string());
        }

        // Simulate operation latency
        std::thread::sleep(self.operation_latency);

        // Check rate limits
        if !self.check_rate_limit(user_id) {
            return DNSOperationResult::RateLimited;
        }

        // Check if certificate exists
        let mut certificates = self.certificates.lock().unwrap();
        let cert = match certificates.get_mut(domain) {
            Some(cert) => cert,
            None => return DNSOperationResult::CertificateError("No certificate found for this domain".to_string()),
        };

        // Update certificate status
        cert.status = CertificateStatus::Revoked;
        
        DNSOperationResult::Success
    }

    /// Get all certificates for a user
    pub fn get_certificates(&self, user_id: &str) -> Vec<Certificate> {
        // Get zones the user has permission for
        let user_zones = self.get_zones(user_id);
        
        // Collect certificates for domains in those zones
        let certificates = self.certificates.lock().unwrap();
        certificates.values()
            .filter(|cert| {
                // Extract the zone from the certificate domain
                let domain_parts: Vec<&str> = cert.domain.split('.').collect();
                if domain_parts.len() < 2 {
                    return false;
                }
                
                let zone = domain_parts[1..].join(".");
                user_zones.contains(&zone)
            })
            .cloned()
            .collect()
    }

    /// Get a specific certificate
    pub fn get_certificate(&self, user_id: &str, domain: &str) -> Option<Certificate> {
        // Extract the zone from the domain
        let domain_parts: Vec<&str> = domain.split('.').collect();
        if domain_parts.len() < 2 {
            return None;
        }
        
        let zone = domain_parts[1..].join(".");
        
        // Check if user has permission for the zone
        if !self.has_permission(user_id, &zone) {
            return None;
        }
        
        // Get the certificate
        let certificates = self.certificates.lock().unwrap();
        certificates.get(domain).cloned()
    }
}

/// Mock DNS authentication service
pub struct MockDNSAuthenticator {
    /// Valid API keys
    api_keys: HashMap<String, String>,
    /// Failure rate for simulating random failures
    failure_rate: f64,
}

impl MockDNSAuthenticator {
    /// Create a new mock DNS authenticator
    pub fn new() -> Self {
        let mut api_keys = HashMap::new();
        api_keys.insert("user1".to_string(), "key1".to_string());
        api_keys.insert("user2".to_string(), "key2".to_string());
        api_keys.insert("admin".to_string(), "admin_key".to_string());

        Self {
            api_keys,
            failure_rate: 0.05,
        }
    }

    /// Set the failure rate for simulating random failures
    pub fn set_failure_rate(&mut self, rate: f64) {
        self.failure_rate = rate;
    }

    /// Verify an API key
    pub fn verify_api_key(&self, user_id: &str, api_key: &str) -> bool {
        // Inject faults if configured
        if fault_injection::should_inject_fault("dns_auth") {
            return false;
        }

        // Check if the API key is valid
        if let Some(valid_key) = self.api_keys.get(user_id) {
            valid_key == api_key
        } else {
            false
        }
    }

    /// Register a new API key
    pub fn register_api_key(&mut self, user_id: &str, api_key: &str) {
        self.api_keys.insert(user_id.to_string(), api_key.to_string());
    }
}

/// DNS harness for testing DNS operations
pub struct DNSHarness {
    /// DNS manager
    pub manager: MockDNSManager,
    /// DNS authenticator
    pub authenticator: MockDNSAuthenticator,
}

impl DNSHarness {
    /// Create a new DNS harness
    pub fn new() -> Self {
        Self {
            manager: MockDNSManager::new(),
            authenticator: MockDNSAuthenticator::new(),
        }
    }

    /// Create a zone with authentication
    pub fn create_zone(&self, user_id: &str, api_key: &str, zone: DNSZone) -> DNSOperationResult {
        // Verify API key
        if !self.authenticator.verify_api_key(user_id, api_key) {
            return DNSOperationResult::AuthenticationFailed;
        }

        // Create zone
        self.manager.create_zone(user_id, zone)
    }

    /// Delete a zone with authentication
    pub fn delete_zone(&self, user_id: &str, api_key: &str, zone_name: &str) -> DNSOperationResult {
        // Verify API key
        if !self.authenticator.verify_api_key(user_id, api_key) {
            return DNSOperationResult::AuthenticationFailed;
        }

        // Delete zone
        self.manager.delete_zone(user_id, zone_name)
    }

    /// Add a record with authentication
    pub fn add_record(&self, user_id: &str, api_key: &str, zone_name: &str, record: DNSRecord) -> DNSOperationResult {
        // Verify API key
        if !self.authenticator.verify_api_key(user_id, api_key) {
            return DNSOperationResult::AuthenticationFailed;
        }

        // Add record
        self.manager.add_record(user_id, zone_name, record)
    }

    /// Delete a record with authentication
    pub fn delete_record(&self, user_id: &str, api_key: &str, zone_name: &str, 
                         domain: &str, record_type: DNSRecordType) -> DNSOperationResult {
        // Verify API key
        if !self.authenticator.verify_api_key(user_id, api_key) {
            return DNSOperationResult::AuthenticationFailed;
        }

        // Delete record
        self.manager.delete_record(user_id, zone_name, domain, record_type)
    }

    /// Update a record with authentication
    pub fn update_record(&self, user_id: &str, api_key: &str, zone_name: &str, 
                         domain: &str, record_type: DNSRecordType, new_record: DNSRecord) -> DNSOperationResult {
        // Verify API key
        if !self.authenticator.verify_api_key(user_id, api_key) {
            return DNSOperationResult::AuthenticationFailed;
        }

        // Update record
        self.manager.update_record(user_id, zone_name, domain, record_type, new_record)
    }

    /// Get all zones for a user with authentication
    pub fn get_zones(&self, user_id: &str, api_key: &str) -> Result<Vec<String>, DNSOperationResult> {
        // Verify API key
        if !self.authenticator.verify_api_key(user_id, api_key) {
            return Err(DNSOperationResult::AuthenticationFailed);
        }

        // Get zones
        Ok(self.manager.get_zones(user_id))
    }

    /// Get a zone by name with authentication
    pub fn get_zone(&self, user_id: &str, api_key: &str, zone_name: &str) -> Result<DNSZone, DNSOperationResult> {
        // Verify API key
        if !self.authenticator.verify_api_key(user_id, api_key) {
            return Err(DNSOperationResult::AuthenticationFailed);
        }

        // Get zone
        if let Some(zone) = self.manager.get_zone(user_id, zone_name) {
            Ok(zone)
        } else {
            Err(DNSOperationResult::ZoneNotFound)
        }
    }

    /// Request a certificate with authentication
    pub fn request_certificate(&self, user_id: &str, api_key: &str, domain: &str, cert_type: CertificateType, validation_method: ValidationMethod) -> DNSOperationResult {
        // Verify API key
        if !self.authenticator.verify_api_key(user_id, api_key) {
            return DNSOperationResult::AuthenticationFailed;
        }

        // Request certificate
        self.manager.request_certificate(user_id, domain, cert_type, validation_method)
    }

    /// Verify certificate with authentication
    pub fn verify_certificate(&self, user_id: &str, api_key: &str, domain: &str) -> DNSOperationResult {
        // Verify API key
        if !self.authenticator.verify_api_key(user_id, api_key) {
            return DNSOperationResult::AuthenticationFailed;
        }

        // Verify certificate
        self.manager.verify_certificate(user_id, domain)
    }

    /// Renew certificate with authentication
    pub fn renew_certificate(&self, user_id: &str, api_key: &str, domain: &str) -> DNSOperationResult {
        // Verify API key
        if !self.authenticator.verify_api_key(user_id, api_key) {
            return DNSOperationResult::AuthenticationFailed;
        }

        // Renew certificate
        self.manager.renew_certificate(user_id, domain)
    }

    /// Revoke certificate with authentication
    pub fn revoke_certificate(&self, user_id: &str, api_key: &str, domain: &str) -> DNSOperationResult {
        // Verify API key
        if !self.authenticator.verify_api_key(user_id, api_key) {
            return DNSOperationResult::AuthenticationFailed;
        }

        // Revoke certificate
        self.manager.revoke_certificate(user_id, domain)
    }

    /// Get all certificates with authentication
    pub fn get_certificates(&self, user_id: &str, api_key: &str) -> Result<Vec<Certificate>, DNSOperationResult> {
        // Verify API key
        if !self.authenticator.verify_api_key(user_id, api_key) {
            return Err(DNSOperationResult::AuthenticationFailed);
        }

        // Get certificates
        Ok(self.manager.get_certificates(user_id))
    }

    /// Get a specific certificate with authentication
    pub fn get_certificate(&self, user_id: &str, api_key: &str, domain: &str) -> Result<Certificate, DNSOperationResult> {
        // Verify API key
        if !self.authenticator.verify_api_key(user_id, api_key) {
            return Err(DNSOperationResult::AuthenticationFailed);
        }

        // Get certificate
        match self.manager.get_certificate(user_id, domain) {
            Some(cert) => Ok(cert),
            None => Err(DNSOperationResult::CertificateError("Certificate not found".to_string())),
        }
    }
}
