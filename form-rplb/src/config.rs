use std::time::Duration;
use tokio_rustls::rustls::ClientConfig;

#[derive(Clone, Debug)]
pub struct ProxyConfig {
    pub client_tls_config: Option<ClientConfig>,
    pub connection_timeout: Duration,
    pub buffer_size: usize,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            client_tls_config: None,
            connection_timeout: Duration::from_secs(30),
            buffer_size: 8192,
        }
    }
}
