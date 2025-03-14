// form-fuzzing/src/harness/network.rs
//! Network harness for testing networking functionality and NAT traversal

use std::collections::{HashMap, HashSet};
use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, RwLock, Mutex};
use std::time::{Duration, Instant};

use crate::generators::network::{
    NetworkPacket, Protocol, NATConfig, 
    NATType, FilteringBehavior, P2PConnectionRequest
};
use crate::harness::FuzzingHarness;
use rand::Rng;

/// Result of a network operation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetworkResult {
    /// Operation was successful
    Success,
    /// Connection failed with reason
    ConnectionFailed(String),
    /// NAT traversal failed with reason
    NATTraversalFailed(String),
    /// Operation timed out
    Timeout,
    /// Invalid packet received
    InvalidPacket(String),
    /// Internal error occurred
    InternalError(String),
}

/// Simulates NAT behavior
pub struct MockNATSimulator {
    /// NAT configurations for each endpoint (endpoint_id -> NATConfig)
    nat_configs: HashMap<String, NATConfig>,
    /// Endpoint mappings (internal_endpoint -> external_endpoint)
    mappings: HashMap<SocketAddr, SocketAddr>,
    /// Blocked ports
    blocked_ports: HashSet<u16>,
    /// External IP addresses for each internal IP
    external_ips: HashMap<IpAddr, IpAddr>,
}

impl MockNATSimulator {
    /// Create a new NAT simulator
    pub fn new() -> Self {
        Self {
            nat_configs: HashMap::new(),
            mappings: HashMap::new(),
            blocked_ports: HashSet::new(),
            external_ips: HashMap::new(),
        }
    }
    
    /// Register a NAT configuration for an endpoint
    pub fn register_nat_config(&mut self, endpoint_id: &str, config: NATConfig) {
        self.nat_configs.insert(endpoint_id.to_string(), config);
    }
    
    /// Block a port
    pub fn block_port(&mut self, port: u16) {
        self.blocked_ports.insert(port);
    }
    
    /// Unblock a port
    pub fn unblock_port(&mut self, port: u16) {
        self.blocked_ports.remove(&port);
    }
    
    /// Get external endpoint for an internal endpoint
    pub fn get_external_endpoint(&mut self, endpoint_id: &str, internal_endpoint: SocketAddr, destination: Option<SocketAddr>) -> Result<SocketAddr, String> {
        let mut rng = rand::thread_rng();
        
        // Get NAT config
        let config = self.nat_configs.get(endpoint_id)
            .ok_or_else(|| format!("No NAT config for endpoint {}", endpoint_id))?;
            
        // Check if the port is blocked
        if self.blocked_ports.contains(&internal_endpoint.port()) {
            return Err(format!("Port {} is blocked", internal_endpoint.port()));
        }
        
        // Handle special NAT types
        match config.nat_type {
            NATType::None => {
                // No NAT, return the internal endpoint
                return Ok(internal_endpoint);
            },
            NATType::Symmetric => {
                // For symmetric NAT, create a new mapping for each destination
                if let Some(dest) = destination {
                    let external_ip = self.get_external_ip(endpoint_id, internal_endpoint.ip())?;
                    let external_port = self.allocate_external_port(config, internal_endpoint.port())?;
                    
                    let external_endpoint = SocketAddr::new(external_ip, external_port);
                    let key = format!("{}:{}:{}", internal_endpoint, dest.ip(), dest.port());
                    self.mappings.insert(internal_endpoint, external_endpoint);
                    
                    return Ok(external_endpoint);
                } else {
                    return Err("Destination required for symmetric NAT".to_string());
                }
            },
            _ => {
                // For other NAT types, check if we already have a mapping
                if let Some(external_endpoint) = self.mappings.get(&internal_endpoint) {
                    return Ok(*external_endpoint);
                }
                
                // Create a new mapping
                let external_ip = self.get_external_ip(endpoint_id, internal_endpoint.ip())?;
                let external_port = self.allocate_external_port(config, internal_endpoint.port())?;
                
                let external_endpoint = SocketAddr::new(external_ip, external_port);
                self.mappings.insert(internal_endpoint, external_endpoint);
                
                return Ok(external_endpoint);
            }
        }
    }
    
