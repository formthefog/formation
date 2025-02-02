//examples/tls_vanity_proxy.rs
use form_rplb::{proxy::ReverseProxy, backend::Backend, protocol::{Protocol, TlsConfig}, config::ProxyConfig};
use std::{
    fs::File, io::BufReader, net::SocketAddr, path::{Path, PathBuf}, sync::Arc, time::Duration
};
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::TcpListener};
use tokio_rustls::rustls::pki_types::{CertificateDer, PrivateKeyDer};
use tokio_rustls_acme::tokio_rustls::rustls::ServerConfig;

/// Load raw certificate data from a PEM file
fn load_certs(path: impl AsRef<Path>) -> std::io::Result<Vec<CertificateDer<'static>>> {
    let path: PathBuf = path.as_ref().into();
    println!("Attempting to read cert from {path:?}");
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    rustls_pemfile::certs(&mut reader)
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid certificate"))
        .map(|certs| certs.into_iter().map(CertificateDer::from).collect())
}

/// Load raw private key data from a PEM file
fn load_private_key(path: impl AsRef<Path>) -> std::io::Result<PrivateKeyDer<'static>> {
    let path: PathBuf = path.as_ref().into();
    println!("Attempting to read private key from {path:?}");
    let file = File::open(path.clone())?;
    let mut reader = BufReader::new(file);

    if let Ok(keys) = rustls_pemfile::pkcs8_private_keys(&mut reader) {
        if let Some(key) = keys.first() {
            return Ok(PrivateKeyDer::Pkcs8(key.clone().into()));
        }
    }

    let file = File::open(path.clone())?;
    let mut reader = BufReader::new(file);
    if let Ok(keys) = rustls_pemfile::rsa_private_keys(&mut reader) {
        if let Some(key) = keys.first() {
            return Ok(PrivateKeyDer::Pkcs8(key.clone().into()));
        }
    }
    Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid private key"))
}

// Mock HTTP server implementation
async fn run_mock_server(addr: SocketAddr, server_name: &'static str) {
    let listener = TcpListener::bind(addr).await.unwrap();
    println!("Mock server {} listening on {}", server_name, addr);

    loop {
        let (mut socket, client_addr) = listener.accept().await.unwrap();
        let server_id = server_name.to_string();
        println!("Server {} received connection from {}", server_id, client_addr);
        
        tokio::spawn(async move {
            let mut buffer = vec![0; 1024];
            match socket.read(&mut buffer).await {
                Ok(n) => {
                    println!("Server {} received request:\n{}", 
                        server_id, 
                        String::from_utf8_lossy(&buffer[..n]));
                    
                    // Check if this is a favicon request
                    if buffer[..n].windows(12).any(|window| window == b"GET /favicon") {
                        // Send a 404 for favicon
                        let response = "HTTP/1.1 404 Not Found\r\n\
                            Connection: close\r\n\
                            \r\n";
                        socket.write_all(response.as_bytes()).await.unwrap();
                        return;
                    }
                    
                    // Regular response for other requests
                    let body = format!(
                        "<!DOCTYPE html><html>\
                            <head>\
                                <title>Response from {}</title>\
                                <link rel=\"icon\" href=\"data:,\">\
                            </head>\
                            <body>\
                                <h1>Hello from {}</h1>\
                                <p>This response came from backend server {}</p>\
                                <p>Try refreshing to see load balancing in action!</p>\
                            </body>\
                        </html>",
                        server_id, server_id, server_id
                    );
                    
                    let response = format!(
                        "HTTP/1.1 200 OK\r\n\
                        Content-Type: text/html\r\n\
                        Content-Length: {}\r\n\
                        Connection: close\r\n\
                        \r\n\
                        {}",
                        body.len(),
                        body
                    );
                    
                    socket.write_all(response.as_bytes()).await.unwrap();
                    socket.shutdown().await.unwrap();
                }
                Err(e) => eprintln!("Error reading from socket: {}", e),
            }
        });
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let backend1_addr: SocketAddr = "127.0.0.1:8081".parse()?;
    let backend2_addr: SocketAddr = "127.0.0.1:8082".parse()?;

    tokio::spawn(run_mock_server(backend1_addr, "BACKEND_ONE"));
    tokio::spawn(run_mock_server(backend2_addr, "BACKEND_TWO"));

    // Load raw certificate and key data
    let api_certs = load_certs("/home/ans/projects/vrrb/protocol/compute/formation/form-rplb/examples/certs/api.example.internal.pem")?;
    let api_key = load_private_key("/home/ans/projects/vrrb/protocol/compute/formation/form-rplb/examples/certs/api.example.internal-key.pem")?;
    
    let proxy_certs = load_certs("/home/ans/projects/vrrb/protocol/compute/formation/form-rplb/examples/certs/example.internal.pem")?;
    let proxy_key = load_private_key("/home/ans/projects/vrrb/protocol/compute/formation/form-rplb/examples/certs/example.internal-key.pem")?;
    
    // Create server configurations using the builder pattern
    let api_config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(api_certs, api_key)
        .expect("Failed to set API certificate");
    
    let proxy_config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(proxy_certs, proxy_key)
        .expect("Failed to set proxy certificate");
    
    // Create our TlsConfig instances
    let api_tls = TlsConfig::new(Arc::new(api_config));
    let proxy_tls = TlsConfig::new(Arc::new(proxy_config));
    
    // Configure and start the reverse proxy
    let config = ProxyConfig::default();
    let proxy = ReverseProxy::new(config);
    
    // Start our backend servers
    let backend1_addr: SocketAddr = "127.0.0.1:8081".parse()?;
    let backend2_addr: SocketAddr = "127.0.0.1:8082".parse()?;
    
    // Configure backends with TLS
    let api_backend = Backend::new(
        vec![backend1_addr.clone()],
        Protocol::HTTPS(api_tls),
        Duration::from_secs(30),
        1000
    );
    
    let proxy_backend = Backend::new(
        vec![backend2_addr, backend1_addr],
        Protocol::HTTPS(proxy_tls),
        Duration::from_secs(30),
        1000
    );
    
    // Add our TLS-enabled routes
    proxy.add_route("api.example.internal".to_string(), api_backend).await;
    proxy.add_route("example.internal".to_string(), proxy_backend).await;
    
    // Start the proxy server
    let proxy_addr: SocketAddr = "0.0.0.0:8443".parse()?;
    let listener = TcpListener::bind(&proxy_addr).await?;
    
    println!("TLS-enabled proxy listening on https://{}", proxy_addr);
    println!("Available endpoints:");
    println!("  - https://api.example.internal:8443/");
    println!("  - https://example.internal:8443/");
    
    loop {
        let (client_stream, _client_addr) = listener.accept().await?;
        let proxy = proxy.clone();
        
        tokio::spawn(async move {
            if let Err(e) = proxy.handle_http_connection(client_stream).await {
                eprintln!("Error handling TLS connection: {}", e);
            }
        });
    }
}
