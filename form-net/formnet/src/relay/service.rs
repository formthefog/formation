//! Relay service implementation
//!
//! This module implements the relay service that forwards packets
//! between peers that cannot establish direct connections.

use std::collections::{HashMap, HashSet};
use std::net::{SocketAddr, UdpSocket};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use std::thread;

use log::{debug, error, info, warn};
use socket2::{Domain, Socket, Type};
use tokio::sync::mpsc;
use rand::Rng;

use crate::relay::{
    ConnectionRequest, ConnectionResponse, ConnectionStatus, 
    DiscoveryQuery, DiscoveryResponse, Heartbeat, RelayAnnouncement,
    RelayError, RelayHeader, RelayMessage, RelayNodeInfo, RelayPacket,
    RELAY_CAP_IPV4, RELAY_CAP_IPV6, RELAY_CAP_HIGH_BANDWIDTH, RELAY_CAP_LOW_LATENCY,
    Result
};

/// Default interval for maintenance tasks
const MAINTENANCE_INTERVAL: Duration = Duration::from_secs(30);

/// Default session expiration time (1 hour)
const DEFAULT_SESSION_EXPIRATION: Duration = Duration::from_secs(3600);

/// Default maximum number of sessions per client
const DEFAULT_MAX_SESSIONS_PER_CLIENT: usize = 5;

/// Default maximum number of total concurrent sessions
const DEFAULT_MAX_TOTAL_SESSIONS: usize = 1000;

/// Default maximum rate of connection requests per minute
const DEFAULT_MAX_CONNECTION_RATE: usize = 60;

/// Default maximum number of packets per second
const DEFAULT_MAX_PACKETS_PER_SECOND: usize = 100;

/// Default maximum packet size (bytes)
const DEFAULT_MAX_PACKET_SIZE: usize = 1500;

/// Session information for a relay connection
struct RelaySession {
    /// Unique session ID
    id: u64,
    
    /// Public key of the initiating peer
    initiator_pubkey: [u8; 32],
    
    /// Public key of the target peer
    target_pubkey: [u8; 32],
    
    /// When the session was created
    created_at: SystemTime,
    
    /// When the session expires
    expires_at: SystemTime,
    
    /// Last activity time
    last_activity: Instant,
    
    /// Number of packets forwarded from initiator to target
    packets_forwarded_initiator_to_target: u64,
    
    /// Number of packets forwarded from target to initiator
    packets_forwarded_target_to_initiator: u64,
    
    /// Total bytes forwarded from initiator to target
    bytes_forwarded_initiator_to_target: u64,
    
    /// Total bytes forwarded from target to initiator
    bytes_forwarded_target_to_initiator: u64,
    
    /// Last known address of the initiator
    initiator_addr: Option<SocketAddr>,
    
    /// Last known address of the target
    target_addr: Option<SocketAddr>,
}

impl RelaySession {
    /// Create a new relay session
    fn new(id: u64, initiator_pubkey: [u8; 32], target_pubkey: [u8; 32]) -> Self {
        let now = SystemTime::now();
        let expires_at = now + DEFAULT_SESSION_EXPIRATION;
        
        Self {
            id,
            initiator_pubkey,
            target_pubkey,
            created_at: now,
            expires_at,
            last_activity: Instant::now(),
            packets_forwarded_initiator_to_target: 0,
            packets_forwarded_target_to_initiator: 0,
            bytes_forwarded_initiator_to_target: 0,
            bytes_forwarded_target_to_initiator: 0,
            initiator_addr: None,
            target_addr: None,
        }
    }
    
    /// Check if the session is expired
    fn is_expired(&self) -> bool {
        SystemTime::now() > self.expires_at
    }
    
    /// Check if the session is inactive (no activity for a while)
    fn is_inactive(&self, inactivity_threshold: Duration) -> bool {
        self.last_activity.elapsed() > inactivity_threshold
    }
    
    /// Update activity timestamp
    fn update_activity(&mut self) {
        self.last_activity = Instant::now();
    }
    
    /// Extend session expiration
    fn extend_expiration(&mut self, duration: Duration) {
        self.expires_at = SystemTime::now() + duration;
    }
    
    /// Record packet forwarding from initiator to target
    fn record_initiator_to_target(&mut self, bytes: usize) {
        self.packets_forwarded_initiator_to_target += 1;
        self.bytes_forwarded_initiator_to_target += bytes as u64;
        self.update_activity();
    }
    
    /// Record packet forwarding from target to initiator
    fn record_target_to_initiator(&mut self, bytes: usize) {
        self.packets_forwarded_target_to_initiator += 1;
        self.bytes_forwarded_target_to_initiator += bytes as u64;
        self.update_activity();
    }
    
    /// Update the address of the initiator
    fn update_initiator_addr(&mut self, addr: SocketAddr) {
        self.initiator_addr = Some(addr);
    }
    
    /// Update the address of the target
    fn update_target_addr(&mut self, addr: SocketAddr) {
        self.target_addr = Some(addr);
    }
    
    /// Authenticate a packet against this session
    pub fn authenticate_packet(&self, packet: &RelayPacket) -> bool {
        // Check if the packet's header session ID matches this session
        if packet.header.session_id != self.id {
            return false;
        }
        
        // Verify the destination peer ID matches either initiator or target
        let peer_id_matches = packet.header.dest_peer_id == self.initiator_pubkey || 
                             packet.header.dest_peer_id == self.target_pubkey;
        
        if !peer_id_matches {
            return false;
        }
        
        // Ensure the header timestamp is valid (not too old, not future)
        if !packet.header.is_valid() {
            return false;
        }
        
        true
    }
    
    /// Generate a session token for authenticating future requests
    pub fn generate_auth_token(&self) -> Vec<u8> {
        // Combine session ID, both public keys, and a timestamp
        let mut data = Vec::with_capacity(32 + 32 + 8 + 8);
        
        // Add session ID
        data.extend_from_slice(&self.id.to_le_bytes());
        
        // Add initiator and target public keys
        data.extend_from_slice(&self.initiator_pubkey);
        data.extend_from_slice(&self.target_pubkey);
        
        // Add current timestamp
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        data.extend_from_slice(&now.to_le_bytes());
        
        // Use crypto hash function if available, or simpler hash for now
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        data.hash(&mut hasher);
        let hash = hasher.finish();
        
        // Return the token
        data.extend_from_slice(&hash.to_le_bytes());
        data
    }
    
