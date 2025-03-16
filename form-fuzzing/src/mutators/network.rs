// form-fuzzing/src/mutators/network.rs
//! Network mutators for fuzzing network functionality

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::Duration;
use rand::{Rng, seq::SliceRandom};
use crate::generators::network::{
    NetworkPacket, Protocol, NATConfig, NATType,
    MappingBehavior, FilteringBehavior, P2PConnectionRequest, NATConfigGenerator
};
use crate::generators::Generator;
use crate::mutators::Mutator;
use uuid::Uuid;

/// A mutator for network packets
pub struct NetworkPacketMutator;

impl NetworkPacketMutator {
    /// Create a new NetworkPacketMutator
    pub fn new() -> Self {
        Self
    }
    
    /// Flip a random bit in a byte array
    fn flip_random_bit(&self, data: &mut [u8]) {
        let mut rng = rand::thread_rng();
        if data.is_empty() {
            return;
        }
        
        let byte_idx = rng.gen_range(0..data.len());
        let bit_idx = rng.gen_range(0..8);
        data[byte_idx] ^= 1 << bit_idx;
    }
    
    /// Generate an extreme IP address (e.g., multicast, reserved ranges)
    fn generate_extreme_ip(&self) -> IpAddr {
        let mut rng = rand::thread_rng();
        match rng.gen_range(0..10) {
            0 => IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),          // 0.0.0.0 (unspecified)
            1 => IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),        // Loopback
            2 => IpAddr::V4(Ipv4Addr::new(224, rng.gen(), rng.gen(), rng.gen())), // Multicast
            3 => IpAddr::V4(Ipv4Addr::new(255, 255, 255, 255)),  // Broadcast
            4 => IpAddr::V4(Ipv4Addr::new(169, 254, rng.gen(), rng.gen())), // Link-local
            5 => IpAddr::V4(Ipv4Addr::new(192, 0, 2, rng.gen())), // TEST-NET
            6 => IpAddr::V4(Ipv4Addr::new(198, 51, 100, rng.gen())), // TEST-NET-2
            7 => IpAddr::V4(Ipv4Addr::new(203, 0, 113, rng.gen())), // TEST-NET-3
            8 => IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)), // IPv6 loopback
            _ => IpAddr::V6(Ipv6Addr::new(0xff00, rng.gen(), rng.gen(), rng.gen(), 
                                           rng.gen(), rng.gen(), rng.gen(), rng.gen())), // IPv6 multicast
        }
    }
}

impl Mutator<NetworkPacket> for NetworkPacketMutator {
    fn mutate(&self, packet: &mut NetworkPacket) {
        let mut rng = rand::thread_rng();
        
        // Choose a mutation strategy
        let mutation_strategy = rng.gen_range(0..10);
        
        match mutation_strategy {
            0 => {
                // Change protocol
                let protocols = Protocol::all();
                packet.protocol = *protocols.choose(&mut rng).unwrap();
            },
            1 => {
                // Change source IP to an extreme value
                packet.source_ip = self.generate_extreme_ip();
            },
            2 => {
                // Change destination IP to an extreme value
                packet.destination_ip = self.generate_extreme_ip();
            },
            3 => {
                // Change source port to an extreme value
                packet.source_port = match rng.gen_range(0..5) {
                    0 => 0,      // Invalid port
                    1 => 1,      // Privileged port
                    2 => 65535,  // Max port
                    3 => 80,     // Common HTTP port
                    _ => 443,    // Common HTTPS port
                };
            },
            4 => {
                // Change destination port to an extreme value
                packet.destination_port = match rng.gen_range(0..5) {
                    0 => 0,      // Invalid port
                    1 => 1,      // Privileged port
                    2 => 65535,  // Max port
                    3 => 80,     // Common HTTP port
                    _ => 443,    // Common HTTPS port
                };
            },
            5 => {
                // Change TTL to an extreme value
                packet.ttl = match rng.gen_range(0..3) {
                    0 => 0,      // Expired
                    1 => 1,      // About to expire
                    _ => 255,    // Maximum TTL
                };
            },
            6 => {
                // Change packet length
                packet.length = match rng.gen_range(0..3) {
                    0 => 0,          // Empty packet
                    1 => 20,         // Minimum IPv4 header
                    _ => 65535,      // Maximum packet size
                };
            },
            7 => {
                // Toggle fragmentation
                packet.fragmented = !packet.fragmented;
            },
            8 => {
                // Corrupt payload
                if !packet.payload.is_empty() {
                    // Choose corruption strategy
                    match rng.gen_range(0..3) {
                        0 => {
                            // Flip a random bit
                            self.flip_random_bit(&mut packet.payload);
                        },
                        1 => {
                            // Set payload to all zeros or all ones
                            let fill_value = if rng.gen_bool(0.5) { 0 } else { 255 };
                            for byte in packet.payload.iter_mut() {
                                *byte = fill_value;
                            }
                        },
                        _ => {
                            // Completely replace payload with random data
                            for byte in packet.payload.iter_mut() {
                                *byte = rng.gen();
                            }
                        },
                    }
                }
            },
            9 => {
                // Modify headers
                match rng.gen_range(0..3) {
                    0 => {
                        // Add a random header
                        let header_name = format!("x-fuzz-{}", rng.gen::<u8>());
                        let header_value = rng.gen::<u32>().to_string();
                        packet.headers.insert(header_name, header_value);
                    },
                    1 => {
                        // Modify an existing header if any
                        if let Some(key) = packet.headers.keys().next().cloned() {
                            packet.headers.insert(key, rng.gen::<u32>().to_string());
                        }
                    },
                    _ => {
                        // Remove a random header if any
                        if let Some(key) = packet.headers.keys().next().cloned() {
                            packet.headers.remove(&key);
                        }
                    },
                }
            },
            _ => unreachable!(),
        }
    }
}

