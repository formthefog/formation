use futures::StreamExt;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio_stream::wrappers::TcpListenerStream;
use tokio_rustls_acme::{AcmeConfig, caches::DirCache};
use std::net::SocketAddr;
use std::time::Duration;

// Import your reverse proxy types (adjust the module paths as needed)
use form_rplb::{
    proxy::ReverseProxy,
    backend::Backend,
    config::ProxyConfig,
    error::ProxyError,
};

/// A simple mock HTTP server that prints requests and returns a basic response.
/// This simulates your backend servers.
async fn run_mock_server(addr: SocketAddr, server_name: &'static str) {
    let listener = TcpListener::bind(addr).await.unwrap();
    println!("Mock server {} listening on {}", server_name, addr);
    loop {
        let (mut socket, _) = listener.accept().await.unwrap();
        let server_name = server_name.to_string();
        tokio::spawn(async move {
            let mut buffer = vec![0u8; 1024];
            let n = socket.read(&mut buffer).await.unwrap_or(0);
            println!("Server {} received:\n{}", server_name, String::from_utf8_lossy(&buffer[..n]));
            let body = format!(
                "<html><body><h1>Hello from {}</h1></body></html>",
                server_name
            );
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: text/html\r\n\r\n{}",
                body.len(),
                body
            );
            socket.write_all(response.as_bytes()).await.unwrap();
        });
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Start your mock backend servers
    let backend1_addr: SocketAddr = "127.0.0.1:8081".parse()?;
    let backend2_addr: SocketAddr = "127.0.0.1:8082".parse()?;
    tokio::spawn(run_mock_server(backend1_addr, "BACKEND_ONE"));
    tokio::spawn(run_mock_server(backend2_addr, "BACKEND_TWO"));

    // Set up your reverse proxy with a configuration.
    let config = ProxyConfig::default();
    let proxy = ReverseProxy::new(config);
    // For this integration, we assume your backends serve plain HTTP.
    // You can add a route for your public domain.
    let public_backend = Backend::new(
        vec![backend1_addr, backend2_addr],
        // Here we use Protocol::HTTP since the proxy will forward decrypted (plain) traffic.
        form_rplb::protocol::Protocol::HTTP,
        Duration::from_secs(30),
        1000
    );
    proxy.add_route("example.formation.cloud".to_string(), public_backend).await;
    println!("Route added for example.formation.cloud");

    // Set up the TLS listener using tokio-rustls-acme.
    // Here we bind to port 443.
    let tcp_listener = TcpListener::bind("0.0.0.0:443").await?;
    let tcp_incoming = TcpListenerStream::new(tcp_listener);

    // Create the AcmeConfig.
    // NOTE: Adjust the directory_url to production when you’re ready.
    let mut tls_incoming = AcmeConfig::new(["example.formation.cloud"])
        .directory("https://acme-v02.api.letsencrypt.org/directory")
        .contact_push("mailto:admin@formation.cloud")
        .cache(DirCache::new("./rustls_acme_cache"))
        // Advertise an ALPN protocol such as "http/1.1" so that normal clients negotiate correctly.
        .incoming(tcp_incoming, vec![b"http/1.1".to_vec()]);

    println!("Public TLS proxy listening on port 443");

    // For each accepted TLS connection, handle it via the reverse proxy.
    while let Some(tls_conn) = tls_incoming.next().await {
        match tls_conn {
            Ok(tls_stream) => {
                let proxy_clone = proxy.clone();
                tokio::spawn(async move {
                    if let Err(e) = proxy.handle_connection(tls_stream).await {
                        eprintln!("Error handling TLS connection: {}", e);
                    }
                });
            }
            Err(e) => {
                eprintln!("TLS accept error: {:?}", e);
            }
        }
    }

    Ok(())
}