    /// Verify if a provided auth token is valid for this session
    pub fn verify_auth_token(&self, token: &[u8]) -> bool {
        // Simple implementation for now - in a real system, we'd use proper cryptographic verification
        let current_token = self.generate_auth_token();
        
        // Constant-time comparison to prevent timing attacks
        if token.len() != current_token.len() {
            return false;
        }
        
        let mut result = 0;
        for (a, b) in token.iter().zip(current_token.iter()) {
            result |= a ^ b;
        }
        
        result == 0
    }
}

/// Statistics for the relay service
#[derive(Debug, Clone)]
pub struct RelayStats {
    /// Total number of connection requests processed
    pub connection_requests: u64,
    
    /// Number of successful connections established
    pub successful_connections: u64,
    
    /// Number of rejected connection requests
    pub rejected_connections: u64,
    
    /// Number of packets forwarded
    pub packets_forwarded: u64,
    
    /// Total bytes forwarded
    pub bytes_forwarded: u64,
    
    /// Number of active sessions
    pub active_sessions: usize,
    
    /// Number of expired sessions cleaned up
    pub expired_sessions: u64,
    
    /// Number of heartbeats processed
    pub heartbeats_processed: u64,
    
    /// Current bandwidth usage (bytes per second)
    pub current_bandwidth_bps: u64,
    
    /// Peak bandwidth usage (bytes per second)
    pub peak_bandwidth_bps: u64,
    
    /// Average packet size
    pub avg_packet_size: u64,
    
    /// Active clients (peers using the relay)
    pub active_clients: usize,
    
    /// Current CPU usage percentage (0-100)
    pub cpu_usage_pct: u8,
    
    /// Current memory usage (bytes)
    pub memory_usage_bytes: u64,
    
    /// Service uptime in seconds
    pub uptime_seconds: u64,
    
    /// Time when statistics were last reset
    pub last_reset: SystemTime,
}

impl Default for RelayStats {
    fn default() -> Self {
        Self {
            connection_requests: 0,
            successful_connections: 0,
            rejected_connections: 0,
            packets_forwarded: 0,
            bytes_forwarded: 0,
            active_sessions: 0,
            expired_sessions: 0,
            heartbeats_processed: 0,
            current_bandwidth_bps: 0,
            peak_bandwidth_bps: 0,
            avg_packet_size: 0,
            active_clients: 0,
            cpu_usage_pct: 0,
            memory_usage_bytes: 0,
            uptime_seconds: 0,
            last_reset: SystemTime::now(),
        }
    }
}

impl RelayStats {
    /// Reset all counters to zero
    pub fn reset(&mut self) {
        *self = Self::default();
    }
    
    /// Update bandwidth metrics based on recent data transfer
    pub fn update_bandwidth(&mut self, bytes: u64, period: Duration) {
        if period.as_secs() > 0 {
            let bps = bytes * 8 / period.as_secs();
            self.current_bandwidth_bps = bps;
            
            if bps > self.peak_bandwidth_bps {
                self.peak_bandwidth_bps = bps;
            }
        }
    }
    
    /// Record a forwarded packet
    pub fn record_forwarded_packet(&mut self, bytes: usize) {
        self.packets_forwarded += 1;
        self.bytes_forwarded += bytes as u64;
        
        // Update average packet size
        if self.packets_forwarded > 0 {
            self.avg_packet_size = self.bytes_forwarded / self.packets_forwarded;
        }
    }
    
    /// Calculate uptime in seconds
    pub fn calculate_uptime(&mut self, start_time: SystemTime) {
        if let Ok(duration) = SystemTime::now().duration_since(start_time) {
            self.uptime_seconds = duration.as_secs();
        }
    }
}

/// Resource usage limits for the relay node
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    /// Maximum number of total concurrent sessions
    pub max_total_sessions: usize,
    
    /// Maximum number of sessions per client (identified by public key)
    pub max_sessions_per_client: usize,
    
    /// Maximum rate of connection requests per minute
    pub max_connection_rate: usize,
    
    /// Maximum bandwidth in bytes per second
    pub max_bandwidth_bps: Option<u64>,
    
    /// Maximum packet size in bytes
    pub max_packet_size: usize,
    
    /// Maximum packets per second
    pub max_packets_per_second: usize,
    
    /// Session inactivity timeout
    pub session_inactivity_timeout: Duration,
    
    /// Default session expiration
    pub default_session_expiration: Duration,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_total_sessions: DEFAULT_MAX_TOTAL_SESSIONS,
            max_sessions_per_client: DEFAULT_MAX_SESSIONS_PER_CLIENT,
            max_connection_rate: DEFAULT_MAX_CONNECTION_RATE,
            max_bandwidth_bps: None, // No bandwidth limit by default
            max_packet_size: DEFAULT_MAX_PACKET_SIZE,
            max_packets_per_second: DEFAULT_MAX_PACKETS_PER_SECOND,
            session_inactivity_timeout: Duration::from_secs(300), // 5 minutes
            default_session_expiration: DEFAULT_SESSION_EXPIRATION,
        }
    }
}

/// Configuration for the relay node
#[derive(Debug, Clone)]
pub struct RelayConfig {
    /// Listen address for the relay service
    pub listen_addr: SocketAddr,
    
    /// Public key of the relay node
    pub pubkey: [u8; 32],
    
    /// Geographic region of the relay (optional)
    pub region: Option<String>,
    
    /// Capabilities offered by this relay
    pub capabilities: u32,
    
    /// Resource limits
    pub limits: ResourceLimits,
    
    /// Interval for maintenance tasks (cleanup, stats update)
    pub maintenance_interval: Duration,
    
    /// Whether to announce this relay to the network
    pub announce_to_network: bool,
    
    /// List of bootstrap relay nodes to announce to
    pub bootstrap_relays: Vec<String>,
}

impl RelayConfig {
    /// Create a new relay configuration with default values
    pub fn new(listen_addr: SocketAddr, pubkey: [u8; 32]) -> Self {
        Self {
            listen_addr,
            pubkey,
            region: None,
            capabilities: RELAY_CAP_IPV4, // IPv4 support by default
            limits: ResourceLimits::default(),
            maintenance_interval: MAINTENANCE_INTERVAL,
            announce_to_network: false,
            bootstrap_relays: Vec::new(),
        }
    }
    
    /// Set geographic region
    pub fn with_region(mut self, region: impl Into<String>) -> Self {
        self.region = Some(region.into());
        self
    }
    
    /// Set relay capabilities
    pub fn with_capabilities(mut self, capabilities: u32) -> Self {
        self.capabilities = capabilities;
        self
    }
    