    /// Get the external IP for an internal IP
    fn get_external_ip(&mut self, endpoint_id: &str, internal_ip: IpAddr) -> Result<IpAddr, String> {
        let config = self.nat_configs.get(endpoint_id)
            .ok_or_else(|| format!("No NAT config for endpoint {}", endpoint_id))?;
            
        // If we already have an external IP for this internal IP, return it
        if let Some(external_ip) = self.external_ips.get(&internal_ip) {
            return Ok(*external_ip);
        }
        
        // Otherwise, use the external IP from the config
        let external_ip = config.external_ip;
        self.external_ips.insert(internal_ip, external_ip);
        
        Ok(external_ip)
    }
    
    /// Allocate an external port based on NAT mapping behavior
    fn allocate_external_port(&self, config: &NATConfig, internal_port: u16) -> Result<u16, String> {
        let mut rng = rand::thread_rng();
        
        match config.mapping_behavior {
            MappingBehavior::PortPreserving => {
                // Try to preserve the internal port if possible
                if internal_port >= config.port_range_start && internal_port <= config.port_range_end {
                    Ok(internal_port)
                } else {
                    // If port is outside the range, allocate a random port
                    if config.port_range_start >= config.port_range_end {
                        return Err("Invalid port range".to_string());
                    }
                    Ok(rng.gen_range(config.port_range_start..=config.port_range_end))
                }
            },
            MappingBehavior::Consistent => {
                // Always map to the same port in the range
                if config.port_range_start >= config.port_range_end {
                    return Err("Invalid port range".to_string());
                }
                
                // Deterministic mapping based on internal port
                let range_size = config.port_range_end - config.port_range_start + 1;
                let offset = (internal_port as u32 % range_size as u32) as u16;
                Ok(config.port_range_start + offset)
            },
            MappingBehavior::Random => {
                // Map to a random port in the range
                if config.port_range_start >= config.port_range_end {
                    return Err("Invalid port range".to_string());
                }
                Ok(rng.gen_range(config.port_range_start..=config.port_range_end))
            },
        }
    }
    
    /// Check if an incoming packet is allowed by NAT filtering rules
    pub fn is_packet_allowed(&self, endpoint_id: &str, src_endpoint: SocketAddr, dest_endpoint: SocketAddr) -> bool {
        let config = match self.nat_configs.get(endpoint_id) {
            Some(config) => config,
            None => return true, // No NAT config, allow all packets
        };
        
        // Check filtering behavior
        match config.filtering_behavior {
            FilteringBehavior::None => {
                // No filtering, allow all packets
                true
            },
            FilteringBehavior::Address => {
                // Check if we have an existing mapping to this address
                for (internal, external) in &self.mappings {
                    if internal.ip() == dest_endpoint.ip() && 
                       *external == src_endpoint {
                        return true;
                    }
                }
                false
            },
            FilteringBehavior::Endpoint => {
                // Check if we have an existing mapping to this exact endpoint
                self.mappings.iter()
                    .any(|(internal, external)| 
                        *internal == dest_endpoint && *external == src_endpoint
                    )
            },
        }
    }
    
    /// Reset all NAT state
    pub fn reset(&mut self) {
        self.mappings.clear();
        self.blocked_ports.clear();
        self.external_ips.clear();
    }
}

/// Mock packet router to simulate network conditions
pub struct MockPacketRouter {
    /// Network latency in milliseconds
    latency_ms: u32,
    /// Packet loss rate (0.0 - 1.0)
    packet_loss_rate: f64,
    /// Packet corruption rate (0.0 - 1.0)
    corruption_rate: f64,
    /// NAT simulator
    nat_simulator: Arc<Mutex<MockNATSimulator>>,
}

impl MockPacketRouter {
    /// Create a new packet router
    pub fn new(nat_simulator: Arc<Mutex<MockNATSimulator>>) -> Self {
        Self {
            latency_ms: 50, // Default 50ms latency
            packet_loss_rate: 0.0,
            corruption_rate: 0.0,
            nat_simulator,
        }
    }
    
