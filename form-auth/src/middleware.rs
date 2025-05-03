use std::pin::Pin;
use std::task::{Context, Poll};
use std::future::Future;
use axum::{
    body::Body,
    extract::Request,
    response::{IntoResponse, Response},
};
use tower::{Layer, Service};
use k256::ecdsa::VerifyingKey;
use crate::error::AuthError;
use crate::signature::{verify_signature, SignatureData};
use crate::extractor::{extract_from_headers, SignatureConfig, SignatureHeaderNames};

// Define middleware struct
#[derive(Clone)]
pub struct SignatureVerifyLayer {
    verifying_key: VerifyingKey,
    config: SignatureConfig,
    excluded_paths: Vec<String>,
}

impl SignatureVerifyLayer {
    pub fn new(verifying_key: VerifyingKey) -> Self {
        Self {
            verifying_key,
            config: SignatureConfig::default(),
            excluded_paths: vec![],
        }
    }
    
    /// Set custom header names
    pub fn with_header_names(mut self, header_names: SignatureHeaderNames) -> Self {
        self.config.header_names = header_names;
        self
    }
    
    /// Exclude a path from signature verification
    pub fn exclude_path(mut self, path: &str) -> Self {
        self.excluded_paths.push(path.to_string());
        self
    }
}

impl<S> Layer<S> for SignatureVerifyLayer {
    type Service = SignatureVerifyMiddleware<S>;
    
    fn layer(&self, service: S) -> Self::Service {
        SignatureVerifyMiddleware {
            inner: service,
            verifying_key: self.verifying_key.clone(),
            config: self.config.clone(),
            excluded_paths: self.excluded_paths.clone(),
        }
    }
}

// Define our middleware service
#[derive(Clone)]
pub struct SignatureVerifyMiddleware<S> {
    inner: S,
    verifying_key: VerifyingKey,
    config: SignatureConfig,
    excluded_paths: Vec<String>,
}

