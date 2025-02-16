use std::{collections::{HashMap, HashSet}, net::{IpAddr, SocketAddr}, sync::Arc, time::{Duration, SystemTime, UNIX_EPOCH}};
use axum::{extract::{State, Path}, routing::{get, post}, Json, Router};
use form_dns::{api::{DomainRequest, DomainResponse}, store::FormDnsRecord};
use form_p2p::queue::{QueueRequest, QueueResponse, QUEUE_PORT};
use rand::{seq::SliceRandom, thread_rng};
use reqwest::Client;
use serde_json::Value;
use shared::{Association, AssociationContents, Cidr, CidrContents, Peer, PeerContents};
use tiny_keccak::{Hasher, Sha3};
use tokio::{net::TcpListener, sync::Mutex};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use crdts::{bft_reg::Update, map::Op, BFTReg, CvRDT, Map};
use trust_dns_proto::rr::RecordType;
use url::Host;
use crate::{instances::{ClusterMember, Instance, InstanceOp, InstanceState}, network::{AssocOp, CidrOp, CrdtAssociation, CrdtCidr, CrdtDnsRecord, CrdtPeer, DnsOp, NetworkState, PeerOp}, nodes::{Node, NodeOp, NodeState}};
use form_types::state::{Response, Success};

pub type PeerMap = Map<String, BFTReg<CrdtPeer<String>, String>, String>;
pub type CidrMap = Map<String, BFTReg<CrdtCidr<String>, String>, String>;
pub type AssocMap = Map<String, BFTReg<CrdtAssociation<String>, String>, String>;
pub type DnsMap = Map<String, BFTReg<CrdtDnsRecord, String>, String>;
pub type InstanceMap = Map<String, BFTReg<Instance, String>, String>;
pub type NodeMap = Map<String, BFTReg<Node, String>, String>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MergeableNetworkState {
    peers: PeerMap,
    cidrs: CidrMap,
    assocs: AssocMap,
    dns: DnsMap,
}

