use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::TcpStream, sync::{Mutex, mpsc::Receiver}};
use form_rplb::{backend::Backend, config::ProxyConfig, error::ProxyError, protocol::{Protocol, TlsConfig}, proxy::{DomainProtocols, ReverseProxy}, resolver::TlsManager};
use tokio::net::TcpListener;

use crate::store::{FormDnsRecord, SharedStore};

pub struct IntegratedProxy {
    pub store: SharedStore,
    pub reverse_proxy: Arc<ReverseProxy>,
    pub tls_manager: Arc<Mutex<TlsManager>>,
    domain_protocols: Arc<Mutex<HashMap<String, DomainProtocols>>>,
    http_listener: Option<TcpListener>,
    tls_listener: Option<TcpListener>,
}

impl Clone for IntegratedProxy {
    fn clone(&self) -> Self {
        Self {
            store: self.store.clone(),
            reverse_proxy: self.reverse_proxy.clone(),
            tls_manager: self.tls_manager.clone(),
            domain_protocols: self.domain_protocols.clone(),
            http_listener: None,
            tls_listener: None,
        }
    } 
}

impl IntegratedProxy {
    pub async fn new(
        store: SharedStore,
        tls_manager: TlsManager,
        config: ProxyConfig,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let reverse_proxy = Arc::new(ReverseProxy::new(config));
        Ok(Self {
            store,
            reverse_proxy,
            tls_manager: Arc::new(Mutex::new(tls_manager)),
            domain_protocols: Arc::new(Mutex::new(HashMap::new())),
            http_listener: None,
            tls_listener: None,
        })
    }

    pub async fn bind(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.http_listener = Some(TcpListener::bind("0.0.0.0:80").await?);
        self.tls_listener = Some(TcpListener::bind("0.0.0.0:443").await?);
        Ok(())
    }

    pub async fn configure_domain_protocols(
        &self,
        domain: &str,
        enable_http: bool,
        enable_tls: bool,
        force_tls: bool
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut protocols = self.domain_protocols.lock().await;
        protocols.insert(domain.to_string(), DomainProtocols {
            http_enabled: enable_http,
            tls_enabled: enable_tls,
            force_tls,
            ..Default::default()
        });

        Ok(())
    }

    async fn create_backends_for_domain(
        &self,
        domain: &str,
        record: &FormDnsRecord
    ) -> Vec<(Protocol, Backend)> {
        let mut backends = Vec::new();
        let protocols = self.domain_protocols.lock().await;
        let domain_config = protocols.get(domain).cloned().unwrap_or_default();

        if domain_config.http_enabled {
            let mut addresses: Vec<SocketAddr> = record.formnet_ip.iter()
                .map(|ip| *ip)
                .collect();
            addresses.extend(record.public_ip.clone());
            backends.push((
                Protocol::HTTP,
                Backend::new(
                    addresses.clone(),
                    Protocol::HTTP,
                    std::time::Duration::from_secs(30),
                    1000
                )
            ));
        }

        if domain_config.tls_enabled {
            let tls_config = TlsConfig::new(self.tls_manager.lock().await.config.clone());
            let mut addresses: Vec<SocketAddr> = record.formnet_ip.iter()
                .map(|ip| *ip)
                .collect();
            addresses.extend(record.public_ip.clone());
            backends.push((
                Protocol::HTTPS(tls_config.clone()),
                Backend::new(
                    addresses.clone(),
                    Protocol::HTTPS(tls_config),
                    std::time::Duration::from_secs(30),
                    1000
                )
            ))
        }

        backends
    }