    /// Set network latency
    pub fn set_latency(&mut self, latency_ms: u32) {
        self.latency_ms = latency_ms;
    }
    
    /// Set packet loss rate
    pub fn set_packet_loss_rate(&mut self, rate: f64) {
        self.packet_loss_rate = rate.max(0.0).min(1.0);
    }
    
    /// Set packet corruption rate
    pub fn set_corruption_rate(&mut self, rate: f64) {
        self.corruption_rate = rate.max(0.0).min(1.0);
    }
    
    /// Route a packet between source and destination
    pub fn route_packet(&self, src_id: &str, src_endpoint: SocketAddr, dst_id: &str, dst_endpoint: SocketAddr, packet: &NetworkPacket) -> NetworkResult {
        let mut rng = rand::thread_rng();
        
        // Simulate packet loss
        if rng.gen_bool(self.packet_loss_rate) {
            return NetworkResult::ConnectionFailed("Packet lost".to_string());
        }
        
        // Get NAT simulator
        let nat_simulator = self.nat_simulator.lock().unwrap();
        
        // Translate source endpoint through NAT
        let nat_src_endpoint = match nat_simulator.get_external_endpoint(src_id, src_endpoint, Some(dst_endpoint)) {
            Ok(ep) => ep,
            Err(err) => return NetworkResult::NATTraversalFailed(err),
        };
        
        // Check if the packet is allowed by destination NAT
        if !nat_simulator.is_packet_allowed(dst_id, nat_src_endpoint, dst_endpoint) {
            return NetworkResult::NATTraversalFailed("Packet blocked by NAT filtering".to_string());
        }
        
        // Simulate network latency
        if self.latency_ms > 0 {
            std::thread::sleep(Duration::from_millis(self.latency_ms as u64));
        }
        
        // Simulate packet corruption
        if rng.gen_bool(self.corruption_rate) {
            return NetworkResult::InvalidPacket("Packet corrupted".to_string());
        }
        
        // Packet delivered successfully
        NetworkResult::Success
    }
}

/// Mock STUN server
pub struct MockSTUNServer {
    /// Server endpoint
    endpoint: SocketAddr,
    /// NAT simulator
    nat_simulator: Arc<Mutex<MockNATSimulator>>,
}

impl MockSTUNServer {
    /// Create a new STUN server
    pub fn new(endpoint: SocketAddr, nat_simulator: Arc<Mutex<MockNATSimulator>>) -> Self {
        Self {
            endpoint,
            nat_simulator,
        }
    }
    
    /// Get the reflexive address for an endpoint
    pub fn get_reflexive_address(&self, endpoint_id: &str, local_endpoint: SocketAddr) -> Result<SocketAddr, String> {
        let mut nat_simulator = self.nat_simulator.lock().unwrap();
        nat_simulator.get_external_endpoint(endpoint_id, local_endpoint, Some(self.endpoint))
    }
}

/// Mock P2P connection manager
pub struct MockP2PConnectionManager {
    /// NAT simulator
    nat_simulator: Arc<Mutex<MockNATSimulator>>,
    /// Packet router
    packet_router: Arc<RwLock<MockPacketRouter>>,
    /// STUN servers
    stun_servers: Vec<MockSTUNServer>,
}

impl MockP2PConnectionManager {
    /// Create a new P2P connection manager
    pub fn new(nat_simulator: Arc<Mutex<MockNATSimulator>>, packet_router: Arc<RwLock<MockPacketRouter>>) -> Self {
        // Create some default STUN servers
        let mut stun_servers = Vec::new();
        
        // Add a few STUN servers with different IPs
        let stun_ips = [
            "192.0.2.1".parse().unwrap(),
            "192.0.2.2".parse().unwrap(),
            "192.0.2.3".parse().unwrap(),
        ];
        
        for ip in stun_ips.iter() {
            let endpoint = SocketAddr::new(*ip, 3478);
            stun_servers.push(MockSTUNServer::new(endpoint, nat_simulator.clone()));
        }
        
        Self {
            nat_simulator,
            packet_router,
            stun_servers,
        }
    }
    
