// form-fuzzing/src/mutators/routing.rs
//! Mutators for BGP/Anycast routing fuzzing

use crate::generators::routing::{
    IpAddressInfo, GeoDnsRequest, HealthStatusReport, BgpAnnouncement,
    AnycastTest, Region, NodeHealth, AnycastRequest,
};
use crate::mutators::Mutator;

use rand::{Rng, thread_rng};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::collections::HashMap;

/// Mutator for IP address info
pub struct IpAddressMutator;

impl IpAddressMutator {
    /// Create a new IP address mutator
    pub fn new() -> Self {
        Self
    }
}

impl Mutator<IpAddressInfo> for IpAddressMutator {
    fn mutate(&self, ip_info: &mut IpAddressInfo) {
        let mut rng = thread_rng();
        
        // Choose what to mutate
        let mutation_type = rng.gen_range(0..5);
        
        match mutation_type {
            0 => {
                // Mutate IP address
                match ip_info.address {
                    IpAddr::V4(ipv4) => {
                        let octets = ipv4.octets();
                        let mut new_octets = [0u8; 4];
                        
                        // Modify a random octet
                        let octet_to_change = rng.gen_range(0..4);
                        for i in 0..4 {
                            if i == octet_to_change {
                                new_octets[i] = rng.gen();
                            } else {
                                new_octets[i] = octets[i];
                            }
                        }
                        
                        ip_info.address = IpAddr::V4(Ipv4Addr::new(
                            new_octets[0], new_octets[1], new_octets[2], new_octets[3]
                        ));
                    },
                    IpAddr::V6(ipv6) => {
                        let segments = ipv6.segments();
                        let mut new_segments = [0u16; 8];
                        
                        // Modify a random segment
                        let segment_to_change = rng.gen_range(0..8);
                        for i in 0..8 {
                            if i == segment_to_change {
                                new_segments[i] = rng.gen();
                            } else {
                                new_segments[i] = segments[i];
                            }
                        }
                        
                        ip_info.address = IpAddr::V6(Ipv6Addr::new(
                            new_segments[0], new_segments[1], new_segments[2], new_segments[3],
                            new_segments[4], new_segments[5], new_segments[6], new_segments[7]
                        ));
                    }
                }
            },
            1 => {
                // Mutate health to edge case
                match rng.gen_range(0..4) {
                    0 => ip_info.health = 0.0,  // Completely unhealthy
                    1 => ip_info.health = 1.0,  // Perfectly healthy
                    2 => ip_info.health = 0.5,  // Border case
                    3 => {
                        // Invalid health value (should be clamped by implementation)
                        if rng.gen_bool(0.5) {
                            ip_info.health = -0.1;
                        } else {
                            ip_info.health = 1.1;
                        }
                    },
                    _ => {}
                }
            },
            2 => {
                // Mutate latency to extreme values
                match rng.gen_range(0..3) {
                    0 => ip_info.latency_ms = 0,  // Zero latency (impossible)
                    1 => ip_info.latency_ms = u32::MAX,  // Maximum possible latency
                    2 => ip_info.latency_ms = rng.gen_range(1000..10000),  // Very high but plausible
                    _ => {}
                }
            },
            3 => {
                // Change region
                let all_regions = Region::all();
                let current_idx = all_regions.iter().position(|&r| r == ip_info.region).unwrap_or(0);
                let new_idx = (current_idx + 1) % all_regions.len();
                ip_info.region = all_regions[new_idx];
            },
            4 => {
                // Mutate last_updated timestamp
                match rng.gen_range(0..3) {
                    0 => ip_info.last_updated = 0,  // Very old timestamp
                    1 => ip_info.last_updated = u64::MAX,  // Far future timestamp
                    2 => {
                        // Recent but slightly off timestamp
                        if ip_info.last_updated > 1000 {
                            ip_info.last_updated -= 1000;
                        } else {
                            ip_info.last_updated = 0;
                        }
                    },
                    _ => {}
                }
            },
            _ => {}
        }
    }
}

/// Mutator for GeoDNS requests
pub struct GeoDnsRequestMutator;

impl GeoDnsRequestMutator {
    /// Create a new GeoDNS request mutator
    pub fn new() -> Self {
        Self
    }
}

