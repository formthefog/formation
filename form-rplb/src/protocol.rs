use std::sync::Arc;
use tokio_rustls_acme::tokio_rustls::rustls::ServerConfig;

#[derive(Debug, Clone)]
pub struct TlsConfig(Arc<ServerConfig>);

impl TlsConfig {
    pub fn new(config: Arc<ServerConfig>) -> Self {
        Self(config)
    }

    pub fn get_config(&self) -> &ServerConfig {
        &self.0
    }
}

impl PartialEq for TlsConfig {
    fn eq(&self, _other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &_other.0)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Protocol {
    HTTP,
    HTTPS(TlsConfig),
    TCP,
    UDP,
}
