use std::{collections::hash_map::Entry, net::IpAddr};

use crate::store::{FormDnsRecord, SharedStore};
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
        .with_state(state)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum DomainRequest {
    Create {
        domain: String,
        record_type: RecordType,
        ip_addr: Option<IpAddr>,
        cname_target: Option<String>
    },
    Update {
        record_type: RecordType,
        ip_addr: Option<IpAddr>,
        cname_target: Option<String>
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum DomainResponse {
    Success(Success),
    Failure(Option<String>),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Success {
    None,
    Some(FormDnsRecord),
    List(Vec<(String, FormDnsRecord)>)
}

async fn create_record(
    State(state): State<SharedStore>,
    Json(request): Json<DomainRequest>,
) -> Json<DomainResponse> {
    match request {
        DomainRequest::Create { domain, record_type, ip_addr, cname_target } => {
            let record = match record_type {
                RecordType::A => {
                    let (formnet_ip, public_ip) = if let Some(addr) = ip_addr {
                        match addr { 
                            IpAddr::V4(v4) if v4.octets()[0] == 10 => {
                                (ip_addr, None)
                            }
                            IpAddr::V4(_v4) => {
                                (None, ip_addr)
                            }
                            _ => return Json(DomainResponse::Failure(Some("IPV6 Addresses are not valid for A record".to_string()))),
                        }
                    } else {
                        return Json(DomainResponse::Failure(Some("A Record update requires an IP Address be provided".to_string())));
                    };
                    FormDnsRecord {
                        domain: domain.clone(),
                        record_type,
                        formnet_ip,
                        public_ip,
                        cname_target: None,
                        ttl: 3600
                    }
                }
                RecordType::AAAA => {
                    let public_ip = if let Some(ref _addr) = ip_addr {
                        ip_addr
                    } else {
                        return Json(DomainResponse::Failure(Some("AAAA Record updatte requires an IP address to be provided".to_string())));
                    };
                    FormDnsRecord {
                        domain: domain.clone(),
                        record_type,
                        formnet_ip: None,
                        public_ip,
                        cname_target: None,
                        ttl: 3600
                    }
                }
                RecordType::CNAME => {
                    let cname_target = if let Some(ref _target) = cname_target {
                        cname_target.clone()
                    } else {
                        return Json(DomainResponse::Failure(Some("CNAME Record update requires a CNAME target be provided".to_string())));
                    };

                    FormDnsRecord {
                        domain: domain.clone(),
                        record_type,
                        formnet_ip: None,
                        public_ip: None,
                        cname_target,
                        ttl: 3600
                    }
                }
                _ => return Json(DomainResponse::Failure(Some(format!("Sorry, the record type {record_type} is not currently supported"))))
            };

            let mut guard = match state.write() {
                Ok(g) => g,
                Err(e) => return Json(DomainResponse::Failure(Some(e.to_string())))
            };
            guard.insert(&domain, record);
            drop(guard);
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

    let mut guard = match state.write() {
        Ok(g) => g,
        Err(e) => return Json(DomainResponse::Failure(Some(e.to_string())))
    };

    match request {
        DomainRequest::Update { record_type, ip_addr, cname_target} => {
            let record = match record_type {
                RecordType::A => {
                    let record = if let Entry::Occupied(ref mut entry) = guard.entry(&domain) {
                        let record = entry.get_mut();
                        record.record_type = record_type;
                        if let Some(IpAddr::V4(ip)) = ip_addr {
                            if ip.octets()[0] == 10 {
                                record.formnet_ip = ip_addr;
                            } else {
                                record.public_ip = ip_addr;
                            }
                        } else if let Some(_) = ip_addr {
                            record.public_ip = ip_addr;
                        } else {
                            return Json(DomainResponse::Failure(Some("A Record update must include an IP Address".to_string())))
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
                        if let Some(ref _addr) = ip_addr {
                            record.public_ip = ip_addr;
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
            guard.insert(&domain, record);
            drop(guard);
            return Json(DomainResponse::Success(Success::None))
        }
        _ => return Json(DomainResponse::Failure(Some("Invalid request for endpoint /record/create".to_string())))
    }
}

async fn delete_record(
    State(state): State<SharedStore>,
    Path(domain): Path<String>,
) -> Json<DomainResponse> {
    let mut guard = match state.write() {
        Ok(g) => g,
        Err(e) => return Json(DomainResponse::Failure(Some(e.to_string())))
    };

    let removed = guard.remove(&domain);
    drop(guard);

    match removed {
        Some(ip_addr) => return Json(DomainResponse::Success(Success::Some(ip_addr))),
        None => return Json(DomainResponse::Failure(Some(format!("No record for domain {domain}"))))
    }

}

async fn get_record(
    State(state): State<SharedStore>,
    Path(domain): Path<String>
) -> Json<DomainResponse> {
    let guard = match state.read() {
        Ok(g) => g,
        Err(e) => return Json(DomainResponse::Failure(Some(e.to_string()))),
    };

    let opt = guard.get(&domain);

    match opt {
        Some(ip_addr) => return Json(DomainResponse::Success(Success::Some(ip_addr))),
        None => return Json(DomainResponse::Failure(Some(format!("Record does not exist for domain {domain}")))),
    }
}

async fn list_records(
    State(state): State<SharedStore>,
) -> Json<DomainResponse> {
    let guard = match state.read() {
        Ok(g) => g,
        Err(e) => return Json(DomainResponse::Failure(Some(e.to_string()))),
    };

    let cloned = guard.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
    drop(guard);

    return Json(DomainResponse::Success(Success::List(cloned)))
}

pub async fn serve_api(state: SharedStore) -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:3005").await?;
    let routes = build_routes(state);

    axum::serve(listener, routes).await?;

    Ok(())
}
