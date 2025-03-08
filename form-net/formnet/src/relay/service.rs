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
        
        // Cleanup expired or inactive sessions
        let expired_sessions = {
            let mut sessions_guard = sessions.write().unwrap();
            let mut expired = 0;
            
            // Collect IDs of expired or inactive sessions
            let expired_ids: Vec<u64> = sessions_guard
                .iter()
                .filter(|(_, session)| {
                    session.is_expired() || 
                    session.is_inactive(limits.session_inactivity_timeout)
                })
                .map(|(&id, _)| id)
                .collect();
            
            // Remove expired sessions
            for id in expired_ids {
                if let Some(session) = sessions_guard.remove(&id) {
                    expired += 1;
                    
                    // Also remove from initiator and target mappings
                    let initiator_key = hex::encode(&session.initiator_pubkey);
                    let target_key = hex::encode(&session.target_pubkey);
                    
                    if let Ok(mut initiator_map) = initiator_sessions.write() {
                        if let Some(sessions) = initiator_map.get_mut(&initiator_key) {
                            sessions.remove(&id);
                            if sessions.is_empty() {
                                initiator_map.remove(&initiator_key);
                            }
                        }
                    }
                    
                    if let Ok(mut target_map) = target_sessions.write() {
                        if let Some(sessions) = target_map.get_mut(&target_key) {
                            sessions.remove(&id);
                            if sessions.is_empty() {
                                target_map.remove(&target_key);
                            }
                        }
                    }
                }
            }
            
            expired
        };
        
        if expired_sessions > 0 {
            info!("Cleaned up {} expired relay sessions", expired_sessions);
            
            // Update stats
            let mut stats_guard = stats.write().unwrap();
            stats_guard.expired_sessions += expired_sessions as u64;
        }
    }
    
    /// Process a relay packet (forward from one peer to another)
    fn process_relay_packet(
        socket: &Arc<UdpSocket>,
        packet: RelayPacket,
        src_addr: SocketAddr,
        sessions: &Arc<RwLock<HashMap<u64, RelaySession>>>,
        stats: &Arc<RwLock<RelayStats>>
    ) -> Result<()> {
        // Validate the relay header
        if !packet.header.is_valid() {
            return Err(RelayError::Protocol("Invalid relay header".into()));
        }
        
        // Lookup the session
        let session_id = packet.header.session_id;
        let mut sessions_guard = sessions.write().unwrap();
        
        let session = match sessions_guard.get_mut(&session_id) {
            Some(session) => session,
            None => return Err(RelayError::Protocol(format!("Unknown session ID: {}", session_id))),
        };
        
        // Figure out the direction of the packet
        let initiator_pubkey_hex = hex::encode(&session.initiator_pubkey);
        let target_pubkey_hex = hex::encode(&session.target_pubkey);
        let dest_pubkey_hex = hex::encode(&packet.header.dest_peer_id);
        
        let (dest_addr, is_initiator_to_target) = if dest_pubkey_hex == target_pubkey_hex {
            // Packet is from initiator to target
            (session.target_addr, true)
        } else if dest_pubkey_hex == initiator_pubkey_hex {
            // Packet is from target to initiator
            (session.initiator_addr, false)
        } else {
            // Destination doesn't match either end of the session
            return Err(RelayError::Protocol("Destination doesn't match session peers".into()));
        };
        
        // Update session information based on source
        if is_initiator_to_target {
            // Update initiator address if needed
            if session.initiator_addr != Some(src_addr) {
                debug!("Updating initiator address for session {}: {:?}", session_id, src_addr);
                session.update_initiator_addr(src_addr);
            }
        } else {
            // Update target address if needed
            if session.target_addr != Some(src_addr) {
                debug!("Updating target address for session {}: {:?}", session_id, src_addr);
                session.update_target_addr(src_addr);
            }
        }
        
        // Make sure we have a destination address to forward to
        let dest_addr = match dest_addr {
            Some(addr) => addr,
            None => {
                // We don't know the destination address yet
                debug!("Cannot forward packet: unknown destination address for session {}", session_id);
                return Ok(());
            }
        };
        
        // Forward the packet
        let packet_data = bincode::serialize(&packet)
            .map_err(|e| RelayError::Serialization(e))?;
        
        socket.send_to(&packet_data, dest_addr)
            .map_err(|e| RelayError::Io(e))?;
        
        // Update session statistics
        if is_initiator_to_target {
            session.record_initiator_to_target(packet.payload.len());
        } else {
            session.record_target_to_initiator(packet.payload.len());
        }
        
        // Update global statistics
        {
            let mut stats_guard = stats.write().unwrap();
            stats_guard.record_forwarded_packet(packet.payload.len());
        }
        
        Ok(())
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
        
        // Check if the initiator has too many sessions
        let initiator_pubkey_hex = hex::encode(&request.peer_pubkey);
        let target_pubkey_hex = hex::encode(&request.target_pubkey);
        
        {
            let initiator_sessions_guard = initiator_sessions.read().unwrap();
            if let Some(sessions) = initiator_sessions_guard.get(&initiator_pubkey_hex) {
                if sessions.len() >= config.limits.max_sessions_per_client {
                    debug!("Rejecting connection request: too many sessions for initiator");
                    
                    let response = ConnectionResponse::error(
                        request.nonce,
                        ConnectionStatus::ResourceLimit,
                        "Too many sessions"
                    );
                    
                    Self::send_response(socket, response, src_addr)?;
                    
                    // Update stats
                    let mut stats_guard = stats.write().unwrap();
                    stats_guard.rejected_connections += 1;
                    
                    return Ok(());
                }
            }
        }
        
        // Check if we've reached the global session limit
        {
            let sessions_guard = sessions.read().unwrap();
            if sessions_guard.len() >= config.limits.max_total_sessions {
                debug!("Rejecting connection request: global session limit reached");
                
                let response = ConnectionResponse::error(
                    request.nonce,
                    ConnectionStatus::ResourceLimit,
                    "Relay at capacity"
                );
                
                Self::send_response(socket, response, src_addr)?;
                
                // Update stats
                let mut stats_guard = stats.write().unwrap();
                stats_guard.rejected_connections += 1;
                
                return Ok(());
            }
        }
        
        // All checks passed, create a new session
        let session_id = Self::generate_session_id();
        let session = RelaySession::new(session_id, request.peer_pubkey, request.target_pubkey);
        
        // Store initiator's address
        let mut session = session;
        session.update_initiator_addr(src_addr);
        
        // Add session to our maps
        {
            let mut sessions_guard = sessions.write().unwrap();
            sessions_guard.insert(session_id, session);
            
            // Update the initiator -> session mapping
            let mut initiator_sessions_guard = initiator_sessions.write().unwrap();
            initiator_sessions_guard
                .entry(initiator_pubkey_hex.clone())
                .or_insert_with(HashSet::new)
                .insert(session_id);
            
            // Update the target -> session mapping
            let mut target_sessions_guard = target_sessions.write().unwrap();
            target_sessions_guard
                .entry(target_pubkey_hex.clone())
                .or_insert_with(HashSet::new)
                .insert(session_id);
        }
        
        // Send success response
        let response = ConnectionResponse::success(request.nonce, session_id);
        Self::send_response(socket, response, src_addr)?;
        
        // Update statistics
        {
            let mut stats_guard = stats.write().unwrap();
            stats_guard.successful_connections += 1;
        }
        
        info!("Created new relay session {} for peer {} to target {}",
            session_id, initiator_pubkey_hex, target_pubkey_hex);
        
        Ok(())
    }
    
    /// Process a heartbeat message
    fn process_heartbeat(
        socket: &Arc<UdpSocket>,
        heartbeat: Heartbeat,
        sessions: &Arc<RwLock<HashMap<u64, RelaySession>>>,
        stats: &Arc<RwLock<RelayStats>>
    ) -> Result<()> {
        // Update statistics
        {
            let mut stats_guard = stats.write().unwrap();
            stats_guard.heartbeats_processed += 1;
        }
        
        // Get the session
        let session_id = heartbeat.session_id;
        let mut sessions_guard = sessions.write().unwrap();
        
        let session = match sessions_guard.get_mut(&session_id) {
            Some(session) => session,
            None => {
                debug!("Heartbeat for unknown session: {}", session_id);
                return Ok(());
            }
        };
        
        // Update session activity and extend expiration
        session.update_activity();
        session.extend_expiration(DEFAULT_SESSION_EXPIRATION);
        
        debug!("Processed heartbeat for session {}", session_id);
        
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
}

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
} 