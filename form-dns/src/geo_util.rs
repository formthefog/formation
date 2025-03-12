use crate::geo_resolver::{GeoDnsResolver, GeoResolverConfig};
use std::net::IpAddr;
use trust_dns_proto::rr::RecordType;
use once_cell::sync::OnceCell;

// Global GeoDnsResolver instance
static GEO_RESOLVER: OnceCell<GeoDnsResolver> = OnceCell::new();

/// Initialize the global GeoDnsResolver with the specified configuration
pub fn init_geo_resolver(config: GeoResolverConfig) -> bool {
    if GEO_RESOLVER.get().is_some() {
        return false; // Already initialized
    }
    
    let resolver = GeoDnsResolver::new(config);
    GEO_RESOLVER.set(resolver).is_ok()
}

/// Get the global GeoDnsResolver instance, initializing it with default config if needed
pub fn get_geo_resolver() -> &'static GeoDnsResolver {
    GEO_RESOLVER.get_or_init(|| {
        let default_config = GeoResolverConfig::default();
        GeoDnsResolver::new(default_config)
    })
}

/// Sort IPs by proximity to client IP
pub fn sort_ips_by_client_location(
    domain: &str,
    record_type: RecordType,
    client_ip: Option<IpAddr>,
    ips: Vec<IpAddr>
) -> Vec<IpAddr> {
    if ips.is_empty() || client_ip.is_none() {
        return ips;
    }
    
    get_geo_resolver().get_geo_sorted_ips(domain, record_type, client_ip, ips)
}

/// Get client location from IP
pub fn get_client_location(client_ip: IpAddr) -> Option<crate::geolocation::GeoLocation> {
    get_geo_resolver().get_client_location(client_ip)
}

/// Apply geolocation sorting to DNS results
/// This function can be called from FormAuthority's lookup_local method
pub fn apply_geo_sorting(
    domain: &str,
    record_type: RecordType,
    client_ip: Option<IpAddr>,
    socket_addrs: Vec<std::net::SocketAddr>
) -> Vec<std::net::SocketAddr> {
    if socket_addrs.is_empty() || client_ip.is_none() {
        return socket_addrs;
    }
    
    // Extract IPs without port
    let ips: Vec<IpAddr> = socket_addrs.iter().map(|addr| addr.ip()).collect();
    
    // Sort IPs by proximity
    let sorted_ips = sort_ips_by_client_location(domain, record_type, client_ip, ips);
    
    // If no change in order, return original
    if sorted_ips.len() != socket_addrs.len() {
        return socket_addrs;
    }
    
    // Create a map of IP to original SocketAddr to preserve ports
    let addr_map: std::collections::HashMap<IpAddr, std::net::SocketAddr> = 
        socket_addrs.iter().map(|addr| (addr.ip(), *addr)).collect();
    
    // Rebuild socket addresses in the sorted order
    sorted_ips.into_iter()
        .filter_map(|ip| addr_map.get(&ip).cloned())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4};
    
    #[test]
    fn test_apply_geo_sorting_empty() {
        let addrs: Vec<SocketAddr> = vec![];
        let result = apply_geo_sorting(
            "example.com", 
            RecordType::A, 
            Some(IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1))),
            addrs
        );
        assert!(result.is_empty());
    }
    
    #[test]
    fn test_apply_geo_sorting_no_client_ip() {
        let addrs = vec![
            SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(1, 2, 3, 4), 80)),
            SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(5, 6, 7, 8), 80)),
        ];
        
        let result = apply_geo_sorting(
            "example.com", 
            RecordType::A, 
            None,
            addrs.clone()
        );
        
        assert_eq!(result, addrs);
    }
} 