impl From<NetworkState> for MergeableNetworkState {
    fn from(value: NetworkState) -> Self {
        MergeableNetworkState {
            peers: value.peers.clone(),
            cidrs: value.cidrs.clone(),
            assocs: value.associations.clone(),
            dns: value.dns_state.zones.clone()
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MergeableState {
    peers: PeerMap,
    cidrs: CidrMap,
    assocs: AssocMap,
    dns: DnsMap,
    instances: InstanceMap,
    nodes: NodeMap
}

impl From<DataStore> for MergeableState {
    fn from(value: DataStore) -> Self {
        MergeableState {
            peers: value.network_state.peers.clone(),
            cidrs: value.network_state.cidrs.clone(),
            assocs: value.network_state.associations.clone(),
            dns: value.network_state.dns_state.zones.clone(),
            instances: value.instance_state.map.clone(),
            nodes: value.node_state.map.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DataStore {
    network_state: NetworkState,
    instance_state: InstanceState,
    node_state: NodeState
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PeerRequest {
    Op(PeerOp<String>),
    Join(PeerContents<String>),
    Update(PeerContents<String>),
    Delete(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CidrRequest {
    Op(CidrOp<String>),
    Create(CidrContents<String>),
    Update(CidrContents<String>),
    Delete(String),
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AssocRequest {
    Op(AssocOp<String>), 
    Create(AssociationContents<String>),
    Delete((String, String)),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum DnsRequest {
    Op(DnsOp),
    Create(FormDnsRecord),
    Update(FormDnsRecord),
    Delete(String)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum InstanceRequest {
    Op(InstanceOp),
    Create(Instance),
    Update(Instance),
    Delete(String),
    AddClusterMember {
        build_id: String,
        cluster_member: ClusterMember
    },
    RemoveClusterMember {
        build_id: String,
        cluster_member_id: String, 
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NodeRequest {
    Op(NodeOp),
    Create(Node),
    Update(Node),
    Delete(String),
}

impl DataStore {
    pub fn new(node_id: String, pk: String) -> Self {
        let network_state = NetworkState::new(node_id.clone(), pk.clone());
        let instance_state = InstanceState::new(node_id.clone(), pk.clone());
        let node_state = NodeState::new(node_id.clone(), pk.clone());

        Self { network_state, instance_state, node_state }
    }

    pub fn new_from_state(
        node_id: String,
        pk: String,
        other: MergeableState,
    ) -> Self {
        log::info!("Building new datastore from state...");
        let mut local = Self::new(node_id, pk); 
        local.network_state.peers.merge(other.peers);
        local.network_state.cidrs.merge(other.cidrs);
        local.network_state.associations.merge(other.assocs);
        local.network_state.dns_state.zones.merge(other.dns);
        local.instance_state.map.merge(other.instances);
        local.node_state.map.merge(other.nodes);
        log::info!("Built new datastore from state... Returning...");
        local
    }

    pub fn get_all_users(&self) -> HashMap<String, CrdtPeer<String>> {
        log::info!("Getting all peers from datastore network state...");
        self.network_state.peers.iter().filter_map(|item| {
            match item.val.1.val() {
                Some(v) => Some((item.val.0.clone(), v.value().clone())),
                None => None
            }
        }).collect()
    }

    pub fn get_all_cidrs(&self) -> HashMap<String, CrdtCidr<String>> {
        log::info!("Getting all cidrs from datastore network state...");
        log::info!("CIDRS: {:?}", self.network_state.cidrs);
        self.network_state.cidrs.iter().filter_map(|item| {
            match item.val.1.val() {
                Some(v) => Some((item.val.0.clone(), v.value().clone())),
                None => None
            }
        }).collect()
    }

    pub fn get_all_assocs(&self) -> HashMap<String, CrdtAssociation<String>> {
        log::info!("Getting all associations from datastore network state...");
        self.network_state.associations.iter().filter_map(|item| {
            match item.val.1.val() {
                Some(v) => {
                    Some((item.val.0.clone(), v.value().clone()))
                },
                None => None
            }
        }).collect()
    }

    pub fn get_relationships(&self, cidr_id: String) -> Vec<(Cidr<String>, Cidr<String>)> {
        log::info!("Getting relationships for {cidr_id} from datastore network state...");
        let mut assoc_ids = self.get_all_assocs();
        assoc_ids.retain(|k, _| *k == cidr_id);
        let ids: HashSet<String> = assoc_ids.iter().map(|(k, _)| k.clone()).collect();
        ids.iter().filter_map(|id| {
            let split: Vec<&str> = id.split("-").collect();
            let cidr_id_1 = split[0];
            let cidr_id_2 = split[1];
            let cidr_1 = self.network_state.cidrs.get(&cidr_id_1.to_string()).val;
            let cidr_2 = self.network_state.cidrs.get(&cidr_id_2.to_string()).val;
            match (cidr_1, cidr_2) {
                (Some(reg_1), Some(reg_2)) => {
                    let val1 = reg_1.val();
                    let val2 = reg_2.val();
                    match (val1, val2) {
                        (Some(node1), Some(node2)) => {
                            return Some((node1.value().into(), node2.value().into()))
                        }
                        _ => None,
                    }
                }
                _ => None,
            }
        }).collect()
    }

    pub fn get_all_active_admin(&mut self) -> HashMap<String, CrdtPeer<String>> {
        log::info!("Getting all active admins from datastore network state...");
        self.network_state.peers.iter().filter_map(|item| {
            match item.val.1.val() {
                Some(v) => {
                    if v.value().is_admin() {
                        Some((item.val.0.clone(), v.value().clone()))
                    } else {
                        None
                    }
                }
                None => None,
            }
        }).collect()
    }

    async fn handle_peer_request(&mut self, peer_request: PeerRequest) -> Result<(), Box<dyn std::error::Error>> {
        match peer_request {
            PeerRequest::Op(op) => self.handle_peer_op(op).await?,
            PeerRequest::Join(join) => self.handle_peer_join(join).await?,
            PeerRequest::Update(up) => self.handle_peer_update(up).await?,
            PeerRequest::Delete(del) => self.handle_peer_delete(del).await?,
        }

        Ok(())
    }

    async fn handle_peer_op(&mut self, peer_op: PeerOp<String>) -> Result<(), Box<dyn std::error::Error>> {
        match &peer_op {
            Op::Up { dot: _, key, op } => {
                self.network_state.peer_op(peer_op.clone());
                if let (true, _) = self.network_state.peer_op_success(key.clone(), op.clone()) {
                    log::info!("Peer Op succesffully applied...");
                    DataStore::write_to_queue(PeerRequest::Op(peer_op.clone()), 0).await?;
                } else {
                    log::info!("Peer Op rejected...");
                    return Err(
                        Box::new(
                            std::io::Error::new(
                                std::io::ErrorKind::Other,
                                "update was rejected".to_string()
                            )
                        )
                    )
                }
            }
            Op::Rm { .. } => {
                self.network_state.peer_op(peer_op.clone());
                return Ok(());
            }
        }

        Ok(())
    }

    async fn handle_peer_join(&mut self, contents: PeerContents<String>) -> Result<(), Box<dyn std::error::Error>> {
        let op = self.network_state.update_peer_local(contents);
        self.handle_peer_op(op).await?;

        Ok(())
    }

    async fn handle_peer_update(&mut self, contents: PeerContents<String>) -> Result<(), Box<dyn std::error::Error>> {
        let op = self.network_state.update_peer_local(contents);
        self.handle_peer_op(op).await?;

        Ok(())
    }

    async fn handle_peer_delete(&mut self, id: String) -> Result<(), Box<dyn std::error::Error>> {
        let op = self.network_state.remove_peer_local(id);
        self.handle_peer_op(op).await?;

        Ok(())
    }

    async fn handle_cidr_request(&mut self, cidr_request: CidrRequest) -> Result<(), Box<dyn std::error::Error>> {
        match cidr_request {
            CidrRequest::Op(op) => self.handle_cidr_op(op).await?,
            CidrRequest::Create(create) => self.handle_cidr_create(create).await?,
            CidrRequest::Update(update) => self.handle_cidr_update(update).await?,
            CidrRequest::Delete(delete) => self.handle_cidr_delete(delete).await?,
        }

        Ok(())
    }

    async fn handle_cidr_op(&mut self, cidr_op: CidrOp<String>) -> Result<(), Box<dyn std::error::Error>> {
        match &cidr_op {
            Op::Up { dot: _, key, op } => {
                self.network_state.cidr_op(cidr_op.clone());
                if let (true, _) = self.network_state.cidr_op_success(key.clone(), op.clone()) {
                    log::info!("CIDR Op succesffully applied...");
                    DataStore::write_to_queue(CidrRequest::Op(cidr_op.clone()), 1).await?;
                } else {
                    log::info!("CIDR Op rejected...");
                    return Err(
                        Box::new(
                            std::io::Error::new(
                                std::io::ErrorKind::Other,
                                "update was rejected".to_string()
                            )
                        )
                    )
                }
            }
            Op::Rm { .. } => {
                self.network_state.cidr_op(cidr_op.clone());
                return Ok(());
            }
        }
        Ok(())
    }

    async fn handle_cidr_create(&mut self, create: CidrContents<String>) -> Result<(), Box<dyn std::error::Error>> {
        let op = self.network_state.update_cidr_local(create);
        self.handle_cidr_op(op).await?;

        Ok(())
    }

    async fn handle_cidr_update(&mut self, update: CidrContents<String>) -> Result<(), Box<dyn std::error::Error>> {
        let op = self.network_state.update_cidr_local(update);
        self.handle_cidr_op(op).await?;

        Ok(())
    }

    async fn handle_cidr_delete(&mut self, delete: String) -> Result<(), Box<dyn std::error::Error>> {
        let op = self.network_state.remove_cidr_local(delete);
        self.handle_cidr_op(op).await?;

        Ok(())
    }

    async fn handle_assoc_request(&mut self, assoc_request: AssocRequest) -> Result<(), Box<dyn std::error::Error>> {
        match assoc_request {
            AssocRequest::Op(op) => self.handle_assoc_op(op).await?,
            AssocRequest::Create(create) => self.handle_assoc_create(create).await?,
            AssocRequest::Delete(delete) => self.handle_assoc_delete(delete).await?,
        }
        Ok(())
    }

    async fn handle_assoc_op(&mut self, assoc_op: AssocOp<String>) -> Result<(), Box<dyn std::error::Error>> {
        match &assoc_op {
            Op::Up { dot: _, key, op } => {
                self.network_state.associations_op(assoc_op.clone());
                if let (true, _) = self.network_state.associations_op_success(key.clone(), op.clone()) {
                    log::info!("Assoc Op succesffully applied...");
                    DataStore::write_to_queue(AssocRequest::Op(assoc_op.clone()), 2).await?;
                } else {
                    log::info!("Assoc Op rejected...");
                    return Err(
                        Box::new(
                            std::io::Error::new(
                                std::io::ErrorKind::Other,
                                "update was rejected".to_string()
                            )
                        )
                    )
                }
            }
            Op::Rm { .. } => {
                self.network_state.associations_op(assoc_op.clone());
                return Ok(());
            }
        }
        Ok(())
    }

    async fn handle_assoc_create(&mut self, create: AssociationContents<String>) -> Result<(), Box<dyn std::error::Error>> {
        let op = self.network_state.update_association_local(create);
        self.handle_assoc_op(op).await?;

        Ok(())
    }

    async fn handle_assoc_delete(&mut self, delete: (String, String)) -> Result<(), Box<dyn std::error::Error>> {
        let op = self.network_state.remove_association_local(delete);
        self.handle_assoc_op(op).await?;

        Ok(())
    }

    async fn handle_dns_request(&mut self, dns_request: DnsRequest) -> Result<(), Box<dyn std::error::Error>> {
        match dns_request {
            DnsRequest::Op(op) => self.handle_dns_op(op).await?,
            DnsRequest::Create(create) => self.handle_dns_create(create).await?,
            DnsRequest::Update(update) => self.handle_dns_update(update).await?,
            DnsRequest::Delete(domain) => self.handle_dns_delete(domain).await? 
        }

        Ok(())
    }

    async fn handle_dns_op(&mut self, dns_op: DnsOp) -> Result<(), Box<dyn std::error::Error>> {
        match &dns_op {
            Op::Up { dot: _, key, op } => {
                self.network_state.dns_op(dns_op.clone());
                if let (true, _) = self.network_state.dns_op_success(key.clone(), op.clone()) {
                    log::info!("DNS Op succesffully applied...");
                    DataStore::write_to_queue(DnsRequest::Op(dns_op.clone()), 3).await?;
                } else {
                    log::info!("DNS Op rejected...");
                    return Err(
                        Box::new(
                            std::io::Error::new(
                                std::io::ErrorKind::Other,
                                "update was rejected".to_string()
                            )
                        )
                    )
                }
            }
            Op::Rm { .. } => {
                self.network_state.dns_op(dns_op.clone());
                return Ok(());
            }
        }

        Ok(())
    }

    async fn handle_dns_create(&mut self, create: FormDnsRecord) -> Result<(), Box<dyn std::error::Error>> {
        let op = self.network_state.update_dns_local(create.clone());
        self.handle_dns_op(op).await?;
        let instance_ips = create.formnet_ip.clone(); 
        let mut ip_iter = instance_ips.iter();
        while let Some(ip) = ip_iter.next() {
            let mut instance = self.instance_state.get_instance_by_ip(ip.ip())?;
            instance.dns_record = Some(create.clone());
            self.handle_instance_update(instance).await?;

        }
        let mut ip_addr = create.formnet_ip.clone();
        ip_addr.extend(create.public_ip.clone());
        let request = DomainRequest::Create { 
            domain: create.domain.clone(), 
            record_type: create.record_type, 
            ip_addr, 
            cname_target: create.cname_target.clone(), 
            ssl_cert: create.ssl_cert, 
        };

        Client::new()
            .post("http://127.0.0.1:3005/record/create")
            .json(&request)
            .send().await?
            .json::<DomainResponse>().await?;

        Ok(())
    }

    async fn handle_dns_update(&mut self, update: FormDnsRecord) -> Result<(), Box<dyn std::error::Error>> {
        let op = self.network_state.update_dns_local(update.clone());
        self.handle_dns_op(op).await?;

        let instance_ips = update.formnet_ip.clone(); 
        let mut ip_iter = instance_ips.iter();
        let mut replace = false;
        while let Some(ip) = ip_iter.next() {
            let mut instance = self.instance_state.get_instance_by_ip(ip.ip())?;
            if let Some(ref mut record) = &mut instance.dns_record {
                record.formnet_ip.extend(update.formnet_ip.clone()); 
                record.public_ip.extend(update.public_ip.clone());
                record.cname_target = update.cname_target.clone();
                if record.record_type != update.record_type {
                    replace = true;
                    record.record_type = update.record_type;
                }

                if record.domain != update.domain.clone() {
                    replace = true;
                    record.domain = update.domain.clone();
                }

                record.ssl_cert = update.ssl_cert;
                record.ttl = update.ttl;
                
            }
            self.handle_instance_update(instance).await?;

        }
        let mut ip_addr = update.formnet_ip.clone();
        ip_addr.extend(update.public_ip.clone());
        let request = DomainRequest::Update { 
            replace,
            record_type: update.record_type, 
            ip_addr, 
            cname_target: update.cname_target.clone(), 
            ssl_cert: update.ssl_cert, 
        };

        Client::new()
            .post(format!("http://127.0.0.1:3005/record/{}/update", update.domain))
            .json(&request)
            .send().await?
            .json::<DomainResponse>().await?;

        Ok(())
    }

    async fn handle_dns_delete(&mut self, delete: String) -> Result<(), Box<dyn std::error::Error>> {
        let op = self.network_state.remove_dns_local(delete.clone());
        self.handle_dns_op(op).await?;

        Client::new()
            .post(format!("http://127.0.0.1:3005/record/{}/delete", delete))
            .send().await?
            .json::<DomainResponse>().await?;

        Ok(())
    }

    async fn handle_instance_request(&mut self, instance_request: InstanceRequest) -> Result<(), Box<dyn std::error::Error>> {
        match instance_request {
            InstanceRequest::Op(op) => self.handle_instance_op(op).await?,
            InstanceRequest::Create(create) => self.handle_instance_create(create).await?,
            InstanceRequest::Update(update) => self.handle_instance_update(update).await?,
            InstanceRequest::Delete(id) => self.handle_instance_delete(id).await?,
            InstanceRequest::AddClusterMember { build_id, cluster_member }  => self.handle_add_cluster_member(build_id, cluster_member).await?,
            InstanceRequest::RemoveClusterMember { build_id, cluster_member_id }  => self.handle_remove_cluster_member(build_id, cluster_member_id).await?,
        }

        Ok(())
    }

    async fn handle_add_cluster_member(
        &mut self,
        build_id: String,
        cluster_member: ClusterMember
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut instances = self.instance_state.get_instances_by_build_id(build_id);
        let mut iter_mut = instances.iter_mut();
        while let Some(instance) = iter_mut.next() {
            instance.cluster.insert(cluster_member.clone());
            let instance_op = self.instance_state.update_instance_local(instance.clone());
            match instance_op {
                Op::Up { dot: _, ref key, ref op } => {
                    let _ = self.instance_state.instance_op(instance_op.clone());
                    if let (true, _) = self.instance_state.instance_op_success(key.to_string(), op.clone()) {
                        log::info!("Instance Op succesfully applied...");
                        DataStore::write_to_queue(InstanceRequest::Op(instance_op.clone()), 4).await?;
                    } else {
                        return Err(
                            Box::new(
                                std::io::Error::new(
                                    std::io::ErrorKind::Other,
                                    "update was rejected".to_string()
                                )
                            )
                        )
                    }
                }
                Op::Rm { .. } => {
                    self.instance_state.instance_op(instance_op.clone());
                    return Ok(());
                }
            }
        }
        Ok(())
    }

    async fn handle_remove_cluster_member(
        &mut self,
        build_id: String,
        cluster_member_id: String
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut instances = self.instance_state.get_instances_by_build_id(build_id);
        let mut iter_mut = instances.iter_mut();
        while let Some(instance) = iter_mut.next() {
            instance.cluster.remove(&cluster_member_id);
            let instance_op = self.instance_state.update_instance_local(instance.clone());
            match instance_op {
                Op::Up { dot: _, ref key, ref op } => {
                    let _ = self.instance_state.instance_op(instance_op.clone());
                    if let (true, _) = self.instance_state.instance_op_success(key.to_string(), op.clone()) {
                        log::info!("Instance Op succesfully applied...");
                        DataStore::write_to_queue(InstanceRequest::Op(instance_op.clone()), 4).await?;
                    } else {
                        return Err(
                            Box::new(
                                std::io::Error::new(
                                    std::io::ErrorKind::Other,
                                    "update was rejected".to_string()
                                )
                            )
                        )
                    }
                }
                Op::Rm { .. } => {
                    self.instance_state.instance_op(instance_op.clone());
                    return Ok(());
                }
            }
        }
        Ok(())
    }

    async fn handle_instance_op(&mut self, instance_op: InstanceOp) -> Result<(), Box<dyn std::error::Error>> {
        match &instance_op {
            Op::Up { dot: _, key, op } => {
                self.instance_state.instance_op(instance_op.clone());
                if let (true, _) = self.instance_state.instance_op_success(key.clone(), op.clone()) {
                    log::info!("Instance Op succesffully applied...");
                    DataStore::write_to_queue(InstanceRequest::Op(instance_op.clone()), 4).await?;
                } else {
                    log::info!("Instance Op rejected...");
                    return Err(
                        Box::new(
                            std::io::Error::new(
                                std::io::ErrorKind::Other,
                                "update was rejected".to_string()
                            )
                        )
                    )
                }
            }
            Op::Rm { .. } => {
                self.instance_state.instance_op(instance_op.clone());
                return Ok(());
            }
        }
        Ok(())
    }

    async fn handle_instance_create(&mut self, create: Instance) -> Result<(), Box<dyn std::error::Error>> {
        let op = self.instance_state.update_instance_local(create);
        self.handle_instance_op(op).await?;

        Ok(())
    }

    async fn handle_instance_update(&mut self, update: Instance) -> Result<(), Box<dyn std::error::Error>> {
        let op = self.instance_state.update_instance_local(update);
        self.handle_instance_op(op).await?;

        Ok(())
    }

    async fn handle_instance_delete(&mut self, delete: String) -> Result<(), Box<dyn std::error::Error>> {
        let op = self.instance_state.remove_instance_local(delete);
        self.handle_instance_op(op).await?;

        Ok(())
    }

    async fn handle_node_request(&mut self, node_request: NodeRequest) -> Result<(), Box<dyn std::error::Error>> {
        match node_request {
            NodeRequest::Op(op) => self.handle_node_op(op).await?,
            NodeRequest::Create(create) => self.handle_node_create(create).await?,
            NodeRequest::Update(update) => self.handle_node_update(update).await?,
            NodeRequest::Delete(id) => self.handle_node_delete(id).await? 
        }
        Ok(())
    }

    async fn handle_node_op(&mut self, node_op: NodeOp) -> Result<(), Box<dyn std::error::Error>> {
        match &node_op {
            Op::Up { dot: _, key, op } => {
                self.node_state.node_op(node_op.clone());
                if let (true, _) = self.node_state.node_op_success(key.clone(), op.clone()) {
                    log::info!("Peer Op succesffully applied...");
                    DataStore::write_to_queue(NodeRequest::Op(node_op.clone()), 5).await?;
                } else {
                    log::info!("Peer Op rejected...");
                    return Err(
                        Box::new(
                            std::io::Error::new(
                                std::io::ErrorKind::Other,
                                "update was rejected".to_string()
                            )
                        )
                    )
                }
            }
            Op::Rm { .. } => {
                self.node_state.node_op(node_op.clone());
                return Ok(());
            }
        }
        Ok(())
    }

    async fn handle_node_create(&mut self, create: Node) -> Result<(), Box<dyn std::error::Error>> {
        let op = self.node_state.update_node_local(create);
        self.handle_node_op(op).await?;

        Ok(())
    }

    async fn handle_node_update(&mut self, update: Node) -> Result<(), Box<dyn std::error::Error>> {
        let op = self.node_state.update_node_local(update);
        self.handle_node_op(op).await?;

        Ok(())
    }

    async fn handle_node_delete(&mut self, delete: String) -> Result<(), Box<dyn std::error::Error>> {
        let op = self.node_state.remove_node_local(delete);
        self.handle_node_op(op).await?;

        Ok(())
    }

    pub async fn write_to_queue(
        message: impl Serialize + Clone,
        sub_topic: u8,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut hasher = Sha3::v256();
        let mut topic_hash = [0u8; 32];
        hasher.update(b"state");
        hasher.finalize(&mut topic_hash);
        let mut message_code = vec![sub_topic];
        message_code.extend(serde_json::to_vec(&message)?);
        let request = QueueRequest::Write { 
            content: message_code, 
            topic: hex::encode(topic_hash) 
        };

        match Client::new()
            .post(format!("http://127.0.0.1:{}/queue/write_local", QUEUE_PORT))
            .json(&request)
            .send().await?
            .json::<QueueResponse>().await? {
                QueueResponse::OpSuccess => return Ok(()),
                QueueResponse::Failure { reason } => return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("{reason:?}")))),
                _ => return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Invalid response variant for write_local endpoint")))
        }
    }

    pub async fn read_from_queue(
        last: Option<usize>,
        n: Option<usize>,
    ) -> Result<Vec<Vec<u8>>, Box<dyn std::error::Error>> {
        let mut endpoint = format!("http://127.0.0.1:{}/queue/state", QUEUE_PORT);
        if let Some(idx) = last {
            let idx = idx;
            endpoint.push_str(&format!("/{idx}"));
            if let Some(n) = n {
                endpoint.push_str(&format!("/{n}/get_n_after"));
            } else {
                endpoint.push_str("/get_after");
            }
        } else {
            if let Some(n) = n {
                endpoint.push_str(&format!("/{n}/get_n"))
            } else {
                endpoint.push_str("/get")
            }
        }

        match Client::new()
            .get(endpoint.clone())
            .send().await?
            .json::<QueueResponse>().await? {
                QueueResponse::List(list) => {
                    log::info!("read from queue...");
                    Ok(list)
                },
                QueueResponse::Failure { reason } => {
                    Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("{reason:?}"))))
                },
                _ => Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Invalid response variant for {endpoint}")))) 
        }
    }

    pub async fn broadcast<R: DeserializeOwned>(
        &mut self,
        request: impl Serialize + Clone,
        endpoint: &str
    ) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Bradcasting Op to all active admins...");
        let peers = self.get_all_active_admin();
        for (id, peer) in peers {
            log::info!("Sending Op to all {id} at {}...", peer.ip().to_string());
            if let Err(e) = self.send::<R>(&peer.ip().to_string(), endpoint, request.clone()).await {
                eprintln!("Error sending {endpoint} request to {id}: {}: {e}", peer.ip().to_string());
            };
            log::info!("Successfully sent Op {id} at {}...", peer.ip().to_string());
        }

        Ok(())
    }

    pub async fn send<R: DeserializeOwned>(&mut self, ip: &str, endpoint: &str, request: impl Serialize) -> Result<(), Box<dyn std::error::Error>> {
        match Client::new()
            .post(format!("http://{ip}:3004/{endpoint}"))
            .json(&request)
            .send()
            .await {
                Ok(resp) => match resp.json::<R>().await {
                    Ok(_) => println!("Succesfully shared request with {ip}"),
                    Err(e) => eprintln!("Unable to decode response to request from {ip}: {e}")
                }
                Err(e) => {
                    eprintln!("Unable to share request with {ip}: {e}");
                }
            };

        Ok(())
    }

    pub fn app(state: Arc<Mutex<DataStore>>) -> Router {
        Router::new()
            .route("/ping", get(pong))
            .route("/bootstrap/joined_formnet", post(complete_bootstrap))
            .route("/bootstrap/full_state", get(full_state))
            .route("/bootstrap/network_state", get(network_state))
            .route("/bootstrap/peer_state", get(peer_state))
            .route("/bootstrap/cidr_state", get(cidr_state))
            .route("/bootstrap/assoc_state", get(assoc_state))
            .route("/user/create", post(create_user))
            .route("/user/update", post(update_user))
            .route("/user/disable", post(disable_user))
            .route("/user/redeem", post(redeem_invite)) 
            .route("/user/:id/get", get(get_user))
            .route("/user/:ip/get_from_ip", get(get_user_from_ip))
            .route("/user/delete", post(delete_user))
            .route("/user/:id/get_all_allowed", get(get_all_allowed))
            .route("/user/list", get(list_users))
            .route("/user/list_admin", get(list_admin))
            .route("/user/:cidr/list", get(list_by_cidr))
            .route("/user/delete_expired", post(delete_expired))
            .route("/cidr/create", post(create_cidr))
            .route("/cidr/update", post(update_cidr))
            .route("/cidr/delete", post(delete_cidr))
            .route("/cidr/:id/get", get(get_cidr))
            .route("/cidr/list", get(list_cidr))
            .route("/assoc/create", post(create_assoc))
            .route("/assoc/delete", post(delete_assoc))
            .route("/assoc/list", get(list_assoc))
            .route("/assoc/:cidr_id/relationships", get(relationships))
            .route("/dns/:domain/:build_id/request_vanity", post(request_vanity))
            .route("/dns/:domain/:build_id/request_public", post(request_public))
            .route("/dns/create", post(create_dns))
            .route("/dns/update", post(update_dns))
            .route("/dns/:domain/delete", post(delete_dns))
            .route("/dns/:domain/get", get(get_dns_record))
            .route("/dns/list", get(list_dns_records))
            .route("/instance/create", post(create_instance))
            .route("/instance/update", post(update_instance))
            .route("/instance/:instance_id/get", get(get_instance))
            .route("/instance/:build_id/get_by_build_id", get(get_instance_by_build_id))
            .route("/instance/:build_id/get_instance_ips", get(get_instance_ips))
            .route("/instance/:instance_id/delete", post(delete_instance))
            .route("/instance/list", get(list_instances))
            .route("/node/create", post(create_node))
            .route("/node/update", post(update_node))
            .route("/node/:id/get", get(get_node))
            .route("/node/:id/delete", post(delete_node))
            .route("/node/list", get(list_nodes))
            .with_state(state)
    }

    pub async fn run(self, mut shutdown: tokio::sync::broadcast::Receiver<()>) -> Result<(), Box<dyn std::error::Error>> {
        let datastore = Arc::new(Mutex::new(self));
        let router = Self::app(datastore.clone());
        let listener = TcpListener::bind("0.0.0.0:3004").await?;
        log::info!("Running datastore server...");
        tokio::spawn(async move {
            if let Err(e) = axum::serve(listener, router).await {
                eprintln!("Error serving State API Server: {e}");
            }
        });

        let mut n = 0;
        let polling_interval = 100;
        loop {
            tokio::select! {
                Ok(messages) = DataStore::read_from_queue(Some(n), None) => {
                    n += messages.len();
                    for message in messages {
                        log::info!("pulled message from queue");
                        let ds = datastore.clone();
                        tokio::spawn(async move {
                            if let Err(e) = process_message(message, ds).await {
                                eprintln!("Error processing message: {e}");
                            }
                        });
                    }
                }
                _ = tokio::time::sleep(Duration::from_millis(polling_interval)) => {
                }
                _ = shutdown.recv() => {
                    break;
                }
            }
        }

        Ok(())
    }
}

pub async fn complete_bootstrap(State(state): State<Arc<Mutex<DataStore>>>) {
    let mut guard = state.lock().await;
    let operators = guard.get_all_active_admin();
    drop(guard);

    let size = operators.len();

    if size == 0 {
        return;
    }

    let sample_size = if size < 10 {
        size
    } else {
        ((size as f64)* 0.33).ceil() as usize
    };

    let mut keys: Vec<&String> = operators.keys().collect();
    let mut rng = thread_rng();
    keys.shuffle(&mut rng);

    let mut sample = HashMap::new();

    for key in keys.into_iter().take(sample_size) {
        if let Some(value) = operators.get(key) {
            sample.insert(key.clone(), value.clone());
        }
    }

    let client = Client::new();

    tokio::spawn(async move {
        let mut sample_iter = sample.iter();
        while let Some((id, peer)) = sample_iter.next() {
            match client.get(
                format!("http://{}:3004/bootstrap/full_state", peer.ip())
            ).send().await {
                Ok(r) => match r.json::<MergeableState>().await {
                    Ok(mergeable_state) => {
                        let mut guard = state.lock().await; 
                        guard.network_state.peers.merge(mergeable_state.peers);
                        guard.network_state.cidrs.merge(mergeable_state.cidrs);
                        guard.network_state.associations.merge(mergeable_state.assocs);
                        guard.network_state.dns_state.zones.merge(mergeable_state.dns);
                        guard.instance_state.map.merge(mergeable_state.instances);
                        guard.node_state.map.merge(mergeable_state.nodes);
                        drop(guard);
                    }
                    Err(e) => {
                        log::error!("Error attempting to deserialize mergeable state from {id} at {}: {e}", peer.ip());
                    }
                }
                Err(e) => {
                    log::error!("Error attempting to get mergeable state from {id} at {}: {e}", peer.ip());
                }
            }
        }
    });
}

pub async fn process_message(message: Vec<u8>, state: Arc<Mutex<DataStore>>) -> Result<(), Box<dyn std::error::Error>> {
    if message.is_empty() {
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Message was empty")));
    }

    let subtopic = message[0];
    let payload = &message[1..];

    let mut guard = state.lock().await;

    match subtopic {
        0 => {
            log::info!("Pulled peer request from queue, processing...");
            let peer_request: PeerRequest = serde_json::from_slice(payload)?;
            guard.handle_peer_request(peer_request).await?;
        },
        1 => {
            log::info!("Pulled cidr request from queue, processing...");
            let cidr_request: CidrRequest = serde_json::from_slice(payload)?;
            guard.handle_cidr_request(cidr_request).await?;
        },
        2 => {
            let assoc_request: AssocRequest = serde_json::from_slice(payload)?;
            guard.handle_assoc_request(assoc_request).await?;
        },
        3 => {
            let dns_request: DnsRequest = serde_json::from_slice(payload)?;
            guard.handle_dns_request(dns_request).await?;
        },
        4 => {
            let instance_request: InstanceRequest = serde_json::from_slice(payload)?;
            guard.handle_instance_request(instance_request).await?;
        },
        5 => {
            let node_request: NodeRequest = serde_json::from_slice(payload)?;
            guard.handle_node_request(node_request).await?;
        },
        _ => unreachable!()
    }

    drop(guard);

    Ok(())
}

async fn pong() -> Json<Value> {
    log::info!("Received Ping Request, sending Pong...");
    Json(serde_json::json!({"ping":"pong"}))
}

async fn full_state(
    State(state): State<Arc<Mutex<DataStore>>>,
) -> Json<MergeableState> {
    log::info!("Received full state request, returning...");
    let datastore = state.lock().await.clone();
    Json(datastore.into())
}

async fn network_state(
    State(state): State<Arc<Mutex<DataStore>>>,
) -> Json<MergeableNetworkState> {
    log::info!("Received network state request, returning...");
    let network_state = state.lock().await.network_state.clone();
    Json(network_state.into())
}

async fn peer_state(
    State(state): State<Arc<Mutex<DataStore>>>, 
) -> Json<PeerMap> {
    log::info!("Received peer state request, returning...");
    let peer_state = state.lock().await.network_state.peers.clone();
    Json(peer_state)
}

async fn cidr_state(
    State(state): State<Arc<Mutex<DataStore>>>, 
) -> Json<CidrMap> {
    log::info!("Received cidr state request, returning...");
    let cidr_state = state.lock().await.network_state.cidrs.clone();
    Json(cidr_state)
}

async fn assoc_state(
    State(state): State<Arc<Mutex<DataStore>>>, 
) -> Json<AssocMap> {
    log::info!("Received assoc state request, returning...");
    let assoc_state = state.lock().await.network_state.associations.clone();
    Json(assoc_state)
}

async fn create_user(
    State(state): State<Arc<Mutex<DataStore>>>,
    Json(user): Json<PeerRequest>
) -> Json<Response<Peer<String>>> {
    log::info!("Received create user request...");
    let mut datastore = state.lock().await;
    match user {
        PeerRequest::Op(map_op) => {
            log::info!("Create user request is an Op from another peer");
            match &map_op {
                crdts::map::Op::Up { ref key, ref op, .. } => {
                    datastore.network_state.peer_op(map_op.clone());
                    if let (true, v) = datastore.network_state.peer_op_success(key.clone(), op.clone()) {
                        log::info!("Peer Op succesffully applied...");
                        return Json(Response::Success(Success::Some(v.into())))
                    } else {
                        log::info!("Peer Op rejected...");
                        return Json(Response::Failure { reason: Some("update was rejected".to_string()) })
                    }
                }
                crdts::map::Op::Rm { .. } => {
                    return Json(Response::Failure { reason: Some("Invalid Op type for Create User".into()) });
                }
            }
        }
        PeerRequest::Join(contents) => {
            log::info!("Create user request was a direct request...");
            log::info!("Building Map Op...");
            let map_op = datastore.network_state.update_peer_local(contents);
            log::info!("Map op created... Applying...");
            datastore.network_state.peer_op(map_op.clone());
            match &map_op {
                crdts::map::Op::Rm { .. } => {
                    return Json(Response::Failure { reason: Some("Map generated RM context instead of Add context on Join request".to_string()) });
                }
                crdts::map::Op::Up { ref key, ref op, .. } => {
                    if let (true, v) = datastore.network_state.peer_op_success(key.clone(), op.clone()) {
                        log::info!("Map Op was successful, broadcasting...");
                        let request = PeerRequest::Op(map_op);
                        match datastore.broadcast::<Response<Peer<String>>>(request, "/user/create").await {
                            Ok(()) => return Json(Response::Success(Success::Some(v.into()))),
                            Err(e) => eprintln!("Error broadcasting DeletePeerRequest: {e}")
                        }
                        return Json(Response::Success(Success::Some(v.into())))
                    } else {
                        return Json(Response::Failure { reason: Some("update was rejected".to_string()) })
                    }
                }
            }
        }
        _ => {
            return Json(Response::Failure { reason: Some("Invalid request for create user".into()) });
        }
    }
}

async fn update_user(
    State(state): State<Arc<Mutex<DataStore>>>,
    Json(user): Json<PeerRequest>
) -> Json<Response<Peer<String>>> {
    log::info!("Received update user request...");
    let mut datastore = state.lock().await;
    match user {
        PeerRequest::Op(map_op) => {
            log::info!("Update user request is an Op from another peer");
            match &map_op {
                crdts::map::Op::Up { ref key, ref op, .. } => {
                    datastore.network_state.peer_op(map_op.clone());
                    if let (true, v) = datastore.network_state.peer_op_success(key.clone(), op.clone()) {
                        return Json(Response::Success(Success::Some(v.into())))
                    } else {
                        return Json(Response::Failure { reason: Some("update was rejected".to_string()) })
                    }
                }
                crdts::map::Op::Rm { .. } => {
                    return Json(Response::Failure { reason: Some("Invalid Op type for Create User".into()) });
                }
            }
        }
        PeerRequest::Update(contents) => {
            log::info!("Update user request was a direct request...");
            log::info!("Building Map Op...");
            let map_op = datastore.network_state.update_peer_local(contents);
            datastore.network_state.peer_op(map_op.clone());
            match &map_op {
                crdts::map::Op::Rm { .. } => {
                    return Json(Response::Failure { reason: Some("Map generated RM context instead of Add context on Join request".to_string()) });
                }
                crdts::map::Op::Up { ref key, ref op, .. } => {
                    if let (true, v) = datastore.network_state.peer_op_success(key.clone(), op.clone()) {
                        log::info!("Map Op was successful, broadcasting...");
                        let request = PeerRequest::Op(map_op);
                        match datastore.broadcast::<Response<Peer<String>>>(request, "/user/update").await {
                            Ok(()) => return Json(Response::Success(Success::Some(v.into()))),
                            Err(e) => eprintln!("Error broadcasting DeletePeerRequest: {e}")
                        }
                        return Json(Response::Success(Success::Some(v.into())))
                    } else {
                        return Json(Response::Failure { reason: Some("update was rejected".to_string()) })
                    }
                }
            }
        }
        _ => {
            return Json(Response::Failure { reason: Some("Invalid request for update user".into()) });
        }
    }
}

async fn disable_user(
    State(state): State<Arc<Mutex<DataStore>>>,
    Json(user): Json<PeerRequest>
) -> Json<Response<Peer<String>>> {
    log::info!("Received disable user request...");
    let mut datastore = state.lock().await;
    match user {
        PeerRequest::Op(map_op) => {
            log::info!("Disable user request is an Op from another peer");
            match &map_op {
                crdts::map::Op::Up { ref key, ref op, .. } => {
                    datastore.network_state.peer_op(map_op.clone());
                    if let (true, v) = datastore.network_state.peer_op_success(key.clone(), op.clone()) {
                        log::info!("Map Op was successful, broadcasting...");
                        return Json(Response::Success(Success::Some(v.into())))
                    } else {
                        return Json(Response::Failure { reason: Some("update was rejected".to_string()) })
                    }
                }
                crdts::map::Op::Rm { .. } => {
                    return Json(Response::Failure { reason: Some("Invalid Op type for Create User".into()) });
                }
            }
        }
        PeerRequest::Update(contents) => {
            log::info!("Disable user request was a direct request...");
            log::info!("Building Map Op...");
            let map_op = datastore.network_state.update_peer_local(contents);
            datastore.network_state.peer_op(map_op.clone());
            match &map_op {
                crdts::map::Op::Rm { .. } => {
                    return Json(Response::Failure { reason: Some("Map generated RM context instead of Add context on Join request".to_string()) });
                }
                crdts::map::Op::Up { ref key, ref op, .. } => {
                    if let (true, v) = datastore.network_state.peer_op_success(key.clone(), op.clone()) {
                        log::info!("Map Op was successful, broadcasting...");
                        let request = PeerRequest::Op(map_op);
                        match datastore.broadcast::<Response<Peer<String>>>(request, "/user/disable").await {
                            Ok(()) => return Json(Response::Success(Success::Some(v.into()))),
                            Err(e) => eprintln!("Error broadcasting DeletePeerRequest: {e}")
                        }
                        return Json(Response::Success(Success::Some(v.into())))
                    } else {
                        return Json(Response::Failure { reason: Some("update was rejected".to_string()) })
                    }
                }
            }
        }
        _ => {
            return Json(Response::Failure { reason: Some("Invalid request for update user".into()) });
        }
    }
}

async fn redeem_invite(
    State(state): State<Arc<Mutex<DataStore>>>,
    Json(user): Json<PeerRequest>
) -> Json<Response<Peer<String>>> {
    log::info!("Received redeem invite user request...");
    let mut datastore = state.lock().await;
    match user {
        PeerRequest::Op(map_op) => {
            log::info!("Redeem invite user request is an Op from another peer");
            match &map_op {
                crdts::map::Op::Up { ref key, ref op, .. } => {
                    datastore.network_state.peer_op(map_op.clone());
                    if let (true, v) = datastore.network_state.peer_op_success(key.clone(), op.clone()) {
                        return Json(Response::Success(Success::Some(v.into())))
                    } else {
                        return Json(Response::Failure { reason: Some("update was rejected".to_string()) })
                    }
                }
                crdts::map::Op::Rm { .. } => {
                    return Json(Response::Failure { reason: Some("Invalid Op type for Create User".into()) });
                }
            }
        }
        PeerRequest::Update(contents) => {
            log::info!("Redeem invite user request was a direct request...");
            log::info!("Building Map Op...");
            let map_op = datastore.network_state.update_peer_local(contents);
            datastore.network_state.peer_op(map_op.clone());
            match &map_op {
                crdts::map::Op::Rm { .. } => {
                    return Json(Response::Failure { reason: Some("Map generated RM context instead of Add context on Join request".to_string()) });
                }
                crdts::map::Op::Up { ref key, ref op, .. } => {
                    if let (true, v) = datastore.network_state.peer_op_success(key.clone(), op.clone()) {
                        log::info!("Map Op was successful, broadcasting...");
                        let request = PeerRequest::Op(map_op);
                        match datastore.broadcast::<Response<Peer<String>>>(request, "/user/redeem").await {
                            Ok(()) => return Json(Response::Success(Success::Some(v.into()))),
                            Err(e) => eprintln!("Error broadcasting DeletePeerRequest: {e}")
                        }
                        return Json(Response::Success(Success::Some(v.into())))
                    } else {
                        return Json(Response::Failure { reason: Some("update was rejected".to_string()) })
                    }
                }
            }
        }
        _ => {
            return Json(Response::Failure { reason: Some("Invalid request for update user".into()) });
        }
    }
}

async fn delete_user(
    State(state): State<Arc<Mutex<DataStore>>>,
    Json(user): Json<PeerRequest>
) -> Json<Response<Peer<String>>> {
    let mut datastore = state.lock().await;
    match user {
        PeerRequest::Op(map_op) => {
            log::info!("delete user request is an Op from another peer");
            match &map_op {
                crdts::map::Op::Up { .. } => {
                    return Json(Response::Failure { reason: Some("Invalid Op type for delete User".into()) });
                }
                crdts::map::Op::Rm { .. } => {
                    datastore.network_state.peer_op(map_op);
                    return Json(Response::Success(Success::None));
                }
            }
        }
        PeerRequest::Delete(contents) => {
            log::info!("Delete user request was a direct request...");
            log::info!("Building Map Op...");
            let map_op = datastore.network_state.remove_peer_local(contents);
            datastore.network_state.peer_op(map_op.clone());
            match &map_op {
                crdts::map::Op::Rm { .. } => {
                    let request = PeerRequest::Op(map_op);
                    log::info!("Map Op was successful, broadcasting...");
                    match datastore.broadcast::<Response<Peer<String>>>(request, "/user/delete").await {
                        Ok(()) => return Json(Response::Success(Success::None)),
                        Err(e) => eprintln!("Error broadcasting DeletePeerRequest: {e}")
                    }
                    return Json(Response::Success(Success::None));
                }
                crdts::map::Op::Up { .. } => {
                    return Json(Response::Failure { reason: Some("Map generated Add context instead of Rm context on Delete request".to_string()) });
                }
            }
        }
        _ => {
            return Json(Response::Failure { reason: Some("Invalid request for update user".into()) });
        }
    }
}


async fn get_user(
    State(state): State<Arc<Mutex<DataStore>>>,
    Path(id): Path<String>
) -> Json<Response<Peer<String>>> {
    log::info!("Request to get peer {id}");
    if let Some(peer) = state.lock().await.network_state.peers.get(&id).val {
        log::info!("Found register for peer {id}");
        if let Some(val) = peer.val() {
            log::info!("Found value for peer {id}");
            return Json(Response::Success(Success::Some(val.value().into())))
        } else {
            return Json(Response::Failure { reason: Some("Entry exists, but value is None".into()) })
        }
    } 
    Json(Response::Failure { reason: Some("Entry does not exist in the CrdtMap".to_string()) })
}

async fn get_user_from_ip( 
    State(state): State<Arc<Mutex<DataStore>>>,
    Path(ip): Path<String>
) -> Json<Response<Peer<String>>> {
    log::info!("Request to get peer by IP {ip}");
    if let Some(peer) = state.lock().await.network_state.get_peer_by_ip(ip.clone()) {
        log::info!("Found peer {} by IP {ip}", peer.id());
        return Json(Response::Success(Success::Some(peer.into())))
    } 

    return Json(Response::Failure { reason: Some(format!("Peer with IP {ip} does not exist")) })
}

async fn get_all_allowed(
    State(state): State<Arc<Mutex<DataStore>>>,
    Path(id): Path<String>,
) -> Json<Response<Peer<String>>> {
    log::info!("Requesting all allowed peers for peer {id}");
    let mut peers = state.lock().await.get_all_users();
    if let Some(peer) = state.lock().await.network_state.peers.get(&id).val {
        if let Some(node) = peer.val() {
            let peer = node.value();
            let cidr = peer.cidr();
            peers.retain(|_, v| v.cidr() == cidr);
            let all_allowed: Vec<Peer<String>> = peers.iter().map(|(_, v)| v.clone().into()).collect();
            log::info!("Retrieved all allowed peers for peer {id}. Total {}", all_allowed.len());
            return Json(Response::Success(Success::List(all_allowed)))
        }
    } 

    return Json(Response::Failure { reason: Some("Peer for which allowed peers was requested does not exist".into()) })
}

async fn list_users(
    State(state): State<Arc<Mutex<DataStore>>>,
) -> Json<Response<Peer<String>>> {
    log::info!("Requesting a list of all users in the network...");
    let peers = state.lock().await.get_all_users().iter().map(|(_, v)| v.clone().into()).collect();
    log::info!("Retrieved a list of all users in the network... Returning...");
    Json(Response::Success(Success::List(peers)))
}

async fn list_admin(
    State(state): State<Arc<Mutex<DataStore>>>,
) -> Json<Response<Peer<String>>> {
    log::info!("Requesting a list of all users in the network...");
    let peers = state.lock().await.get_all_active_admin().iter().map(|(_, v)| v.clone().into()).collect();
    log::info!("Retrieved a list of all users in the network... Returning...");
    Json(Response::Success(Success::List(peers)))
}

async fn list_by_cidr(
    State(state): State<Arc<Mutex<DataStore>>>,
    Path(cidr): Path<String>
) -> Json<Response<Peer<String>>> {
    log::info!("Retrieving a list of all users in the cidr {cidr}...");
    let mut peers = state.lock().await.get_all_users();
    peers.retain(|_, v| v.cidr() == cidr);
    log::info!("Retrieved a list of all users in the cidr {cidr}... Returning");
    Json(Response::Success(Success::List(peers.iter().map(|(_, v)| v.clone().into()).collect())))
}

async fn delete_expired(
    State(state): State<Arc<Mutex<DataStore>>>
) -> Json<Response<Peer<String>>> {
    log::info!("Deleting all users that are expired and not redeemed...");
    let all_peers = state.lock().await.get_all_users();
    let now = match SystemTime::now()
        .duration_since(UNIX_EPOCH) {
            Ok(n) => n.as_secs(),
            Err(_) => return Json(Response::Failure { reason: Some("Unable to get current timestamp".to_string()) }),
    };

    let mut removed_peers = all_peers.clone();
    removed_peers.retain(|_, v| {
        match v.invite_expires() {
            Some(expires) => {
                (expires < now) && (!v.is_redeemed())
            }
            None => false
        }
    });

    let mut datastore = state.lock().await;
    for (id, _) in &removed_peers {
        let op = datastore.network_state.remove_peer_local(id.clone());
        datastore.network_state.peer_op(op);
        let request = PeerRequest::Delete(id.clone()); 
        match datastore.broadcast::<Response<Peer<String>>>(request, "/user/delete").await {
            Ok(()) => return Json(Response::Success(Success::List(removed_peers.iter().map(|(_, v)| v.clone().into()).collect()))),
            Err(e) => eprintln!("Error broadcasting DeletePeerRequest: {e}")
        }
    }

    log::info!("Deleted all users that are expired and not redeemed...");
    Json(Response::Success(Success::List(removed_peers.iter().map(|(_, v)| v.clone().into()).collect())))
}

async fn create_cidr(
    State(state): State<Arc<Mutex<DataStore>>>,
    Json(cidr): Json<CidrRequest>,
) -> Json<Response<Cidr<String>>> {
    let mut datastore = state.lock().await;
    match cidr {
        CidrRequest::Op(map_op) => {
            match &map_op {
                crdts::map::Op::Up { ref key, ref op, .. } => {
                    datastore.network_state.cidr_op(map_op.clone());
                    if let (true, v) = datastore.network_state.cidr_op_success(key.clone(), op.clone()) {
                        return Json(Response::Success(Success::Some(v.into())))
                    } else {
                        return Json(Response::Failure { reason: Some("update was rejected".to_string()) })
                    }
                }
                crdts::map::Op::Rm { .. } => {
                    return Json(Response::Failure { reason: Some("Invalid Op type for Create CIDR".into()) });
                }
            }
        }
        CidrRequest::Create(contents) => {
            let map_op = datastore.network_state.update_cidr_local(contents);
            datastore.network_state.cidr_op(map_op.clone());
            match &map_op {
                crdts::map::Op::Rm { .. } => {
                    return Json(Response::Failure { reason: Some("Map generated RM context instead of Add context on Create request".to_string()) });
                }
                crdts::map::Op::Up { ref key, ref op, .. } => {
                    if let (true, v) = datastore.network_state.cidr_op_success(key.clone(), op.clone()) {
                        let request = CidrRequest::Op(map_op);
                        match datastore.broadcast::<Response<Cidr<String>>>(request, "/cidr/create").await {
                            Ok(()) => return Json(Response::Success(Success::Some(v.into()))),
                            Err(e) => eprintln!("Error broadcasting CreateCidrRequest: {e}")
                        }
                        return Json(Response::Success(Success::Some(v.into())))
                    } else {
                        return Json(Response::Failure { reason: Some("update was rejected".to_string()) })
                    }
                }
            }
        }
        _ => {
            return Json(Response::Failure { reason: Some("Invalid request for create cidr".into()) });
        }
    }
} 

async fn update_cidr(
    State(state): State<Arc<Mutex<DataStore>>>,
    Json(cidr): Json<CidrRequest>,
) -> Json<Response<Cidr<String>>> {
    let mut datastore = state.lock().await;
    match cidr {
        CidrRequest::Op(map_op) => {
            match &map_op {
                crdts::map::Op::Up { ref key, ref op, .. } => {
                    datastore.network_state.cidr_op(map_op.clone());
                    if let (true, v) = datastore.network_state.cidr_op_success(key.clone(), op.clone()) {
                        return Json(Response::Success(Success::Some(v.into())))
                    } else {
                        return Json(Response::Failure { reason: Some("update was rejected".to_string()) })
                    }
                }
                crdts::map::Op::Rm { .. } => {
                    return Json(Response::Failure { reason: Some("Invalid Op type for Update CIDR".into()) });
                }
            }
        }
        CidrRequest::Update(contents) => {
            let map_op = datastore.network_state.update_cidr_local(contents);
            datastore.network_state.cidr_op(map_op.clone());
            match &map_op {
                crdts::map::Op::Rm { .. } => {
                    return Json(Response::Failure { reason: Some("Map generated RM context instead of Add context on Update request".to_string()) });
                }
                crdts::map::Op::Up { ref key, ref op, .. } => {
                    if let (true, v) = datastore.network_state.cidr_op_success(key.clone(), op.clone()) {
                        let request = CidrRequest::Op(map_op);
                        match datastore.broadcast::<Response<Cidr<String>>>(request, "/cidr/update").await {
                            Ok(()) => return Json(Response::Success(Success::Some(v.into()))),
                            Err(e) => eprintln!("Error broadcasting UpdateCidrRequest: {e}")
                        }
                        return Json(Response::Success(Success::Some(v.into())))
                    } else {
                        return Json(Response::Failure { reason: Some("update was rejected".to_string()) })
                    }
                }
            }
        }
        _ => {
            return Json(Response::Failure { reason: Some("Invalid request for update cidr".into()) });
        }
    }
} 

async fn delete_cidr(
    State(state): State<Arc<Mutex<DataStore>>>,
    Json(cidr): Json<CidrRequest>,
) -> Json<Response<Cidr<String>>> {
    let mut datastore = state.lock().await;
    match cidr {
        CidrRequest::Op(map_op) => {
            match &map_op {
                crdts::map::Op::Up { .. } => {
                    return Json(Response::Failure { reason: Some("Invalid Op type for delete cidr".into()) });
                }
                crdts::map::Op::Rm { .. } => {
                    datastore.network_state.cidr_op(map_op);
                    return Json(Response::Success(Success::None));
                }
            }
        }
        CidrRequest::Delete(contents) => {
            let map_op = datastore.network_state.remove_cidr_local(contents);
            datastore.network_state.cidr_op(map_op.clone());
            match &map_op {
                crdts::map::Op::Rm { .. } => {
                    let request = CidrRequest::Op(map_op);
                    match datastore.broadcast::<Response<Cidr<String>>>(request, "/cidr/delete").await {
                        Ok(()) => return Json(Response::Success(Success::None)),
                        Err(e) => eprintln!("Error broadcasting DeleteCidrRequest: {e}")
                    }
                    return Json(Response::Success(Success::None));
                }
                crdts::map::Op::Up { .. } => {
                    return Json(Response::Failure { reason: Some("Map generated Add context instead of Rm context on Delete request".to_string()) });
                }
            }
        }
        _ => {
            return Json(Response::Failure { reason: Some("Invalid request for remove cidr".into()) });
        }
    }
} 

async fn get_cidr(
    State(state): State<Arc<Mutex<DataStore>>>,
    Path(id): Path<String>
) -> Json<Response<Cidr<String>>> {
    let guard = state.lock().await;
    log::info!("Request to get cidr: {id}");
    let keys: Vec<String> = guard.network_state.cidrs.keys().map(|ctx| ctx.val.clone()).collect();
    log::info!("Existing keys: {keys:?}");
    if let Some(cidr) = guard.network_state.cidrs.get(&id).val {
        if let Some(val) = cidr.val() {
            return Json(Response::Success(Success::Some(val.value().into())))
        } else {
            return Json(Response::Failure { reason: Some("Entry exists, but value is None".into()) })
        }
    } 
    Json(Response::Failure { reason: Some("Entry does not exist in the CrdtMap".to_string()) })
} 

async fn list_cidr(
    State(state): State<Arc<Mutex<DataStore>>>,
) -> Json<Response<Cidr<String>>> {
    log::info!("Received list cidr request");
    let cidrs = state.lock().await.get_all_cidrs().iter().map(|(_, v)| v.clone().into()).collect();
    Json(Response::Success(Success::List(cidrs)))
} 

async fn create_assoc(
    State(state): State<Arc<Mutex<DataStore>>>,
    Json(assoc): Json<AssocRequest>
) -> Json<Response<Association<String, (String, String)>>> {
    let mut datastore = state.lock().await;
    match assoc {
        AssocRequest::Op(map_op) => {
            match &map_op {
                crdts::map::Op::Up { ref key, ref op, .. } => {
                    datastore.network_state.associations_op(map_op.clone());
                    if let (true, v) = datastore.network_state.associations_op_success(key.clone(), op.clone()) {
                        return Json(Response::Success(Success::Some(v.into())))
                    } else {
                        return Json(Response::Failure { reason: Some("update was rejected".to_string()) })
                    }
                }
                crdts::map::Op::Rm { .. } => {
                    return Json(Response::Failure { reason: Some("Invalid Op type for Create Association".into()) });
                }
            }
        }
        AssocRequest::Create(contents) => {
            let map_op = datastore.network_state.update_association_local(contents);
            datastore.network_state.associations_op(map_op.clone());
            match &map_op {
                crdts::map::Op::Rm { .. } => {
                    return Json(Response::Failure { reason: Some("Map generated RM context instead of Add context on Create Association request".to_string()) });
                }
                crdts::map::Op::Up { ref key, ref op, .. } => {
                    if let (true, v) = datastore.network_state.associations_op_success(key.clone(), op.clone()) {
                        let request = AssocRequest::Op(map_op);
                        match datastore.broadcast::<Response<Association<String, (String, String)>>>(request, "/assoc/create").await {
                            Ok(()) => return Json(Response::Success(Success::Some(v.into()))),
                            Err(e) => eprintln!("Error broadcasting CreateAssoc Request: {e}")
                        }
                        return Json(Response::Success(Success::Some(v.into())))
                    } else {
                        return Json(Response::Failure { reason: Some("update was rejected".to_string()) })
                    }
                }
            }
        }
        _ => {
            return Json(Response::Failure { reason: Some("Invalid request for create Association".into()) });
        }
    }
}

async fn delete_assoc(
    State(state): State<Arc<Mutex<DataStore>>>,
    Json(assoc): Json<AssocRequest>,
) -> Json<Response<Association<String, (String, String)>>> {
    let mut datastore = state.lock().await;
    match assoc {
        AssocRequest::Op(map_op) => {
            match &map_op {
                crdts::map::Op::Up { .. } => {
                    return Json(Response::Failure { reason: Some("Invalid Op type for delete association".into()) });
                }
                crdts::map::Op::Rm { .. } => {
                    datastore.network_state.associations_op(map_op);
                    return Json(Response::Success(Success::None));
                }
            }
        }
        AssocRequest::Delete(contents) => {
            let map_op = datastore.network_state.remove_association_local(contents);
            datastore.network_state.associations_op(map_op.clone());
            match &map_op {
                crdts::map::Op::Rm { .. } => {
                    let request = AssocRequest::Op(map_op);
                    match datastore.broadcast::<Response<Association<String, (String, String)>>>(request, "/assoc/delete").await {
                        Ok(()) => return Json(Response::Success(Success::None)),
                        Err(e) => eprintln!("Error broadcasting DeleteAssocRequest: {e}")
                    }
                    return Json(Response::Success(Success::None));
                }
                crdts::map::Op::Up { .. } => {
                    return Json(Response::Failure { reason: Some("Map generated Add context instead of Rm context on Delete request".to_string()) });
                }
            }
        }
        _ => {
            return Json(Response::Failure { reason: Some("Invalid request for remove association".into()) });
        }
    }
}

async fn list_assoc(
    State(state): State<Arc<Mutex<DataStore>>>,
) -> Json<Response<Association<String, (String, String)>>> {
    let assocs = state.lock().await.get_all_assocs().iter().map(|(_, v)| v.clone().into()).collect();
    Json(Response::Success(Success::List(assocs)))
}

async fn relationships(
    State(state): State<Arc<Mutex<DataStore>>>,
    Path(cidr_id): Path<String>
) -> Json<Response<Vec<(Cidr<String>, Cidr<String>)>>> {
    let ships = state.lock().await.get_relationships(cidr_id);
    Json(Response::Success(Success::Relationships(ships)))
}

async fn request_vanity(
    State(state): State<Arc<Mutex<DataStore>>>,
    Path((domain, build_id)): Path<(String, String)>,
) -> Json<Response<Host>> {
    let datastore = state.lock().await;
    let assigned = datastore.network_state.dns_state.zones.iter().any(|ctx| {
        let (d, _) = ctx.val;
        if *d == domain {
            true
        } else {
            false
        }
    });

    if assigned {
        return Json(
            Response::Failure { 
                reason: Some(
                    format!("Domain name requested is already assigned, if it is assigned to one of your instances run `form [OPTIONS] dns remove` first")
                ) 
            }
        )
    }

    let mut instances = datastore.instance_state.map.iter().filter_map(|ctx| {
        let (_, v) = ctx.val;
        if let Some(v) = v.val() {
            let instance = v.value();
            if instance.build_id == build_id {
                Some(instance.clone())
            } else {
                None
            }
        } else {
            None
        }
    }).collect::<Vec<Instance>>();

    let node_hosts = datastore.node_state.map.iter().filter_map(|ctx| {
        let (i, v) = ctx.val;
        let is_host = instances.iter().any(|inst| inst.node_id == *i);
        if is_host {
            if let Some(reg_node) = v.val() {
                Some(reg_node.value().host.clone())
            } else {
                None
            }
        } else {
            None
        }
    }).collect::<Vec<Host>>();

    let formnet_ip = instances.iter().filter_map(|inst| {
        inst.formnet_ip
    }).collect::<Vec<IpAddr>>();

    let dns_a_record = FormDnsRecord {
        domain,
        record_type: RecordType::A,
        formnet_ip: formnet_ip.iter().map(|ip| {
            SocketAddr::new(*ip, 80)
        }).collect(),
        public_ip: vec![],
        cname_target: None,
        ssl_cert: false,
        ttl: 3600
    };

    let request = DnsRequest::Create(dns_a_record.clone());

    match Client::new().post("http://127.0.0.1:3004/dns/create")
        .json(&request)
        .send().await {
            Ok(resp) => {
                match resp.json::<Response<FormDnsRecord>>().await {
                    Ok(r) => {
                        match r {
                            Response::Failure { reason } => {
                                return Json(Response::Failure { reason })
                            }
                            _ => {}
                        }
                    }
                    Err(e) => {
                        return Json(Response::Failure { reason: Some(e.to_string()) })
                    }
                }
            }
            Err(e) => {
                return Json(Response::Failure { reason: Some(e.to_string()) })
            }
        };

    instances.iter_mut().for_each(|inst| {
        inst.dns_record = Some(dns_a_record.clone());
    });

    for instance in instances {
        let request = InstanceRequest::Update(instance);
        match Client::new().post("http://127.0.0.1:3004/instance/update")
            .json(&request)
            .send().await {
                Ok(resp) => {
                    match resp.json::<Response<FormDnsRecord>>().await {
                        Ok(r) => {
                            match r {
                                Response::Failure { reason } => {
                                    return Json(Response::Failure { reason })
                                }
                                _ => {}
                            }
                        }
                        Err(e) => {
                            return Json(Response::Failure { reason: Some(e.to_string()) })
                        }
                    }
                }
                Err(e) => {
                    return Json(Response::Failure { reason: Some(e.to_string()) })
                }
            };
    }

    drop(datastore);

    Json(Response::Success(Success::List(node_hosts)))

}

async fn request_public(
    State(state): State<Arc<Mutex<DataStore>>>,
    Path((domain, build_id)): Path<(String, String)>,
) -> Json<Response<Host>> {
    let datastore = state.lock().await;
    let assigned = datastore.network_state.dns_state.zones.iter().any(|ctx| {
        let (d, _) = ctx.val;
        if *d == domain {
            true
        } else {
            false
        }
    });

    if assigned {
        return Json(
            Response::Failure { 
                reason: Some(
                    format!("Domain name requested is already assigned, if it is assigned to one of your instances run `form [OPTIONS] dns remove` first")
                ) 
            }
        )
    }

    let mut instances = datastore.instance_state.map.iter().filter_map(|ctx| {
        let (_, v) = ctx.val;
        if let Some(v) = v.val() {
            let instance = v.value();
            if instance.build_id == build_id {
                Some(instance.clone())
            } else {
                None
            }
        } else {
            None
        }
    }).collect::<Vec<Instance>>();

    let node_hosts = datastore.node_state.map.iter().filter_map(|ctx| {
        let (i, v) = ctx.val;
        let is_host = instances.iter().any(|inst| inst.node_id == *i);
        if is_host {
            if let Some(reg_node) = v.val() {
                Some(reg_node.value().host.clone())
            } else {
                None
            }
        } else {
            None
        }
    }).collect::<Vec<Host>>();

    let formnet_ip = instances.iter().filter_map(|inst| {
        inst.formnet_ip
    }).collect::<Vec<IpAddr>>();

    let cname_target = node_hosts.iter().find_map(|h| {
        match h {
            Host::Domain(domain) => Some(domain), 
            _ => None
        }
    }).cloned();

    let a_record_target = node_hosts.iter().filter_map(|h| {
        match h {
            Host::Ipv4(ipv4) => Some(IpAddr::V4(ipv4.clone())),
            _ => None,
        }
    }).collect::<Vec<IpAddr>>();

    let dns_a_record = FormDnsRecord {
        domain: domain.clone(),
        record_type: RecordType::A,
        formnet_ip: formnet_ip.iter().map(|ip| {
            SocketAddr::new(*ip, 80)
        }).collect(),
        public_ip: a_record_target.iter().map(|ip| {
            SocketAddr::new(*ip, 80)
        }).collect(),
        cname_target,
        ssl_cert: false,
        ttl: 3600
    };

    let request = DnsRequest::Create(dns_a_record.clone());

    match Client::new().post("http://127.0.0.1:3004/dns/create")
        .json(&request)
        .send().await {
            Ok(resp) => {
                match resp.json::<Response<FormDnsRecord>>().await {
                    Ok(r) => {
                        match r {
                            Response::Failure { reason } => {
                                return Json(Response::Failure { reason })
                            }
                            _ => {}
                        }
                    }
                    Err(e) => {
                        return Json(Response::Failure { reason: Some(e.to_string()) })
                    }
                }
            }
            Err(e) => {
                return Json(Response::Failure { reason: Some(e.to_string()) })
            }
        };

    instances.iter_mut().for_each(|inst| {
        inst.dns_record = Some(dns_a_record.clone());
    });

    for instance in instances {
        let request = InstanceRequest::Update(instance);
        match Client::new().post("http://127.0.0.1:3004/instance/update")
            .json(&request)
            .send().await {
                Ok(resp) => {
                    match resp.json::<Response<FormDnsRecord>>().await {
                        Ok(r) => {
                            match r {
                                Response::Failure { reason } => {
                                    return Json(Response::Failure { reason })
                                }
                                _ => {}
                            }
                        }
                        Err(e) => {
                            return Json(Response::Failure { reason: Some(e.to_string()) })
                        }
                    }
                }
                Err(e) => {
                    return Json(Response::Failure { reason: Some(e.to_string()) })
                }
            };
    }

    drop(datastore);

    Json(Response::Success(Success::List(node_hosts)))

}

async fn create_dns(
    State(state): State<Arc<Mutex<DataStore>>>,
    Json(request): Json<DnsRequest>
) -> Json<Response<FormDnsRecord>> {
    log::info!("Received create user request...");
    let mut datastore = state.lock().await;
    match request {
        DnsRequest::Op(map_op) => {
            log::info!("Create DNS Record request is an Op from another peer");
            match &map_op {
                crdts::map::Op::Up { ref key, ref op, .. } => {
                    datastore.network_state.dns_op(map_op.clone());
                    return Json(handle_create_dns_op(&datastore.network_state, key, op.clone()).await)
                }
                crdts::map::Op::Rm { .. } => {
                    return Json(Response::Failure { reason: Some("Invalid Op type for Create DNS Record".into()) });
                }
            }
        }
        DnsRequest::Create(contents) => {
            log::info!("Create user request was a direct request...");
            log::info!("Building Map Op...");
            let map_op = datastore.network_state.update_dns_local(contents);
            log::info!("Map op created... Applying...");
            datastore.network_state.dns_op(map_op.clone());
            match &map_op {
                crdts::map::Op::Rm { .. } => {
                    return Json(Response::Failure { reason: Some("Map generated RM context instead of Add context on Create DNS Record request".to_string()) });
                }
                crdts::map::Op::Up { ref key, ref op, .. } => {
                    return Json(handle_create_dns_op(&datastore.network_state, key, op.clone()).await);
                }
            }
        }
        _ => {
            return Json(Response::Failure { reason: Some("Invalid request for create DNS".into()) });
        }
    }
}

async fn update_dns(
    State(state): State<Arc<Mutex<DataStore>>>,
    Json(request): Json<DnsRequest>
) -> Json<Response<FormDnsRecord>> {
    log::info!("Received create user request...");
    let mut datastore = state.lock().await;
    match request {
        DnsRequest::Op(map_op) => {
            log::info!("Update DNS Request from an Op from another peer");
            match &map_op {
                crdts::map::Op::Up { ref key, ref op, .. } => {
                    datastore.network_state.dns_op(map_op.clone());
                    return Json(handle_update_dns_op(&datastore.network_state, key, op.clone()).await);
                }
                crdts::map::Op::Rm { .. } => {
                    return Json(Response::Failure { reason: Some("Invalid Op type for Update DNS".into()) });
                }
            }
        }
        DnsRequest::Update(contents) => {
            log::info!("Create user request was a direct request...");
            log::info!("Building Map Op...");
            let map_op = datastore.network_state.update_dns_local(contents);
            log::info!("Map op created... Applying...");
            datastore.network_state.dns_op(map_op.clone());
            match &map_op {
                crdts::map::Op::Rm { .. } => {
                    return Json(Response::Failure { reason: Some("Map generated RM context instead of Add context on Update request".to_string()) });
                }
                crdts::map::Op::Up { ref key, ref op, .. } => {
                    return Json(handle_update_dns_op(&datastore.network_state, key, op.clone()).await)
                }
            }
        }
        _ => {
            return Json(Response::Failure { reason: Some("Invalid request for update dns".into()) });
        }
    }
}

async fn delete_dns(
    State(state): State<Arc<Mutex<DataStore>>>,
    Path(domain): Path<String>,
    Json(request): Json<DnsRequest>,
) -> Json<Response<FormDnsRecord>> {
    let mut datastore = state.lock().await;
    match request {
        DnsRequest::Op(map_op) => {
            log::info!("Delete DNS request is an Op from another peer");
            match &map_op {
                crdts::map::Op::Up { .. } => {
                    return Json(Response::Failure { reason: Some("Invalid Op type for delete dns".into()) });
                }
                crdts::map::Op::Rm { .. } => {
                    datastore.network_state.dns_op(map_op);
                    return Json(send_dns_delete_request(&domain).await)
                }
            }
        }
        DnsRequest::Delete(domain) => {
            log::info!("Delete DNS request was a direct request...");
            log::info!("Building Map Op...");
            let map_op = datastore.network_state.remove_dns_local(domain.clone());
            log::info!("Map op created... Applying...");
            datastore.network_state.dns_op(map_op.clone());
            match &map_op {
                crdts::map::Op::Rm { .. } => {
                    let request = DnsRequest::Op(map_op);
                    match datastore.broadcast::<Response<FormDnsRecord>>(request, &format!("/dns/{}/delete", domain.clone())).await {
                        Ok(()) => return Json(Response::Success(Success::None)),
                        Err(e) => eprintln!("Error broadcasting Delete DNS request: {e}")
                    }
                    return Json(send_dns_delete_request(&domain).await);
                }
                crdts::map::Op::Up { .. } => {
                    return Json(Response::Failure { reason: Some("Map generated Add context instead of Rm context on Delete request".to_string()) });
                }
            }
        }
        _ => {
            return Json(Response::Failure { reason: Some("Invalid request for create user".into()) });
        }
    }
}

async fn get_dns_record(
    State(state): State<Arc<Mutex<DataStore>>>,
    Path(domain): Path<String>,
) -> Json<Response<FormDnsRecord>>{
    let datastore = state.lock().await;
    if let Some(record) = datastore.network_state.dns_state.zones.get(&domain).val {
        if let Some(val) = record.val() {
            let dns_record = val.value();
            return Json(Response::Success(Success::Some(dns_record.into())))
        }
    };

    return Json(Response::Failure { reason: Some(format!("Record does not exist for domain {domain}")) }) 
}

async fn list_dns_records(
    State(state): State<Arc<Mutex<DataStore>>>,
) -> Json<Response<FormDnsRecord>> {
    let datastore = state.lock().await;
    let dns_record_list = datastore.network_state.dns_state.zones.iter().filter_map(|ctx|{ 
        let (_domain, reg) = ctx.val;
        match reg.val() {
            Some(node) => Some(node.value().into()),
            None => None,
        }
    }).collect::<Vec<FormDnsRecord>>();

    if !dns_record_list.is_empty() {
        return Json(Response::Success(Success::List(dns_record_list)))
    } else {
        return Json(Response::Failure { reason: Some("Unable to find any valid DNS records".to_string()) }) 
    }

}

pub async fn request_full_state(to_dial: &str) -> Result<MergeableState, Box<dyn std::error::Error>> {
    let resp = Client::new()
        .get(format!("http://{to_dial}:3004/bootstrap/full_state"))
        .send().await?.json().await?;

    println!("{:?}", resp);
    Ok(resp)
}

pub async fn request_netwok_state(to_dial: String) -> Result<MergeableNetworkState, Box<dyn std::error::Error>> {
    let resp = Client::new()
        .get(format!("http://{to_dial}:3004/bootstrap/network_state"))
        .send().await?.json().await?;
    Ok(resp)
}

pub async fn request_peer_state(to_dial: String) -> Result<PeerMap, Box<dyn std::error::Error>> {
    let resp = Client::new()
        .get(format!("http://{to_dial}:3004/bootstrap/peer_state"))
        .send().await?.json().await?;
    Ok(resp)
}

pub async fn request_cidr_state(to_dial: String) -> Result<CidrMap, Box<dyn std::error::Error>> {
    let resp = Client::new()
        .get(format!("http://{to_dial}:3004/bootstrap/cidr_state"))
        .send().await?.json().await?;

    Ok(resp)
}

pub async fn request_associations_state(to_dial: String) -> Result<AssocMap, Box<dyn std::error::Error>> {
    let resp = Client::new()
        .get(format!("http://{to_dial}:3004/bootstrap/assoc_state"))
        .send().await?.json().await?;

    Ok(resp)
}

async fn build_dns_request(v: Option<CrdtDnsRecord>, op_type: &str) -> (DomainRequest, Option<Response<FormDnsRecord>>) {
    if op_type == "create" {
        if v.is_none() {
            let request = DomainRequest::Create {
                domain: "".to_string(), 
                record_type: RecordType::NULL,
                ip_addr: vec![],
                cname_target: None,
                ssl_cert: false
            };
            return (request, Some(Response::Failure { reason: Some("Create request requires a record".into()) }))
        }
        return build_create_request(v.unwrap()).await
    }

    if op_type == "update" {
        if v.is_none() {
            let request = DomainRequest::Update {
                replace: false, 
                record_type: RecordType::NULL,
                ip_addr: vec![],
                cname_target: None,
                ssl_cert: false,
            };
            return (request, Some(Response::Failure { reason: Some("Update request requires a record".into()) }))
        }
        return build_update_request(v.unwrap()).await
    }
    let request = DomainRequest::Update {
        replace: false, 
        record_type: RecordType::NULL,
        ip_addr: vec![],
        cname_target: None,
        ssl_cert: false,
    };
    return (request, Some(Response::Failure { reason: Some("Update request requires a record".into()) }))
}

async fn build_create_request(v: CrdtDnsRecord) -> (DomainRequest, Option<Response<FormDnsRecord>>) {
    if let RecordType::A = v.record_type() { 
        return build_create_a_record_request(v).await
    } else if let RecordType::AAAA = v.record_type() {
        return build_create_aaaa_record_request(v).await
    } else if let RecordType::CNAME = v.record_type() {
        return build_create_cname_record_request(v).await
    } else {
        let request = DomainRequest::Create {
            domain: v.domain(), 
            record_type: RecordType::NULL,
            ip_addr: vec![],
            cname_target: None,
            ssl_cert: v.ssl_cert()
        };
        return (request, Some(Response::Failure { reason: Some("Only A, AAAA and CNAME records are supported".to_string()) }));
    };
}

async fn build_update_request(v: CrdtDnsRecord) -> (DomainRequest, Option<Response<FormDnsRecord>>) {
    if let RecordType::A = v.record_type() { 
        return build_update_a_record_request(v).await
    } else if let RecordType::AAAA = v.record_type() {
        return build_update_aaaa_record_request(v).await
    } else if let RecordType::CNAME = v.record_type() {
        return build_update_cname_record_request(v).await
    } else {
        let request = DomainRequest::Update {
            replace: false, 
            record_type: RecordType::NULL,
            ip_addr: vec![],
            cname_target: None,
            ssl_cert: v.ssl_cert()
        };
        return (request, Some(Response::Failure { reason: Some("Only A, AAAA and CNAME records are supported".to_string()) }));
    }
}

async fn build_create_a_record_request(v: CrdtDnsRecord) -> (DomainRequest, Option<Response<FormDnsRecord>>) {
    if !v.formnet_ip().is_empty() {
        let mut ips = v.formnet_ip();
        if !v.public_ip().is_empty() {
            ips.extend(v.public_ip());
        }
        let request = DomainRequest::Create { 
            domain: v.domain().clone(),
            record_type: v.record_type(), 
            ip_addr: ips,
            cname_target: None,
            ssl_cert: v.ssl_cert()
        };
        return (request, None)
    } else {
        let request = DomainRequest::Create { 
            domain: v.domain().clone(), 
            record_type: v.record_type(),
            ip_addr: v.public_ip(), 
            cname_target: None,
            ssl_cert: v.ssl_cert()
        }; 
        return (request, None)
    }
}

async fn build_update_a_record_request(v: CrdtDnsRecord) -> (DomainRequest, Option<Response<FormDnsRecord>>) {
    if !v.formnet_ip().is_empty() {
        let mut ips = v.formnet_ip();
        if !v.public_ip().is_empty() {
            ips.extend(v.public_ip());
        }
        let request = DomainRequest::Update { 
            replace: true,
            record_type: v.record_type(), 
            ip_addr: ips, 
            cname_target: None,
            ssl_cert: v.ssl_cert()
        };
        return(request, None);
    } else {
        let request = DomainRequest::Create { 
            domain: v.domain().clone(), 
            record_type: v.record_type(),
            ip_addr: v.public_ip(), 
            cname_target: None,
            ssl_cert: v.ssl_cert()
        }; 
        (request, None)
    }
}

async fn build_create_aaaa_record_request(v: CrdtDnsRecord) -> (DomainRequest, Option<Response<FormDnsRecord>>) {
    if !v.public_ip().is_empty() {
        let request = DomainRequest::Create { 
            domain: v.domain().clone(), 
            record_type: v.record_type(),
            ip_addr: v.public_ip(), 
            cname_target: None,
            ssl_cert: v.ssl_cert()
        }; 
        return (request, None)
    } else {
        let request = DomainRequest::Create {
            domain: v.domain().clone(),
            record_type: v.record_type(),
            ip_addr: v.public_ip(), 
            cname_target: None,
            ssl_cert: v.ssl_cert()
        };
        return (request, Some(Response::Failure { reason: Some("AAAA Record Updates require a public IP V6 address".to_string()) }))
    }
}

async fn build_update_aaaa_record_request(v: CrdtDnsRecord) -> (DomainRequest, Option<Response<FormDnsRecord>>) {
    let request = DomainRequest::Update { 
        replace: true, 
        record_type: v.record_type(),
        ip_addr: v.public_ip(), 
        cname_target: None,
        ssl_cert: v.ssl_cert()
    }; 
    (request, None)
}

async fn build_create_cname_record_request(v: CrdtDnsRecord) -> (DomainRequest, Option<Response<FormDnsRecord>>) {
    let request = DomainRequest::Create {
        domain: v.domain().clone(),
        record_type: v.record_type(),
        ip_addr: {
            let mut ips = v.formnet_ip();
            ips.extend(v.public_ip());
            ips
        },
        cname_target: v.cname_target().clone(),
        ssl_cert: v.ssl_cert()
    };
    (request, None)
}

async fn build_update_cname_record_request(v: CrdtDnsRecord) -> (DomainRequest, Option<Response<FormDnsRecord>>) {
    let request = DomainRequest::Update {
        replace: true,
        record_type: v.record_type(),
        ip_addr: {
            let mut ips = v.formnet_ip();
            ips.extend(v.public_ip());
            ips
        },
        cname_target: v.cname_target().clone(),
        ssl_cert: v.ssl_cert()
    };
    (request, None)
}

async fn send_dns_create_request(r: DomainRequest) -> Option<Response<FormDnsRecord>> {
    match reqwest::Client::new()
        .post("http://127.0.0.1:3005/record/create")
        .json(&r)
        .send().await {

        Ok(resp) => match resp.json::<DomainResponse>().await {
            Ok(r) => match r {
                DomainResponse::Success(_) => {}
                DomainResponse::Failure(reason) => {
                    return Some(Response::Failure { reason })
                }
            }
            Err(e) => return Some(Response::Failure { reason: Some(e.to_string())})
        }
        Err(e) => {
            return Some(Response::Failure { reason: Some(e.to_string())})
        }
    }
    None
}

async fn send_dns_update_request(r: DomainRequest, domain: &str) -> Option<Response<FormDnsRecord>> {
    match reqwest::Client::new()
        .post(format!("http://127.0.0.1:3005/record/{}/update", domain))
        .json(&r)
        .send().await {

        Ok(resp) => match resp.json::<DomainResponse>().await {
            Ok(r) => match r {
                DomainResponse::Success(_) => {}
                DomainResponse::Failure(reason) => {
                    return Some(Response::Failure { reason })
                }
            }
            Err(e) => return Some(Response::Failure { reason: Some(e.to_string()) })
        }
        Err(e) => {
            return Some(Response::Failure { reason: Some(e.to_string())})
        }
    }
    None
}

async fn send_dns_delete_request(domain: &str) -> Response<FormDnsRecord> {
    match reqwest::Client::new()
        .post(format!("http://127.0.0.1:3005/record/{}/delete", domain))
        .send().await {
        Ok(resp) => match resp.json::<DomainResponse>().await {
            Ok(r) => match r {
                DomainResponse::Success(_) => return Response::Success(Success::None), 
                DomainResponse::Failure(reason) => {
                    return Response::Failure { reason }
                }
            }
            Err(e) => return Response::Failure { reason: Some(e.to_string()) }
        }
        Err(e) => {
            return Response::Failure { reason: Some(e.to_string())}
        }
    }
}


async fn handle_create_dns_op(network_state: &NetworkState, key: &str, op: Update<CrdtDnsRecord, String>) -> Response<FormDnsRecord> {
    if let (true, v) = network_state.dns_op_success(key.to_string(), op.clone()) {
        log::info!("DNS Op succesfully applied... Attempting to build dns request with {v:?}");
        let (request, failure) = build_dns_request(Some(v.clone()), "create").await;
        if let Some(failure) = failure {
            return failure
        }

        let failure = send_dns_create_request(request.clone()).await;
        if let Some(failure) = failure {
            return failure;
        }
        return Response::Success(Success::Some(v.into()))
    } else {
        log::info!("DNS Op rejected...");
        return Response::Failure { reason: Some("update was rejected".to_string()) }
    }
}

async fn handle_update_dns_op(network_state: &NetworkState, key: &str, op: Update<CrdtDnsRecord, String>) -> Response<FormDnsRecord> {
    if let (true, v) = network_state.dns_op_success(key.to_string(), op.clone()) {
        log::info!("Peer Op succesffully applied...");
        let (request, failure) = build_dns_request(Some(v.clone()), "create").await;
        if let Some(failure) = failure {
            return failure
        }
        let failure = send_dns_update_request(request.clone(), &v.domain()).await;
        if let Some(failure) = failure {
            return failure;
        }
        return Response::Success(Success::Some(v.into()))
    } else {
        log::info!("Peer Op rejected...");
        return Response::Failure { reason: Some("update was rejected".to_string()) }
    }
}

async fn create_instance(
    State(state): State<Arc<Mutex<DataStore>>>,
    Json(request): Json<InstanceRequest>
) -> Json<Response<Instance>> {
    let mut datastore = state.lock().await;
    match request {
        InstanceRequest::Op(map_op) => {
            log::info!("Create Instance request is an Op from another peer");
            match &map_op {
                crdts::map::Op::Up { ref key, ref op, .. } => {
                    datastore.instance_state.instance_op(map_op.clone());
                    if let (true, v) = datastore.instance_state.instance_op_success(key.clone(), op.clone()) {
                        log::info!("Instance Op succesffully applied...");
                        return Json(Response::Success(Success::Some(v.into())))
                    } else {
                        log::info!("Instance Op rejected...");
                        return Json(Response::Failure { reason: Some("update was rejected".to_string()) })
                    }
                }
                crdts::map::Op::Rm { .. } => {
                    return Json(Response::Failure { reason: Some("Invalid Op type for Create Instance".into()) });
                }
            }
        }
        InstanceRequest::Create(contents) => {
            log::info!("Create Instance request was a direct request...");
            log::info!("Building Map Op...");
            let map_op = datastore.instance_state.update_instance_local(contents);
            log::info!("Map op created... Applying...");
            datastore.instance_state.instance_op(map_op.clone());
            match &map_op {
                crdts::map::Op::Rm { .. } => {
                    return Json(Response::Failure { reason: Some("Map generated RM context instead of Add context on Create request".to_string()) });
                }
                crdts::map::Op::Up { ref key, ref op, .. } => {
                    if let (true, v) = datastore.instance_state.instance_op_success(key.clone(), op.clone()) {
                        log::info!("Map Op was successful, broadcasting...");
                        let request = InstanceRequest::Op(map_op);
                        match datastore.broadcast::<Response<Instance>>(request, "/instance/create").await {
                            Ok(()) => return Json(Response::Success(Success::Some(v.into()))),
                            Err(e) => eprintln!("Error broadcasting Instance Create Request: {e}")
                        }
                        return Json(Response::Success(Success::Some(v.into())))
                    } else {
                        return Json(Response::Failure { reason: Some("update was rejected".to_string()) })
                    }
                }
            }
        }
        _ => {
            return Json(Response::Failure { reason: Some("Invalid request for create instance".into()) });
        }
    }
}

async fn update_instance(
    State(state): State<Arc<Mutex<DataStore>>>,
    Json(request): Json<InstanceRequest>
) -> Json<Response<Instance>> {
    let mut datastore = state.lock().await;
    match request {
        InstanceRequest::Op(map_op) => {
            log::info!("Update Instance request is an Op from another peer");
            match &map_op {
                crdts::map::Op::Up { ref key, ref op, .. } => {
                    datastore.instance_state.instance_op(map_op.clone());
                    if let (true, v) = datastore.instance_state.instance_op_success(key.clone(), op.clone()) {
                        log::info!("Instance Op succesffully applied...");
                        return Json(Response::Success(Success::Some(v.into())))
                    } else {
                        log::info!("Instance Op rejected...");
                        return Json(Response::Failure { reason: Some("update was rejected".to_string()) })
                    }
                }
                crdts::map::Op::Rm { .. } => {
                    return Json(Response::Failure { reason: Some("Invalid Op type for Update Instance".into()) });
                }
            }
        }
        InstanceRequest::Update(contents) => {
            log::info!("Update Instance request was a direct request...");
            log::info!("Building Map Op...");
            let map_op = datastore.instance_state.update_instance_local(contents);
            log::info!("Map op created... Applying...");
            datastore.instance_state.instance_op(map_op.clone());
            match &map_op {
                crdts::map::Op::Rm { .. } => {
                    return Json(Response::Failure { reason: Some("Map generated RM context instead of Add context on Update request".to_string()) });
                }
                crdts::map::Op::Up { ref key, ref op, .. } => {
                    if let (true, v) = datastore.instance_state.instance_op_success(key.clone(), op.clone()) {
                        log::info!("Map Op was successful, broadcasting...");
                        let request = InstanceRequest::Op(map_op);
                        match datastore.broadcast::<Response<Instance>>(request, "/instance/update").await {
                            Ok(()) => return Json(Response::Success(Success::Some(v.into()))),
                            Err(e) => eprintln!("Error broadcasting Instance Update Request: {e}")
                        }
                        return Json(Response::Success(Success::Some(v.into())))
                    } else {
                        return Json(Response::Failure { reason: Some("update was rejected".to_string()) })
                    }
                }
            }
        }
        _ => {
            return Json(Response::Failure { reason: Some("Invalid request for update instance".into()) });
        }
    }
}

async fn get_instance(
    State(state): State<Arc<Mutex<DataStore>>>,
    Path(id): Path<String>,
) -> Json<Response<Instance>> {
    let datastore = state.lock().await;
    log::info!("Attempting to get instance {id}");
    if let Some(instance) = datastore.instance_state.get_instance(id.clone()) {
        return Json(Response::Success(Success::Some(instance)))
    }

    return Json(Response::Failure { reason: Some(format!("Unable to find instance with instance_id, node_id: {}", id))})
}


async fn get_instance_by_build_id(
    State(state): State<Arc<Mutex<DataStore>>>,
    Path(id): Path<String>,
) -> Json<Response<Instance>> {
    let datastore = state.lock().await;
    log::info!("Attempting to get instance {id}");
    let instances: Vec<Instance> = datastore.instance_state.map.iter().filter_map(|ctx| {
        let (_, reg) = ctx.val;
        if let Some(val) = reg.val() {
            let instance = val.value();
            if instance.build_id == id {
                Some(instance)
            } else {
                None
            }
        } else {
            None
        }
    }).collect();

    return Json(Response::Success(Success::List(instances)));
}

async fn get_instance_ips(
    State(state): State<Arc<Mutex<DataStore>>>,
    Path(id): Path<String>,
) -> Json<Response<IpAddr>> {
    let datastore = state.lock().await;
    log::info!("Attempting to get instance {id}");
    let instances: Vec<Instance> = datastore.instance_state.map.iter().filter_map(|ctx| {
        let (id, reg) = ctx.val;
        if let Some(val) = reg.val() {
            let instance = val.value();
            if instance.build_id == *id {
                Some(instance)
            } else {
                None
            }
        } else {
            None
        }
    }).collect();

    let ips = instances.iter().filter_map(|inst| {
        inst.formnet_ip
    }).collect();

    return Json(Response::Success(Success::List(ips)));
}


async fn delete_instance(
    State(state): State<Arc<Mutex<DataStore>>>,
    Path(_id): Path<(String, String)>,
    Json(request): Json<InstanceRequest>
) -> Json<Response<Instance>> {
    let mut datastore = state.lock().await;
    match request {
        InstanceRequest::Op(map_op) => {
            log::info!("Delete Instance request is an Op from another peer");
            match &map_op {
                crdts::map::Op::Up { .. } => {
                    return Json(Response::Failure { reason: Some("Invalid Op type for delete dns".into()) });
                }
                crdts::map::Op::Rm { .. } => {
                    datastore.instance_state.instance_op(map_op);
                    return Json(Response::Success(Success::None))
                }
            }
        }
        InstanceRequest::Delete(id) => {
            log::info!("Delete Instance request was a direct request...");
            log::info!("Building Map Op...");
            let map_op = datastore.instance_state.remove_instance_local(id.clone());
            log::info!("Map op created... Applying...");
            datastore.instance_state.instance_op(map_op.clone());
            match &map_op {
                crdts::map::Op::Rm { .. } => {
                    let request = InstanceRequest::Op(map_op);
                    match datastore.broadcast::<Response<Instance>>(request, &format!("/instance/{}/delete", id.clone())).await {
                        Ok(()) => return Json(Response::Success(Success::None)),
                        Err(e) => eprintln!("Error broadcasting Delete Instance request: {e}")
                    }
                    return Json(Response::Success(Success::None));
                }
                crdts::map::Op::Up { .. } => {
                    return Json(Response::Failure { reason: Some("Map generated Add context instead of Rm context on Delete request".to_string()) });
                }
            }
        }
        _ => {
            return Json(Response::Failure { reason: Some("Invalid request for delete instance".into()) });
        }
    }
}

async fn list_instances(
    State(state): State<Arc<Mutex<DataStore>>>,
) -> Json<Response<Instance>> {
    let datastore = state.lock().await;
    let list: Vec<Instance> = datastore.instance_state.map().iter().filter_map(|ctx| {
        let (_, value) = ctx.val;
        match value.val() {
            Some(node) => {
                return Some(node.value())
            }
            None => return None
        }
    }).collect(); 

    return Json(Response::Success(Success::List(list)))
}

async fn create_node(
    State(state): State<Arc<Mutex<DataStore>>>,
    Json(request): Json<NodeRequest>
) -> Json<Response<Node>> {
    let mut datastore = state.lock().await;
    match request {
        NodeRequest::Op(map_op) => {
            log::info!("Create Node request is an Op from another peer");
            match &map_op {
                crdts::map::Op::Up { ref key, ref op, .. } => {
                    datastore.node_state.node_op(map_op.clone());
                    if let (true, v) = datastore.node_state.node_op_success(key.clone(), op.clone()) {
                        log::info!("Node Op succesffully applied...");
                        return Json(Response::Success(Success::Some(v.into())))
                    } else {
                        log::info!("Node Op rejected...");
                        return Json(Response::Failure { reason: Some("update was rejected".to_string()) })
                    }
                }
                crdts::map::Op::Rm { .. } => {
                    return Json(Response::Failure { reason: Some("Invalid Op type for Create Node".into()) });
                }
            }
        }
        NodeRequest::Create(contents) => {
            log::info!("Create Node request was a direct request...");
            log::info!("Building Map Op...");
            let map_op = datastore.node_state.update_node_local(contents);
            log::info!("Map op created... Applying...");
            datastore.node_state.node_op(map_op.clone());
            match &map_op {
                crdts::map::Op::Rm { .. } => {
                    return Json(Response::Failure { reason: Some("Map generated RM context instead of Add context on Create request".to_string()) });
                }
                crdts::map::Op::Up { ref key, ref op, .. } => {
                    if let (true, v) = datastore.node_state.node_op_success(key.clone(), op.clone()) {
                        log::info!("Map Op was successful, broadcasting...");
                        let request = NodeRequest::Op(map_op);
                        match datastore.broadcast::<Response<Node>>(request, "/node/create").await {
                            Ok(()) => return Json(Response::Success(Success::Some(v.into()))),
                            Err(e) => eprintln!("Error broadcasting Node Create Request: {e}")
                        }
                        return Json(Response::Success(Success::Some(v.into())))
                    } else {
                        return Json(Response::Failure { reason: Some("update was rejected".to_string()) })
                    }
                }
            }
        }
        _ => {
            return Json(Response::Failure { reason: Some("Invalid request for create node".into()) });
        }
    }
}

async fn update_node(
    State(state): State<Arc<Mutex<DataStore>>>,
    Json(request): Json<NodeRequest>
) -> Json<Response<Node>> {
    let mut datastore = state.lock().await;
    match request {
        NodeRequest::Op(map_op) => {
            log::info!("Update Node request is an Op from another peer");
            match &map_op {
                crdts::map::Op::Up { ref key, ref op, .. } => {
                    datastore.node_state.node_op(map_op.clone());
                    if let (true, v) = datastore.node_state.node_op_success(key.clone(), op.clone()) {
                        log::info!("Node Op succesffully applied...");
                        return Json(Response::Success(Success::Some(v.into())))
                    } else {
                        log::info!("Node Op rejected...");
                        return Json(Response::Failure { reason: Some("update was rejected".to_string()) })
                    }
                }
                crdts::map::Op::Rm { .. } => {
                    return Json(Response::Failure { reason: Some("Invalid Op type for Create Node".into()) });
                }
            }
        }
        NodeRequest::Update(contents) => {
            log::info!("Update Node request was a direct request...");
            log::info!("Building Map Op...");
            let map_op = datastore.node_state.update_node_local(contents);
            log::info!("Map op created... Applying...");
            datastore.node_state.node_op(map_op.clone());
            match &map_op {
                crdts::map::Op::Rm { .. } => {
                    return Json(Response::Failure { reason: Some("Map generated RM context instead of Add context on Create request".to_string()) });
                }
                crdts::map::Op::Up { ref key, ref op, .. } => {
                    if let (true, v) = datastore.node_state.node_op_success(key.clone(), op.clone()) {
                        log::info!("Map Op was successful, broadcasting...");
                        let request = NodeRequest::Op(map_op);
                        match datastore.broadcast::<Response<Node>>(request, "/node/update").await {
                            Ok(()) => return Json(Response::Success(Success::Some(v.into()))),
                            Err(e) => eprintln!("Error broadcasting Node Update Request: {e}")
                        }
                        return Json(Response::Success(Success::Some(v.into())))
                    } else {
                        return Json(Response::Failure { reason: Some("update was rejected".to_string()) })
                    }
                }
            }
        }
        _ => {
            return Json(Response::Failure { reason: Some("Invalid request for Update Node".into()) });
        }
    }
}

async fn delete_node(
    State(state): State<Arc<Mutex<DataStore>>>,
    Path(node_id): Path<String>,
    Json(request): Json<NodeRequest>
) -> Json<Response<Node>> {
    let mut datastore = state.lock().await;
    match request {
        NodeRequest::Op(map_op) => {
            log::info!("Delete Node request is an Op from another peer");
            match &map_op {
                crdts::map::Op::Up { .. } => {
                    return Json(Response::Failure { reason: Some("Invalid Op type for delete dns".into()) });
                }
                crdts::map::Op::Rm { .. } => {
                    datastore.node_state.node_op(map_op);
                    return Json(Response::Success(Success::None))
                }
            }
        }
        NodeRequest::Delete(_id) => {
            log::info!("Delete Node request was a direct request...");
            log::info!("Building Map Op...");
            let map_op = datastore.node_state.remove_node_local(node_id.clone());
            log::info!("Map op created... Applying...");
            datastore.node_state.node_op(map_op.clone());
            match &map_op {
                crdts::map::Op::Rm { .. } => {
                    let request = NodeRequest::Op(map_op);
                    match datastore.broadcast::<Response<Node>>(request, &format!("/node/{}/delete", node_id.clone())).await {
                        Ok(()) => return Json(Response::Success(Success::None)),
                        Err(e) => eprintln!("Error broadcasting Delete Node request: {e}")
                    }
                    return Json(Response::Success(Success::None));
                }
                crdts::map::Op::Up { .. } => {
                    return Json(Response::Failure { reason: Some("Map generated Add context instead of Rm context on Delete request".to_string()) });
                }
            }
        }
        _ => {
            return Json(Response::Failure { reason: Some("Invalid request for delete Node".into()) });
        }
    }
}

async fn get_node(
    State(state): State<Arc<Mutex<DataStore>>>,
    Path(node_id): Path<String>,
) -> Json<Response<Node>> {
    let datastore = state.lock().await;
    if let Some(node) = datastore.node_state.get_node(node_id.clone()) {
        return Json(Response::Success(Success::Some(node)))
    }

    return Json(Response::Failure { reason: Some(format!("Unable to find instance with id: {node_id}"))})
}

async fn list_nodes(
    State(state): State<Arc<Mutex<DataStore>>>,
) -> Json<Response<Node>> {
    let datastore = state.lock().await;
    let list: Vec<Node> = datastore.node_state.map().iter().filter_map(|ctx| {
        let (_, value) = ctx.val;
        match value.val() {
            Some(node) => {
                return Some(node.value())
            }
            None => return None
        }
    }).collect(); 

    return Json(Response::Success(Success::List(list)))
}

#[cfg(test)]
mod tests {
    use crate::instances::{InstanceAnnotations, InstanceCluster, InstanceEncryption, InstanceMetadata, InstanceMonitoring, InstanceResources, InstanceSecurity, InstanceStatus};
    use crate::nodes::{NodeAnnotations, NodeAvailability, NodeCapacity, NodeMetadata, NodeMonitoring};

    use super::*;
    use k256::ecdsa::SigningKey;
    use crdts::{CmRDT, Map};
    use ipnet::IpNet;
    use rand::thread_rng;
    use std::collections::BTreeMap;
    use std::str::FromStr;
    use trust_dns_proto::rr::RecordType;

    // This test builds a MergableState (your datastore state without private keys or node ids)
    // with one entry in each of the maps and then serializes and deserializes it.
    #[test]
    fn test_mergeable_state_serialization() -> Result<(), Box<dyn std::error::Error>> {
        // Define an actor string and create a dummy signing key.
        let actor = "test_actor".to_string();
        let sk = SigningKey::random(&mut thread_rng());
        // We need a hex-encoded private key string (this is just for signing our CRDT updates)
        let pk_str = hex::encode(sk.to_bytes());
        let signing_key = SigningKey::from_slice(&hex::decode(pk_str.clone())?)?;
        
        // Create empty maps for each of the types
        let mut peers: PeerMap = Map::new();
        let mut cidrs: CidrMap = Map::new();
        let mut assocs: AssocMap = Map::new();
        let mut dns: DnsMap = Map::new();
        let mut instances: InstanceMap = Map::new();
        let mut nodes: NodeMap = Map::new();

        // --- Insert a fake peer ---
        let fake_peer = CrdtPeer {
            id: "peer1".to_string(),
            name: "Peer One".to_string(),
            ip: "127.0.0.1".parse()?,
            cidr_id: "cidr1".to_string(),
            public_key: "fake_public_key".to_string(),
            endpoint: None,
            keepalive: Some(30),
            is_admin: false,
            is_disabled: false,
            is_redeemed: false,
            invite_expires: None,
            candidates: vec![],
        };
        let peer_ctx = peers.read_ctx().derive_add_ctx(actor.clone());
        let peer_op = peers.update("peer1".to_string(), peer_ctx, |reg, _| {
            // update returns a signed op (or an error)
            reg.update(fake_peer, actor.clone(), signing_key.clone()).expect("Unable to sign update, Panicking")
        });
        peers.apply(peer_op);

        // --- Insert a fake cidr ---
        let fake_cidr = CrdtCidr {
            id: "cidr1".to_string(),
            name: "CIDR One".to_string(),
            cidr: IpNet::from_str("192.168.0.0/24")?,
            parent: None,
        };
        let cidr_ctx = cidrs.read_ctx().derive_add_ctx(actor.clone());
        let cidr_op = cidrs.update("cidr1".to_string(), cidr_ctx, |reg, _| {
            reg.update(fake_cidr, actor.clone(), signing_key.clone()).expect("Unable to sign update, Panicking")

        });
        cidrs.apply(cidr_op);

        // --- Insert a fake association ---
        let fake_assoc = CrdtAssociation {
            id: ("cidr1".to_string(), "cidr2".to_string()),
            cidr_1: "cidr1".to_string(),
            cidr_2: "cidr2".to_string(),
        };
        let assoc_ctx = assocs.read_ctx().derive_add_ctx(actor.clone());
        let assoc_op = assocs.update("assoc1".to_string(), assoc_ctx, |reg, _| {
            reg.update(fake_assoc, actor.clone(), signing_key.clone()).expect("Unable to sign update, Panicking")

        });
        assocs.apply(assoc_op);

        // --- Insert a fake DNS record ---
        let fake_dns = CrdtDnsRecord {
            domain: "example.com".to_string(),
            record_type: RecordType::A,
            formnet_ip: vec!["127.0.0.1:80".parse()?],
            public_ip: vec!["192.0.2.1:80".parse()?],
            cname_target: None,
            ttl: 300,
            ssl_cert: false,
        };
        let dns_ctx = dns.read_ctx().derive_add_ctx(actor.clone());
        let dns_op = dns.update("example.com".to_string(), dns_ctx, |reg, _| {
            reg.update(fake_dns, actor.clone(), signing_key.clone()).expect("Unable to sign update, Panicking")

        });
        dns.apply(dns_op);

        // --- Insert a fake instance ---
        let fake_instance = Instance {
            instance_id: "instance1".to_string(),
            node_id: "node1".to_string(),
            build_id: "build1".to_string(),
            instance_owner: "owner1".to_string(),
            created_at: 0,
            updated_at: 0,
            last_snapshot: 0,
            formnet_ip: None,
            dns_record: None,
            status: InstanceStatus::Created,
            host_region: "us-east".to_string(),
            resources: InstanceResources {
                vcpus: 2,
                memory_mb: 2048,
                bandwidth_mbps: 100,
                gpu: None,
            },
            cluster: InstanceCluster {
                members: BTreeMap::new(),
            },
            formfile: "".to_string(),
            snapshots: None,
            metadata: InstanceMetadata {
                tags: vec!["tag1".to_string()],
                description: "Fake instance".to_string(),
                annotations: InstanceAnnotations {
                    deployed_by: "test".to_string(),
                    network_id: 1,
                    build_commit: None,
                },
                security: InstanceSecurity {
                    encryption: InstanceEncryption {
                        is_encrypted: false,
                        scheme: None,
                    },
                    tee: false,
                    hsm: false,
                },
                monitoring: InstanceMonitoring {
                    logging_enabled: false,
                    metrics_endpoint: "http://localhost".to_string(),
                },
            },
        };
        let inst_ctx = instances.read_ctx().derive_add_ctx(actor.clone());
        let inst_op = instances.update("instance1".to_string(), inst_ctx, |reg, _| {
            reg.update(fake_instance, actor.clone(), signing_key.clone()).expect("Unable to sign update, Panicking")

        });
        instances.apply(inst_op);

        // --- Insert a fake node ---
        let fake_node = Node {
            node_id: "node1".to_string(),
            node_owner: "owner_node".to_string(),
            created_at: 0,
            updated_at: 0,
            last_heartbeat: 0,
            host_region: "us-west".to_string(),
            capacity: NodeCapacity {
                vcpus: 4,
                memory_mb: 8192,
                bandwidth_mbps: 1000,
                gpu: None,
            },
            availability: NodeAvailability {
                uptime_seconds: 3600,
                load_average: 10,
                status: "online".to_string(),
            },
            metadata: NodeMetadata {
                tags: vec!["node_tag".to_string()],
                description: "Fake node".to_string(),
                annotations: NodeAnnotations {
                    roles: vec!["compute".to_string()],
                    datacenter: "dc1".to_string(),
                },
                monitoring: NodeMonitoring {
                    logging_enabled: true,
                    metrics_endpoint: "http://node.metrics".to_string(),
                },
            },
        };
        let node_ctx = nodes.read_ctx().derive_add_ctx(actor.clone());
        let node_op = nodes.update("node1".to_string(), node_ctx, |reg, _| {
            reg.update(fake_node, actor.clone(), signing_key.clone()).expect("Unable to sign update, Panicking")

        });
        nodes.apply(node_op);

        // --- Build the mergeable state ---
        let mergeable_state = MergeableState {
            peers,
            cidrs,
            assocs,
            dns,
            instances,
            nodes,
        };

        assert!(serde_json::to_string(&mergeable_state.peers).is_ok());
        assert!(serde_json::to_string(&mergeable_state.cidrs).is_ok());
        assert!(serde_json::to_string(&mergeable_state.assocs).is_ok());
        assert!(serde_json::to_string(&mergeable_state.dns).is_ok());
        assert!(serde_json::to_string(&mergeable_state.instances).is_ok());
        assert!(serde_json::to_string(&mergeable_state.nodes).is_ok());

        // --- Serialization ---
        let serialized = serde_json::to_string_pretty(&mergeable_state)?;
        println!("Serialized mergeable state:\n{}", serialized);

        // --- Deserialization ---
        let _deserialized: MergeableState = serde_json::from_str(&serialized)?;
        // (You can compare mergeable_state and deserialized here if your types implement PartialEq.)
        println!("Deserialization succeeded.");

        Ok(())
    }
}

