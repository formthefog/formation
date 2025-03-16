// form-fuzzing/src/generators/routing.rs
//! Generators for BGP/Anycast routing fuzzing

use crate::generators::Generator;
use rand::{Rng, distributions::Alphanumeric, thread_rng, seq::SliceRandom};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::time::{SystemTime, UNIX_EPOCH};
use std::collections::HashMap;
use uuid::Uuid;

/// Geographic region for IP address generation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Region {
    /// North America (US, Canada, Mexico)
    NorthAmerica,
    /// South America
    SouthAmerica,
    /// Europe
    Europe,
    /// Asia
    Asia,
    /// Africa
    Africa,
    /// Oceania (Australia, New Zealand, Pacific Islands)
    Oceania,
}

impl Region {
    /// Get all available regions
    pub fn all() -> Vec<Region> {
        vec![
            Region::NorthAmerica,
            Region::SouthAmerica,
            Region::Europe,
            Region::Asia,
            Region::Africa,
            Region::Oceania,
        ]
    }
    
    /// Get a random region
    pub fn random() -> Self {
        let regions = Self::all();
        *regions.choose(&mut thread_rng()).unwrap()
    }
    
    /// Get the name of the region
    pub fn name(&self) -> &'static str {
        match self {
            Region::NorthAmerica => "North America",
            Region::SouthAmerica => "South America",
            Region::Europe => "Europe",
            Region::Asia => "Asia",
            Region::Africa => "Africa",
            Region::Oceania => "Oceania",
        }
    }
    
    /// Get a region by name
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "North America" => Some(Region::NorthAmerica),
            "South America" => Some(Region::SouthAmerica),
            "Europe" => Some(Region::Europe),
            "Asia" => Some(Region::Asia),
            "Africa" => Some(Region::Africa),
            "Oceania" => Some(Region::Oceania),
            _ => None,
        }
    }
}

/// IP address with metadata
#[derive(Debug, Clone)]
pub struct IpAddressInfo {
    /// IP address
    pub address: IpAddr,
    /// Region where the IP is located
    pub region: Region,
    /// Health status (0.0 = unhealthy, 1.0 = completely healthy)
    pub health: f32,
    /// Latency in milliseconds
    pub latency_ms: u32,
    /// Node ID
    pub node_id: String,
    /// Last updated timestamp
    pub last_updated: u64,
}

/// Generator for IP addresses from a specific region
pub struct RegionalIpGenerator {
    /// Region to generate IPs for
    region: Region,
    /// Whether to generate IPv6 addresses (otherwise IPv4)
    ipv6: bool,
    /// Percentage of healthy IPs (0.0-1.0)
    healthy_percentage: f32,
}

impl RegionalIpGenerator {
    /// Create a new regional IP generator
    pub fn new(region: Region) -> Self {
        Self {
            region,
            ipv6: false,
            healthy_percentage: 0.8,
        }
    }
    
    /// Set whether to generate IPv6 addresses
    pub fn with_ipv6(mut self, ipv6: bool) -> Self {
        self.ipv6 = ipv6;
        self
    }
    
    /// Set the percentage of healthy IPs
    pub fn with_healthy_percentage(mut self, percentage: f32) -> Self {
        self.healthy_percentage = percentage.max(0.0).min(1.0);
        self
    }
    