/// A mutator for NAT configurations
pub struct NATConfigMutator;

impl NATConfigMutator {
    /// Create a new NATConfigMutator
    pub fn new() -> Self {
        Self
    }
}

impl Mutator<NATConfig> for NATConfigMutator {
    fn mutate(&self, config: &mut NATConfig) {
        let mut rng = rand::thread_rng();
        
        // Choose a mutation strategy
        let mutation_strategy = rng.gen_range(0..10);
        
        match mutation_strategy {
            0 => {
                // Change NAT type
                let nat_types = NATType::all();
                config.nat_type = *nat_types.choose(&mut rng).unwrap();
            },
            1 => {
                // Change mapping behavior
                let mapping_behaviors = MappingBehavior::all();
                config.mapping_behavior = *mapping_behaviors.choose(&mut rng).unwrap();
            },
            2 => {
                // Change filtering behavior
                let filtering_behaviors = FilteringBehavior::all();
                config.filtering_behavior = *filtering_behaviors.choose(&mut rng).unwrap();
            },
            3 => {
                // Change port range to extreme values
                match rng.gen_range(0..3) {
                    0 => {
                        // Very narrow port range
                        config.port_range_start = rng.gen_range(1024..65000);
                        config.port_range_end = config.port_range_start + rng.gen_range(1..10);
                    },
                    1 => {
                        // Very wide port range
                        config.port_range_start = 1;
                        config.port_range_end = 65535;
                    },
                    _ => {
                        // Invalid port range (start > end)
                        config.port_range_start = rng.gen_range(1024..65535);
                        config.port_range_end = rng.gen_range(1..1023);
                    },
                }
            },
            4 => {
                // Change mapping timeout to extreme values
                match rng.gen_range(0..3) {
                    0 => {
                        // Very short timeout
                        config.mapping_timeout = Duration::from_secs(1);
                    },
                    1 => {
                        // Very long timeout
                        config.mapping_timeout = Duration::from_secs(24 * 60 * 60); // 1 day
                    },
                    _ => {
                        // Zero timeout
                        config.mapping_timeout = Duration::from_secs(0);
                    },
                }
            },
            5 => {
                // Change IPs to specific values
                let packet_mutator = NetworkPacketMutator::new();
                config.internal_ip = packet_mutator.generate_extreme_ip();
                config.external_ip = packet_mutator.generate_extreme_ip();
            },
            6 => {
                // Modify upstream NAT
                if config.upstream_nat.is_none() && rng.gen_bool(0.7) {
                    // Add an upstream NAT if there isn't one
                    let nat_generator = crate::generators::network::NATConfigGenerator::new();
                    config.upstream_nat = Some(Box::new(nat_generator.generate()));
                } else if config.upstream_nat.is_some() {
                    // Either remove or mutate the existing upstream NAT
                    if rng.gen_bool(0.3) {
                        config.upstream_nat = None;
                    } else if let Some(ref mut upstream) = config.upstream_nat {
                        self.mutate(upstream);
                    }
                }
            },
            7 => {
                // Toggle feature flags
                match rng.gen_range(0..3) {
                    0 => config.mapping_refresh_enabled = !config.mapping_refresh_enabled,
                    1 => config.upnp_enabled = !config.upnp_enabled,
                    _ => config.pmp_enabled = !config.pmp_enabled,
                }
            },
            8 => {
                // Change max connections to extreme values
                match rng.gen_range(0..3) {
                    0 => config.max_connections = 0,          // No connections allowed
                    1 => config.max_connections = 1,          // Single connection
                    _ => config.max_connections = 1_000_000,  // Very high number
                }
            },
            9 => {
                // Combined mutations
                // Apply multiple small changes at once
                if rng.gen_bool(0.5) {
                    config.mapping_refresh_enabled = !config.mapping_refresh_enabled;
                }
                if rng.gen_bool(0.5) {
                    config.upnp_enabled = !config.upnp_enabled;
                }
                if rng.gen_bool(0.5) {
                    config.pmp_enabled = !config.pmp_enabled;
                }
                
                // Slightly adjust port range
                if rng.gen_bool(0.5) {
                    config.port_range_start = rng.gen_range(1024..60000);
                    config.port_range_end = config.port_range_start + rng.gen_range(100..5000);
                }
            },
            _ => unreachable!(),
        }
    }
}