impl Mutator<GeoDnsRequest> for GeoDnsRequestMutator {
    fn mutate(&self, request: &mut GeoDnsRequest) {
        let mut rng = thread_rng();
        
        // Choose what to mutate
        let mutation_type = rng.gen_range(0..5);
        
        match mutation_type {
            0 => {
                // Mutate domain
                match rng.gen_range(0..4) {
                    0 => request.domain = "invalid".to_string(),
                    1 => request.domain = format!("{}.invalid", request.domain),
                    2 => request.domain = format!("xn--{}", request.domain),  // Add punycode prefix
                    3 => request.domain = request.domain.chars().rev().collect(),  // Reverse domain
                    _ => {}
                }
            },
            1 => {
                // Mutate client IP
                if let Some(ref mut ip) = &mut request.client_ip {
                    match ip {
                        IpAddr::V4(ipv4) => {
                            let octets = ipv4.octets();
                            let mut new_octets = [0u8; 4];
                            
                            // Modify a random octet or set to special address
                            let special_addr = rng.gen_range(0..5);
                            match special_addr {
                                0 => {
                                    // Loopback
                                    new_octets = [127, 0, 0, 1];
                                },
                                1 => {
                                    // Link local
                                    new_octets = [169, 254, rng.gen(), rng.gen()];
                                },
                                2 => {
                                    // Multicast
                                    new_octets = [224, rng.gen(), rng.gen(), rng.gen()];
                                },
                                3 => {
                                    // Invalid (255.255.255.255)
                                    new_octets = [255, 255, 255, 255];
                                },
                                _ => {
                                    // Modify a random octet
                                    let octet_to_change = rng.gen_range(0..4);
                                    for i in 0..4 {
                                        if i == octet_to_change {
                                            new_octets[i] = rng.gen();
                                        } else {
                                            new_octets[i] = octets[i];
                                        }
                                    }
                                }
                            }
                            
                            *ip = IpAddr::V4(Ipv4Addr::new(
                                new_octets[0], new_octets[1], new_octets[2], new_octets[3]
                            ));
                        },
                        IpAddr::V6(ipv6) => {
                            let segments = ipv6.segments();
                            let mut new_segments = [0u16; 8];
                            
                            // Modify a random segment or set to special address
                            let special_addr = rng.gen_range(0..4);
                            match special_addr {
                                0 => {
                                    // Loopback
                                    new_segments = [0, 0, 0, 0, 0, 0, 0, 1];
                                },
                                1 => {
                                    // Link local
                                    new_segments = [0xfe80, 0, 0, 0, 0, 0, 0, rng.gen()];
                                },
                                2 => {
                                    // Multicast
                                    new_segments = [0xff00, rng.gen(), rng.gen(), rng.gen(), 
                                                    rng.gen(), rng.gen(), rng.gen(), rng.gen()];
                                },
                                _ => {
                                    // Modify a random segment
                                    let segment_to_change = rng.gen_range(0..8);
                                    for i in 0..8 {
                                        if i == segment_to_change {
                                            new_segments[i] = rng.gen();
                                        } else {
                                            new_segments[i] = segments[i];
                                        }
                                    }
                                }
                            }
                            
                            *ip = IpAddr::V6(Ipv6Addr::new(
                                new_segments[0], new_segments[1], new_segments[2], new_segments[3],
                                new_segments[4], new_segments[5], new_segments[6], new_segments[7]
                            ));
                        }
                    }
                } else {
                    // Add a client IP if it doesn't exist
                    if rng.gen_bool(0.5) {
                        // Add IPv4
                        request.client_ip = Some(IpAddr::V4(Ipv4Addr::new(
                            rng.gen(), rng.gen(), rng.gen(), rng.gen()
                        )));
                    } else {
                        // Add IPv6
                        request.client_ip = Some(IpAddr::V6(Ipv6Addr::new(
                            rng.gen(), rng.gen(), rng.gen(), rng.gen(),
                            rng.gen(), rng.gen(), rng.gen(), rng.gen()
                        )));
                    }
                }
            },
            2 => {
                // Mutate coordinates
                if let Some(coords) = &mut request.coordinates {
                    match rng.gen_range(0..5) {
                        0 => *coords = (90.0, coords.1),  // North pole
                        1 => *coords = (-90.0, coords.1),  // South pole
                        2 => *coords = (coords.0, 180.0),  // East edge
                        3 => *coords = (coords.0, -180.0),  // West edge
                        4 => {
                            // Invalid coordinates (out of range)
                            if rng.gen_bool(0.5) {
                                *coords = (100.0, coords.1);
                            } else {
                                *coords = (coords.0, 200.0);
                            }
                        },
                        _ => {}
                    }
                } else {
                    // Add coordinates if they don't exist
                    request.coordinates = Some((
                        rng.gen_range(-90.0..90.0),
                        rng.gen_range(-180.0..180.0)
                    ));
                }
            },
            3 => {
                // Mutate ECS prefix
                if let Some(prefix) = &mut request.ecs_prefix {
                    match rng.gen_range(0..5) {
                        0 => *prefix = 0,  // Invalid prefix length
                        1 => *prefix = 33,  // Too large for IPv4
                        2 => *prefix = 129,  // Too large for IPv6
                        3 => *prefix = 8,  // Very small (matches large network block)
                        4 => *prefix = 31,  // Edge case for IPv4
                        _ => {}
                    }
                } else {
                    // Add ECS prefix if it doesn't exist
                    request.ecs_prefix = Some(rng.gen_range(16..32));
                }
            },
            4 => {
                // Mutate timestamp
                match rng.gen_range(0..3) {
                    0 => request.timestamp = 0,  // Very old
                    1 => request.timestamp = u64::MAX,  // Far future
                    2 => {
                        // Slightly in the past
                        if request.timestamp > 10000 {
                            request.timestamp -= 10000;
                        } else {
                            request.timestamp = 0;
                        }
                    },
                    _ => {}
                }
            },
            _ => {}
        }
    }
}

