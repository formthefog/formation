use tokio::net::TcpStream;
use std::{net::SocketAddr, time::Duration};

use crate::protocol::Protocol;

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct Backend {
    addresses: Vec<SocketAddr>,
    protocol: Protocol,
    health_check_interval: Duration,
    max_connections: usize
}

impl Backend {
    pub fn new(
        addresses: Vec<SocketAddr>,
        protocol: Protocol,
        health_check_interval: Duration,
        max_connections: usize,
    ) -> Self {
        Self {
            addresses,
            protocol,
            health_check_interval,
            max_connections,
        }
    }

    pub async fn health_check(&self) -> Vec<bool> {
        let mut results = Vec::with_capacity(self.addresses.len());
        
        for addr in &self.addresses {
            let is_healthy = match TcpStream::connect(addr).await {
                Ok(_) => true,
                Err(_) => false,
            };
            results.push(is_healthy);
        }
        
        results
    }

    pub fn addresses(&self) -> Vec<SocketAddr> {
        self.addresses.clone()
    }

    pub fn protocol(&self) -> Protocol {
        self.protocol.clone()
    }
}