    /// Add a STUN server
    pub fn add_stun_server(&mut self, endpoint: SocketAddr) {
        self.stun_servers.push(MockSTUNServer::new(endpoint, self.nat_simulator.clone()));
    }
    
    /// Try to establish a direct connection
    pub fn try_direct_connection(&self, local_id: &str, local_endpoint: SocketAddr, peer_id: &str, peer_endpoint: SocketAddr) -> NetworkResult {
        // Try to send a packet from local to peer
        let packet = NetworkPacket {
            protocol: Protocol::UDP,
            source_ip: local_endpoint.ip(),
            source_port: local_endpoint.port(),
            destination_ip: peer_endpoint.ip(),
            destination_port: peer_endpoint.port(),
            length: 64,
            ttl: 64,
            fragmented: false,
            payload: vec![0; 64],
            headers: HashMap::new(),
        };
        
        // Route the packet
        let router = self.packet_router.read().unwrap();
        router.route_packet(local_id, local_endpoint, peer_id, peer_endpoint, &packet)
    }
    
    /// Try to establish a connection using STUN
    pub fn try_stun_connection(&self, local_id: &str, local_endpoint: SocketAddr, peer_id: &str, peer_endpoint: SocketAddr) -> NetworkResult {
        // Get the reflexive address for both endpoints
        let local_reflexive = match self.stun_servers[0].get_reflexive_address(local_id, local_endpoint) {
            Ok(addr) => addr,
            Err(err) => return NetworkResult::NATTraversalFailed(format!("Local STUN failed: {}", err)),
        };
        
        let peer_reflexive = match self.stun_servers[0].get_reflexive_address(peer_id, peer_endpoint) {
            Ok(addr) => addr,
            Err(err) => return NetworkResult::NATTraversalFailed(format!("Peer STUN failed: {}", err)),
        };
        
        // Create a packet using the reflexive addresses
        let packet = NetworkPacket {
            protocol: Protocol::UDP,
            source_ip: local_reflexive.ip(),
            source_port: local_reflexive.port(),
            destination_ip: peer_reflexive.ip(),
            destination_port: peer_reflexive.port(),
            length: 64,
            ttl: 64,
            fragmented: false,
            payload: vec![0; 64],
            headers: HashMap::new(),
        };
        
        // Route the packet
        let router = self.packet_router.read().unwrap();
        router.route_packet(local_id, local_endpoint, peer_id, peer_endpoint, &packet)
    }
    
    /// Try to establish a connection using a relay (TURN)
    pub fn try_relay_connection(&self, local_id: &str, local_endpoint: SocketAddr, peer_id: &str, peer_endpoint: SocketAddr) -> NetworkResult {
        // This is a simplified simulation of TURN relay
        // In reality, TURN is much more complex
        
        // Create a relay endpoint
        let relay_ip = "192.0.2.100".parse().unwrap();
        let relay_port = 49152;
        let relay_endpoint = SocketAddr::new(relay_ip, relay_port);
        
        // First, route from local to relay
        let local_to_relay = NetworkPacket {
            protocol: Protocol::UDP,
            source_ip: local_endpoint.ip(),
            source_port: local_endpoint.port(),
            destination_ip: relay_endpoint.ip(),
            destination_port: relay_endpoint.port(),
            length: 64,
            ttl: 64,
            fragmented: false,
            payload: vec![0; 64],
            headers: HashMap::new(),
        };
        
        let router = self.packet_router.read().unwrap();
        let local_result = router.route_packet(local_id, local_endpoint, "relay", relay_endpoint, &local_to_relay);
        
        // Check if the first leg was successful
        if local_result != NetworkResult::Success {
            return local_result;
        }
        
        // Then, route from relay to peer
        let relay_to_peer = NetworkPacket {
            protocol: Protocol::UDP,
            source_ip: relay_endpoint.ip(),
            source_port: relay_endpoint.port(),
            destination_ip: peer_endpoint.ip(),
            destination_port: peer_endpoint.port(),
            length: 64,
            ttl: 64,
            fragmented: false,
            payload: vec![0; 64],
            headers: HashMap::new(),
        };
        
        router.route_packet("relay", relay_endpoint, peer_id, peer_endpoint, &relay_to_peer)
    }
    