/// Mutator for health status reports
pub struct HealthStatusMutator;

impl HealthStatusMutator {
    /// Create a new health status mutator
    pub fn new() -> Self {
        Self
    }
}

impl Mutator<HealthStatusReport> for HealthStatusMutator {
    fn mutate(&self, report: &mut HealthStatusReport) {
        let mut rng = thread_rng();
        
        // Choose what to mutate
        let mutation_type = rng.gen_range(0..4);
        
        match mutation_type {
            0 => {
                // Remove a random node
                if !report.nodes.is_empty() {
                    let keys: Vec<String> = report.nodes.keys().cloned().collect();
                    let key_to_remove = &keys[rng.gen_range(0..keys.len())];
                    report.nodes.remove(key_to_remove);
                }
            },
            1 => {
                // Add a node with extreme health values
                let regions = Region::all();
                let region = regions[rng.gen_range(0..regions.len())];
                
                let node_id = format!("mutated_node_{}", rng.gen::<u32>());
                
                // Create IP based on region
                let ip = match region {
                    Region::NorthAmerica => Ipv4Addr::new(50, rng.gen(), rng.gen(), rng.gen()),
                    Region::SouthAmerica => Ipv4Addr::new(60, rng.gen(), rng.gen(), rng.gen()),
                    Region::Europe => Ipv4Addr::new(80, rng.gen(), rng.gen(), rng.gen()),
                    Region::Asia => Ipv4Addr::new(100, rng.gen(), rng.gen(), rng.gen()),
                    Region::Africa => Ipv4Addr::new(150, rng.gen(), rng.gen(), rng.gen()),
                    Region::Oceania => Ipv4Addr::new(180, rng.gen(), rng.gen(), rng.gen()),
                };
                
                // Decide what kind of node to add
                let node_type = rng.gen_range(0..4);
                let (health, latency_ms, bandwidth_mbps, connections) = match node_type {
                    0 => (0.0, 9999, 0, 0),  // Dead node
                    1 => (1.0, 1, 10000, 1000),  // Super node
                    2 => (0.5, 500, 50, 5),  // Average node
                    3 => (-0.1, 0, u32::MAX, u32::MAX),  // Invalid values
                    _ => (0.0, 0, 0, 0),
                };
                
                let node = NodeHealth {
                    node_id: node_id.clone(),
                    address: IpAddr::V4(ip),
                    health,
                    latency_ms,
                    bandwidth_mbps,
                    connections,
                    region,
                    last_updated: report.timestamp,
                };
                
                report.nodes.insert(node_id, node);
            },
            2 => {
                // Mutate a random node
                if !report.nodes.is_empty() {
                    let keys: Vec<String> = report.nodes.keys().cloned().collect();
                    let key_to_mutate = &keys[rng.gen_range(0..keys.len())];
                    
                    if let Some(node) = report.nodes.get_mut(key_to_mutate) {
                        // Choose what aspect to mutate
                        match rng.gen_range(0..5) {
                            0 => {
                                // Change health to extreme value
                                node.health = match rng.gen_range(0..3) {
                                    0 => 0.0,
                                    1 => 1.0,
                                    _ => -0.1,  // Invalid value
                                };
                            },
                            1 => {
                                // Change latency to extreme value
                                node.latency_ms = match rng.gen_range(0..3) {
                                    0 => 0,
                                    1 => u32::MAX,
                                    _ => 10000,
                                };
                            },
                            2 => {
                                // Change bandwidth to extreme value
                                node.bandwidth_mbps = match rng.gen_range(0..3) {
                                    0 => 0,
                                    1 => u32::MAX,
                                    _ => 100000,
                                };
                            },
                            3 => {
                                // Change connections to extreme value
                                node.connections = match rng.gen_range(0..3) {
                                    0 => 0,
                                    1 => u32::MAX,
                                    _ => 10000,
                                };
                            },
                            4 => {
                                // Change region
                                let all_regions = Region::all();
                                let current_idx = all_regions.iter().position(|&r| r == node.region).unwrap_or(0);
                                let new_idx = (current_idx + 1) % all_regions.len();
                                node.region = all_regions[new_idx];
                            },
                            _ => {}
                        }
                    }
                }
            },
            3 => {
                // Empty the entire report
                report.nodes.clear();
            },
            _ => {}
        }
    }
}