    /// Generate a random IP address for a region
    fn generate_ip_for_region(&self, region: Region) -> IpAddr {
        let mut rng = thread_rng();
        
        // In a real implementation, we would use appropriate IP ranges for each region
        // For this example, we'll use simplified ranges
        if self.ipv6 {
            // Generate IPv6
            let segment = match region {
                Region::NorthAmerica => rng.gen_range(0x2001..0x2002),
                Region::SouthAmerica => rng.gen_range(0x2002..0x2003),
                Region::Europe => rng.gen_range(0x2003..0x2004),
                Region::Asia => rng.gen_range(0x2004..0x2005),
                Region::Africa => rng.gen_range(0x2005..0x2006),
                Region::Oceania => rng.gen_range(0x2006..0x2007),
            };
            
            let a = segment;
            let b = rng.gen();
            let c = rng.gen();
            let d = rng.gen();
            let e = rng.gen();
            let f = rng.gen();
            let g = rng.gen();
            let h = rng.gen();
            
            IpAddr::V6(Ipv6Addr::new(a, b, c, d, e, f, g, h))
        } else {
            // Generate IPv4
            let first_octet = match region {
                Region::NorthAmerica => rng.gen_range(50..60),
                Region::SouthAmerica => rng.gen_range(60..70),
                Region::Europe => rng.gen_range(80..90),
                Region::Asia => rng.gen_range(100..110),
                Region::Africa => rng.gen_range(150..160),
                Region::Oceania => rng.gen_range(180..190),
            };
            
            let second_octet = rng.gen();
            let third_octet = rng.gen();
            let fourth_octet = rng.gen();
            
            IpAddr::V4(Ipv4Addr::new(first_octet, second_octet, third_octet, fourth_octet))
        }
    }
    
    /// Generate a random node ID
    fn generate_node_id(&self) -> String {
        let mut rng = thread_rng();
        format!("node_{}", generate_random_hex(16))
    }
}

impl Generator<IpAddressInfo> for RegionalIpGenerator {
    fn generate(&self) -> IpAddressInfo {
        let mut rng = thread_rng();
        
        // Generate IP address
        let address = self.generate_ip_for_region(self.region);
        
        // Generate health status
        let health = if rng.gen_bool(self.healthy_percentage as f64) {
            // Healthy
            rng.gen_range(0.7..1.0)
        } else {
            // Unhealthy
            rng.gen_range(0.0..0.7)
        };
        
        // Generate latency based on health
        let latency_base = match self.region {
            Region::NorthAmerica => 20,
            Region::SouthAmerica => 40,
            Region::Europe => 30,
            Region::Asia => 50,
            Region::Africa => 70,
            Region::Oceania => 60,
        };
        
        let latency_variance = (1.0 - health) * 200.0;
        let latency_ms = latency_base + (rng.gen::<f32>() * latency_variance) as u32;
        
        // Generate node ID and timestamp
        let node_id = self.generate_node_id();
        let last_updated = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        IpAddressInfo {
            address,
            region: self.region,
            health,
            latency_ms,
            node_id,
            last_updated,
        }
    }
}

/// Generator for DNS requests with geolocation parameters
pub struct GeoDnsRequestGenerator {
    /// Include client IP
    include_client_ip: bool,
    /// Include client coordinates
    include_coordinates: bool,
    /// Include ECS (EDNS Client Subnet)
    include_ecs: bool,
}

impl GeoDnsRequestGenerator {
    /// Create a new GeoDNS request generator
    pub fn new() -> Self {
        Self {
            include_client_ip: true,
            include_coordinates: false,
            include_ecs: false,
        }
    }
    
    /// Set whether to include client IP
    pub fn with_client_ip(mut self, include: bool) -> Self {
        self.include_client_ip = include;
        self
    }
    
    /// Set whether to include client coordinates
    pub fn with_coordinates(mut self, include: bool) -> Self {
        self.include_coordinates = include;
        self
    }
    
    /// Set whether to include EDNS Client Subnet
    pub fn with_ecs(mut self, include: bool) -> Self {
        self.include_ecs = include;
        self
    }
    
    /// Generate a random domain name
    fn generate_domain(&self) -> String {
        let mut rng = thread_rng();
        let domains = [
            "bootstrap.formation.net",
            "nodes.formation.net",
            "entry.formation.net",
            "anycast.formation.net",
            "p2p.formation.net",
        ];
        
        domains[rng.gen_range(0..domains.len())].to_string()
    }
    
    /// Generate random coordinates
    fn generate_coordinates(&self) -> (f32, f32) {
        let mut rng = thread_rng();
        
        // Latitude: -90 to 90
        let latitude = rng.gen_range(-90.0..90.0);
        
        // Longitude: -180 to 180
        let longitude = rng.gen_range(-180.0..180.0);
        
        (latitude, longitude)
    }
    
