use axum::{
    http::Request,
    body::Body,
    extract::{FromRequest, Json, Path, Query},
};
use http::HeaderMap;
use serde::{Serialize, Deserialize};
use serde_json::Value;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use crate::error::AuthError;
use crate::signature::SignatureData;

/// Configuration for signature extraction
#[derive(Debug, Clone)]
pub struct SignatureConfig {
    /// Header names for signature data
    pub header_names: SignatureHeaderNames,
    
    /// Whether to check the request body for signature
    pub check_body: bool,
    
    /// Whether timestamp is required
    pub require_timestamp: bool,
}

impl Default for SignatureConfig {
    fn default() -> Self {
        Self {
            header_names: SignatureHeaderNames::default(),
            check_body: true,
            require_timestamp: true,
        }
    }
}

/// Header names for signature data
#[derive(Debug, Clone)]
pub struct SignatureHeaderNames {
    /// Header name for the signature
    pub signature: String,
    
    /// Header name for the recovery ID
    pub recovery_id: String,
    
    /// Header name for the timestamp
    pub timestamp: String,
}

impl Default for SignatureHeaderNames {
    fn default() -> Self {
        Self {
            signature: "X-Signature".to_string(),
            recovery_id: "X-Recovery-ID".to_string(),
            timestamp: "X-Timestamp".to_string(),
        }
    }
}

/// Extract a header value as a string
fn extract_header_value(headers: &HeaderMap, header_name: &str) -> Result<Option<String>, AuthError> {
    match headers.get(header_name) {
        Some(value) => {
            let value_str = value.to_str()
                .map_err(|_| AuthError::InvalidHeader(format!("Invalid header value: {:?}", value)))?
                .to_string();
            Ok(Some(value_str))
        },
        None => Ok(None),
    }
}

/// Extract and verify timestamp string
fn extract_timestamp(headers: &HeaderMap, header_name: &str, required: bool) -> Result<Option<i64>, AuthError> {
    match headers.get(header_name) {
        Some(value) => {
            // Parse the timestamp
            let timestamp_str = value.to_str()
                .map_err(|_| AuthError::InvalidHeader(format!("Invalid timestamp header: {:?}", value)))?;
            
            let timestamp = timestamp_str.parse::<i64>()
                .map_err(|_| AuthError::InvalidHeader(format!("Invalid timestamp format: {}", timestamp_str)))?;
            
            Ok(Some(timestamp))
        },
        None if required => Err(AuthError::MissingData(format!("Missing {} header", header_name))),
        None => Ok(None),
    }
}

/// Extract signature data from request headers
pub fn extract_from_headers(headers: &HeaderMap, config: &SignatureConfig) -> Result<Option<SignatureData>, AuthError> {
    // Extract signature
    let signature = match extract_header_value(headers, &config.header_names.signature)? {
        Some(sig) => sig,
        None => return Ok(None), // No signature, return None
    };
    
    // Extract recovery ID (required if signature is present)
    let recovery_id = extract_header_value(headers, &config.header_names.recovery_id)?
        .ok_or_else(|| AuthError::MissingData(format!("Missing {} header", &config.header_names.recovery_id)))?;
    
    // Extract timestamp (optional based on config)
    let timestamp = extract_timestamp(headers, &config.header_names.timestamp, config.require_timestamp)?
        .unwrap_or(0); // Default to 0 if not required and not present
    
    Ok(Some(SignatureData::new(
        signature,
        recovery_id,
        timestamp,
        String::new() // Empty message for now, will be filled later
    )))
}

/// Extract signature data from a JSON request body
/// 
/// This extracts signature, recovery_id, and timestamp fields from the JSON body,
/// and removes them from the body before returning the signature data.
pub async fn extract_from_json_body(body: &Json<Value>) -> Result<Option<SignatureData>, AuthError> {
    let json = &body.0;
    
    // Check if the JSON contains signature fields
    let has_signature = json.get("signature").is_some();
    
    if !has_signature {
        return Ok(None);
    }
    
    // Extract required fields
    let signature = json.get("signature")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| AuthError::MissingData("signature".to_string()))?;
    
    let recovery_id = json.get("recovery_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| AuthError::MissingData("recovery_id".to_string()))?;
    
    let timestamp = json.get("timestamp")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| AuthError::MissingData("timestamp".to_string()))?;
    
    // Create a new JSON body without the signature fields for hashing
    let mut json_without_sig = json.clone();
    
    if let Value::Object(ref mut map) = json_without_sig {
        map.remove("signature");
        map.remove("recovery_id");
        map.remove("timestamp");
    }
    
    // Convert the filtered JSON to a string for the message
    let message = serde_json::to_string(&json_without_sig)
        .map_err(|e| AuthError::Internal(format!("Failed to serialize JSON: {}", e)))?;
    
    Ok(Some(SignatureData::new(
        signature,
        recovery_id,
        timestamp,
        message
    )))
}