    /// Set resource limits
    pub fn with_limits(mut self, limits: ResourceLimits) -> Self {
        self.limits = limits;
        self
    }
    
    /// Enable or disable network announcements
    pub fn with_announcements(mut self, enabled: bool) -> Self {
        self.announce_to_network = enabled;
        self
    }
    
    /// Set bootstrap relays for announcements
    pub fn with_bootstrap_relays(mut self, relays: Vec<String>) -> Self {
        self.bootstrap_relays = relays;
        self
    }
}

/// The RelayNode is the main service implementation that handles
/// relay connections between peers.
pub struct RelayNode {
    /// Configuration for the relay
    config: RelayConfig,
    
    /// Active sessions by session ID
    sessions: Arc<RwLock<HashMap<u64, RelaySession>>>,
    
    /// Map of public key to session IDs (for initiated sessions)
    initiator_sessions: Arc<RwLock<HashMap<String, HashSet<u64>>>>,
    
    /// Map of public key to session IDs (for target sessions)
    target_sessions: Arc<RwLock<HashMap<String, HashSet<u64>>>>,
    
    /// Connection rate tracking
    connection_attempts: Arc<Mutex<Vec<Instant>>>,
    
    /// Statistics for the relay service
    stats: Arc<RwLock<RelayStats>>,
    
    /// Start time of the service for uptime tracking
    start_time: SystemTime,
    
    /// Shutdown signal sender
    shutdown_sender: Option<mpsc::Sender<()>>,
    
    /// Message rate tracking
    packet_times: Arc<Mutex<Vec<Instant>>>,
    
    /// Socket for UDP communication
    socket: Option<Arc<UdpSocket>>,
}

impl RelayNode {
    /// Create a new relay node with the given configuration
    pub fn new(config: RelayConfig) -> Self {
        Self {
            config,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            initiator_sessions: Arc::new(RwLock::new(HashMap::new())),
            target_sessions: Arc::new(RwLock::new(HashMap::new())),
            connection_attempts: Arc::new(Mutex::new(Vec::new())),
            stats: Arc::new(RwLock::new(RelayStats::default())),
            start_time: SystemTime::now(),
            shutdown_sender: None,
            packet_times: Arc::new(Mutex::new(Vec::new())),
            socket: None,
        }
    }
    
    /// Start the relay service
    pub fn start(&mut self) -> Result<()> {
        info!("Starting relay service on {}", self.config.listen_addr);
        
        // Create and bind UDP socket
        let socket = UdpSocket::bind(&self.config.listen_addr)
            .map_err(|e| RelayError::Io(e))?;
        
        // Set non-blocking mode
        socket.set_nonblocking(true)
            .map_err(|e| RelayError::Io(e))?;
        
        let socket = Arc::new(socket);
        self.socket = Some(socket.clone());
        
        // Set up shutdown channel
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        self.shutdown_sender = Some(shutdown_tx);
        
        // Create a clone of necessary structures for the main loop thread
        let sessions = self.sessions.clone();
        let initiator_sessions = self.initiator_sessions.clone();
        let target_sessions = self.target_sessions.clone();
        let connection_attempts = self.connection_attempts.clone();
        let stats = self.stats.clone();
        let packet_times = self.packet_times.clone();
        let start_time = self.start_time;
        let config = self.config.clone();
        
        // Start the main processing loop in a separate thread
        thread::spawn(move || {
            let mut buffer = [0u8; 2048];
            let mut last_maintenance = Instant::now();
            
            loop {
                // Check if we need to perform maintenance
                if last_maintenance.elapsed() >= config.maintenance_interval {
                    Self::perform_maintenance(
                        &sessions, 
                        &initiator_sessions, 
                        &target_sessions, 
                        &stats,
                        &config.limits,
                        start_time
                    );
                    last_maintenance = Instant::now();
                }
                
                // Check for shutdown signal
                if shutdown_rx.try_recv().is_ok() {
                    info!("Relay service shutting down");
                    break;
                }
                
                // Try to receive a packet
                match socket.recv_from(&mut buffer) {
                    Ok((len, src_addr)) => {
                        // Record packet receipt time for rate limiting
                        Self::record_packet_time(&packet_times, &config.limits);
                        
                        // Process the received packet
                        if let Err(e) = Self::process_packet(
                            &socket,
                            &buffer[..len],
                            src_addr,
                            &sessions,
                            &initiator_sessions,
                            &target_sessions,
                            &connection_attempts,
                            &stats,
                            &config
                        ) {
                            warn!("Error processing packet: {}", e);
                        }
                    },
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        // No data available, sleep briefly
                        thread::sleep(Duration::from_millis(10));
                    },
                    Err(e) => {
                        error!("Error receiving packet: {}", e);
                        thread::sleep(Duration::from_millis(100));
                    }
                }
            }
        });
        