    /// Generate a random IP address for client
    fn generate_client_ip(&self) -> IpAddr {
        let regional_generator = RegionalIpGenerator::new(Region::random())
            .with_ipv6(thread_rng().gen_bool(0.2));
        
        regional_generator.generate_ip_for_region(Region::random())
    }
    
    /// Generate ECS prefix
    fn generate_ecs_prefix(&self) -> u8 {
        let mut rng = thread_rng();
        rng.gen_range(16..32)
    }
}

/// DNS request with geolocation data
#[derive(Debug, Clone)]
pub struct GeoDnsRequest {
    /// Domain being requested
    pub domain: String,
    /// Client IP address
    pub client_ip: Option<IpAddr>,
    /// Client coordinates (latitude, longitude)
    pub coordinates: Option<(f32, f32)>,
    /// EDNS Client Subnet prefix
    pub ecs_prefix: Option<u8>,
    /// Request ID
    pub request_id: String,
    /// Timestamp
    pub timestamp: u64,
}

impl Generator<GeoDnsRequest> for GeoDnsRequestGenerator {
    fn generate(&self) -> GeoDnsRequest {
        let mut rng = thread_rng();
        
        // Generate domain
        let domain = self.generate_domain();
        
        // Generate client IP if enabled
        let client_ip = if self.include_client_ip {
            Some(self.generate_client_ip())
        } else {
            None
        };
        
        // Generate coordinates if enabled
        let coordinates = if self.include_coordinates {
            Some(self.generate_coordinates())
        } else {
            None
        };
        
        // Generate ECS prefix if enabled
        let ecs_prefix = if self.include_ecs {
            Some(self.generate_ecs_prefix())
        } else {
            None
        };
        
        // Generate request ID and timestamp
        let request_id = format!("req_{}", generate_random_hex(8));
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        GeoDnsRequest {
            domain,
            client_ip,
            coordinates,
            ecs_prefix,
            request_id,
            timestamp,
        }
    }
}

/// Generator for health status reports
pub struct HealthStatusGenerator {
    /// Minimum number of nodes to include
    min_nodes: usize,
    /// Maximum number of nodes to include
    max_nodes: usize,
    /// Percentage of healthy nodes (0.0-1.0)
    healthy_percentage: f32,
}

impl HealthStatusGenerator {
    /// Create a new health status generator
    pub fn new() -> Self {
        Self {
            min_nodes: 5,
            max_nodes: 20,
            healthy_percentage: 0.8,
        }
    }
    
    /// Set the minimum and maximum number of nodes
    pub fn with_node_range(mut self, min: usize, max: usize) -> Self {
        self.min_nodes = min;
        self.max_nodes = max;
        self
    }
    
    /// Set the percentage of healthy nodes
    pub fn with_healthy_percentage(mut self, percentage: f32) -> Self {
        self.healthy_percentage = percentage.max(0.0).min(1.0);
        self
    }
}

/// Health status report for a network
#[derive(Debug, Clone)]
pub struct HealthStatusReport {
    /// Node statuses, mapped by node ID
    pub nodes: HashMap<String, NodeHealth>,
    /// Report ID
    pub report_id: String,
    /// Timestamp
    pub timestamp: u64,
}

/// Health information for a single node
#[derive(Debug, Clone)]
pub struct NodeHealth {
    /// Node ID
    pub node_id: String,
    /// Node IP address
    pub address: IpAddr,
    /// Health status (0.0 = unhealthy, 1.0 = completely healthy)
    pub health: f32,
    /// Latency in milliseconds
    pub latency_ms: u32,
    /// Available bandwidth in Mbps
    pub bandwidth_mbps: u32,
    /// Connection count
    pub connections: u32,
    /// Region where the node is located
    pub region: Region,
    /// Last updated timestamp
    pub last_updated: u64,
}