/// A mutator for P2P connection requests
pub struct P2PConnectionRequestMutator;

impl P2PConnectionRequestMutator {
    /// Create a new P2PConnectionRequestMutator
    pub fn new() -> Self {
        Self
    }
    
    /// Generate a malformed server for mutation
    pub fn generate_malformed_server(&self) -> String {
        let mut rng = rand::thread_rng();
        
        let malformed_servers = [
            // Empty server
            "",
            // Invalid formatting
            "invalid-format",
            // Invalid domain
            "@#$%^&*().example.com:3478",
            // Invalid port
            "stun.example.com:99999",
            // Missing port
            "stun.example.com:",
            // No domain, just IP
            "192.168.1.1:3478",
            // Invalid URL scheme
            "http://stun.example.com:3478",
            // Extremely long domain
            &format!("{}.example.com:3478", "x".repeat(200)),
        ];
        
        malformed_servers[rng.gen_range(0..malformed_servers.len())].to_string()
    }
    
    // Alias method for backward compatibility
    pub fn generate_random_stun_server(&self) -> String {
        self.generate_malformed_server()
    }
    
    // Alias method for backward compatibility
    pub fn generate_random_turn_server(&self) -> String {
        self.generate_malformed_server()
    }
    
    // Fix for endpoint mutation
    pub fn mutate_endpoint(&self, endpoint: &mut String) {
        let mut rng = rand::thread_rng();
        
        if rng.gen_bool(0.5) {
            // Replace with an invalid endpoint
            let invalid_endpoints = [
                "",
                "invalid-endpoint",
                "127.0.0.1:99999", // Invalid port
                "999.999.999.999:8000", // Invalid IP
                "::1]", // Invalid IPv6
                "[2001:db8::1:8000", // Missing closing bracket
            ];
            
            *endpoint = invalid_endpoints[rng.gen_range(0..invalid_endpoints.len())].to_string();
        } else {
            // Modify existing endpoint
            *endpoint = format!("{}:{}", 
                format!("{}.{}.{}.{}", 
                    rng.gen_range(1..255), 
                    rng.gen_range(1..255),
                    rng.gen_range(1..255),
                    rng.gen_range(1..255)
                ),
                rng.gen_range(1024..65535)
            );
        }
    }
    