    /// Establish a P2P connection
    pub fn establish_connection(&self, request: &P2PConnectionRequest) -> NetworkResult {
        let mut rng = rand::thread_rng();
        
        // Check if there are any protocols and candidates
        if request.protocols.is_empty() {
            return NetworkResult::ConnectionFailed("No protocols specified".to_string());
        }
        
        if request.local_candidates.is_empty() || request.peer_candidates.is_empty() {
            return NetworkResult::ConnectionFailed("Missing candidates".to_string());
        }
        
        // Try connection methods in order
        let start_time = Instant::now();
        
        for _ in 0..request.connection_attempts {
            // Randomly select local and peer candidates
            let local_candidate = *request.local_candidates.choose(&mut rng).unwrap();
            let peer_candidate = *request.peer_candidates.choose(&mut rng).unwrap();
            
            // Check timeout
            if start_time.elapsed() > request.timeout {
                return NetworkResult::Timeout;
            }
            
            // Try direct connection if enabled
            if request.use_direct {
                let result = self.try_direct_connection(&request.local_id, local_candidate, &request.peer_id, peer_candidate);
                if result == NetworkResult::Success {
                    return result;
                }
            }
            
            // Check timeout again
            if start_time.elapsed() > request.timeout {
                return NetworkResult::Timeout;
            }
            
            // Try STUN if enabled and there are STUN servers
            if request.use_ice && !request.stun_servers.is_empty() {
                let result = self.try_stun_connection(&request.local_id, local_candidate, &request.peer_id, peer_candidate);
                if result == NetworkResult::Success {
                    return result;
                }
            }
            
            // Check timeout again
            if start_time.elapsed() > request.timeout {
                return NetworkResult::Timeout;
            }
            
            // Try relay if enabled and there are TURN servers
            if request.use_relay && !request.turn_servers.is_empty() {
                let result = self.try_relay_connection(&request.local_id, local_candidate, &request.peer_id, peer_candidate);
                if result == NetworkResult::Success {
                    return result;
                }
            }
            
            // Wait a bit before trying again
            std::thread::sleep(Duration::from_millis(100));
        }
        
        // All connection attempts failed
        NetworkResult::ConnectionFailed("All connection attempts failed".to_string())
    }
}

/// Network harness for testing network functionality
pub struct NetworkHarness {
    /// NAT simulator
    nat_simulator: Arc<Mutex<MockNATSimulator>>,
    /// Packet router
    packet_router: Arc<RwLock<MockPacketRouter>>,
    /// P2P connection manager
    p2p_manager: Option<MockP2PConnectionManager>,
}

impl NetworkHarness {
    /// Create a new network harness
    pub fn new() -> Self {
        // Create NAT simulator and packet router
        let nat_simulator = Arc::new(Mutex::new(MockNATSimulator::new()));
        let packet_router = Arc::new(RwLock::new(MockPacketRouter::new(nat_simulator.clone())));
        
        Self {
            nat_simulator,
            packet_router,
            p2p_manager: None,
        }
    }
    
    /// Test packet routing
    pub fn test_packet_routing(&mut self, src_id: &str, dst_id: &str, packet: &NetworkPacket) -> NetworkResult {
        // Create source and destination endpoints
        let src_endpoint = SocketAddr::new(packet.source_ip, packet.source_port);
        let dst_endpoint = SocketAddr::new(packet.destination_ip, packet.destination_port);
        
        // Route the packet
        let router = self.packet_router.read().unwrap();
        router.route_packet(src_id, src_endpoint, dst_id, dst_endpoint, packet)
    }
    