impl Generator<HealthStatusReport> for HealthStatusGenerator {
    fn generate(&self) -> HealthStatusReport {
        let mut rng = thread_rng();
        
        // Determine number of nodes
        let node_count = rng.gen_range(self.min_nodes..=self.max_nodes);
        
        // Generate nodes
        let mut nodes = HashMap::new();
        for _ in 0..node_count {
            let region = Region::random();
            let ip_generator = RegionalIpGenerator::new(region)
                .with_healthy_percentage(self.healthy_percentage);
            
            let ip_info = ip_generator.generate();
            
            // Generate additional health information
            let bandwidth_mbps = if ip_info.health > 0.7 {
                // Healthy node, good bandwidth
                rng.gen_range(100..1000)
            } else {
                // Unhealthy node, poor bandwidth
                rng.gen_range(1..100)
            };
            
            let connections = if ip_info.health > 0.7 {
                // Healthy node, more connections
                rng.gen_range(10..100)
            } else {
                // Unhealthy node, fewer connections
                rng.gen_range(0..10)
            };
            
            // Create node health information
            let node_health = NodeHealth {
                node_id: ip_info.node_id.clone(),
                address: ip_info.address,
                health: ip_info.health,
                latency_ms: ip_info.latency_ms,
                bandwidth_mbps,
                connections,
                region,
                last_updated: ip_info.last_updated,
            };
            
            nodes.insert(ip_info.node_id, node_health);
        }
        
        // Generate report ID and timestamp
        let report_id = format!("health_{}", generate_random_hex(8));
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        HealthStatusReport {
            nodes,
            report_id,
            timestamp,
        }
    }
}

/// Generator for BGP announcements
pub struct BgpAnnouncementGenerator {
    /// Include multiple prefixes
    include_multiple_prefixes: bool,
    /// Include communities
    include_communities: bool,
}

impl BgpAnnouncementGenerator {
    /// Create a new BGP announcement generator
    pub fn new() -> Self {
        Self {
            include_multiple_prefixes: false,
            include_communities: true,
        }
    }
    
    /// Set whether to include multiple prefixes
    pub fn with_multiple_prefixes(mut self, include: bool) -> Self {
        self.include_multiple_prefixes = include;
        self
    }
    
    /// Set whether to include communities
    pub fn with_communities(mut self, include: bool) -> Self {
        self.include_communities = include;
        self
    }
    
    /// Generate a random IP prefix
    fn generate_prefix(&self) -> (IpAddr, u8) {
        let mut rng = thread_rng();
        let region = Region::random();
        
        let ip_generator = RegionalIpGenerator::new(region);
        let ip = ip_generator.generate_ip_for_region(region);
        
        // Generate prefix length
        let prefix_len = match ip {
            IpAddr::V4(_) => rng.gen_range(16..32),
            IpAddr::V6(_) => rng.gen_range(32..64),
        };
        
        (ip, prefix_len)
    }
    
    /// Generate random BGP AS numbers
    fn generate_as_path(&self) -> Vec<u32> {
        let mut rng = thread_rng();
        let path_length = rng.gen_range(1..5);
        
        let mut path = Vec::with_capacity(path_length);
        for _ in 0..path_length {
            // Generate AS number (avoiding reserved ranges)
            let as_num = rng.gen_range(1000..65000);
            path.push(as_num);
        }
        
        path
    }
    
    /// Generate random BGP communities
    fn generate_communities(&self) -> Vec<(u16, u16)> {
        let mut rng = thread_rng();
        let community_count = rng.gen_range(0..5);
        
        let mut communities = Vec::with_capacity(community_count);
        for _ in 0..community_count {
            let high = rng.gen::<u16>();
            let low = rng.gen::<u16>();
            communities.push((high, low));
        }
        
        communities
    }
}

/// BGP announcement for a prefix
#[derive(Debug, Clone)]
pub struct BgpAnnouncement {
    /// IP prefixes being announced
    pub prefixes: Vec<(IpAddr, u8)>,
    /// AS path
    pub as_path: Vec<u32>,
    /// Communities
    pub communities: Vec<(u16, u16)>,
    /// Next hop
    pub next_hop: IpAddr,
    /// Local preference
    pub local_pref: Option<u32>,
    /// MED (Multi-Exit Discriminator)
    pub med: Option<u32>,
    /// Whether the announcement is a withdrawal
    pub is_withdrawal: bool,
    /// Announcement ID
    pub announcement_id: String,
    /// Timestamp
    pub timestamp: u64,
}