    // Add mutate_protocol method
    fn mutate_protocol(&self, original: Option<&Protocol>) -> Protocol {
        let mut rng = rand::thread_rng();
        
        if let Some(protocol) = original {
            // Mutate existing protocol
            if rng.gen_bool(0.3) {
                // 30% chance to keep the same protocol
                *protocol
            } else {
                // 70% chance to change to a different protocol
                let all_protocols = Protocol::all();
                let mut available: Vec<_> = all_protocols.into_iter()
                    .filter(|p| p != protocol)
                    .collect();
                    
                if available.is_empty() {
                    // If somehow we filtered out all protocols, return the original
                    *protocol
                } else {
                    *available.choose(&mut rng).unwrap()
                }
            }
        } else {
            // Generate a new protocol
            Protocol::random()
        }
    }
}

impl Mutator<P2PConnectionRequest> for P2PConnectionRequestMutator {
    fn mutate(&self, input: &mut P2PConnectionRequest) {
        let mut rng = rand::thread_rng();
        
        // Choose which field to mutate
        match rng.gen_range(0..8) {
            0 => {
                // Mutate local ID
                input.local_id = if rng.gen_bool(0.3) {
                    // Empty ID (invalid)
                    "".to_string()
                } else {
                    // Random valid ID
                    Uuid::new_v4().to_string()
                };
            },
            1 => {
                // Mutate peer ID
                input.peer_id = if rng.gen_bool(0.3) {
                    // Empty ID (invalid)
                    "".to_string()
                } else {
                    // Random valid ID
                    Uuid::new_v4().to_string()
                };
            },
            2 => {
                // Mutate timeout
                input.timeout = Duration::from_secs(rng.gen_range(1..30)); // Set a reasonable timeout value
            },
            3 => {
                // Mutate protocols
                if input.protocols.is_empty() || rng.gen_bool(0.5) {
                    // Add a protocol
                    input.protocols.push(self.mutate_protocol(None));
                } else {
                    // Mutate existing protocol
                    let idx = rng.gen_range(0..input.protocols.len());
                    let protocol = input.protocols.remove(idx);
                    input.protocols.push(self.mutate_protocol(Some(&protocol)));
                }
            },
            4 => {
                // Mutate STUN servers
                if input.stun_servers.is_empty() || rng.gen_bool(0.5) {
                    // Add a server
                    input.stun_servers.push(self.generate_malformed_server());
                } else if input.stun_servers.len() > 1 && rng.gen_bool(0.3) {
                    // Remove a server
                    let idx = rng.gen_range(0..input.stun_servers.len());
                    input.stun_servers.remove(idx);
                } else {
                    // Mutate server
                    let idx = rng.gen_range(0..input.stun_servers.len());
                    input.stun_servers[idx] = self.generate_malformed_server();
                }
            },
            5 => {
                // Mutate TURN servers
                if input.turn_servers.is_empty() || rng.gen_bool(0.5) {
                    // Add a server
                    input.turn_servers.push(self.generate_malformed_server());
                } else if input.turn_servers.len() > 1 && rng.gen_bool(0.3) {
                    // Remove a server
                    let idx = rng.gen_range(0..input.turn_servers.len());
                    input.turn_servers.remove(idx);
                } else {
                    // Mutate server
                    let idx = rng.gen_range(0..input.turn_servers.len());
                    input.turn_servers[idx] = self.generate_malformed_server();
                }
            },
            6 => {
                // Random drastic mutation - completely invalid request
                *input = P2PConnectionRequest {
                    local_id: "".to_string(),
                    peer_id: "".to_string(),
                    stun_servers: vec![],
                    turn_servers: vec![],
                    protocols: vec![],
                    timeout: Duration::from_secs(rng.gen_range(0..10)), // Very low timeout
                    use_ice: false,
                    use_direct: false,
                    use_relay: false,
                    local_candidates: vec![],
                    peer_candidates: vec![],
                    connection_attempts: 0,
                    keep_alive_interval: Duration::from_secs(0)
                };
            },
            _ => {
                // Combination of mutations
                if rng.gen_bool(0.5) {
                    input.local_id = Uuid::new_v4().to_string();
                }
                if rng.gen_bool(0.5) {
                    input.peer_id = Uuid::new_v4().to_string();
                }
                if rng.gen_bool(0.5) {
                    input.timeout = Duration::from_secs(rng.gen_range(1..30));
                }
            },
        }
    }
}

// Helper functions for network mutations

