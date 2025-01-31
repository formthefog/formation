use crate::{backend::Backend, config::ProxyConfig, error::ProxyError, protocol::Protocol};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt}, net::TcpStream, sync::Mutex
};
use tokio_rustls::{rustls::{self, ServerConfig}, TlsAcceptor};
use tokio_rustls_acme::{caches::DirCache, tokio_rustls::server::TlsStream, AcmeConfig, Incoming};
use tokio_stream::wrappers::TcpListenerStream;
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tokio::sync::RwLock;
use futures::future::try_join_all;
use futures::StreamExt;
use rand::seq::SliceRandom;

pub type StandardIncoming = Incoming<TcpStream, std::io::Error, TcpListenerStream, std::io::Error, std::io::Error>;

pub struct TlsStreamManager {
    pub incoming: StandardIncoming,
}

impl TlsStreamManager {
    pub fn refresh(&mut self) {
        todo!()
    }
}

#[derive(Clone, Debug)]
pub struct ReverseProxy {
    routes: Arc<RwLock<HashMap<String, Backend>>>,
    config: ProxyConfig,
}

impl ReverseProxy {
    pub fn new(config: ProxyConfig) -> Self {
        Self {
            routes: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    pub fn config(&self) -> ProxyConfig {
        self.config.clone()
    }

    pub async fn add_route(&self, domain: String, backend: Backend) {
        let mut routes = self.routes.write().await;
        routes.insert(domain, backend);
    }

    pub async fn remove_route(&self, domain: &str) -> Option<Backend> {
        let mut routes = self.routes.write().await;
        routes.remove(domain)
    }

    pub async fn get_route(&self, domain: &str) -> Option<Backend> {
        let routes = self.routes.read().await;
        routes.get(domain).cloned()
    }

    pub async fn select_backend(&self, domain: &str) -> Result<SocketAddr, ProxyError> {
        let routes = self.routes.read().await;
        let backend = routes.get(domain)
            .ok_or_else(|| ProxyError::NoBackend(domain.to_string()))?;
            
        backend.addresses()
            .choose(&mut rand::thread_rng())
            .copied()
            .ok_or_else(|| ProxyError::NoBackend(domain.to_string()))
    }

    async fn get_backend(&self, domain: &str) -> Result<Backend, ProxyError> {
        let routes = self.routes.read().await;
        if let Some(backend) = routes.get(domain) {
            Ok(backend.clone())
        } else {
            Err(ProxyError::NoBackend(domain.to_string()))
        }
    }

    pub async fn handle_connection(
        &self,
        mut client_stream: TcpStream,
    ) -> Result<(), ProxyError> {
        println!("Peeking into request");
        let mut peek_buffer = vec![0; self.config.buffer_size];
        let n = client_stream.peek(&mut peek_buffer).await?;
        let is_tls = n >= 5 && peek_buffer[0] == 0x16 && peek_buffer[1] == 0x03;

        if is_tls {
            println!("Request is TLS");
            // 1. Extract the SNI to know which cert to user
            // 2. Perform TLS handshake
            // 3. Process the HTTP request inside the TLS tunnel
            println!("Extracting SNI");
            let domain = extract_sni(&peek_buffer[..n])?;
            println!("Domain for request: {domain}");

            let backend = self.get_backend(&domain).await?;
            println!("Backends for request: {backend:?}");

            match &backend.protocol() {
                Protocol::HTTPS(tls_config) => {
                    println!("Building server config");
                    // Set up TLS connection with client
                    let server_config = Arc::new(tls_config.get_config().clone());

                    // Create TLS stream and perform handshake
                    let mut tls_stream = establish_tls_connection(client_stream, server_config.clone()).await?;
                    println!("Creating TLS stream to perform handshake");

                    let mut buffer = vec![0; self.config.buffer_size];
                    let n = tls_stream.read(&mut buffer).await?;

                    let backend_addr = self.select_backend(&domain).await?;
                    let mut backend_stream = tokio::time::timeout(
                        self.config.connection_timeout,
                        TcpStream::connect(backend_addr)
                    ).await.map_err(|e| ProxyError::InvalidRequest(e.to_string()))??;

                    backend_stream.write_all(&buffer[..n]).await?;

                    let backend_tls_stream = establish_tls_connection(backend_stream, server_config.clone()).await?;

                    println!("Handshake complete creating proxy stream");
                    let (mut client_read, mut client_write) = tokio::io::split(tls_stream);
                    let (mut backend_read, mut backend_write) = tokio::io::split(backend_tls_stream);

                    let client_to_backend = tokio::io::copy(&mut client_read, &mut backend_write);
                    let backend_to_client = tokio::io::copy(&mut backend_read, &mut client_write);

                    try_join_all(vec![client_to_backend, backend_to_client]).await?;

                }
                _ => {
                    return Err(ProxyError::InvalidRequest(format!("Expected TLS protocol, did not find it")));
                }
            }
        } else {
            let mut buffer = vec![0; self.config.buffer_size];
            let n = client_stream.read(&mut buffer).await?;

            let request = String::from_utf8_lossy(&buffer[..n]);
            let domain = self.extract_domain(&request)?;

            let backend_addr = self.select_backend(&domain).await?;
            let mut backend_stream = tokio::time::timeout(
                self.config.connection_timeout,
                TcpStream::connect(backend_addr)
            ).await.map_err(|e| ProxyError::InvalidRequest(e.to_string()))??;

            backend_stream.write_all(&buffer[..n]).await.map_err(|e| {
                ProxyError::Io(e)
            })?;

            let (mut client_read, mut client_write) = client_stream.split();
            let (mut backend_read, mut backend_write) = backend_stream.split();

            let client_to_backend = tokio::io::copy(&mut client_read, &mut backend_write);
            let backend_to_client = tokio::io::copy(&mut backend_read, &mut client_write);

            try_join_all(vec![client_to_backend, backend_to_client]).await?;
        };

        Ok(())
    }

    pub fn extract_domain(&self, request: &str) -> Result<String, ProxyError> {
        let host_line = request.lines()
            .find(|line| line.starts_with("Host: "))
            .ok_or_else(|| ProxyError::InvalidRequest("No Host header found".to_string()))?;
        
        Ok(host_line[6..].trim().to_string())
    }
}

async fn handle_tls_connection(mut stream: TlsStream<TcpStream>, proxy: Arc<Mutex<ReverseProxy>>) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

async fn build_tls_config(tcp_incoming: TcpListenerStream) -> StandardIncoming {
    AcmeConfig::new(["example.formation.cloud"])
        .directory_lets_encrypt(true)
        .contact_push("mailto:admin@formation.cloud")
        .cache(DirCache::new("./rustls_acme_cache"))
        .incoming(tcp_incoming, vec![b"http/1.1".to_vec()])
}

async fn serve_tls(
    mut shutdown: tokio::sync::broadcast::Receiver<()>,
    tls_stream: Arc<Mutex<TlsStreamManager>>, 
    proxy: Arc<Mutex<ReverseProxy>>
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        if let Ok(_) = shutdown.recv().await {
            break;
        }

        while let Some(tls_conn) = tls_stream.lock().await.incoming.next().await {
            match tls_conn {
                Ok(tls_stream) => {
                    let proxy_clone = proxy.clone(); 
                    if let Err(e) = handle_tls_connection(tls_stream, proxy_clone).await {
                        todo!()
                    }
                }
                Err(_) => {}
            }
        }

        tls_stream.lock().await.refresh();
    }
    Ok(())
}

/// Creates a TLS stream from a TCP connection using the provided server configuration.
/// This function handles the TLS handshake process and returns a properly configured
/// server-side TLS stream.
async fn establish_tls_connection(
    tcp_stream: TcpStream,
    server_config: Arc<ServerConfig>,
) -> Result<TlsStream<TcpStream>, ProxyError> {
    todo!()
}

/// Extracts the Server Name Indication (SNI) from a TLS ClientHello message.
/// 
/// The TLS ClientHello message structure is defined in RFC 5246 (TLS 1.2) and RFC 8446 (TLS 1.3).
/// The SNI extension is defined in RFC 6066 Section 3.
/// 
/// Structure of a TLS ClientHello (simplified):
/// Byte   0       - Record Type (0x16 for Handshake)
/// Bytes  1-2     - TLS Version
/// Bytes  3-4     - Record Length
/// Byte   5       - Handshake Type (0x01 for ClientHello)
/// Bytes  6-8     - Handshake Length
/// Bytes  9-10    - Protocol Version
/// Bytes  11-42   - Random (32 bytes)
/// Byte   43      - Session ID Length
/// Variable       - Session ID
/// 2 bytes        - Cipher Suites Length
/// Variable       - Cipher Suites
/// 1 byte         - Compression Methods Length
/// Variable       - Compression Methods
/// 2 bytes        - Extensions Length
/// Variable       - Extensions
pub fn extract_sni(client_hello: &[u8]) -> Result<String, rustls::Error> {
    // First, verify we have enough data for the basic TLS header
    if client_hello.len() < 5 {
        return Err(rustls::Error::General("ClientHello too short".into()));
    }

    // Validate this is a TLS handshake
    if client_hello[0] != 0x16 {  // Record Type
        return Err(rustls::Error::General("Not a TLS handshake".into()));
    }

    // Skip record header (5 bytes) and validate handshake type
    if client_hello.len() < 6 || client_hello[5] != 0x01 {  // ClientHello type
        return Err(rustls::Error::General("Not a ClientHello".into()));
    }

    // Start after the fixed portion (random + protocol version)
    let mut pos = 43;  // 5 (record) + 4 (handshake) + 2 (version) + 32 (random)
    
    if pos >= client_hello.len() {
        return Err(rustls::Error::General("Message too short for session ID".into()));
    }

    // Skip session ID
    let session_id_len = client_hello[pos] as usize;
    pos += 1 + session_id_len;

    if pos + 2 > client_hello.len() {
        return Err(rustls::Error::General("Message too short for cipher suites".into()));
    }

    // Skip cipher suites
    let cipher_suites_len = ((client_hello[pos] as usize) << 8) | (client_hello[pos + 1] as usize);
    pos += 2 + cipher_suites_len;

    if pos + 1 > client_hello.len() {
        return Err(rustls::Error::General("Message too short for compression methods".into()));
    }

    // Skip compression methods
    let compression_methods_len = client_hello[pos] as usize;
    pos += 1 + compression_methods_len;

    if pos + 2 > client_hello.len() {
        return Err(rustls::Error::General("Message too short for extensions".into()));
    }

    // Process extensions
    let extensions_len = ((client_hello[pos] as usize) << 8) | (client_hello[pos + 1] as usize);
    pos += 2;
    let extensions_end = pos + extensions_len;

    if extensions_end > client_hello.len() {
        return Err(rustls::Error::General("Message too short for extensions data".into()));
    }

    // Search for the SNI extension (type 0)
    while pos + 4 <= extensions_end {
        let extension_type = ((client_hello[pos] as u16) << 8) | (client_hello[pos + 1] as u16);
        let extension_len = ((client_hello[pos + 2] as usize) << 8) | (client_hello[pos + 3] as usize);
        pos += 4;

        if extension_type == 0 {  // SNI extension type
            if pos + 2 > extensions_end {
                return Err(rustls::Error::General("SNI extension truncated".into()));
            }

            // Parse SNI extension
            let sni_list_len = ((client_hello[pos] as usize) << 8) | (client_hello[pos + 1] as usize);
            pos += 2;

            if pos + sni_list_len > extensions_end {
                return Err(rustls::Error::General("SNI extension data truncated".into()));
            }

            let mut sni_pos = pos;
            while sni_pos + 3 <= pos + sni_list_len {
                let name_type = client_hello[sni_pos];
                let name_len = ((client_hello[sni_pos + 1] as usize) << 8) | 
                              (client_hello[sni_pos + 2] as usize);
                sni_pos += 3;

                if sni_pos + name_len > pos + sni_list_len {
                    return Err(rustls::Error::General("SNI hostname truncated".into()));
                }

                // name_type 0 is hostname
                if name_type == 0 {
                    return String::from_utf8(client_hello[sni_pos..sni_pos + name_len].to_vec())
                        .map_err(|_| rustls::Error::General("Invalid UTF-8 in SNI hostname".into()));
                }

                sni_pos += name_len;
            }
        }

        pos += extension_len;
    }

    Err(rustls::Error::General("No SNI extension found".into()))
}
