use std::net::SocketAddr;

use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::TcpListener};

// Mock HTTP server implementation
async fn run_mock_server(addr: SocketAddr, server_name: &'static str) {
    let listener = TcpListener::bind(addr).await.unwrap();
    log::info!("Mock server {} listening on {}", server_name, addr);

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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    simple_logger::SimpleLogger::new().init().unwrap();
    tokio::spawn(run_mock_server("127.0.0.1:8081".parse()?, "BACKEND_ONE"));
    tokio::spawn(run_mock_server("127.0.0.1:8082".parse()?, "BACKEND_TWO"));

    tokio::signal::ctrl_c().await?;
    Ok(())
}
