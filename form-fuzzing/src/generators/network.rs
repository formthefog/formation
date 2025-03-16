// form-fuzzing/src/generators/network.rs
//! Network component generators for fuzzing network functionality

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::Duration;
use rand::{Rng, prelude::SliceRandom};
use crate::generators::Generator;

/// Network protocols supported for fuzzing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    UDP,
    TCP,
    QUIC,
    SCTP,
    DCCP,
    RUDP, // Reliable UDP
}

impl Protocol {
    /// Get all available protocols
    pub fn all() -> Vec<Protocol> {
        vec![
            Protocol::UDP,
            Protocol::TCP,
            Protocol::QUIC,
            Protocol::SCTP,
            Protocol::DCCP,
            Protocol::RUDP,
        ]
    }
    
    /// Get a random protocol
    pub fn random() -> Self {
        let mut rng = rand::thread_rng();
        *Self::all().choose(&mut rng).unwrap()
    }
}

/// NAT types for network simulation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NATType {
    None,       // No NAT (public IP)
    FullCone,   // Maps all requests from internal IP:port to same external IP:port
    RestrictedCone, // Like full cone, but restricts incoming traffic to specific external IPs
    PortRestricted, // Like restricted cone, but restricts to specific external IP:port
    Symmetric,   // Maps each internal IP:port to different external IP:port for each destination
    Hairpin,     // Supports hairpin translation (internal clients can connect via external IP)
    Double,      // Double NAT (two layers of NAT)
    Carrier,     // Carrier-grade NAT (large-scale NAT with unpredictable behavior)
}

impl NATType {
    /// Get all available NAT types
    pub fn all() -> Vec<NATType> {
        vec![
            NATType::None,
            NATType::FullCone,
            NATType::RestrictedCone,
            NATType::PortRestricted,
            NATType::Symmetric,
            NATType::Hairpin,
            NATType::Double,
            NATType::Carrier,
        ]
    }
    
    /// Get a random NAT type
    pub fn random() -> Self {
        let mut rng = rand::thread_rng();
        *Self::all().choose(&mut rng).unwrap()
    }
}

/// NAT mapping behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MappingBehavior {
    Consistent,    // Always maps to the same external port
    PortPreserving, // Tries to preserve internal port if possible
    Random,        // Maps to random external ports
}

impl MappingBehavior {
    /// Get all available mapping behaviors
    pub fn all() -> Vec<MappingBehavior> {
        vec![
            MappingBehavior::Consistent,
            MappingBehavior::PortPreserving,
            MappingBehavior::Random,
        ]
    }
    
    /// Get a random mapping behavior
    pub fn random() -> Self {
        let mut rng = rand::thread_rng();
        *Self::all().choose(&mut rng).unwrap()
    }
}

/// NAT filtering behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilteringBehavior {
    Endpoint,    // Filters based on specific IP:port combinations
    Address,     // Filters based on IP addresses only
    None,        // No filtering (allows all incoming traffic)
}

impl FilteringBehavior {
    /// Get all available filtering behaviors
    pub fn all() -> Vec<FilteringBehavior> {
        vec![
            FilteringBehavior::Endpoint,
            FilteringBehavior::Address,
            FilteringBehavior::None,
        ]
    }
    
    /// Get a random filtering behavior
    pub fn random() -> Self {
        let mut rng = rand::thread_rng();
        *Self::all().choose(&mut rng).unwrap()
    }
}

/// Network packet for fuzzing packet routing
#[derive(Debug, Clone)]
pub struct NetworkPacket {
    pub protocol: Protocol,
    pub source_ip: IpAddr,
    pub source_port: u16,
    pub destination_ip: IpAddr,
    pub destination_port: u16,
    pub length: usize,
    pub ttl: u8,
    pub fragmented: bool,
    pub payload: Vec<u8>,
    pub headers: HashMap<String, String>,
}

/// A generator for network packets
pub struct NetworkPacketGenerator;

impl NetworkPacketGenerator {
    /// Create a new NetworkPacketGenerator
    pub fn new() -> Self {
        Self
    }
    
    /// Generate a random IP address
    fn generate_ip(&self) -> IpAddr {
        let mut rng = rand::thread_rng();
        if rng.gen_bool(0.7) {
            // Generate IPv4 (more common)
            let a = rng.gen_range(1..=254);
            let b = rng.gen_range(0..=255);
            let c = rng.gen_range(0..=255);
            let d = rng.gen_range(1..=254);
            IpAddr::V4(Ipv4Addr::new(a, b, c, d))
        } else {
            // Generate IPv6
            let segments: [u16; 8] = std::array::from_fn(|_| rng.gen());
            IpAddr::V6(Ipv6Addr::new(
                segments[0], segments[1], segments[2], segments[3],
                segments[4], segments[5], segments[6], segments[7],
            ))
        }
    }
}

