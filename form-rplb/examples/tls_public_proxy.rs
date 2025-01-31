// examples/tls_public_proxy.rs
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::Arc,
    time::Duration,
};

use form_rplb::{
    backend::Backend,
    certs::{ChallengeMap, start_acme_challenge_server, obtain_cert_http_challenge, FormSniResolver},
    config::ProxyConfig,
    protocol::{Protocol, TlsConfig},
    proxy::ReverseProxy,
};
use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_rustls::rustls::ServerConfig;

// Mock HTTPs server implementation
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
    tokio::spawn(run_mock_server(backend1_addr.clone(), "BACKEND_ONE"));
    tokio::spawn(run_mock_server(backend2_addr.clone(), "BACKEND_TWO"));
    // (1) Start the ACME challenge server on port 80.
    // This will serve challenge tokens at http://<your-domain>/.well-known/acme-challenge/<token>
    let challenge_map = Arc::new(ChallengeMap::new());
    start_acme_challenge_server(challenge_map.clone(), 80).await;
    println!("ACME challenge server started on port 80.");

    // (2) Wait briefly to ensure the challenge server is up.
    tokio::time::sleep(Duration::from_secs(1)).await;

    // (3) Acquire a certificate for your public domain.
    // IMPORTANT: Before going live, update the directory URL in obtain_cert_http_challenge from staging to production.
    let domain = "example.formation.cloud";
    println!("Obtaining certificate for domain: {}", domain);
    let domain_cert = obtain_cert_http_challenge(domain, challenge_map.clone())?;
    println!("Certificate obtained for {}", domain);

    // (4) Create a dynamic SNI resolver and add your domain certificate.
    let mut domain_map = HashMap::new();
    domain_map.insert(domain.to_string(), domain_cert);
    let sni_resolver = FormSniResolver { domain_map };

    // (5) Build the TLS server configuration with the SNI resolver.
    // Note: We use the builder’s safe defaults and no client authentication.
    let tls_config = ServerConfig::builder()
        .with_no_client_auth()
        .with_cert_resolver(Arc::new(sni_resolver));

    // Wrap the TLS config in your TlsConfig type for use in your proxy.
    let tls_config_wrapper = TlsConfig::new(tls_config);

    // (6) Set up your reverse proxy.
    let proxy_config = ProxyConfig::default();
    let proxy = ReverseProxy::new(proxy_config);

    // (7) Configure one or more backends.
    // For this example, we’ll assume a single backend listening on 127.0.0.1:8081.
    let backend = Backend::new(
        vec![backend1_addr, backend2_addr],
        Protocol::HTTPS(tls_config_wrapper), // Use HTTPS backend protocol.
        Duration::from_secs(30),
        1000,
    );
    proxy.add_route(domain.to_string(), backend).await;
    println!("Route added for {}", domain);

    // (8) Start listening for incoming TLS connections.
    // Here, we bind to port 443 (the standard HTTPS port).
    let proxy_addr: SocketAddr = "0.0.0.0:443".parse()?;
    let listener = TcpListener::bind(&proxy_addr).await?;
    println!("Public TLS proxy listening on https://{}", proxy_addr);

    // (9) Handle incoming connections by spawning tasks.
    loop {
        let (client_stream, client_addr) = listener.accept().await?;
        println!("Received request from {client_addr:?}");
        let proxy = proxy.clone();
        println!("Accepted connection from {}", client_addr);
        tokio::spawn(async move {
            if let Err(e) = proxy.handle_connection(client_stream).await {
                eprintln!("Error handling connection from {}: {}", client_addr, e);
            }
        });
    }
}