    pub async fn add_routes(&self, domain: &str) -> Result<(), Box<dyn std::error::Error>> {
        let dns_guard = self.store.read().await;
        if let Some(record) = dns_guard.get(domain) {
            let mut should_enable_tls = domain.ends_with(".fog") || self.tls_manager.lock().await.domains.contains_key(domain);
            if !should_enable_tls {
                if record.ssl_cert {
                    let _ = self.tls_manager.lock().await.add_domain(domain.to_string(), false).await;
                }
                should_enable_tls = true;
            }
            if !self.domain_protocols.lock().await.contains_key(domain) {
                self.configure_domain_protocols(
                    domain, 
                    true, 
                    should_enable_tls, 
                    should_enable_tls
                ).await?;
            }

            let backends = self.create_backends_for_domain(domain, &record).await;
            for (_protocol, backend) in backends {
                self.reverse_proxy.add_route(domain.to_string(), backend).await;
            }
        }

        Ok(())
    }

    pub async fn handle_http(&self, mut stream: TcpStream) -> Result<(), ProxyError> {
        let buffer_size = self.reverse_proxy.config().buffer_size;
        let mut buffer = vec![0; buffer_size];

        let n = stream.read(&mut buffer).await?;
        let request = String::from_utf8_lossy(&buffer[..n]);
        let domain = self.reverse_proxy.extract_domain(&request)?;

        let protocols = self.domain_protocols.lock().await;
        if let Some(config) = protocols.get(&domain) {
            if config.force_tls {
                let response = format!(
                    "HTTP/1.1 301 Moved Permanently\r\n
                    Location: https://{}{}\r\n\
                    Connection: close\r\n\
                    \r\n",
                    domain,
                    request.lines().next().and_then(|l| l.split_whitespace().nth(1)).unwrap_or("/")
                );

                stream.write_all(response.as_bytes()).await?;
                return Ok(())
            }
        }

        self.reverse_proxy.handle_http_connection(stream, &domain, request.to_string()).await
    }

    pub async fn handle_https(&self, stream: TcpStream) -> Result<(), ProxyError> {
        let tls_manager = self.tls_manager.lock().await;
        let acceptor = tls_manager.acceptor.clone();
        let server_config = tls_manager.config.clone();

        match acceptor.accept(stream).await? {
            Some(handshake) => {
                let domain = handshake
                    .client_hello()
                    .server_name()
                    .ok_or_else(|| ProxyError::InvalidRequest("No SNI header".into()))?
                    .to_string();

                let protocols = self.domain_protocols.lock().await;
                if let Some(config) = protocols.get(&domain) {
                    if !config.tls_enabled {
                        return Err(
                            ProxyError::InvalidRequest(
                                format!("HTTPS not enabled for {domain}")
                            )
                        )
                    }
                }
                let tls_stream = handshake.into_stream(server_config.clone()).await?;
                self.reverse_proxy.handle_tls_connection(tls_stream, &domain, server_config.clone()).await
            }
            None => Ok(())
        }
    }

    pub async fn run(&self, mut rx: Receiver<FormDnsRecord>) -> Result<(), Box<dyn std::error::Error>> {
        let http_listener = self.http_listener.as_ref().ok_or("HTTP Listener not initialized")?;
        let tls_listener = self.tls_listener.as_ref().ok_or("TLS Listener not initialized")?;

        loop {
            tokio::select! {
                Ok((stream, _)) = http_listener.accept() => {
                    let proxy = Arc::new(self.clone());
                    tokio::spawn(async move {
                        if let Err(e) = proxy.handle_http(stream).await {
                            eprintln!("Error handling HTTP connection: {e}");
                        }
                    });
                }
                Ok((stream, _)) = tls_listener.accept() => {
                    let proxy = Arc::new(self.clone());
                    tokio::spawn(async move {
                        if let Err(e) = proxy.handle_https(stream).await {
                            eprintln!("Error handling HTTPS connection: {e}");
                        }
                    });
                }
                Some(record) = rx.recv() => {
                    let domain = record.domain.clone();
                    if let Err(e) = self.add_routes(&domain).await {
                        eprintln!("Error trying to add new route to proxy for domain {domain}: {e}");
                    }
                }
            }
        }
    }
}