impl Generator<NetworkPacket> for NetworkPacketGenerator {
    fn generate(&self) -> NetworkPacket {
        let mut rng = rand::thread_rng();
        
        let protocol = Protocol::random();
        let source_ip = self.generate_ip();
        let source_port = rng.gen_range(1024..65535);
        let destination_ip = self.generate_ip();
        let destination_port = rng.gen_range(1024..65535);
        let length = rng.gen_range(20..1500); // Typical packet sizes
        let ttl = rng.gen_range(1..=255);
        let fragmented = rng.gen_bool(0.1); // 10% chance of fragmentation
        
        // Generate random payload
        let payload_length = rng.gen_range(0..length);
        let mut payload = vec![0u8; payload_length];
        rng.fill(&mut payload[..]);
        
        // Generate headers based on protocol
        let mut headers = HashMap::new();
        match protocol {
            Protocol::TCP => {
                headers.insert("seq".to_string(), rng.gen::<u32>().to_string());
                headers.insert("ack".to_string(), rng.gen::<u32>().to_string());
                headers.insert("window".to_string(), rng.gen::<u16>().to_string());
            },
            Protocol::UDP => {
                // UDP has minimal headers
            },
            Protocol::QUIC => {
                headers.insert("version".to_string(), format!("1.{}", rng.gen_range(0..5)));
                headers.insert("connection_id".to_string(), format!("{:x}", rng.gen::<u64>()));
            },
            Protocol::SCTP => {
                headers.insert("verification_tag".to_string(), rng.gen::<u32>().to_string());
                headers.insert("checksum".to_string(), rng.gen::<u32>().to_string());
            },
            Protocol::DCCP => {
                headers.insert("seq".to_string(), rng.gen::<u64>().to_string());
                headers.insert("service_code".to_string(), rng.gen::<u32>().to_string());
            },
            Protocol::RUDP => {
                headers.insert("seq".to_string(), rng.gen::<u32>().to_string());
                headers.insert("ack".to_string(), rng.gen::<u32>().to_string());
            },
        }
        
        NetworkPacket {
            protocol,
            source_ip,
            source_port,
            destination_ip,
            destination_port,
            length,
            ttl,
            fragmented,
            payload,
            headers,
        }
    }
}

/// NAT configuration for fuzzing NAT traversal
#[derive(Debug, Clone)]
pub struct NATConfig {
    pub nat_type: NATType,
    pub mapping_behavior: MappingBehavior,
    pub filtering_behavior: FilteringBehavior,
    pub port_range_start: u16,
    pub port_range_end: u16,
    pub mapping_timeout: Duration,
    pub internal_ip: IpAddr,
    pub external_ip: IpAddr,
    pub upstream_nat: Option<Box<NATConfig>>, // For double NAT scenarios
    pub mapping_refresh_enabled: bool,
    pub upnp_enabled: bool,
    pub pmp_enabled: bool,
    pub max_connections: u32,
}

/// A generator for NAT configurations
pub struct NATConfigGenerator;

impl NATConfigGenerator {
    /// Create a new NATConfigGenerator
    pub fn new() -> Self {
        Self
    }
}

impl Generator<NATConfig> for NATConfigGenerator {
    fn generate(&self) -> NATConfig {
        let mut rng = rand::thread_rng();
        
        let nat_type = NATType::random();
        let mapping_behavior = MappingBehavior::random();
        let filtering_behavior = FilteringBehavior::random();
        
        // Generate port range
        let port_range_start = rng.gen_range(1024..60000);
        let port_range_end = rng.gen_range(port_range_start + 100..65535);
        
        // Generate timeout (30 seconds to 1 hour)
        let mapping_timeout = Duration::from_secs(rng.gen_range(30..3600));
        
        // Generate IP addresses
        let packet_gen = NetworkPacketGenerator::new();
        let internal_ip = packet_gen.generate_ip();
        let external_ip = packet_gen.generate_ip();
        
        // For double NAT, recursively generate an upstream NAT
        let upstream_nat = if nat_type == NATType::Double && rng.gen_bool(0.8) {
            // 80% chance of actually having an upstream NAT for Double NAT type
            Some(Box::new(self.generate()))
        } else {
            None
        };
        
        // Other NAT features
        let mapping_refresh_enabled = rng.gen_bool(0.7); // 70% chance of being enabled
        let upnp_enabled = rng.gen_bool(0.5);
        let pmp_enabled = rng.gen_bool(0.3);
        let max_connections = rng.gen_range(100..10000);
        
        NATConfig {
            nat_type,
            mapping_behavior,
            filtering_behavior,
            port_range_start,
            port_range_end,
            mapping_timeout,
            internal_ip,
            external_ip,
            upstream_nat,
            mapping_refresh_enabled,
            upnp_enabled,
            pmp_enabled,
            max_connections,
        }
    }
}