// Implement the Service trait for our middleware
impl<S> Service<Request> for SignatureVerifyMiddleware<S>
where
    S: Service<Request, Response = Response> + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;
    
    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }
    
    fn call(&mut self, req: Request) -> Self::Future {
        // Check if path is excluded
        let path = req.uri().path().to_string();
        if self.excluded_paths.iter().any(|excluded| path.starts_with(excluded)) {
            let future = self.inner.call(req);
            return Box::pin(async move {
                future.await
            });
        }
        
        // Try to extract signature data
        let result: Result<(), AuthError> = match extract_from_headers(req.headers(), &self.config) {
            Ok(Some(mut sig_data)) => {
                // For simplicity in this example, use the request path as message
                // In a real scenario, you'd get this from the request body
                sig_data.message = path.clone();
                
                // Verify signature
                match verify_signature(&sig_data, &self.verifying_key) {
                    Ok(true) => {
                        // Signature is valid, continue to the inner service
                        let future = self.inner.call(req);
                        return Box::pin(async move {
                            future.await
                        });
                    },
                    Ok(false) => Err(AuthError::InvalidSignature),
                    Err(err) => Err(err),
                }
            },
            Ok(None) => Err(AuthError::MissingSignature),
            Err(err) => Err(err),
        };
        
        // Handle error cases
        let response = result.unwrap_err().into_response();
        Box::pin(async move { Ok(response) })
    }
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{self, Request, StatusCode},
        routing::get,
        Router,
    };
    use http::header::HeaderValue;
    use k256::ecdsa::{SigningKey, VerifyingKey};
    use rand_core::OsRng;
    use tower::ServiceExt;
    use crate::signature::sign_message;
    use crate::extractor::SignatureHeaderNames;
    use super::SignatureVerifyLayer;
    
    // Helper to create a mock request with signature headers
    fn create_signed_request(path: &str, message: &str, key: &SigningKey) -> Request<Body> {
        let timestamp = chrono::Utc::now().timestamp();
        let (signature, recovery_id) = sign_message(message, timestamp, key).unwrap();
        
        let mut req = Request::builder()
            .uri(path)
            .method("GET");
            
        let headers = req.headers_mut().unwrap();
        headers.insert("X-Signature", HeaderValue::from_str(&signature).unwrap());
        headers.insert("X-Recovery-ID", HeaderValue::from_str(&recovery_id).unwrap());
        headers.insert("X-Timestamp", HeaderValue::from_str(&timestamp.to_string()).unwrap());
        
        req.body(Body::from(message.to_string())).unwrap()
    }
    
    #[tokio::test]
    async fn test_signature_middleware_valid_signature() {
        // Create signing and verification keys
        let signing_key = SigningKey::random(&mut OsRng);
        let verifying_key = VerifyingKey::from(&signing_key);
        
        // Set up the middleware with the test verification key
        let middleware = SignatureVerifyLayer::new(verifying_key)
            .with_header_names(SignatureHeaderNames {
                signature: "X-Signature".to_string(), 
                recovery_id: "X-Recovery-ID".to_string(),
                timestamp: "X-Timestamp".to_string()
            });
        
        // Create a test app with a handler that returns 200
        let app = Router::new()
            .route("/test", get(|| async { "Success" }))
            .layer(middleware);
            
        // Create a signed request
        let path = "/test";
        let request = create_signed_request(path, path, &signing_key);
        
        // Process the request through the middleware
        let response = app.oneshot(request).await.unwrap();
        
        // Check that it passed authentication
        assert_eq!(response.status(), StatusCode::OK);
    }
    
    #[tokio::test]
    async fn test_signature_middleware_invalid_signature() {
        // Create keys
        let signing_key = SigningKey::random(&mut OsRng);
        let wrong_verifying_key = VerifyingKey::from(&SigningKey::random(&mut OsRng)); // Different key
        
        // Set up the middleware with the WRONG verification key
        let middleware = SignatureVerifyLayer::new(wrong_verifying_key)
            .with_header_names(SignatureHeaderNames {
                signature: "X-Signature".to_string(), 
                recovery_id: "X-Recovery-ID".to_string(),
                timestamp: "X-Timestamp".to_string()
            });
        
        // Create a test app
        let app = Router::new()
            .route("/test", get(|| async { "Success" }))
            .layer(middleware);
            
        // Create a signed request with the first key
        let path = "/test";
        let request = create_signed_request(path, path, &signing_key);
        
        // Process the request - should be rejected
        let response = app.oneshot(request).await.unwrap();
        
        // Should fail with 401 Unauthorized
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
    
    #[tokio::test]
    async fn test_signature_middleware_missing_headers() {
        // Create key
        let verifying_key = VerifyingKey::from(&SigningKey::random(&mut OsRng));
        
        // Set up the middleware
        let middleware = SignatureVerifyLayer::new(verifying_key);
        
        // Create a test app
        let app = Router::new()
            .route("/test", get(|| async { "Success" }))
            .layer(middleware);
            
        // Create a request without signature headers
        let request = Request::builder()
            .uri("/test")
            .method("GET")
            .body(Body::empty())
            .unwrap();
        
        // Process the request - should be rejected
        let response = app.oneshot(request).await.unwrap();
        
        // Should fail with 401 Unauthorized
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
    
    #[tokio::test]
    async fn test_middleware_with_excluded_paths() {
        // Create key
        let verifying_key = VerifyingKey::from(&SigningKey::random(&mut OsRng));
        
        // Set up the middleware with an excluded path
        let middleware = SignatureVerifyLayer::new(verifying_key)
            .exclude_path("/health");
        
        // Create a test app
        let app = Router::new()
            .route("/test", get(|| async { "Protected" }))
            .route("/health", get(|| async { "Health Check" }))
            .layer(middleware);
            
        // Request to excluded path without signature headers
        let health_request = Request::builder()
            .uri("/health")
            .method("GET")
            .body(Body::empty())
            .unwrap();
        
        // Should pass without authentication
        let health_response = app.clone().oneshot(health_request).await.unwrap();
        assert_eq!(health_response.status(), StatusCode::OK);
        
        // Request to protected path without headers
        let protected_request = Request::builder()
            .uri("/test")
            .method("GET")
            .body(Body::empty())
            .unwrap();
        
        // Should be rejected
        let protected_response = app.oneshot(protected_request).await.unwrap();
        assert_eq!(protected_response.status(), StatusCode::UNAUTHORIZED);
    }
} 