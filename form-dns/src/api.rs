use std::{collections::hash_map::Entry, net::{IpAddr, Ipv4Addr, SocketAddr}};

use crate::store::{FormDnsRecord, SharedStore, VerificationResult, VerificationStatus};
use serde::{Serialize, Deserialize};
use axum::{extract::{Path, State}, routing::{delete, get, post}, Json, Router};
use tokio::net::TcpListener;
use trust_dns_proto::rr::RecordType;

pub fn build_routes(state: SharedStore) -> Router {
    Router::new()
        .route("/record/create", post(create_record))
        .route("/record/:domain/update", post(update_record))
        .route("/record/:domain/delete", delete(delete_record))
        .route("/record/:domain/get", get(get_record))
        .route("/record/list", get(list_records))
        .route("/server/create", post(new_server))
        .route("/record/:domain/initiate_verification", post(initiate_verification))
        .route("/record/:domain/check_verification", post(check_verification))
        .route("/bootstrap/add", post(add_bootstrap_node))
        .route("/bootstrap/remove", post(remove_bootstrap_node))
        .route("/bootstrap/list", get(list_bootstrap_nodes))
        .with_state(state)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum DomainRequest {
    Create {
        domain: String,
        record_type: RecordType,
        ip_addr: Vec<SocketAddr>,
        cname_target: Option<String>,
        ssl_cert: bool,
    },
    Update {
        replace: bool,
        record_type: RecordType,
        ip_addr: Vec<SocketAddr>,
        cname_target: Option<String>,
        ssl_cert: bool,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum DomainResponse {
    Success(Success),
    Failure(Option<String>),
    VerificationSuccess(VerificationResult),
    VerificationFailure(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Success {
    None,
    Some(FormDnsRecord),
    List(Vec<(String, FormDnsRecord)>)
}

// New data types for bootstrap node management
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BootstrapNodeRequest {
    pub node_id: String,
    pub ip_address: IpAddr,
    pub region: Option<String>,
    pub ttl: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BootstrapNodeResponse {
    Success,
    Failure(String),
    NodesList(Vec<BootstrapNodeInfo>),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BootstrapNodeInfo {
    pub node_id: String,
    pub ip_address: IpAddr,
    pub region: Option<String>,
    pub ttl: u32,
    pub health_status: String,  // "healthy", "unhealthy", etc.
}

async fn create_record(
    State(state): State<SharedStore>,
    Json(request): Json<DomainRequest>,
) -> Json<DomainResponse> {
    log::info!("Received Create request..."); 
    match request {
        DomainRequest::Create { domain, record_type, ip_addr, cname_target, ssl_cert } => {
            log::info!("Create request for {domain}: {record_type}..."); 
            log::info!("Create ips?: {ip_addr:?}...");
            log::info!("Create CNAME target?: {cname_target:?}...");
            let record = match record_type {
                RecordType::A => {
                    let (formnet_ip, public_ip) = if !ip_addr.is_empty() {
                        let mut formnet_ips = vec![];
                        let mut public_ips = vec![];
                        for addr in ip_addr { 
                            match addr.ip() {
                                IpAddr::V4(v4) if v4.octets()[0] == 10 => {
                                    log::info!("Formnet IP: {v4}..."); 
                                    formnet_ips.push(addr);
                                }
                                IpAddr::V4(v4) => {
                                    log::info!("Public IP: {v4}..."); 
                                    public_ips.push(addr);
                                }
                                _ => return Json(DomainResponse::Failure(Some("IPV6 Addresses are not valid for A record".to_string()))),
                            }
                        }
                        (formnet_ips, public_ips)
                    } else {
                        return Json(DomainResponse::Failure(Some("A Record update requires an IP Address be provided".to_string())));
                    };
                    FormDnsRecord {
                        domain: domain.clone(),
                        record_type,
                        formnet_ip,
                        public_ip,
                        cname_target: None,
                        ssl_cert,
                        ttl: 3600,
                        verification_status: Some(VerificationStatus::NotVerified),
                        verification_timestamp: None,
                    }
                }
                RecordType::AAAA => {
                    let public_ip = if !ip_addr.is_empty() {
                        let mut public_ips = vec![];
                        for addr in ip_addr {
                            match addr.ip() {
                                IpAddr::V6(v6) => {
                                    log::info!("Public IP: {v6}..."); 
                                    public_ips.push(addr);
                                }
                                _ => {
                                    return Json(DomainResponse::Failure(Some("AAAA Record requires a V6 IP Address".to_string())));
                                }
                            }
                        }
                        public_ips
                    } else {
                        return Json(DomainResponse::Failure(Some("AAAA Record update requires an IP address to be provided".to_string())));
                    };
                    FormDnsRecord {
                        domain: domain.clone(),
                        record_type,
                        formnet_ip: vec![],
                        public_ip,
                        cname_target: None,
                        ssl_cert,
                        ttl: 3600,
                        verification_status: Some(VerificationStatus::NotVerified),
                        verification_timestamp: None,
                    }
                }
                RecordType::CNAME => {
                    let cname_target = if let Some(ref target) = cname_target {
                        log::info!("CNAME Target: {target}..."); 
                        cname_target.clone()
                    } else {
                        return Json(DomainResponse::Failure(Some("CNAME Record update requires a CNAME target be provided".to_string())));
                    };

                    FormDnsRecord {
                        domain: domain.clone(),
                        record_type,
                        formnet_ip: vec![],
                        public_ip: vec![],
                        cname_target,
                        ssl_cert,
                        ttl: 3600,
                        verification_status: Some(VerificationStatus::NotVerified),
                        verification_timestamp: None,
                    }
                }
                _ => return Json(DomainResponse::Failure(Some(format!("Sorry, the record type {record_type} is not currently supported"))))
            };

            log::info!("Build record: {record:?}...");
            let mut guard = state.write().await;
            log::info!("Adding record for {domain}...");
            guard.insert(&domain, record).await;
            drop(guard);
            log::info!("Domain {domain} record added successfully...");
            return Json(DomainResponse::Success(Success::None))
        },
        _ => return Json(DomainResponse::Failure(Some("Invalid request for endpoint /record/create".to_string())))
    }
}

async fn update_record(
    State(state): State<SharedStore>,
    Path(domain): Path<String>,
    Json(request): Json<DomainRequest>,
) -> Json<DomainResponse> {
    log::info!("Received Update request for {domain}...");
    let mut guard = state.write().await;
    match request {
        DomainRequest::Update { replace, record_type, ip_addr, cname_target, ssl_cert} => {
            let record = match record_type {
                RecordType::A => {
                    let record = if let Entry::Occupied(ref mut entry) = guard.entry(&domain) {
                        let record = entry.get_mut();
                        record.record_type = record_type;
                        let (formnet_ips, public_ips) = if !ip_addr.is_empty() {
                            let mut formnet_ips = vec![]; 
                            let mut public_ips = vec![];
                            for addr in ip_addr {
                                match addr.ip() {
                                    IpAddr::V4(ip) => {
                                        if ip.octets()[0] == 10 {
                                            formnet_ips.push(addr);
                                        } else {
                                            public_ips.push(addr);
                                        }
                                    }
                                    _ => return Json(DomainResponse::Failure(Some("A Records require an IPV4 address".to_string())))
                                }
                            }
                            (formnet_ips, public_ips)
                        } else if !ip_addr.is_empty() {
                            (vec![], ip_addr)
                        } else {
                            return Json(DomainResponse::Failure(Some("A Record update must include an IP Address".to_string())))
                        };
                        if replace {
                            record.formnet_ip = formnet_ips;
                            record.public_ip = public_ips;
                            record.ssl_cert = ssl_cert;
                        } else {
                            record.formnet_ip.extend(formnet_ips);
                            record.public_ip.extend(public_ips);
                            record.ssl_cert = ssl_cert;
                        }
                        record.clone()
                    } else {
                        return Json(DomainResponse::Failure(Some("A Record updates can only occur if the record exists, use /record/create endpoint instead".to_string())))
                    };
                    record
                }
                RecordType::AAAA => {
                    let record = if let Entry::Occupied(ref mut entry) = guard.entry(&domain) {
                        let record = entry.get_mut();
                        record.record_type = record_type;
                        if !ip_addr.is_empty() {
                            record.public_ip.extend(ip_addr);
                            record.ssl_cert = ssl_cert;
                        } else {
                            return Json(DomainResponse::Failure(Some("AAAA Record updates must include an IP Address".to_string())));
                        }
                        record.clone()
                    } else {
                        return Json(DomainResponse::Failure(Some("AAAA Record update can only occur if the record exists, use /record/create endpoint instead".to_string())))
                    };
                    record
                }
                RecordType::CNAME => {
                    let record = if let Entry::Occupied(ref mut entry) = guard.entry(&domain) {
                        let record = entry.get_mut();
                        record.record_type = record_type;
                        if let Some(ref _target) = cname_target {
                            record.cname_target = cname_target.clone();
                            record.ssl_cert = ssl_cert;
                        } else {
                            return Json(DomainResponse::Failure(Some("CNAME Record update must include a CNAME target".to_string())))
                        }
                        record.clone()
                    } else {
                        return Json(DomainResponse::Failure(Some("CNAME record updates can only occur if the record exists, use /record/create endpoint instead".to_string()))) 
                    };
                    record
                }
                _ => return Json(DomainResponse::Failure(Some(format!("Sorry, the record type {record_type} is not currently supported"))))

            };
            log::info!("Successfully built record {record:?}");
            guard.insert(&domain, record).await;
            drop(guard);
            log::info!("Successfully updated record for {domain}");
            return Json(DomainResponse::Success(Success::None))
        }
        _ => return Json(DomainResponse::Failure(Some("Invalid request for endpoint /record/create".to_string())))
    }
}

async fn delete_record(
    State(state): State<SharedStore>,
    Path(domain): Path<String>,
) -> Json<DomainResponse> {
    log::info!("Received request to delete record for {domain}...");
    let mut guard = state.write().await;
    let removed = guard.remove(&domain);
    drop(guard);
    log::info!("Successfully removed record for {domain}...");

    match removed {
        Some(ip_addr) => return Json(DomainResponse::Success(Success::Some(ip_addr))),
        None => return Json(DomainResponse::Failure(Some(format!("No record for domain {domain}"))))
    }

}

async fn get_record(
    State(state): State<SharedStore>,
    Path(domain): Path<String>
) -> Json<DomainResponse> {
    log::info!("Received Get request for {domain}"); 
    let guard = state.read().await;
    let opt = guard.get(&domain);

    match opt {
        Some(ip_addr) => {
            log::info!("Record for {domain} found, returning..."); 
            return Json(DomainResponse::Success(Success::Some(ip_addr)))
        }
        None => return Json(DomainResponse::Failure(Some(format!("Record does not exist for domain {domain}")))),
    }
}

async fn list_records(
    State(state): State<SharedStore>,
) -> Json<DomainResponse> {
    log::info!("Received List request");
    let guard = state.read().await; 
    let cloned: Vec<(String, FormDnsRecord)> = guard.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
    drop(guard);
    log::info!("Returning records list with {} records...", cloned.len());

    return Json(DomainResponse::Success(Success::List(cloned)))
}

async fn new_server(
    State(state): State<SharedStore>,
    Json(ip_addr): Json<Ipv4Addr>
) -> Json<()> {
    let mut guard = state.write().await;
    if let Err(e) = guard.add_server(ip_addr) {
        log::error!("Error trying to add server {}: {}", ip_addr.clone(), e);
    }

    Json(())
}

/// Endpoint to initiate domain verification
async fn initiate_verification(
    State(state): State<SharedStore>,
    Path(domain): Path<String>,
) -> Json<DomainResponse> {
    log::info!("Received initiate_verification request for domain: {}", domain);
    
    let mut store = state.write().await;
    match store.initiate_verification(&domain).await {
        Ok(result) => {
            Json(DomainResponse::VerificationSuccess(result))
        },
        Err(err) => {
            log::error!("Domain verification initiation failed: {}", err);
            Json(DomainResponse::VerificationFailure(err))
        }
    }
}

/// Endpoint to check verification status
async fn check_verification(
    State(state): State<SharedStore>,
    Path(domain): Path<String>,
) -> Json<DomainResponse> {
    log::info!("Received check_verification request for domain: {}", domain);
    
    let mut store = state.write().await;
    match store.check_verification(&domain).await {
        Ok(result) => {
            Json(DomainResponse::VerificationSuccess(result))
        },
        Err(err) => {
            log::error!("Domain verification check failed: {}", err);
            Json(DomainResponse::VerificationFailure(err))
        }
    }
}

/// Add a new bootstrap node to the bootstrap domain
async fn add_bootstrap_node(
    State(state): State<SharedStore>,
    Json(request): Json<BootstrapNodeRequest>,
) -> Json<BootstrapNodeResponse> {
    log::info!("Received request to add bootstrap node: {} at {}", 
               request.node_id, request.ip_address);
    
    let domain = "bootstrap.formation.cloud";
    let mut guard = state.write().await;
    
    // Create a socket address from the IP and default WireGuard port
    let socket_addr = SocketAddr::new(request.ip_address, 51820);
    
    // Check if the bootstrap domain record exists
    if let Some(mut record) = guard.get(domain).clone() {
        // Update existing record
        if !record.public_ip.contains(&socket_addr) {
            record.public_ip.push(socket_addr);
            
            // Set custom TTL if provided
            if let Some(ttl) = request.ttl {
                record.ttl = ttl;
            } else {
                // Use a low TTL for bootstrap domain for faster failover
                record.ttl = 60;
            }
            
            // Set verification status to verified
            record.verification_status = Some(VerificationStatus::Verified);
            
            // Save the updated record
            guard.insert(domain, record).await;
            log::info!("Added bootstrap node {} to domain {}", request.ip_address, domain);
            return Json(BootstrapNodeResponse::Success);
        } else {
            return Json(BootstrapNodeResponse::Failure(
                format!("Bootstrap node {} already exists", request.ip_address)
            ));
        }
    } else {
        // Create a new bootstrap domain record
        let record = FormDnsRecord {
            domain: domain.to_string(),
            record_type: RecordType::A,
            formnet_ip: vec![],
            public_ip: vec![socket_addr],
            cname_target: None,
            ssl_cert: false,
            ttl: request.ttl.unwrap_or(60), // Low TTL for bootstrap domain
            verification_status: Some(VerificationStatus::Verified),
            verification_timestamp: None,
        };
        
        guard.insert(domain, record).await;
        log::info!("Created bootstrap domain record with node {}", request.ip_address);
        return Json(BootstrapNodeResponse::Success);
    }
}

/// Remove a bootstrap node from the bootstrap domain
async fn remove_bootstrap_node(
    State(state): State<SharedStore>,
    Json(request): Json<BootstrapNodeRequest>,
) -> Json<BootstrapNodeResponse> {
    log::info!("Received request to remove bootstrap node: {}", request.ip_address);
    
    let domain = "bootstrap.formation.cloud";
    let mut guard = state.write().await;
    
    // Create a socket address from the IP and default WireGuard port
    let socket_addr = SocketAddr::new(request.ip_address, 51820);
    
    if let Some(mut record) = guard.get(domain).clone() {
        // Remove the node from the list
        let original_len = record.public_ip.len();
        record.public_ip.retain(|addr| addr != &socket_addr);
        
        if record.public_ip.len() < original_len {
            // Save the updated record
            guard.insert(domain, record).await;
            log::info!("Removed bootstrap node {} from domain {}", request.ip_address, domain);
            return Json(BootstrapNodeResponse::Success);
        } else {
            return Json(BootstrapNodeResponse::Failure(
                format!("Bootstrap node {} not found", request.ip_address)
            ));
        }
    } else {
        return Json(BootstrapNodeResponse::Failure(
            format!("Bootstrap domain {} not found", domain)
        ));
    }
}

/// List all bootstrap nodes
async fn list_bootstrap_nodes(
    State(state): State<SharedStore>,
) -> Json<BootstrapNodeResponse> {
    log::info!("Received request to list bootstrap nodes");
    
    let domain = "bootstrap.formation.cloud";
    let guard = state.read().await;
    
    if let Some(record) = guard.get(domain) {
        // Get the health repository to check node health status
        let health_repo = guard.get_health_repository();
        let nodes_info = if let Some(health_repo) = health_repo {
            let health_guard = health_repo.read().await;
            
            record.public_ip.iter().map(|addr| {
                let ip = addr.ip();
                let health_status = if health_guard.is_available(&ip) {
                    "healthy"
                } else {
                    "unhealthy"
                }.to_string();
                
                BootstrapNodeInfo {
                    node_id: format!("node-{}", ip), // Use IP as default node ID
                    ip_address: ip,
                    region: None, // We don't store region info yet
                    ttl: record.ttl,
                    health_status,
                }
            }).collect()
        } else {
            record.public_ip.iter().map(|addr| {
                BootstrapNodeInfo {
                    node_id: format!("node-{}", addr.ip()),
                    ip_address: addr.ip(),
                    region: None,
                    ttl: record.ttl,
                    health_status: "unknown".to_string(),
                }
            }).collect()
        };
        
        return Json(BootstrapNodeResponse::NodesList(nodes_info));
    } else {
        return Json(BootstrapNodeResponse::NodesList(vec![]));
    }
}

pub async fn serve_api(state: SharedStore) -> Result<(), Box<dyn std::error::Error>> {
    log::info!("Launching DNS server API");
    let listener = TcpListener::bind("127.0.0.1:3005").await?;
    log::info!("Binding listener to localhost port 3005...");
    let routes = build_routes(state);
    log::info!("Building endpoints...");

    log::info!("DNS server api listening on localhost:3005...");
    axum::serve(listener, routes).await?;

    Ok(())
}