/// Mutate an IP address
fn mutate_ip_address(ip: &IpAddr) -> IpAddr {
    let mut rng = rand::thread_rng();
    
    match ip {
        IpAddr::V4(ipv4) => {
            // Choose a mutation strategy for IPv4
            match rng.gen_range(0..5) {
                0 => {
                    // Completely random IPv4
                    IpAddr::V4(Ipv4Addr::new(rng.gen(), rng.gen(), rng.gen(), rng.gen()))
                },
                1 => {
                    // Change a single octet
                    let octets = ipv4.octets();
                    let mut new_octets = octets;
                    let index = rng.gen_range(0..4);
                    new_octets[index] = rng.gen();
                    IpAddr::V4(Ipv4Addr::from(new_octets))
                },
                2 => {
                    // Convert to common private address
                    match rng.gen_range(0..3) {
                        0 => IpAddr::V4(Ipv4Addr::new(10, rng.gen(), rng.gen(), rng.gen())), // 10.0.0.0/8
                        1 => IpAddr::V4(Ipv4Addr::new(172, rng.gen_range(16..32), rng.gen(), rng.gen())), // 172.16.0.0/12
                        _ => IpAddr::V4(Ipv4Addr::new(192, 168, rng.gen(), rng.gen())), // 192.168.0.0/16
                    }
                },
                3 => {
                    // Convert to IPv6 equivalent
                    IpAddr::V6(ipv4_to_ipv6(ipv4))
                },
                4 => {
                    // Use special addresses
                    match rng.gen_range(0..5) {
                        0 => IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), // Localhost
                        1 => IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),   // Unspecified
                        2 => IpAddr::V4(Ipv4Addr::new(255, 255, 255, 255)), // Broadcast
                        3 => IpAddr::V4(Ipv4Addr::new(169, 254, rng.gen(), rng.gen())), // Link-local
                        _ => IpAddr::V4(Ipv4Addr::new(224, rng.gen(), rng.gen(), rng.gen())), // Multicast
                    }
                },
                _ => unreachable!(),
            }
        },
        IpAddr::V6(ipv6) => {
            // Choose a mutation strategy for IPv6
            match rng.gen_range(0..5) {
                0 => {
                    // Completely random IPv6
                    IpAddr::V6(Ipv6Addr::new(
                        rng.gen(), rng.gen(), rng.gen(), rng.gen(),
                        rng.gen(), rng.gen(), rng.gen(), rng.gen(),
                    ))
                },
                1 => {
                    // Change a single segment
                    let segments = ipv6.segments();
                    let mut new_segments = segments;
                    let index = rng.gen_range(0..8);
                    new_segments[index] = rng.gen();
                    IpAddr::V6(Ipv6Addr::from(new_segments))
                },
                2 => {
                    // Convert to IPv4
                    if let Some(ipv4) = ipv6_to_ipv4(ipv6) {
                        IpAddr::V4(ipv4)
                    } else {
                        // If conversion fails, generate random IPv4
                        IpAddr::V4(Ipv4Addr::new(rng.gen(), rng.gen(), rng.gen(), rng.gen()))
                    }
                },
                3 => {
                    // Use ULA address (fc00::/7)
                    let mut segments = [0u16; 8];
                    segments[0] = 0xfc00 | (rng.gen::<u16>() & 0x00ff);
                    for i in 1..8 {
                        segments[i] = rng.gen();
                    }
                    IpAddr::V6(Ipv6Addr::from(segments))
                },
                4 => {
                    // Use special addresses
                    match rng.gen_range(0..4) {
                        0 => IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)), // Localhost
                        1 => IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0)), // Unspecified
                        2 => IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0xffff, 0xc0a8, 0x0001)), // IPv4-mapped
                        _ => IpAddr::V6(Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, rng.gen())), // Link-local
                    }
                },
                _ => unreachable!(),
            }
        },
    }
}

/// Convert IPv4 to IPv6
fn ipv4_to_ipv6(ipv4: &Ipv4Addr) -> Ipv6Addr {
    let octets = ipv4.octets();
    Ipv6Addr::new(0, 0, 0, 0, 0, 0xffff, 
        u16::from(octets[0]) << 8 | u16::from(octets[1]),
        u16::from(octets[2]) << 8 | u16::from(octets[3]))
}

