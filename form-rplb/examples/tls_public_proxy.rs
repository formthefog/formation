use tokio_rustls_acme::tokio_rustls::rustls::ServerConfig;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio_rustls_acme::caches::DirCache;
use tokio_rustls_acme::{AcmeAcceptor, AcmeConfig};
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() {
    simple_logger::init_with_level(log::Level::Info).unwrap();
    let mut state = AcmeConfig::new(vec!["example.formation.cloud"])
        .contact_push(format!("mailto:{}", "admin@formation.cloud"))
        .cache_option(Some(DirCache::new("./rustls_cache")))
        .directory_lets_encrypt(true)
        .state();
    let rustls_config = ServerConfig::builder()
        .with_no_client_auth()
        .with_cert_resolver(state.resolver());
    let acceptor = state.acceptor();

    tokio::spawn(async move {
        loop {
            match state.next().await.unwrap() {
                Ok(ok) => log::info!("event: {:?}", ok),
                Err(err) => log::error!("error: {:?}", err),
            }
        }
    });

    serve(acceptor, Arc::new(rustls_config)).await;
}

async fn serve(acceptor: AcmeAcceptor, rustls_config: Arc<ServerConfig>) {
    let listener = tokio::net::TcpListener::bind("0.0.0.0:443")
        .await
        .unwrap();
    let body = format!(
        "<!DOCTYPE html><html>\
            <head>\
                <title>Response from Server</title>\
                <link rel=\"icon\" href=\"data:,\">\
            </head>\
            <body>\
                <h1>Hello from Server</h1>\
                <p>This response came from backend server Server</p>\
                <p>Try refreshing to see load balancing in action!</p>\
            </body>\
        </html>",
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
    ).as_bytes().to_vec();

    loop {
        let inner_response = response.clone();
        let tcp = listener.accept().await.unwrap().0;
        let rustls_config = rustls_config.clone();
        let accept_future = acceptor.accept(tcp);

        tokio::spawn(async move {
            match accept_future.await.unwrap() {
                None => log::info!("received TLS-ALPN-01 validation request"),
                Some(start_handshake) => {
                    let mut tls = start_handshake.into_stream(rustls_config).await.unwrap();
                    tls.write_all(&inner_response).await.unwrap();
                    tls.shutdown().await.unwrap();
                }
            }
        });
    }
}