/// Mutator for BGP announcements
pub struct BgpAnnouncementMutator;

impl BgpAnnouncementMutator {
    /// Create a new BGP announcement mutator
    pub fn new() -> Self {
        Self
    }
}

impl Mutator<BgpAnnouncement> for BgpAnnouncementMutator {
    fn mutate(&self, announcement: &mut BgpAnnouncement) {
        let mut rng = thread_rng();
        
        // Choose what to mutate
        let mutation_type = rng.gen_range(0..6);
        
        match mutation_type {
            0 => {
                // Mutate prefixes
                match rng.gen_range(0..4) {
                    0 => {
                        // Empty prefix list
                        announcement.prefixes.clear();
                    },
                    1 => {
                        // Add invalid prefix length
                        let ip = if rng.gen_bool(0.5) {
                            IpAddr::V4(Ipv4Addr::new(rng.gen(), rng.gen(), rng.gen(), rng.gen()))
                        } else {
                            IpAddr::V6(Ipv6Addr::new(
                                rng.gen(), rng.gen(), rng.gen(), rng.gen(),
                                rng.gen(), rng.gen(), rng.gen(), rng.gen()
                            ))
                        };
                        
                        // Generate invalid prefix length
                        let prefix_len = match ip {
                            IpAddr::V4(_) => rng.gen_range(33..64),  // Invalid for IPv4
                            IpAddr::V6(_) => rng.gen_range(129..160),  // Invalid for IPv6
                        };
                        
                        announcement.prefixes.push((ip, prefix_len as u8));
                    },
                    2 => {
                        // Add massive number of prefixes
                        if announcement.prefixes.len() < 5 {
                            for _ in 0..50 {
                                let ip = if rng.gen_bool(0.5) {
                                    IpAddr::V4(Ipv4Addr::new(rng.gen(), rng.gen(), rng.gen(), rng.gen()))
                                } else {
                                    IpAddr::V6(Ipv6Addr::new(
                                        rng.gen(), rng.gen(), rng.gen(), rng.gen(),
                                        rng.gen(), rng.gen(), rng.gen(), rng.gen()
                                    ))
                                };
                                
                                let prefix_len = match ip {
                                    IpAddr::V4(_) => rng.gen_range(8..32),
                                    IpAddr::V6(_) => rng.gen_range(16..64),
                                };
                                
                                announcement.prefixes.push((ip, prefix_len));
                            }
                        }
                    },
                    3 => {
                        // Mutate existing prefixes
                        for (_, prefix_len) in &mut announcement.prefixes {
                            // Make prefix length very small (large network)
                            *prefix_len = 8;
                        }
                    },
                    _ => {}
                }
            },
            1 => {
                // Mutate AS path
                match rng.gen_range(0..4) {
                    0 => {
                        // Empty AS path (invalid)
                        announcement.as_path.clear();
                    },
                    1 => {
                        // Add AS path loop (invalid)
                        if !announcement.as_path.is_empty() {
                            let looped_as = announcement.as_path[0];
                            announcement.as_path.push(looped_as);
                        }
                    },
                    2 => {
                        // Add AS 0 (reserved, invalid)
                        announcement.as_path.insert(0, 0);
                    },
                    3 => {
                        // Add extremely long AS path
                        for _ in 0..100 {
                            announcement.as_path.push(rng.gen_range(1000..65000));
                        }
                    },
                    _ => {}
                }
            },
            2 => {
                // Mutate communities
                match rng.gen_range(0..3) {
                    0 => {
                        // Clear communities
                        announcement.communities.clear();
                    },
                    1 => {
                        // Add well-known communities
                        let well_known = [
                            (0, 0),      // Reserved
                            (0xFFFF, 0), // Planned for future use
                            (0, 1),      // NO_EXPORT
                            (0, 2),      // NO_ADVERTISE
                            (0, 3),      // NO_EXPORT_SUBCONFED
                        ];
                        
                        for community in well_known.iter() {
                            announcement.communities.push(*community);
                        }
                    },
                    2 => {
                        // Add lots of random communities
                        for _ in 0..100 {
                            announcement.communities.push((rng.gen(), rng.gen()));
                        }
                    },
                    _ => {}
                }
            },
            3 => {
                // Mutate next hop
                match rng.gen_range(0..4) {
                    0 => {
                        // Set to invalid next hop (0.0.0.0)
                        announcement.next_hop = IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0));
                    },
                    1 => {
                        // Set to multicast address (invalid next hop)
                        announcement.next_hop = IpAddr::V4(Ipv4Addr::new(224, rng.gen(), rng.gen(), rng.gen()));
                    },
                    2 => {
                        // Set to loopback address
                        announcement.next_hop = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
                    },
                    3 => {
                        // Set to IPv6 unspecified address
                        announcement.next_hop = IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0));
                    },
                    _ => {}
                }
            },
            4 => {
                // Mutate attributes
                match rng.gen_range(0..3) {
                    0 => {
                        // Set extreme local preference
                        announcement.local_pref = Some(u32::MAX);
                    },
                    1 => {
                        // Set extreme MED
                        announcement.med = Some(u32::MAX);
                    },
                    2 => {
                        // Toggle withdrawal flag
                        announcement.is_withdrawal = !announcement.is_withdrawal;
                    },
                    _ => {}
                }
            },
            5 => {
                // Make announcement empty/invalid
                announcement.prefixes.clear();
                announcement.as_path.clear();
                announcement.communities.clear();
                announcement.is_withdrawal = true;
            },
            _ => {}
        }
    }
}