    /// Test NAT traversal
    pub fn test_nat_traversal(&mut self, local_nat: NATConfig, remote_nat: NATConfig) -> NetworkResult {
        // Register NAT configurations
        {
            let mut nat_simulator = self.nat_simulator.lock().unwrap();
            nat_simulator.register_nat_config("local", local_nat.clone());
            nat_simulator.register_nat_config("remote", remote_nat.clone());
        }
        
        // Create random local and remote endpoints
        let mut rng = rand::thread_rng();
        let local_ip = local_nat.internal_ip;
        let local_port = rng.gen_range(1024..65535);
        let local_endpoint = SocketAddr::new(local_ip, local_port);
        
        let remote_ip = remote_nat.internal_ip;
        let remote_port = rng.gen_range(1024..65535);
        let remote_endpoint = SocketAddr::new(remote_ip, remote_port);
        
        // Create a test packet
        let packet = NetworkPacket {
            protocol: Protocol::UDP, // Use UDP for NAT traversal
            source_ip: local_endpoint.ip(),
            source_port: local_endpoint.port(),
            destination_ip: remote_endpoint.ip(),
            destination_port: remote_endpoint.port(),
            length: 64,
            ttl: 64,
            fragmented: false,
            payload: vec![0; 64],
            headers: HashMap::new(),
        };
        
        // Route the packet
        let router = self.packet_router.read().unwrap();
        router.route_packet("local", local_endpoint, "remote", remote_endpoint, &packet)
    }
    
    /// Test P2P connection
    pub fn test_p2p_connection(&mut self, request: &P2PConnectionRequest) -> NetworkResult {
        // Make sure P2P manager is initialized
        if self.p2p_manager.is_none() {
            self.p2p_manager = Some(MockP2PConnectionManager::new(
                self.nat_simulator.clone(),
                self.packet_router.clone()
            ));
        }
        
        // Register NAT configurations for local and peer
        {
            let mut nat_simulator = self.nat_simulator.lock().unwrap();
            
            // Create NAT configs if not specified
            let local_nat = NATConfig {
                nat_type: NATType::random(),
                mapping_behavior: MappingBehavior::random(),
                filtering_behavior: FilteringBehavior::random(),
                port_range_start: 1024,
                port_range_end: 65535,
                mapping_timeout: Duration::from_secs(300),
                internal_ip: if !request.local_candidates.is_empty() {
                    request.local_candidates[0].ip()
                } else {
                    "192.168.1.2".parse().unwrap()
                },
                external_ip: "203.0.113.1".parse().unwrap(),
                upstream_nat: None,
                mapping_refresh_enabled: true,
                upnp_enabled: false,
                pmp_enabled: false,
                max_connections: 1000,
            };
            
            let peer_nat = NATConfig {
                nat_type: NATType::random(),
                mapping_behavior: MappingBehavior::random(),
                filtering_behavior: FilteringBehavior::random(),
                port_range_start: 1024,
                port_range_end: 65535,
                mapping_timeout: Duration::from_secs(300),
                internal_ip: if !request.peer_candidates.is_empty() {
                    request.peer_candidates[0].ip()
                } else {
                    "192.168.2.2".parse().unwrap()
                },
                external_ip: "203.0.113.2".parse().unwrap(),
                upstream_nat: None,
                mapping_refresh_enabled: true,
                upnp_enabled: false,
                pmp_enabled: false,
                max_connections: 1000,
            };
            
            nat_simulator.register_nat_config(&request.local_id, local_nat);
            nat_simulator.register_nat_config(&request.peer_id, peer_nat);
        }
        
        // Establish connection
        self.p2p_manager.as_ref().unwrap().establish_connection(request)
    }
}

impl FuzzingHarness for NetworkHarness {
    fn setup(&mut self) {
        // Reset NAT simulator
        let mut nat_simulator = self.nat_simulator.lock().unwrap();
        nat_simulator.reset();
        
        // Create P2P manager
        self.p2p_manager = Some(MockP2PConnectionManager::new(
            self.nat_simulator.clone(),
            self.packet_router.clone()
        ));
    }
    
    fn teardown(&mut self) {
        // Clean up resources
        let mut nat_simulator = self.nat_simulator.lock().unwrap();
        nat_simulator.reset();
        self.p2p_manager = None;
    }
    
    fn reset(&mut self) {
        self.teardown();
        self.setup();
    }
}