/// Extract signature data from a request, trying headers first then body
pub async fn extract_signature_data(
    req: &Request<Body>,
    config: &SignatureConfig,
) -> Result<SignatureData, AuthError> {
    // First try to extract from headers
    if let Some(sig_data) = extract_from_headers(req.headers(), config)? {
        return Ok(sig_data);
    }
    
    // If not found in headers and body checking is enabled, try to extract from body
    if config.check_body {
        // This is a placeholder - in a real implementation, you would need to
        // actually parse the body and check for signature fields
        // This is complex with axum since we can't easily clone the body
        return Err(AuthError::MissingSignature);
    }
    
    Err(AuthError::MissingSignature)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderMap;
    use http::header::HeaderValue;
    use serde_json::json;
    
    #[test]
    fn test_extract_from_headers() {
        // Create test config
        let config = SignatureConfig {
            header_names: SignatureHeaderNames {
                signature: "X-Signature".to_string(),
                recovery_id: "X-Recovery-ID".to_string(),
                timestamp: "X-Timestamp".to_string(),
            },
            check_body: true,
            require_timestamp: true,
        };
        
        // Create headers with valid signature data
        let mut headers = HeaderMap::new();
        headers.insert("X-Signature", HeaderValue::from_static("1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"));
        headers.insert("X-Recovery-ID", HeaderValue::from_static("00"));
        headers.insert("X-Timestamp", HeaderValue::from_static("1625097600"));
        
        // Test successful extraction
        let result = extract_from_headers(&headers, &config).unwrap();
        assert!(result.is_some());
        let sig_data = result.unwrap();
        assert_eq!(sig_data.signature, "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef");
        assert_eq!(sig_data.recovery_id, "00");
        assert_eq!(sig_data.timestamp, 1625097600);
        assert_eq!(sig_data.message, "");
        
        // Test missing signature
        let mut missing_sig_headers = headers.clone();
        missing_sig_headers.remove("X-Signature");
        let result = extract_from_headers(&missing_sig_headers, &config).unwrap();
        assert!(result.is_none());
        
        // Test missing recovery ID
        let mut missing_recid_headers = headers.clone();
        missing_recid_headers.remove("X-Recovery-ID");
        let result = extract_from_headers(&missing_recid_headers, &config);
        assert!(result.is_err());
        
        // Test missing timestamp
        let mut missing_ts_headers = headers.clone();
        missing_ts_headers.remove("X-Timestamp");
        let result = extract_from_headers(&missing_ts_headers, &config);
        assert!(result.is_err());
        
        // Test with timestamp not required
        let no_ts_config = SignatureConfig {
            require_timestamp: false,
            ..config
        };
        let result = extract_from_headers(&missing_ts_headers, &no_ts_config).unwrap();
        assert!(result.is_some());
    }
    
    #[test]
    fn test_extract_from_json_body() {
        // Create a test JSON body with signature data
        let body = Json(json!({
            "signature": "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
            "recovery_id": "00",
            "timestamp": 1625097600,
            "data": {
                "name": "test",
                "value": 123
            }
        }));
        
        // Test successful extraction
        let rt = tokio_test::block_on(async {
            let result = extract_from_json_body(&body).await;
            result
        });
        
        assert!(rt.is_ok());
        let sig_data = rt.unwrap().unwrap();
        assert_eq!(sig_data.signature, "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef");
        assert_eq!(sig_data.recovery_id, "00");
        assert_eq!(sig_data.timestamp, 1625097600);
        
        // The message should be the JSON with signature fields removed
        assert!(sig_data.message.contains("data"));
        assert!(sig_data.message.contains("name"));
        assert!(sig_data.message.contains("value"));
        assert!(!sig_data.message.contains("signature"));
        assert!(!sig_data.message.contains("recovery_id"));
        assert!(!sig_data.message.contains("timestamp"));
        
        // Test missing signature
        let body_missing_sig = Json(json!({
            "recovery_id": "00",
            "timestamp": 1625097600
        }));
        let result = tokio_test::block_on(async {
            extract_from_json_body(&body_missing_sig).await
        });
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
        
        // Test missing recovery_id
        let body_missing_recid = Json(json!({
            "signature": "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
            "timestamp": 1625097600
        }));
        let result = tokio_test::block_on(async {
            extract_from_json_body(&body_missing_recid).await
        });
        assert!(result.is_err());
        
        // Test missing timestamp
        let body_missing_ts = Json(json!({
            "signature": "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
            "recovery_id": "00"
        }));
        let result = tokio_test::block_on(async {
            extract_from_json_body(&body_missing_ts).await
        });
        assert!(result.is_err());
    }
} 