        Ok(())
    }
    
    /// Stop the relay service
    pub fn stop(&mut self) {
        info!("Stopping relay service");
        
        if let Some(sender) = self.shutdown_sender.take() {
            let _ = sender.try_send(());
        }
        
        self.socket = None;
    }
    
    /// Get relay node information
    pub fn get_node_info(&self) -> RelayNodeInfo {
        let stats = self.stats.read().unwrap();
        let load = std::cmp::min(
            ((stats.active_sessions as f32 / self.config.limits.max_total_sessions as f32) * 100.0) as u8,
            100
        );
        
        RelayNodeInfo {
            pubkey: self.config.pubkey,
            endpoints: vec![self.config.listen_addr.to_string()],
            region: self.config.region.clone(),
            capabilities: self.config.capabilities,
            load,
            latency: None, // We don't know our latency from the client perspective
            max_sessions: self.config.limits.max_total_sessions as u32,
            protocol_version: 1, // Current protocol version
        }
    }
    
    /// Get current statistics
    pub fn get_stats(&self) -> RelayStats {
        self.stats.read().unwrap().clone()
    }
    
    /// Record a packet receipt time for rate limiting
    fn record_packet_time(packet_times: &Arc<Mutex<Vec<Instant>>>, limits: &ResourceLimits) -> bool {
        let now = Instant::now();
        let mut times = packet_times.lock().unwrap();
        
        // Remove old packet times (older than 1 second)
        times.retain(|&time| now.duration_since(time) < Duration::from_secs(1));
        
        // Check if we're exceeding the rate limit
        if times.len() >= limits.max_packets_per_second {
            return false;
        }
        
        // Record this packet
        times.push(now);
        true
    }
    
    /// Process a received packet
    #[allow(clippy::too_many_arguments)]
    fn process_packet(
        socket: &Arc<UdpSocket>,
        data: &[u8],
        src_addr: SocketAddr,
        sessions: &Arc<RwLock<HashMap<u64, RelaySession>>>,
        initiator_sessions: &Arc<RwLock<HashMap<String, HashSet<u64>>>>,
        target_sessions: &Arc<RwLock<HashMap<String, HashSet<u64>>>>,
        connection_attempts: &Arc<Mutex<Vec<Instant>>>,
        stats: &Arc<RwLock<RelayStats>>,
        config: &RelayConfig
    ) -> Result<()> {
        // Ensure packet is not too large
        if data.len() > config.limits.max_packet_size {
            return Err(RelayError::Protocol("Packet too large".into()));
        }
        
        // Try to deserialize as a relay packet
        if let Ok(packet) = bincode::deserialize::<RelayPacket>(data) {
            return Self::process_relay_packet(socket, packet, src_addr, sessions, stats);
        }
        
        // Try to deserialize as a connection request
        if let Ok(request) = bincode::deserialize::<ConnectionRequest>(data) {
            return Self::process_connection_request(
                socket,
                request,
                src_addr,
                sessions,
                initiator_sessions,
                target_sessions,
                connection_attempts,
                stats,
                config
            );
        }
        
        // Try to deserialize as a heartbeat
        if let Ok(heartbeat) = bincode::deserialize::<Heartbeat>(data) {
            return Self::process_heartbeat(socket, heartbeat, sessions, stats);
        }
        
        // Try to deserialize as a discovery query
        if let Ok(query) = bincode::deserialize::<DiscoveryQuery>(data) {
            return Self::process_discovery_query(socket, query, src_addr, stats, config);
        }
        
        // Unknown packet type
        Err(RelayError::Protocol("Unknown packet type".into()))
    }
    
    /// Perform maintenance tasks (cleanup expired sessions, update stats)
    #[allow(clippy::too_many_arguments)]
    fn perform_maintenance(
        sessions: &Arc<RwLock<HashMap<u64, RelaySession>>>,
        initiator_sessions: &Arc<RwLock<HashMap<String, HashSet<u64>>>>,
        target_sessions: &Arc<RwLock<HashMap<String, HashSet<u64>>>>,
        stats: &Arc<RwLock<RelayStats>>,
        limits: &ResourceLimits,
        start_time: SystemTime
    ) {
        debug!("Performing relay service maintenance");
        
        // Update statistics
        {
            let mut stats_guard = stats.write().unwrap();
            
            // Update uptime
            stats_guard.calculate_uptime(start_time);
            
            // Update active sessions count
            let session_count = sessions.read().unwrap().len();
            stats_guard.active_sessions = session_count;
            
            // Update active clients count
            let initiator_count = initiator_sessions.read().unwrap().len();
            stats_guard.active_clients = initiator_count;
            
            // TODO: Add CPU/memory usage tracking
            // This would be OS-specific and require additional dependencies
        }
        
        // Clean up expired sessions
        let expired_count = {
            let all_sessions = sessions.read().unwrap();
            let inactivity_threshold = limits.session_inactivity_timeout;
            
            // Find expired sessions
            let expired_sessions: Vec<u64> = all_sessions.iter()
                .filter(|(_, session)| session.is_expired() || session.is_inactive(inactivity_threshold))
                .map(|(id, _)| *id)
                .collect();
            
            // Get a count for stats
            let count = expired_sessions.len();
            
            // Remove each expired session
            for session_id in expired_sessions {
                // Get session details for map cleanup
                if let Some(session) = all_sessions.get(&session_id) {
                    let initiator_id = hex::encode(&session.initiator_pubkey);
                    let target_id = hex::encode(&session.target_pubkey);
                    
                    // Remove from sessions map
                    let mut sessions_write = sessions.write().unwrap();
                    sessions_write.remove(&session_id);
                    
                    // Remove from initiator map
                    let mut initiator_map = initiator_sessions.write().unwrap();
                    if let Some(sessions) = initiator_map.get_mut(&initiator_id) {
                        sessions.remove(&session_id);
                        // Clean up empty sets
                        if sessions.is_empty() {
                            initiator_map.remove(&initiator_id);
                        }
                    }
                    
                    // Remove from target map
                    let mut target_map = target_sessions.write().unwrap();
                    if let Some(sessions) = target_map.get_mut(&target_id) {
                        sessions.remove(&session_id);
                        // Clean up empty sets
                        if sessions.is_empty() {
                            target_map.remove(&target_id);
                        }
                    }
                }
            }
            
            count
        };
        
        // Update stats with expired session count
        {
            let mut stats_write = stats.write().unwrap();
            stats_write.expired_sessions += expired_count as u64;
            stats_write.active_sessions = {
                let sessions_read = sessions.read().unwrap();
                sessions_read.len()
            };
            
            // Update active clients count
            let initiator_map = initiator_sessions.read().unwrap();
            stats_write.active_clients = initiator_map.len();
            
            // Update uptime
            stats_write.calculate_uptime(start_time);
        }
    }
    
    /// Process a relay packet
    fn process_relay_packet(
        socket: &Arc<UdpSocket>,
        packet: RelayPacket,
        src_addr: SocketAddr,
        sessions: &Arc<RwLock<HashMap<u64, RelaySession>>>,
        stats: &Arc<RwLock<RelayStats>>
    ) -> Result<()> {
        // Find the session for this packet
        let result = {
            let sessions_guard = sessions.read().unwrap();
            let session = sessions_guard.get(&packet.header.session_id);
            
            if let Some(session) = session {
                // Authenticate the packet against the session
                if !session.authenticate_packet(&packet) {
                    debug!("Packet authentication failed for session {}", packet.header.session_id);
                    return Err(RelayError::Authentication("Packet authentication failed".to_string()));
                }
                
                // Determine which direction this packet is going
                let is_from_initiator = session.initiator_pubkey != packet.header.dest_peer_id;
                
                // Get the destination address
                let dest_addr = if is_from_initiator {
                    if let Some(addr) = session.target_addr {
                        addr
                    } else {
                        return Err(RelayError::Protocol("Target address not yet known".to_string()));
                    }
                } else {
                    if let Some(addr) = session.initiator_addr {
                        addr
                    } else {
                        return Err(RelayError::Protocol("Initiator address not yet known".to_string()));
                    }
                };
                
                // Record statistics
                let size = packet.payload.len();
                if is_from_initiator {
                    let mut session = session.clone();
                    session.record_initiator_to_target(size);
                    
                    // Update source address if changed
                    session.update_initiator_addr(src_addr);
                    
                    // Update session in map
                    drop(sessions_guard);
                    let mut sessions_write = sessions.write().unwrap();
                    sessions_write.insert(packet.header.session_id, session);
                } else {
                    let mut session = session.clone();
                    session.record_target_to_initiator(size);
                    
                    // Update source address if changed
                    session.update_target_addr(src_addr);
                    
                    // Update session in map
                    drop(sessions_guard);
                    let mut sessions_write = sessions.write().unwrap();
                    sessions_write.insert(packet.header.session_id, session);
                }
                
                // Forward the packet
                Ok((dest_addr, packet.payload.clone()))
            } else {
                Err(RelayError::Protocol(format!("Session {} not found", packet.header.session_id)))
            }
        };
        
        // Forward the packet if a valid session was found
        match result {
            Ok((dest_addr, payload)) => {
                // Send the payload to the destination
                if let Err(e) = socket.send_to(&payload, dest_addr) {
                    return Err(RelayError::Io(e));
                }
                
                // Update stats
                {
                    let mut stats_guard = stats.write().unwrap();
                    stats_guard.packets_forwarded += 1;
                    stats_guard.bytes_forwarded += payload.len() as u64;
                    stats_guard.record_forwarded_packet(payload.len());
                }
                
                Ok(())
            },
            Err(e) => Err(e),
        }
    }
    
    /// Process a connection request
    #[allow(clippy::too_many_arguments)]
    fn process_connection_request(
        socket: &Arc<UdpSocket>,
        request: ConnectionRequest,
        src_addr: SocketAddr,
        sessions: &Arc<RwLock<HashMap<u64, RelaySession>>>,
        initiator_sessions: &Arc<RwLock<HashMap<String, HashSet<u64>>>>,
        target_sessions: &Arc<RwLock<HashMap<String, HashSet<u64>>>>,
        connection_attempts: &Arc<Mutex<Vec<Instant>>>,
        stats: &Arc<RwLock<RelayStats>>,
        config: &RelayConfig
    ) -> Result<()> {
        // Update statistics
        {
            let mut stats_guard = stats.write().unwrap();
            stats_guard.connection_requests += 1;
        }
        
        // Validate the request
        if !request.is_valid() {
            debug!("Rejecting invalid connection request");
            
            let response = ConnectionResponse::error(
                request.nonce,
                ConnectionStatus::Rejected,
                "Invalid request"
            );
            
            Self::send_response(socket, response, src_addr)?;
            return Ok(());
        }
        
        // Check rate limits for connection attempts
        {
            let mut attempts = connection_attempts.lock().unwrap();
            let now = Instant::now();
            
            // Remove attempts older than 1 minute
            attempts.retain(|time| now.duration_since(*time) < Duration::from_secs(60));
            
            // Check if we're exceeding the rate limit
            if attempts.len() >= config.limits.max_connection_rate {
                debug!("Rejecting connection request due to rate limiting");
                
                let response = ConnectionResponse::error(
                    request.nonce,
                    ConnectionStatus::ResourceLimit,
                    "Rate limit exceeded"
                );
                
                Self::send_response(socket, response, src_addr)?;
                
                // Update stats
                let mut stats_guard = stats.write().unwrap();
                stats_guard.rejected_connections += 1;
                
                return Ok(());
            }
            
            // Record this attempt
            attempts.push(now);
        }
        
        // Enhanced authentication check - validate the auth token if provided
        if let Some(auth_token) = &request.auth_token {
            // In a real implementation, we'd validate the token using cryptographic verification
            // For the implementation task, we'll use a simplified approach first
            let token_valid = if auth_token.len() > 8 {
                // Basic validation - in practice, we'd use proper signature verification
                let timestamp_bytes = &auth_token[0..8];
                let mut timestamp_array = [0u8; 8];
                timestamp_bytes.iter().enumerate().for_each(|(i, &b)| {
                    if i < 8 {
                        timestamp_array[i] = b;
                    }
                });
                
                let timestamp = u64::from_le_bytes(timestamp_array);
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                
                // Token should not be too old (5 minutes max)
                now.saturating_sub(timestamp) < 300
            } else {
                false
            };
            
            if !token_valid {
                let response = ConnectionResponse::error(
                    request.nonce,
                    ConnectionStatus::AuthFailed,
                    "Invalid authentication token"
                );
                
                // Send the error response
                Self::send_response(socket, response, src_addr)?;
                
                // Update stats
                {
                    let mut stats = stats.write().unwrap();
                    stats.rejected_connections += 1;
                }
                
                return Ok(());
            }
        }
        
        // Continue with normal session creation process...
        
        // Generate a unique session ID
        let session_id = Self::generate_session_id();
        
        // Create a new session
        let session = RelaySession::new(session_id, request.peer_pubkey, request.target_pubkey);
        
        // Add the session to our maps
        {
            // Add to main sessions map
            let mut sessions_map = sessions.write().unwrap();
            sessions_map.insert(session_id, session);
            
            // Add to initiator sessions map
            let initiator_id = hex::encode(&request.peer_pubkey);
            let mut initiator_map = initiator_sessions.write().unwrap();
            let entry = initiator_map.entry(initiator_id).or_insert_with(HashSet::new);
            entry.insert(session_id);
            
            // Add to target sessions map
            let target_id = hex::encode(&request.target_pubkey);
            let mut target_map = target_sessions.write().unwrap();
            let entry = target_map.entry(target_id).or_insert_with(HashSet::new);
            entry.insert(session_id);
        }
        
        // Update stats
        {
            let mut stats = stats.write().unwrap();
            stats.successful_connections += 1;
            stats.active_sessions = {
                let sessions_map = sessions.read().unwrap();
                sessions_map.len()
            };
            
            // Update active clients count
            let initiator_map = initiator_sessions.read().unwrap();
            stats.active_clients = initiator_map.len();
        }
        
        // Send success response with the session ID
        let response = ConnectionResponse::success(request.nonce, session_id);
        Self::send_response(socket, response, src_addr)?;
        
        Ok(())
    }
    
    /// Process a heartbeat message to keep a session alive
    fn process_heartbeat(
        socket: &Arc<UdpSocket>,
        heartbeat: Heartbeat,
        sessions: &Arc<RwLock<HashMap<u64, RelaySession>>>,
        stats: &Arc<RwLock<RelayStats>>
    ) -> Result<()> {
        let session_id = heartbeat.session_id;
        
        // Extend the session expiration
        let session_found = {
            let mut sessions_guard = sessions.write().unwrap();
            
            if let Some(session) = sessions_guard.get_mut(&session_id) {
                // Update activity
                session.update_activity();
                
                // Extend expiration
                session.extend_expiration(DEFAULT_SESSION_EXPIRATION);
                
                true
            } else {
                false
            }
        };
        
        if !session_found {
            return Err(RelayError::Protocol(format!("Unknown session ID: {}", session_id)));
        }
        
        // Update heartbeat statistics
        {
            let mut stats_guard = stats.write().unwrap();
            stats_guard.heartbeats_processed += 1;
        }
        
        // Send back an acknowledgment if needed
        // For now, we just log the heartbeat
        debug!("Processed heartbeat for session {}, sequence {}", 
            heartbeat.session_id, heartbeat.sequence);
        
        Ok(())
    }
    
    /// Process a discovery query
    fn process_discovery_query(
        socket: &Arc<UdpSocket>,
        query: DiscoveryQuery,
        src_addr: SocketAddr,
        stats: &Arc<RwLock<RelayStats>>,
        config: &RelayConfig
    ) -> Result<()> {
        // Validate the query
        if !query.is_valid() {
            debug!("Ignoring invalid discovery query");
            return Ok(());
        }
        
        // Check if we match the required capabilities
        if (config.capabilities & query.min_capabilities) != query.min_capabilities {
            debug!("Ignoring discovery query: capabilities mismatch");
            return Ok(());
        }
        
        // Check if we match the requested region
        if let Some(ref region) = query.region {
            if let Some(ref our_region) = config.region {
                if region != our_region {
                    debug!("Ignoring discovery query: region mismatch");
                    return Ok(());
                }
            } else {
                // Query wants a specific region but we don't have one set
                debug!("Ignoring discovery query: we have no region");
                return Ok(());
            }
        }
        
        // Create node info to send in response
        let stats_guard = stats.read().unwrap();
        
        let mut node_info = RelayNodeInfo {
            pubkey: config.pubkey,
            endpoints: vec![config.listen_addr.to_string()],
            region: config.region.clone(),
            capabilities: config.capabilities,
            load: std::cmp::min(
                ((stats_guard.active_sessions as f32 / config.limits.max_total_sessions as f32) * 100.0) as u8,
                100
            ),
            latency: None,
            max_sessions: config.limits.max_total_sessions as u32,
            protocol_version: 1,
        };
        
        drop(stats_guard);
        
        // Create the response
        let response = DiscoveryResponse {
            request_nonce: query.nonce,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            relays: vec![node_info],
            more_available: false,
        };
        
        // Send the response
        let response_data = bincode::serialize(&response)
            .map_err(|e| RelayError::Serialization(e))?;
            
        socket.send_to(&response_data, src_addr)
            .map_err(|e| RelayError::Io(e))?;
        
        debug!("Sent discovery response to {:?}", src_addr);
        
        Ok(())
    }
    
    /// Generate a unique session ID
    fn generate_session_id() -> u64 {
        let mut rng = rand::thread_rng();
        rng.gen::<u64>()
    }
    
    /// Send a connection response back to the client
    fn send_response(
        socket: &Arc<UdpSocket>,
        response: ConnectionResponse,
        dest_addr: SocketAddr
    ) -> Result<()> {
        let response_data = bincode::serialize(&response)
            .map_err(|e| RelayError::Serialization(e))?;
            
        socket.send_to(&response_data, dest_addr)
            .map_err(|e| RelayError::Io(e))?;
            
        Ok(())
    }
    
    /// Export relay metrics in prometheus format
    pub fn metrics(&self) -> String {
        let stats = self.stats.read().unwrap();
        
        let mut output = String::new();
        
        // Header with help text
        output.push_str("# HELP formnet_relay_connections Total number of relay connections\n");
        output.push_str("# TYPE formnet_relay_connections counter\n");
        output.push_str(&format!("formnet_relay_connections {}\n", stats.successful_connections));
        
        output.push_str("# HELP formnet_relay_active_sessions Current number of active relay sessions\n");
        output.push_str("# TYPE formnet_relay_active_sessions gauge\n");
        output.push_str(&format!("formnet_relay_active_sessions {}\n", stats.active_sessions));
        
        output.push_str("# HELP formnet_relay_packets_forwarded Total packets forwarded by relay\n");
        output.push_str("# TYPE formnet_relay_packets_forwarded counter\n");
        output.push_str(&format!("formnet_relay_packets_forwarded {}\n", stats.packets_forwarded));
        
        output.push_str("# HELP formnet_relay_bytes_forwarded Total bytes forwarded by relay\n");
        output.push_str("# TYPE formnet_relay_bytes_forwarded counter\n");
        output.push_str(&format!("formnet_relay_bytes_forwarded {}\n", stats.bytes_forwarded));
        
        output.push_str("# HELP formnet_relay_connection_requests Total connection requests received\n");
        output.push_str("# TYPE formnet_relay_connection_requests counter\n");
        output.push_str(&format!("formnet_relay_connection_requests {}\n", stats.connection_requests));
        
        output.push_str("# HELP formnet_relay_rejected_connections Total connection requests rejected\n");
        output.push_str("# TYPE formnet_relay_rejected_connections counter\n");
        output.push_str(&format!("formnet_relay_rejected_connections {}\n", stats.rejected_connections));
        
        output.push_str("# HELP formnet_relay_bandwidth_bps Current bandwidth usage in bits per second\n");
        output.push_str("# TYPE formnet_relay_bandwidth_bps gauge\n");
        output.push_str(&format!("formnet_relay_bandwidth_bps {}\n", stats.current_bandwidth_bps));
        
        output.push_str("# HELP formnet_relay_peak_bandwidth_bps Peak bandwidth usage in bits per second\n");
        output.push_str("# TYPE formnet_relay_peak_bandwidth_bps gauge\n");
        output.push_str(&format!("formnet_relay_peak_bandwidth_bps {}\n", stats.peak_bandwidth_bps));
        
        output.push_str("# HELP formnet_relay_uptime_seconds Relay service uptime in seconds\n");
        output.push_str("# TYPE formnet_relay_uptime_seconds counter\n");
        output.push_str(&format!("formnet_relay_uptime_seconds {}\n", stats.uptime_seconds));
        
        output
    }
    
    /// Adds a new session to the relay
    pub fn create_session(&self, initiator_pubkey: [u8; 32], target_pubkey: [u8; 32]) -> Result<u64> {
        // Check if we've reached the maximum number of sessions
        {
            let sessions = self.sessions.read().unwrap();
            if sessions.len() >= self.config.limits.max_total_sessions {
                return Err(RelayError::ResourceLimit(
                    "Maximum number of total sessions reached".to_string()
                ));
            }
        }
        
        // Check if initiator has reached their session limit
        {
            let initiator_id = hex::encode(&initiator_pubkey);
            let initiator_map = self.initiator_sessions.read().unwrap();
            
            if let Some(sessions) = initiator_map.get(&initiator_id) {
                if sessions.len() >= self.config.limits.max_sessions_per_client {
                    return Err(RelayError::ResourceLimit(
                        "Maximum number of sessions per client reached".to_string()
                    ));
                }
            }
        }
        
        // Generate a unique session ID
        let session_id = Self::generate_session_id();
        
        // Create the new session
        let session = RelaySession::new(session_id, initiator_pubkey, target_pubkey);
        
        // Add to sessions map
        {
            let mut sessions = self.sessions.write().unwrap();
            sessions.insert(session_id, session);
        }
        
        // Add to initiator and target maps
        {
            let initiator_id = hex::encode(&initiator_pubkey);
            let target_id = hex::encode(&target_pubkey);
            
            // Update initiator map
            {
                let mut initiator_map = self.initiator_sessions.write().unwrap();
                let entry = initiator_map.entry(initiator_id).or_insert_with(HashSet::new);
                entry.insert(session_id);
            }
            
            // Update target map
            {
                let mut target_map = self.target_sessions.write().unwrap();
                let entry = target_map.entry(target_id).or_insert_with(HashSet::new);
                entry.insert(session_id);
            }
        }
        
        // Update stats
        {
            let mut stats = self.stats.write().unwrap();
            stats.active_sessions = {
                let sessions = self.sessions.read().unwrap();
                sessions.len()
            };
        }
        
        Ok(session_id)
    }
    
    /// Closes and removes a session
    pub fn remove_session(&self, session_id: u64) -> Result<()> {
        // Retrieve session information first
        let (initiator_pubkey, target_pubkey) = {
            let sessions = self.sessions.read().unwrap();
            let session = match sessions.get(&session_id) {
                Some(s) => s,
                None => return Err(RelayError::Protocol(format!("Session {} not found", session_id))),
            };
            
            (session.initiator_pubkey, session.target_pubkey)
        };
        
        // Compute IDs for maps
        let initiator_id = hex::encode(&initiator_pubkey);
        let target_id = hex::encode(&target_pubkey);
        
        // Remove from sessions map
        {
            let mut sessions = self.sessions.write().unwrap();
            sessions.remove(&session_id);
        }
        
        // Remove from initiator map
        {
            let mut initiator_map = self.initiator_sessions.write().unwrap();
            if let Some(sessions) = initiator_map.get_mut(&initiator_id) {
                sessions.remove(&session_id);
                
                // Clean up empty sets
                if sessions.is_empty() {
                    initiator_map.remove(&initiator_id);
                }
            }
        }
        
        // Remove from target map
        {
            let mut target_map = self.target_sessions.write().unwrap();
            if let Some(sessions) = target_map.get_mut(&target_id) {
                sessions.remove(&session_id);
                
                // Clean up empty sets
                if sessions.is_empty() {
                    target_map.remove(&target_id);
                }
            }
        }
        
        // Update stats
        {
            let mut stats = self.stats.write().unwrap();
            stats.active_sessions = {
                let sessions = self.sessions.read().unwrap();
                sessions.len()
            };
        }
        
        Ok(())
    }
    
    /// Find a session by ID and verify it's valid
    pub fn get_session(&self, session_id: u64) -> Option<RelaySession> {
        let sessions = self.sessions.read().unwrap();
        sessions.get(&session_id).cloned()
    }
    
    /// Find all sessions for a given public key (as either initiator or target)
    pub fn find_sessions_for_pubkey(&self, pubkey: &[u8; 32]) -> Vec<u64> {
        let peer_id = hex::encode(pubkey);
        let mut result = Vec::new();
        
        // Check initiator sessions
        {
            let initiator_map = self.initiator_sessions.read().unwrap();
            if let Some(sessions) = initiator_map.get(&peer_id) {
                result.extend(sessions);
            }
        }
        
        // Check target sessions
        {
            let target_map = self.target_sessions.read().unwrap();
            if let Some(sessions) = target_map.get(&peer_id) {
                for session_id in sessions {
                    if !result.contains(session_id) {
                        result.push(*session_id);
                    }
                }
            }
        }
        
        result
    }
    
    /// Extend the expiration of a session
    pub fn extend_session(&self, session_id: u64, duration: Duration) -> Result<()> {
        let mut sessions = self.sessions.write().unwrap();
        
        if let Some(session) = sessions.get_mut(&session_id) {
            session.extend_expiration(duration);
            Ok(())
        } else {
            Err(RelayError::Protocol(format!("Session {} not found", session_id)))
        }
    }
    
    /// Update session statistics when forwarding a packet
    pub fn update_session_stats(&self, session_id: u64, bytes: usize, is_initiator_to_target: bool) -> Result<()> {
        let mut sessions = self.sessions.write().unwrap();
        
        if let Some(session) = sessions.get_mut(&session_id) {
            if is_initiator_to_target {
                session.record_initiator_to_target(bytes);
            } else {
                session.record_target_to_initiator(bytes);
            }
            Ok(())
        } else {
            Err(RelayError::Protocol(format!("Session {} not found", session_id)))
        }
    }
    
    /// Get all expired or inactive sessions
    fn get_expired_sessions(&self) -> Vec<u64> {
        let sessions = self.sessions.read().unwrap();
        let inactivity_threshold = self.config.limits.session_inactivity_timeout;
        
        sessions.iter()
            .filter(|(_, session)| session.is_expired() || session.is_inactive(inactivity_threshold))
            .map(|(id, _)| *id)
            .collect()
    }
    
    /// Clean up expired sessions
    fn cleanup_expired_sessions(&self) -> usize {
        let expired_sessions = self.get_expired_sessions();
        let count = expired_sessions.len();
        
        for session_id in expired_sessions {
            let _ = self.remove_session(session_id);
        }
        
        // Update stats
        {
            let mut stats = self.stats.write().unwrap();
            stats.expired_sessions += count as u64;
        }
        
        count
    }
}

