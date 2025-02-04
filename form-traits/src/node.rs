use std::net::SocketAddr;
use std::hash::Hash;
use std::collections::HashSet;
use alloy_core::primitives::Address;
use k256::ecdsa::{SigningKey, Signature, RecoveryId};
use crate::{Topic, Event};

pub trait ServiceType {}

pub trait SystemInfo {
    type ServiceType: ServiceType + Clone;

    fn cpu_cores(&self) -> u32;
    fn cpu_arch(&self) -> String;
    fn cpu_frequency(&self) -> Option<String>;
    fn total_memory(&self) -> f64;
    fn available_memory(&self) -> u64;
    fn total_storage(&self) -> u64;
    fn available_storage(&self) -> u64;
}

pub trait NodeInfo: Hash {
    type SystemInfo: SystemInfo + Clone;
    type ServiceType: ServiceType + Clone;

    fn id(&self) -> Address;
    fn ip_address(&self) -> SocketAddr;
    fn system_info(&self) -> Self::SystemInfo;
    fn supported_services(&self) -> Vec<Self::ServiceType>; 
    fn version(&self) -> String;
    fn uptime(&self) -> u64;
    fn last_seen(&self) -> u64;

    fn cpu_cores(&self) -> u32 {
        self.system_info().cpu_cores()
    }

    fn cpu_arch(&self) -> String {
        self.system_info().cpu_arch().clone()
    }

    fn cpu_frequency(&self) -> Option<String> {
        self.system_info().cpu_frequency().clone()
    }

    fn total_memory(&self) -> f64 {
        self.system_info().total_memory()
    }

    fn available_memory(&self) -> u64 {
        self.system_info().available_memory()
    }

    fn total_storage(&self) -> u64 {
        self.system_info().total_storage()
    }

    fn available_storage(&self) -> u64 {
        self.system_info().available_storage()
    }
}

#[async_trait::async_trait]
pub trait Node {
    type Info: NodeInfo;
    type Error: std::error::Error;

    fn info(&self) -> Self::Info;
    fn peers(&self) -> HashSet<Self::Info>;
    fn signing_key(&self) -> SigningKey; 
    async fn publish(&self, topic: Box<dyn Topic>, message: Box<dyn Event + Send + 'static>) -> Result<(), Self::Error>; 
    fn sign_heartbeat(&self, peer: &Self::Info, timestamp: &i64) -> Result<(Signature, RecoveryId), Self::Error>; 
    fn sign_heartbeat_response(&self, payload: Vec<u8>) -> Result<(Signature, RecoveryId), Self::Error>;
    fn sign_join(&self, peer: &Self::Info, timestamp: &i64) -> Result<(Signature, RecoveryId), Self::Error>;
    fn sign_join_response(&self, payload: Vec<u8>) -> Result<(Signature, RecoveryId), Self::Error>;
    fn sign_quorum_gossip_request(&self) -> Result<(Signature, RecoveryId), Self::Error>;
    fn sign_quorum_gossip_response(&self, payload: Vec<u8>) -> Result<(Signature, RecoveryId), Self::Error>;
    fn sign_network_gossip_request(&self) -> Result<(Signature, RecoveryId), Self::Error>;
    fn sign_network_gossip_response(&self, payload: Vec<u8>) -> Result<(Signature, RecoveryId), Self::Error>;
    fn sign_direct_message_request(&self) -> Result<(Signature, RecoveryId), Self::Error>;
    fn sign_direct_message_response(&self, payload: Vec<u8>) -> Result<(Signature, RecoveryId), Self::Error>;
    fn sign_user_response(&self, payload: Vec<u8>) -> Result<(Signature, RecoveryId), Self::Error>;
    
    fn id(&self) -> Address {
        self.info().id().clone()
    }

    fn ip_address(&self) -> SocketAddr {
        self.info().ip_address().clone()
    }

    fn system_info<S: SystemInfo>(&self) -> <<Self as Node>::Info as NodeInfo>::SystemInfo {
        self.info().system_info().clone()
    }

    fn supported_services<S: ServiceType>(&self) -> Vec<<<Self as Node>::Info as NodeInfo>::ServiceType> {
        self.info().supported_services().clone()
    }

    fn version(&self) -> String {
        self.info().version().clone()
    }

    fn uptime(&self) -> u64 {
        self.info().uptime()
    }

    fn last_seen(&self) -> u64 {
        self.info().last_seen()
    }

    fn cpu_cores(&self) -> u32 {
        self.info().cpu_cores()
    }

    fn cpu_arch(&self) -> String {
        self.info().cpu_arch().clone()
    }

    fn cpu_frequency(&self) -> Option<String> {
        self.info().cpu_frequency().clone()
    }

    fn total_memory(&self) -> f64 {
        self.info().total_memory()
    }

    fn available_memory(&self) -> u64 {
        self.info().available_memory()
    }

    fn total_storage(&self) -> u64 {
        self.info().total_storage()
    }

    fn available_storage(&self) -> u64 {
        self.info().available_storage()
    }
}
