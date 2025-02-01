use form_rplb::backend::Backend;
use form_rplb::config::ProxyConfig;
use form_rplb::protocol::Protocol;
use form_rplb::protocol::TlsConfig;
use form_rplb::proxy::ReverseProxy;
use form_rplb::resolver::TlsManager;
use tokio_rustls_acme::tokio_rustls::rustls::ServerConfig;
use std::net::SocketAddr;
use std::sync::Mutex;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use std::sync::Arc;


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
    // Start our backend servers (same as before)
    let backend1_addr: SocketAddr = "127.0.0.1:8081".parse().unwrap();
    let backend2_addr: SocketAddr = "127.0.0.1:8082".parse().unwrap();
    
    tokio::spawn(run_mock_server(backend1_addr, "BACKEND_ONE"));
    tokio::spawn(run_mock_server(backend2_addr, "BACKEND_TWO"));

    let tls_manager = Arc::new(Mutex::new(TlsManager::new(vec!["example.formation.cloud".to_string()])));
    
    // Add our test domain to the TLS manager
//    if let Ok(mut guard) = tls_manager.lock() {
//        guard.add_domain("example.formation.cloud".to_string(), true)?;
//        println!("Added domain...");
//    }

    let guard = tls_manager.lock().unwrap();
    let _resolver = guard.resolver.clone(); 
    let server_config = guard.config.clone(); 

    let config = ProxyConfig::default();
    let proxy = Arc::new(ReverseProxy::new(config));

    let backend = Backend::new(
        vec![backend1_addr.clone(), backend2_addr.clone()],
        Protocol::HTTPS(TlsConfig::new(server_config.clone())),
        Duration::from_secs(30),
        1000
    );

    proxy.add_route("example.formation.cloud".to_string(), backend).await;
    let acceptor = guard.acceptor.clone(); 

    drop(guard);

    let listener = TcpListener::bind("0.0.0.0:443").await?;
    println!("TLS proxy listening on :443");
    println!("Configured for domains:");
    println!("  - https://example.formation.cloud/");

    let (tx, mut rx) = tokio::sync::mpsc::channel(1024);
    let inner_manager = tls_manager.clone();

    let handle = tokio::spawn(async move {
        loop {
            tokio::select! {
                result = listener.accept() => {
                    match result {
                        Ok((stream, client_addr)) => {
                            match acceptor.accept(stream).await {
                                Ok(Some(handshake)) => {
                                    println!("Request accepted");
                                    let domain = if let Some(d) = handshake.client_hello().server_name() {
                                        d.to_string()
                                    } else {
                                        String::new()
                                    }; 
                                    println!("For domain: {}", domain.clone());
                                    match handshake.into_stream(server_config.clone()).await {
                                        Ok(mut tls_stream) => {
                                            let inner_proxy = proxy.clone();
                                            tokio::spawn(async move {
                                                if let Err(e) = inner_proxy.handle_tls_connection(tls_stream, &domain).await {
                                                    eprintln!("Error handling TLS connection from {}: {e}", client_addr.to_string());
                                                }
                                            });
                                        }
                                        Err(e) => eprintln!("TLS handshake error from {}: {e}", client_addr.to_string())
                                    }
                                }
                                Ok(None) => {
                                    // ACME validation request handled
                                }
                                Err(e) => eprintln!("Error accepting TLS connection from {}: {e}", client_addr.to_string())
                            }
                        }
                        Err(e) => eprintln!("Error accepting TCP connection: {e}")
                    }
                }
                domain_opt = rx.recv() => {
                    if let Some(domain) = domain_opt {
                        if let Ok(mut guard) = inner_manager.lock() {
                            if let Err(e) = guard.add_domain(domain, true) {
                                eprintln!("Error adding domain to tls manager: {e}");
                            }
                        }
                    }
                }
            }
        }
    });

    tokio::time::sleep(std::time::Duration::from_secs(60)).await;
    tx.send("dev.formation.cloud".to_string()).await?;
    tokio::signal::ctrl_c().await?;
    handle.abort();

    Ok(())
}
