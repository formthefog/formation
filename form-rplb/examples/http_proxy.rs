use form_rplb::{proxy::ReverseProxy, backend::Backend, protocol::Protocol, config::ProxyConfig};
use std::{net::SocketAddr, time::Duration};
use tokio::{
    net::{TcpListener, TcpStream},
    io::{AsyncReadExt, AsyncWriteExt},
};

// Mock HTTP server implementation
async fn run_mock_server(addr: SocketAddr, server_name: &'static str) {
    let listener = TcpListener::bind(addr).await.unwrap();
    println!("Mock server {} listening on {}", server_name, addr);

    loop {
        let (mut socket, _) = listener.accept().await.unwrap();
        let server_id = server_name.to_string();
        
        tokio::spawn(async move {
            let mut buffer = vec![0; 1024];
            let n = socket.read(&mut buffer).await.unwrap();
            println!("Server {} received request:\n{}", server_id, String::from_utf8_lossy(&buffer[..n]));

            let body = format!(
                "<!DOCTYPE html>\
                <html>\
                  <head>\
                    <title>Response from {}</title>\
                  </head>
                  <body>\
                    <h1>Hello from {}</h1>\
                    <p>This response came from backend server {}</p>\
                    <p>Try refreshing to see load balancing in action!</p>\
                  </body>\
                </html>",
                server_id, server_id, server_id
            );
            
            // Simple HTTP response with server identifier
            let response = format!(
                "HTTP/1.1 200 OK\r\n\
                Content-Type: text/html; charset=utf-8\r\n\
                Connection: close\r\n\
                Content-Length: {}\r\n\
                \r\n\
                {}",
                body.len(),
                body
            );
            
            socket.write_all(response.as_bytes()).await.unwrap();
        });
    }
}

async fn make_http_request(addr: SocketAddr, host: &str, path: &str) -> String {
    let mut stream = TcpStream::connect(addr).await.unwrap();
    
    let request = format!(
        "GET {} HTTP/1.1\r\n\
        Host: {}\r\n\
        Connection: close\r\n\
        \r\n",
        path, host
    );
    
    stream.write_all(request.as_bytes()).await.unwrap();
    
    let mut response = String::new();
    stream.read_to_string(&mut response).await.unwrap();
    response
}

#[tokio::main]
async fn main() {
    // Start mock backend servers
    let backend1_addr: SocketAddr = "127.0.0.1:8081".parse().unwrap();
    let backend2_addr: SocketAddr = "127.0.0.1:8082".parse().unwrap();
    
    tokio::spawn(run_mock_server(backend1_addr, "BACKEND_ONE"));
    tokio::spawn(run_mock_server(backend2_addr, "BACKEND_TWO"));
    
    // Configure and start the reverse proxy
    let config = ProxyConfig::default();
    let proxy = ReverseProxy::new(config);
    
    // Configure backends
    let example_backend = Backend::new(
        vec![backend1_addr, backend2_addr],
        Protocol::HTTP,
        Duration::from_secs(30),
        1000
    );
    
    let api_backend = Backend::new(
        vec![backend2_addr],
        Protocol::HTTP,
        Duration::from_secs(30),
        1000
    );
    
    proxy.add_route("example.internal".to_string(), example_backend).await;
    proxy.add_route("api.example.internal".to_string(), api_backend).await;
    
    // Start proxy server
    let proxy_addr: SocketAddr = "127.0.0.1:80".parse().unwrap();
    let listener = TcpListener::bind(proxy_addr).await.unwrap();
    println!("Proxy server listening on http://{}", proxy_addr);
    println!("You can now access:");
    println!("  - http://example.local/     (load balanced between both backends)");
    println!("  - http://api.example.local/ (always goes to backend 2)");
    println!("Proxy server listening on {}", proxy_addr);
    
    // Main proxy loop
    loop {
        let (client_stream, _client_addr) = listener.accept().await.unwrap();
        let proxy = proxy.clone();
        
        tokio::spawn(async move {
            if let Err(e) = proxy.handle_http_connection(client_stream).await {
                eprintln!("Error handling connection: {}", e);
            }
        });
    }
}