/// RelayService is a thin wrapper around RelayNode
/// 
/// This type exists to maintain API consistency while using the RelayNode implementation
/// for the actual relay service functionality. A separate service wrapper was considered
/// in the implementation plan, but RelayNode already implements all necessary functionality.
pub type RelayService = RelayNode;

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};
    
    /// Create a test relay config
    fn create_test_config() -> RelayConfig {
        let listen_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);
        let pubkey = [0u8; 32]; // All zeros for testing
        
        RelayConfig::new(listen_addr, pubkey)
    }
    
    #[test]
    fn test_relay_node_creation() {
        let config = create_test_config();
        let node = RelayNode::new(config);
        
        let stats = node.get_stats();
        assert_eq!(stats.active_sessions, 0);
        assert_eq!(stats.packets_forwarded, 0);
    }
    
    #[test]
    fn test_relay_packet_time_tracking() {
        let packet_times = Arc::new(Mutex::new(Vec::new()));
        let limits = ResourceLimits::default();
        
        // Should allow packets up to the limit
        for _ in 0..limits.max_packets_per_second {
            assert!(RelayNode::record_packet_time(&packet_times, &limits));
        }
        
        // Should reject after reaching the limit
        assert!(!RelayNode::record_packet_time(&packet_times, &limits));
    }
    
    #[test]
    fn test_session_authentication() {
        use super::*;
        use rand::RngCore;
        
        // Create session
        let mut rng = rand::thread_rng();
        let session_id = rng.next_u64();
        let mut initiator_pubkey = [0u8; 32];
        let mut target_pubkey = [0u8; 32];
        rng.fill_bytes(&mut initiator_pubkey);
        rng.fill_bytes(&mut target_pubkey);
        
        let session = RelaySession::new(session_id, initiator_pubkey, target_pubkey);
        
        // Create relay packet with valid session
        let header = RelayHeader::new(target_pubkey, session_id);
        let payload = vec![1, 2, 3, 4];
        let packet = RelayPacket {
            header,
            payload,
        };
        
        // Test authentication for valid packet
        assert!(session.authenticate_packet(&packet));
        
        // Test authentication fails with wrong session ID
        let header_wrong_id = RelayHeader::new(target_pubkey, session_id + 1);
        let packet_wrong_id = RelayPacket {
            header: header_wrong_id,
            payload: payload.clone(),
        };
        assert!(!session.authenticate_packet(&packet_wrong_id));
        
        // Test authentication fails with wrong peer ID
        let mut wrong_pubkey = [0u8; 32];
        rng.fill_bytes(&mut wrong_pubkey);
        let header_wrong_peer = RelayHeader::new(wrong_pubkey, session_id);
        let packet_wrong_peer = RelayPacket {
            header: header_wrong_peer,
            payload: payload.clone(),
        };
        assert!(!session.authenticate_packet(&packet_wrong_peer));
    }
    
    #[test]
    fn test_session_auth_token() {
        use super::*;
        
        // Create session
        let session_id = 12345;
        let initiator_pubkey = [1u8; 32];
        let target_pubkey = [2u8; 32];
        
        let session = RelaySession::new(session_id, initiator_pubkey, target_pubkey);
        
        // Generate token
        let token = session.generate_auth_token();
        
        // Verify token is valid
        assert!(session.verify_auth_token(&token));
        
        // Verify modified token is invalid
        if !token.is_empty() {
            let mut invalid_token = token.clone();
            invalid_token[0] ^= 0xFF;
            assert!(!session.verify_auth_token(&invalid_token));
        }
    }
    
    #[test]
    fn test_relay_session_management() {
        use super::*;
        use std::thread;
        use std::net::{IpAddr, Ipv4Addr};
        
        // Create a test config
        let config = create_test_config();
        
        // Initialize a relay node
        let mut relay = RelayNode::new(config);
        
        // Generate test keys
        let initiator_pubkey = [1u8; 32];
        let target_pubkey = [2u8; 32];
        
        // Create a session
        let session_id = relay.create_session(initiator_pubkey, target_pubkey).unwrap();
        assert!(session_id > 0, "Session ID should be positive");
        
        // Verify session exists
        let session = relay.get_session(session_id);
        assert!(session.is_some(), "Session should exist");
        let session = session.unwrap();
        assert_eq!(session.initiator_pubkey, initiator_pubkey);
        assert_eq!(session.target_pubkey, target_pubkey);
        
        // Find sessions by pubkey
        let initiator_sessions = relay.find_sessions_for_pubkey(&initiator_pubkey);
        assert_eq!(initiator_sessions.len(), 1, "Should find one session for initiator");
        assert_eq!(initiator_sessions[0], session_id, "Should find correct session ID");
        
        let target_sessions = relay.find_sessions_for_pubkey(&target_pubkey);
        assert_eq!(target_sessions.len(), 1, "Should find one session for target");
        assert_eq!(target_sessions[0], session_id, "Should find correct session ID");
        
        // Update session stats
        let bytes = 1024;
        relay.update_session_stats(session_id, bytes, true).unwrap();
        
        // Check statistics were updated
        let updated_session = relay.get_session(session_id).unwrap();
        assert_eq!(updated_session.packets_forwarded_initiator_to_target, 1, 
            "Should have recorded one forwarded packet");
        assert_eq!(updated_session.bytes_forwarded_initiator_to_target, bytes as u64, 
            "Should have recorded correct byte count");
        
        // Test session expiration
        // First, create a session that will expire quickly
        let temp_session_id = relay.create_session(initiator_pubkey, target_pubkey).unwrap();
        
        // Set a short inactivity timeout in the config
        relay.config.limits.session_inactivity_timeout = Duration::from_millis(10);
        
        // Wait for the session to become inactive
        thread::sleep(Duration::from_millis(20));
        
        // Cleanup expired sessions
        let cleaned = relay.cleanup_expired_sessions();
        assert_eq!(cleaned, 1, "Should have cleaned up one session");
        
        // Verify the session is gone
        let temp_session = relay.get_session(temp_session_id);
        assert!(temp_session.is_none(), "Temporary session should be removed");
        
        // But the original session should still be there
        let original_session = relay.get_session(session_id);
        assert!(original_session.is_some(), "Original session should still exist");
        
        // Finally, remove the original session
        relay.remove_session(session_id).unwrap();
        
        // Verify it's gone
        let removed_session = relay.get_session(session_id);
        assert!(removed_session.is_none(), "Session should be removed");
    }
} 