impl Generator<BgpAnnouncement> for BgpAnnouncementGenerator {
    fn generate(&self) -> BgpAnnouncement {
        let mut rng = thread_rng();
        
        // Generate prefixes
        let mut prefixes = vec![self.generate_prefix()];
        
        if self.include_multiple_prefixes {
            // Add additional prefixes
            let additional_count = rng.gen_range(1..5);
            for _ in 0..additional_count {
                prefixes.push(self.generate_prefix());
            }
        }
        
        // Generate AS path and next hop
        let as_path = self.generate_as_path();
        let next_hop = {
            let region = Region::random();
            let ip_generator = RegionalIpGenerator::new(region);
            ip_generator.generate_ip_for_region(region)
        };
        
        // Generate communities if enabled
        let communities = if self.include_communities {
            self.generate_communities()
        } else {
            Vec::new()
        };
        
        // Generate optional attributes
        let local_pref = if rng.gen_bool(0.7) {
            Some(rng.gen_range(1..1000))
        } else {
            None
        };
        
        let med = if rng.gen_bool(0.3) {
            Some(rng.gen_range(1..1000))
        } else {
            None
        };
        
        // Determine if this is a withdrawal
        let is_withdrawal = rng.gen_bool(0.1);
        
        // Generate announcement ID and timestamp
        let announcement_id = format!("bgp_{}", generate_random_hex(8));
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        BgpAnnouncement {
            prefixes,
            as_path,
            communities,
            next_hop,
            local_pref,
            med,
            is_withdrawal,
            announcement_id,
            timestamp,
        }
    }
}

/// Generator for anycast routing tests
pub struct AnycastTestGenerator {
    /// Minimum number of requests to include
    min_requests: usize,
    /// Maximum number of requests to include
    max_requests: usize,
}

impl AnycastTestGenerator {
    /// Create a new anycast test generator
    pub fn new() -> Self {
        Self {
            min_requests: 5,
            max_requests: 20,
        }
    }
    
    /// Set the minimum and maximum number of requests
    pub fn with_request_range(mut self, min: usize, max: usize) -> Self {
        self.min_requests = min;
        self.max_requests = max;
        self
    }
}

/// Test for anycast routing
#[derive(Debug, Clone)]
pub struct AnycastTest {
    /// Client requests from different regions
    pub requests: Vec<AnycastRequest>,
    /// Expected nodes to be returned
    pub expected_nodes: HashMap<String, Vec<String>>,
    /// Test ID
    pub test_id: String,
    /// Timestamp
    pub timestamp: u64,
}

/// Client request for anycast testing
#[derive(Debug, Clone)]
pub struct AnycastRequest {
    /// Client IP address
    pub client_ip: IpAddr,
    /// Client region
    pub client_region: Region,
    /// Domain being requested
    pub domain: String,
    /// Request ID
    pub request_id: String,
}