/// P2P connection request for fuzzing peer-to-peer connections
#[derive(Debug, Clone)]
pub struct P2PConnectionRequest {
    pub local_id: String,
    pub peer_id: String,
    pub stun_servers: Vec<String>,
    pub turn_servers: Vec<String>,
    pub protocols: Vec<Protocol>,
    pub use_ice: bool,
    pub use_direct: bool,
    pub use_relay: bool,
    pub timeout: Duration,
    pub local_candidates: Vec<SocketAddr>,
    pub peer_candidates: Vec<SocketAddr>,
    pub connection_attempts: u32,
    pub keep_alive_interval: Duration,
}

impl Default for P2PConnectionRequest {
    fn default() -> Self {
        Self {
            local_id: String::new(),
            peer_id: String::new(),
            stun_servers: Vec::new(),
            turn_servers: Vec::new(),
            protocols: Vec::new(),
            use_ice: false,
            use_direct: false,
            use_relay: false,
            timeout: Duration::from_secs(30),
            local_candidates: Vec::new(),
            peer_candidates: Vec::new(),
            connection_attempts: 1,
            keep_alive_interval: Duration::from_secs(60),
        }
    }
}

/// A generator for P2P connection requests
pub struct P2PConnectionRequestGenerator;

impl P2PConnectionRequestGenerator {
    /// Create a new P2PConnectionRequestGenerator
    pub fn new() -> Self {
        Self
    }
    
    /// Generate a random peer ID
    fn generate_peer_id(&self) -> String {
        let mut rng = rand::thread_rng();
        let id_length = rng.gen_range(16..64);
        const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
        
        let id: String = (0..id_length)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect();
            
        id
    }
    
    /// Generate a STUN/TURN server
    fn generate_server(&self) -> String {
        let mut rng = rand::thread_rng();
        let packet_gen = NetworkPacketGenerator::new();
        let ip = packet_gen.generate_ip();
        let port = rng.gen_range(1024..65535);
        
        // Common STUN/TURN server ports
        let common_ports = [3478, 3479, 5349, 5350, 19302];
        let server_port = if rng.gen_bool(0.7) {
            *common_ports.choose(&mut rng).unwrap()
        } else {
            port
        };
        
        format!("{}:{}", ip, server_port)
    }
}

impl Generator<P2PConnectionRequest> for P2PConnectionRequestGenerator {
    fn generate(&self) -> P2PConnectionRequest {
        let mut rng = rand::thread_rng();
        
        let local_id = self.generate_peer_id();
        let peer_id = self.generate_peer_id();
        
        // Generate STUN/TURN servers
        let stun_count = rng.gen_range(1..5);
        let stun_servers = (0..stun_count)
            .map(|_| self.generate_server())
            .collect();
            
        let turn_count = rng.gen_range(0..3);
        let turn_servers = (0..turn_count)
            .map(|_| self.generate_server())
            .collect();
            
        // Select random protocols to try
        let all_protocols = Protocol::all();
        let protocol_count = rng.gen_range(1..=all_protocols.len());
        let protocols: Vec<Protocol> = all_protocols
            .choose_multiple(&mut rng, protocol_count)
            .cloned()
            .collect();
            
        // Connection options
        let use_ice = rng.gen_bool(0.8); // 80% chance of using ICE
        let use_direct = rng.gen_bool(0.5);
        let use_relay = rng.gen_bool(0.3); // Less common to use relay
        
        // Timeouts (1-60 seconds)
        let timeout = Duration::from_secs(rng.gen_range(1..60));
        
        // Generate candidates
        let packet_gen = NetworkPacketGenerator::new();
        let local_candidate_count = rng.gen_range(1..5);
        let mut local_candidates = Vec::with_capacity(local_candidate_count);
        for _ in 0..local_candidate_count {
            let ip = packet_gen.generate_ip();
            let port = rng.gen_range(1024..65535);
            local_candidates.push(SocketAddr::new(ip, port));
        }
        
        let peer_candidate_count = rng.gen_range(1..5);
        let mut peer_candidates = Vec::with_capacity(peer_candidate_count);
        for _ in 0..peer_candidate_count {
            let ip = packet_gen.generate_ip();
            let port = rng.gen_range(1024..65535);
            peer_candidates.push(SocketAddr::new(ip, port));
        }
        
        // Other parameters
        let connection_attempts = rng.gen_range(1..10);
        let keep_alive_interval = Duration::from_secs(rng.gen_range(10..120));
        
        P2PConnectionRequest {
            local_id,
            peer_id,
            stun_servers,
            turn_servers,
            protocols,
            use_ice,
            use_direct,
            use_relay,
            timeout,
            local_candidates,
            peer_candidates,
            connection_attempts,
            keep_alive_interval,
        }
    }
}