/// Try to convert IPv6 to IPv4 if it's an IPv4-mapped IPv6 address
fn ipv6_to_ipv4(ipv6: &Ipv6Addr) -> Option<Ipv4Addr> {
    let segments = ipv6.segments();
    
    // Check if it's an IPv4-mapped IPv6 address
    if segments[0] == 0 && segments[1] == 0 && segments[2] == 0 && segments[3] == 0 &&
       segments[4] == 0 && segments[5] == 0xffff {
        let octet1 = (segments[6] >> 8) as u8;
        let octet2 = (segments[6] & 0xff) as u8;
        let octet3 = (segments[7] >> 8) as u8;
        let octet4 = (segments[7] & 0xff) as u8;
        
        Some(Ipv4Addr::new(octet1, octet2, octet3, octet4))
    } else {
        None
    }
}

/// Mutate a port number
fn mutate_port(port: u16) -> u16 {
    let mut rng = rand::thread_rng();
    
    // Choose a mutation strategy
    match rng.gen_range(0..5) {
        0 => {
            // Completely random port
            rng.gen()
        },
        1 => {
            // Slightly modify the port
            let offset = rng.gen_range(-10..10);
            port.wrapping_add(offset as u16)
        },
        2 => {
            // Use common well-known port
            match rng.gen_range(0..10) {
                0 => 21,   // FTP
                1 => 22,   // SSH
                2 => 25,   // SMTP
                3 => 53,   // DNS
                4 => 80,   // HTTP
                5 => 443,  // HTTPS
                6 => 3389, // RDP
                7 => 8080, // Alt HTTP
                8 => 3306, // MySQL
                _ => 5432, // PostgreSQL
            }
        },
        3 => {
            // Use ephemeral port range
            rng.gen_range(49152..65535)
        },
        4 => {
            // Use special values
            match rng.gen_range(0..3) {
                0 => 0,     // Typically not used
                1 => 1,     // Very low port
                _ => 65535, // Maximum port
            }
        },
        _ => unreachable!(),
    }
}

/// Mutate a protocol
fn mutate_protocol(protocol: &Protocol) -> Protocol {
    let mut rng = rand::thread_rng();
    
    // Choose a mutation strategy
    match rng.gen_range(0..3) {
        0 => {
            // Use a standard protocol
            match rng.gen_range(0..4) {
                0 => Protocol::TCP,
                1 => Protocol::UDP,
                2 => Protocol::QUIC,
                _ => Protocol::SCTP,
            }
        },
        1 => {
            // Use a custom protocol
            Protocol::RUDP
        },
        2 => {
            // Modify the current protocol slightly for custom protocols
            if let Protocol::RUDP = protocol {
                let offset = rng.gen_range(0..3); // Use positive range only
                Protocol::RUDP // Just return RUDP again
            } else {
                // For standard protocols, pick a different standard one
                let current = match protocol {
                    Protocol::TCP => 0,
                    Protocol::UDP => 1,
                    Protocol::QUIC => 2,
                    Protocol::SCTP => 3,
                    Protocol::DCCP => 4,
                    _ => 5, // RUDP case
                };
                
                // Pick a different protocol
                let new_index = (current + rng.gen_range(1..5)) % 6;
                match new_index {
                    0 => Protocol::TCP,
                    1 => Protocol::UDP,
                    2 => Protocol::QUIC,
                    3 => Protocol::SCTP,
                    4 => Protocol::DCCP,
                    _ => Protocol::RUDP,
                }
            }
        },
        _ => unreachable!(),
    }
}

/// Mutate a TTL value
fn mutate_ttl(ttl: u8) -> u8 {
    let mut rng = rand::thread_rng();
    
    // Choose a mutation strategy
    match rng.gen_range(0..4) {
        0 => {
            // Completely random TTL
            rng.gen()
        },
        1 => {
            // Slightly modify the TTL
            let offset = rng.gen_range(-10..10);
            ttl.wrapping_add(offset as u8)
        },
        2 => {
            // Use common TTL values
            match rng.gen_range(0..4) {
                0 => 1,    // Minimum
                1 => 64,   // Common default
                2 => 128,  // Common default
                _ => 255,  // Maximum
            }
        },
        3 => {
            // Use extreme values
            match rng.gen_range(0..2) {
                0 => 0,   // Invalid in some implementations
                _ => 255, // Maximum
            }
        },
        _ => unreachable!(),
    }
}