impl Generator<AnycastTest> for AnycastTestGenerator {
    fn generate(&self) -> AnycastTest {
        let mut rng = thread_rng();
        
        // Determine number of requests
        let request_count = rng.gen_range(self.min_requests..=self.max_requests);
        
        // Generate requests
        let mut requests = Vec::with_capacity(request_count);
        let mut expected_nodes = HashMap::new();
        
        for _ in 0..request_count {
            // Generate client region and IP
            let client_region = Region::random();
            let ip_generator = RegionalIpGenerator::new(client_region);
            let client_ip = ip_generator.generate_ip_for_region(client_region);
            
            // Generate domain
            let domains = [
                "bootstrap.formation.net",
                "nodes.formation.net",
                "entry.formation.net",
                "anycast.formation.net",
                "p2p.formation.net",
            ];
            let domain = domains[rng.gen_range(0..domains.len())].to_string();
            
            // Generate request ID
            let request_id = format!("req_{}", generate_random_hex(8));
            
            // Create request
            let request = AnycastRequest {
                client_ip,
                client_region,
                domain: domain.clone(),
                request_id: request_id.clone(),
            };
            
            requests.push(request);
            
            // Generate expected nodes for this request
            // In a real system, this would be based on the routing logic
            let node_count = rng.gen_range(1..5);
            let mut nodes = Vec::with_capacity(node_count);
            
            for _ in 0..node_count {
                // Nodes should be from same or nearby regions
                let node_region = if rng.gen_bool(0.7) {
                    // Same region
                    client_region
                } else {
                    // Different region
                    Region::random()
                };
                
                let node_generator = RegionalIpGenerator::new(node_region);
                let node_info = node_generator.generate();
                
                nodes.push(node_info.node_id);
            }
            
            expected_nodes.insert(request_id, nodes);
        }
        
        // Generate test ID and timestamp
        let test_id = format!("anycast_{}", generate_random_hex(8));
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        AnycastTest {
            requests,
            expected_nodes,
            test_id,
            timestamp,
        }
    }
}

/// Generate a random hex string
fn generate_random_hex(length: usize) -> String {
    let mut rng = thread_rng();
    let chars: Vec<char> = "0123456789abcdef".chars().collect();
    (0..length)
        .map(|_| chars[rng.gen_range(0..chars.len())])
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_regional_ip_generator() {
        let generator = RegionalIpGenerator::new(Region::Europe);
        let ip_info = generator.generate();
        
        assert_eq!(ip_info.region, Region::Europe);
        assert!(ip_info.health >= 0.0 && ip_info.health <= 1.0);
        assert!(ip_info.latency_ms > 0);
        assert!(!ip_info.node_id.is_empty());
    }
    
    #[test]
    fn test_geo_dns_request_generator() {
        let generator = GeoDnsRequestGenerator::new()
            .with_coordinates(true)
            .with_ecs(true);
        
        let request = generator.generate();
        
        assert!(!request.domain.is_empty());
        assert!(request.client_ip.is_some());
        assert!(request.coordinates.is_some());
        assert!(request.ecs_prefix.is_some());
        assert!(!request.request_id.is_empty());
    }
    
    #[test]
    fn test_health_status_generator() {
        let generator = HealthStatusGenerator::new()
            .with_node_range(2, 5);
        
        let report = generator.generate();
        
        assert!(report.nodes.len() >= 2 && report.nodes.len() <= 5);
        assert!(!report.report_id.is_empty());
        
        for (_, node) in &report.nodes {
            assert!(node.health >= 0.0 && node.health <= 1.0);
            assert!(node.latency_ms > 0);
            assert!(node.bandwidth_mbps > 0);
            assert!(node.connections >= 0);
        }
    }
    
    #[test]
    fn test_bgp_announcement_generator() {
        let generator = BgpAnnouncementGenerator::new()
            .with_multiple_prefixes(true);
        
        let announcement = generator.generate();
        
        assert!(!announcement.prefixes.is_empty());
        assert!(!announcement.as_path.is_empty());
        assert!(!announcement.announcement_id.is_empty());
        
        for (ip, prefix_len) in &announcement.prefixes {
            match ip {
                IpAddr::V4(_) => assert!(*prefix_len >= 16 && *prefix_len <= 32),
                IpAddr::V6(_) => assert!(*prefix_len >= 32 && *prefix_len <= 64),
            }
        }
    }
    
    #[test]
    fn test_anycast_test_generator() {
        let generator = AnycastTestGenerator::new()
            .with_request_range(3, 10);
        
        let test = generator.generate();
        
        assert!(test.requests.len() >= 3 && test.requests.len() <= 10);
        assert_eq!(test.requests.len(), test.expected_nodes.len());
        assert!(!test.test_id.is_empty());
        
        for request in &test.requests {
            assert!(test.expected_nodes.contains_key(&request.request_id));
            assert!(!test.expected_nodes[&request.request_id].is_empty());
        }
    }
} 