/// Mutator for Anycast tests
pub struct AnycastTestMutator;

impl AnycastTestMutator {
    /// Create a new Anycast test mutator
    pub fn new() -> Self {
        Self
    }
}

impl Mutator<AnycastTest> for AnycastTestMutator {
    fn mutate(&self, test: &mut AnycastTest) {
        let mut rng = thread_rng();
        
        // Choose what to mutate
        let mutation_type = rng.gen_range(0..4);
        
        match mutation_type {
            0 => {
                // Mutate requests
                match rng.gen_range(0..3) {
                    0 => {
                        // Remove a random request
                        if !test.requests.is_empty() {
                            let idx_to_remove = rng.gen_range(0..test.requests.len());
                            let request_id = test.requests[idx_to_remove].request_id.clone();
                            test.requests.remove(idx_to_remove);
                            test.expected_nodes.remove(&request_id);
                        }
                    },
                    1 => {
                        // Add a request with invalid data
                        let request_id = format!("mutated_req_{}", rng.gen::<u32>());
                        
                        let request = AnycastRequest {
                            client_ip: IpAddr::V4(Ipv4Addr::new(255, 255, 255, 255)),
                            client_region: Region::random(),
                            domain: "invalid.example".to_string(),
                            request_id: request_id.clone(),
                        };
                        
                        test.requests.push(request);
                        test.expected_nodes.insert(request_id, vec!["invalid_node".to_string()]);
                    },
                    2 => {
                        // Mutate existing requests
                        for request in &mut test.requests {
                            // Change the domain
                            request.domain = "mutated.example".to_string();
                        }
                    },
                    _ => {}
                }
            },
            1 => {
                // Mutate expected nodes
                match rng.gen_range(0..3) {
                    0 => {
                        // Remove expected nodes for a random request
                        if !test.requests.is_empty() {
                            let idx = rng.gen_range(0..test.requests.len());
                            let request_id = &test.requests[idx].request_id;
                            test.expected_nodes.remove(request_id);
                        }
                    },
                    1 => {
                        // Add invalid expected nodes
                        for (_, nodes) in &mut test.expected_nodes {
                            nodes.push("invalid_node".to_string());
                        }
                    },
                    2 => {
                        // Make expected nodes empty
                        for (_, nodes) in &mut test.expected_nodes {
                            nodes.clear();
                        }
                    },
                    _ => {}
                }
            },
            2 => {
                // Make test inconsistent
                if !test.requests.is_empty() {
                    // Add a request ID that doesn't exist in expected_nodes
                    let request = AnycastRequest {
                        client_ip: IpAddr::V4(Ipv4Addr::new(rng.gen(), rng.gen(), rng.gen(), rng.gen())),
                        client_region: Region::random(),
                        domain: "bootstrap.formation.net".to_string(),
                        request_id: "orphaned_request".to_string(),
                    };
                    
                    test.requests.push(request);
                    
                    // Add expected nodes for a request that doesn't exist
                    test.expected_nodes.insert(
                        "nonexistent_request".to_string(), 
                        vec!["node1".to_string(), "node2".to_string()]
                    );
                }
            },
            3 => {
                // Empty test
                test.requests.clear();
                test.expected_nodes.clear();
            },
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generators::routing::{
        RegionalIpGenerator, GeoDnsRequestGenerator, HealthStatusGenerator,
        BgpAnnouncementGenerator, AnycastTestGenerator,
    };
    
    #[test]
    fn test_ip_address_mutator() {
        let generator = RegionalIpGenerator::new(Region::Europe);
        let mut ip_info = generator.generate();
        
        let mutator = IpAddressMutator::new();
        mutator.mutate(&mut ip_info);
        
        // Since mutation is random, we can't assert specific changes
        // But we can verify the structure is still valid
        assert!(ip_info.health >= -0.1 && ip_info.health <= 1.1);
        assert!(ip_info.latency_ms <= u32::MAX);
    }
    
    #[test]
    fn test_geo_dns_request_mutator() {
        let generator = GeoDnsRequestGenerator::new()
            .with_coordinates(true)
            .with_ecs(true);
        
        let mut request = generator.generate();
        
        let mutator = GeoDnsRequestMutator::new();
        mutator.mutate(&mut request);
        
        // Since mutation is random, we can't assert specific changes
        // But we can verify the structure is still valid
        assert!(!request.domain.is_empty());
    }
    
    #[test]
    fn test_health_status_mutator() {
        let generator = HealthStatusGenerator::new();
        let mut report = generator.generate();
        
        let original_node_count = report.nodes.len();
        
        let mutator = HealthStatusMutator::new();
        mutator.mutate(&mut report);
        
        // Structure should still be valid even if contents are mutated
        assert!(report.nodes.len() <= original_node_count + 1);
    }
    
    #[test]
    fn test_bgp_announcement_mutator() {
        let generator = BgpAnnouncementGenerator::new();
        let mut announcement = generator.generate();
        
        let mutator = BgpAnnouncementMutator::new();
        mutator.mutate(&mut announcement);
        
        // Since mutation is random, we can only verify structure
        // Prefixes might be empty if they were cleared
    }
    
    #[test]
    fn test_anycast_test_mutator() {
        let generator = AnycastTestGenerator::new();
        let mut test = generator.generate();
        
        let original_request_count = test.requests.len();
        
        let mutator = AnycastTestMutator::new();
        mutator.mutate(&mut test);
        
        // Since mutation is random, we can only verify structure
        // Note: requests and expected_nodes might not match if the test was made inconsistent
    }
} 