/// Mutate a packet payload
fn mutate_payload(payload: &mut Vec<u8>) {
    let mut rng = rand::thread_rng();
    
    if payload.is_empty() {
        // Add some data if payload is empty
        let size = rng.gen_range(1..100);
        for _ in 0..size {
            payload.push(rng.gen());
        }
        return;
    }
    
    // Choose a mutation strategy
    match rng.gen_range(0..5) {
        0 => {
            // Change a single byte
            if !payload.is_empty() {
                let index = rng.gen_range(0..payload.len());
                payload[index] = rng.gen();
            }
        },
        1 => {
            // Truncate the payload
            if payload.len() > 1 {
                let new_len = rng.gen_range(1..payload.len());
                payload.truncate(new_len);
            }
        },
        2 => {
            // Extend the payload
            let extra_bytes = rng.gen_range(1..20);
            for _ in 0..extra_bytes {
                payload.push(rng.gen());
            }
        },
        3 => {
            // Bit flip in random bytes
            let num_bytes = rng.gen_range(1..std::cmp::max(2, payload.len() / 10));
            for _ in 0..num_bytes {
                let index = rng.gen_range(0..payload.len());
                let bit = rng.gen_range(0..8);
                payload[index] ^= 1 << bit;
            }
        },
        4 => {
            // Replace with completely new payload
            let size = rng.gen_range(1..100);
            payload.clear();
            for _ in 0..size {
                payload.push(rng.gen());
            }
        },
        _ => unreachable!(),
    }
}

/// Mutate packet flags
fn mutate_flags(flags: u8) -> u8 {
    let mut rng = rand::thread_rng();
    
    // Choose a mutation strategy
    match rng.gen_range(0..4) {
        0 => {
            // Completely random flags
            rng.gen()
        },
        1 => {
            // Flip a single bit
            let bit = rng.gen_range(0..8);
            flags ^ (1 << bit)
        },
        2 => {
            // Set all bits
            0xff
        },
        3 => {
            // Clear all bits
            0x00
        },
        _ => unreachable!(),
    }
}

/// Mutate NAT type
fn mutate_nat_type(nat_type: &NATType) -> NATType {
    let mut rng = rand::thread_rng();
    
    // Just pick a different NAT type
    match rng.gen_range(0..8) {
        0 => NATType::None,
        1 => NATType::FullCone,
        2 => NATType::RestrictedCone,
        3 => NATType::PortRestricted,
        4 => NATType::Symmetric,
        5 => NATType::Hairpin,
        6 => NATType::Double,
        _ => NATType::Carrier,
    }
}

/// Mutate mapping behavior
fn mutate_mapping_behavior(behavior: &MappingBehavior) -> MappingBehavior {
    let mut rng = rand::thread_rng();
    
    // Pick a different mapping behavior
    match rng.gen_range(0..3) {
        0 => MappingBehavior::Consistent,
        1 => MappingBehavior::PortPreserving,
        _ => MappingBehavior::Random,
    }
}

/// Mutate filtering behavior
fn mutate_filtering_behavior(behavior: &FilteringBehavior) -> FilteringBehavior {
    let mut rng = rand::thread_rng();
    
    // Pick a different filtering behavior
    match rng.gen_range(0..3) {
        0 => FilteringBehavior::Endpoint,
        1 => FilteringBehavior::Address,
        _ => FilteringBehavior::None,
    }
}

/// Mutate timeout value
fn mutate_timeout(timeout: u32) -> u32 {
    let mut rng = rand::thread_rng();
    
    // Choose a mutation strategy
    match rng.gen_range(0..4) {
        0 => {
            // Completely random timeout
            rng.gen()
        },
        1 => {
            // Slightly modify the timeout
            let factor = rng.gen_range(0.5..2.0);
            (timeout as f64 * factor) as u32
        },
        2 => {
            // Use common timeout values
            match rng.gen_range(0..4) {
                0 => 1000,     // 1 second
                1 => 5000,     // 5 seconds
                2 => 30000,    // 30 seconds
                _ => 60000,    // 1 minute
            }
        },
        3 => {
            // Use extreme values
            match rng.gen_range(0..2) {
                0 => 0,        // Very small (might cause issues)
                _ => 3600000,  // Very large (1 hour)
            }
        },
        _ => unreachable!(),
    }
} 