use std::{collections::{HashMap, HashSet}, path::PathBuf, sync::Arc};
use axum::{extract::State, Json};
use form_dns::{api::{DomainRequest, DomainResponse}, store::FormDnsRecord};
use form_p2p::queue::{QueueRequest, QueueResponse, QUEUE_PORT};
use rand::{seq::SliceRandom, thread_rng};
use reqwest::Client;
use form_node_metrics::{capabilities::NodeCapabilities, capacity::NodeCapacity, metrics::NodeMetrics, NodeMetricsRequest};
use serde_json::Value;
use shared::{AssociationContents, Cidr, CidrContents, PeerContents};
use tiny_keccak::{Hasher, Sha3};
use tokio::sync::Mutex;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use crdts::{map::Op, BFTReg, CvRDT, Map, CmRDT};
use crate::{accounts::{Account, AccountOp, AccountState, AuthorizationLevel}, agent::{AIAgent, AgentMap, AgentOp, AgentState}, db::{open_db, write_datastore, DbHandle}, instances::{ClusterMember, Instance, InstanceOp, InstanceState}, model::{AIModel, ModelMap, ModelOp, ModelState}, network::{AssocOp, CidrOp, CrdtAssociation, CrdtCidr, CrdtDnsRecord, CrdtPeer, DnsOp, NetworkState, PeerOp}, nodes::{Node, NodeOp, NodeState}, tasks::{TaskState, Task, TaskOp, TaskStatus, TaskId}};
use lazy_static::lazy_static;
use url::Host;
use hex;

lazy_static! {
    pub static ref DB_HANDLE: DbHandle = open_db(PathBuf::from("/var/lib/formation/db/form.db"));
}

pub type PeerMap = Map<String, BFTReg<CrdtPeer<String>, String>, String>;
pub type CidrMap = Map<String, BFTReg<CrdtCidr<String>, String>, String>;
pub type AssocMap = Map<String, BFTReg<CrdtAssociation<String>, String>, String>;
pub type DnsMap = Map<String, BFTReg<CrdtDnsRecord, String>, String>;
pub type InstanceMap = Map<String, BFTReg<Instance, String>, String>;
pub type NodeMap = Map<String, BFTReg<Node, String>, String>;
pub type AccountMap = Map<String, BFTReg<Account, String>, String>;

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
    nodes: NodeMap,
    accounts: AccountMap,
    agents: AgentMap,
    models: ModelMap
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
            accounts: value.account_state.map.clone(),
            agents: value.agent_state.map.clone(),
            models: value.model_state.map.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DataStore {
    pub network_state: NetworkState,
    pub instance_state: InstanceState,
    pub node_state: NodeState,
    pub account_state: AccountState,
    pub agent_state: AgentState,
    pub model_state: ModelState,
    pub task_state: TaskState,
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AccountRequest {
    Op(AccountOp),
    Create(Account),
    Update(Account),
    Delete(String),
    AddOwnedInstance {
        address: String,
        instance_id: String,
    },
    RemoveOwnedInstance {
        address: String,
        instance_id: String,
    },
    AddAuthorization {
        address: String,
        instance_id: String,
        level: AuthorizationLevel,
    },
    RemoveAuthorization {
        address: String,
        instance_id: String,
    },
    TransferOwnership {
        from_address: String,
        to_address: String,
        instance_id: String,
    },
    AddOwnedAgent {
        address: String,
        agent_id: String,
    },
    RemoveOwnedAgent {
        address: String,
        agent_id: String,
    },
    AddOwnedModel {
        address: String,
        model_id: String,
    },
    RemoveOwnedModel {
        address: String,
        model_id: String,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AgentRequest {
    Op(AgentOp),
    Create(AIAgent),
    Update(AIAgent),
    Delete(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ModelRequest {
    Op(ModelOp),
    Create(AIModel),
    Update(AIModel),
    Delete(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TaskRequest {
    Op(TaskOp),
    Create(Task),
    UpdateStatus { // For a node to report progress/completion/failure
        task_id: TaskId,
        status: TaskStatus,
        progress: Option<u8>,
        result_info: Option<String>,
    },
    AssignNode { // For an admin/scheduler to assign a node
        task_id: TaskId,
        node_id: Option<String>, // Option to unassign
        status_after_assign: Option<TaskStatus>, // e.g., PoCAssigned or Claimed
    },
    // Add other specific update requests as needed, e.g., UpdateResponsibleNodes
}

impl DataStore {
    pub fn new(node_id: String, pk: String) -> Self {
        let network_state = NetworkState::new(node_id.clone(), pk.clone());
        let instance_state = InstanceState::new(node_id.clone(), pk.clone());
        let node_state = NodeState::new(node_id.clone(), pk.clone());
        let account_state = AccountState::new(node_id.clone(), pk.clone());
        let agent_state = AgentState::new(node_id.clone(), pk.clone());
        let model_state = ModelState::new(node_id.clone(), pk.clone());
        let task_state = TaskState::new(node_id.clone(), pk.clone());


        Self { 
            network_state,
            instance_state,
            node_state,
            account_state,
            agent_state,
            model_state,
            task_state,
        } 
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
        local.account_state.map.merge(other.accounts);
        local.agent_state.map.merge(other.agents);
        local.model_state.map.merge(other.models);
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

    pub async fn handle_peer_request(&mut self, peer_request: PeerRequest) -> Result<(), Box<dyn std::error::Error>> {
        match peer_request {
            PeerRequest::Op(op) => self.handle_peer_op(op).await?,
            PeerRequest::Join(join) => self.handle_peer_join(join).await?,
            PeerRequest::Update(up) => self.handle_peer_update(up).await?,
            PeerRequest::Delete(del) => self.handle_peer_delete(del).await?,
        }

        Ok(())
    }

    pub async fn handle_peer_op(&mut self, peer_op: PeerOp<String>) -> Result<(), Box<dyn std::error::Error>> {
        let mut op_applied_successfully = false;
        let op_to_propagate = peer_op; // Original op, will be cloned for propagation if needed, or for local apply

        match &op_to_propagate { // Match on reference to the original op
            Op::Up { dot: _, key, op } => {
                // Apply a clone locally, original op_to_propagate can still be used for gossip
                self.network_state.peer_op(op_to_propagate.clone()); 
                if let (true, _) = self.network_state.peer_op_success(key.clone(), op.clone()) {
                    log::info!("Peer Op::Up successfully applied locally.");
                    op_applied_successfully = true;
                } else {
                    log::error!("Peer Op::Up failed to apply locally or was a no-op.");
                    return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Peer Op::Up failed local application")));
                }
            }
            Op::Rm { .. } => {
                // Apply a clone locally
                self.network_state.peer_op(op_to_propagate.clone()); 
                log::info!("Peer Op::Rm applied locally.");
                op_applied_successfully = true; 
            }
        }

        if op_applied_successfully {
            #[cfg(feature = "devnet")]
            {
                log::info!("devnet mode: PeerOp applied locally. Gossiping directly with op: {:?}", op_to_propagate);
                self.gossip_op_directly(&op_to_propagate, "PeerOp").await?;
            }
            #[cfg(not(feature = "devnet"))]
            {
                log::info!("production mode: Queuing PeerOp ({:?}).", op_to_propagate);
                DataStore::write_to_queue(crate::datastore::PeerRequest::Op(op_to_propagate.clone()), 0).await?;
            }
            write_datastore(&DB_HANDLE, &self.clone())?;
        }

        Ok(())
    }

    pub async fn handle_peer_join(&mut self, contents: PeerContents<String>) -> Result<(), Box<dyn std::error::Error>> {
        let op = self.network_state.update_peer_local(contents);
        self.handle_peer_op(op).await?;

        Ok(())
    }

    pub async fn handle_peer_update(&mut self, contents: PeerContents<String>) -> Result<(), Box<dyn std::error::Error>> {
        let op = self.network_state.update_peer_local(contents);
        self.handle_peer_op(op).await?;

        Ok(())
    }

    pub async fn handle_peer_delete(&mut self, id: String) -> Result<(), Box<dyn std::error::Error>> {
        let op = self.network_state.remove_peer_local(id);
        self.handle_peer_op(op).await?;

        Ok(())
    }

    pub async fn handle_cidr_request(&mut self, cidr_request: CidrRequest) -> Result<(), Box<dyn std::error::Error>> {
        match cidr_request {
            CidrRequest::Op(op) => self.handle_cidr_op(op).await?,
            CidrRequest::Create(create) => self.handle_cidr_create(create).await?,
            CidrRequest::Update(update) => self.handle_cidr_update(update).await?,
            CidrRequest::Delete(delete) => self.handle_cidr_delete(delete).await?,
        }

        Ok(())
    }

    pub async fn handle_cidr_op(&mut self, cidr_op: CidrOp<String>) -> Result<(), Box<dyn std::error::Error>> {
        let mut op_applied_successfully = false;
        let op_to_propagate = cidr_op.clone(); // Clone for propagation

        match &cidr_op {
            Op::Up { dot: _, key, op } => {
                self.network_state.cidr_op(cidr_op.clone()); // Apply locally
                if let (true, _) = self.network_state.cidr_op_success(key.clone(), op.clone()) {
                    log::info!("CIDR Op::Up successfully applied locally.");
                    op_applied_successfully = true;
                } else {
                    log::error!("CIDR Op::Up failed to apply locally or was a no-op.");
                    return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "CIDR Op::Up failed local application")));
                }
            }
            Op::Rm { .. } => {
                self.network_state.cidr_op(cidr_op); // Apply Rm locally
                log::info!("CIDR Op::Rm applied locally.");
                op_applied_successfully = true;
            }
        }

        if op_applied_successfully {
            #[cfg(feature = "devnet")]
            {
                log::info!("devnet mode: CIDR Op applied locally. Gossiping directly with op: {:?}", op_to_propagate);
                self.gossip_op_directly(&op_to_propagate, "CidrOp").await?;
            }
            #[cfg(not(feature = "devnet"))]
            {
                log::info!("production mode: Queuing CIDR Op ({:?}).", op_to_propagate);
                DataStore::write_to_queue(crate::datastore::CidrRequest::Op(op_to_propagate.clone()), 1).await?;
            }
            write_datastore(&DB_HANDLE, &self.clone())?;
        }
        Ok(())
    }

    pub async fn handle_cidr_create(&mut self, create: CidrContents<String>) -> Result<(), Box<dyn std::error::Error>> {
        let op = self.network_state.update_cidr_local(create);
        self.handle_cidr_op(op).await?;

        Ok(())
    }

    pub async fn handle_cidr_update(&mut self, update: CidrContents<String>) -> Result<(), Box<dyn std::error::Error>> {
        let op = self.network_state.update_cidr_local(update);
        self.handle_cidr_op(op).await?;

        Ok(())
    }

    pub async fn handle_cidr_delete(&mut self, delete: String) -> Result<(), Box<dyn std::error::Error>> {
        let op = self.network_state.remove_cidr_local(delete);
        self.handle_cidr_op(op).await?;

        Ok(())
    }

    pub async fn handle_assoc_request(&mut self, assoc_request: AssocRequest) -> Result<(), Box<dyn std::error::Error>> {
        match assoc_request {
            AssocRequest::Op(op) => self.handle_assoc_op(op).await?,
            AssocRequest::Create(create) => self.handle_assoc_create(create).await?,
            AssocRequest::Delete(delete) => self.handle_assoc_delete(delete).await?,
        }
        Ok(())
    }

    pub async fn handle_assoc_op(&mut self, assoc_op: AssocOp<String>) -> Result<(), Box<dyn std::error::Error>> {
        let mut op_applied_successfully = false;
        let op_to_propagate = assoc_op.clone(); // Clone for propagation

        match &assoc_op {
            Op::Up { dot: _, key, op } => {
                self.network_state.associations_op(assoc_op.clone()); // Apply locally
                if let (true, _) = self.network_state.associations_op_success(key.clone(), op.clone()) {
                    log::info!("Assoc Op::Up successfully applied locally.");
                    op_applied_successfully = true;
                } else {
                    log::error!("Assoc Op::Up failed to apply locally or was a no-op.");
                    return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Assoc Op::Up failed local application")));
                }
            }
            Op::Rm { .. } => {
                self.network_state.associations_op(assoc_op); // Apply Rm locally
                log::info!("Assoc Op::Rm applied locally.");
                op_applied_successfully = true;
            }
        }

        if op_applied_successfully {
            #[cfg(feature = "devnet")]
            {
                log::info!("devnet mode: Assoc Op applied locally. Gossiping directly with op: {:?}", op_to_propagate);
                self.gossip_op_directly(&op_to_propagate, "AssocOp").await?;
            }
            #[cfg(not(feature = "devnet"))]
            {
                log::info!("production mode: Queuing Assoc Op ({:?}).", op_to_propagate);
                DataStore::write_to_queue(crate::datastore::AssocRequest::Op(op_to_propagate.clone()), 2).await?;
            }
            write_datastore(&DB_HANDLE, &self.clone())?;
        }
        Ok(())
    }

    pub async fn handle_assoc_create(&mut self, create: AssociationContents<String>) -> Result<(), Box<dyn std::error::Error>> {
        let op = self.network_state.update_association_local(create);
        self.handle_assoc_op(op).await?;

        Ok(())
    }

    pub async fn handle_assoc_delete(&mut self, delete: (String, String)) -> Result<(), Box<dyn std::error::Error>> {
        let op = self.network_state.remove_association_local(delete);
        self.handle_assoc_op(op).await?;

        Ok(())
    }

    pub async fn handle_dns_request(&mut self, dns_request: DnsRequest) -> Result<(), Box<dyn std::error::Error>> {
        match dns_request {
            DnsRequest::Op(op) => self.handle_dns_op(op).await?,
            DnsRequest::Create(create) => self.handle_dns_create(create).await?,
            DnsRequest::Update(update) => self.handle_dns_update(update).await?,
            DnsRequest::Delete(domain) => self.handle_dns_delete(domain).await? 
        }

        Ok(())
    }

    pub async fn handle_dns_op(&mut self, dns_op: DnsOp) -> Result<(), Box<dyn std::error::Error>> {
        let mut op_applied_successfully = false;
        let op_to_propagate = dns_op.clone(); // Clone for propagation

        match &dns_op {
            Op::Up { dot: _, key, op } => {
                self.network_state.dns_op(dns_op.clone()); // Apply locally
                if let (true, _) = self.network_state.dns_op_success(key.clone(), op.clone()) {
                    log::info!("DNS Op::Up successfully applied locally.");
                    op_applied_successfully = true;
                } else {
                    log::error!("DNS Op::Up failed to apply locally or was a no-op.");
                    return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "DNS Op::Up failed local application")));
                }
            }
            Op::Rm { .. } => {
                self.network_state.dns_op(dns_op); // Apply Rm locally
                log::info!("DNS Op::Rm applied locally.");
                op_applied_successfully = true;
            }
        }

        if op_applied_successfully {
            #[cfg(feature = "devnet")]
            {
                log::info!("devnet mode: DNS Op applied locally. Gossiping directly with op: {:?}", op_to_propagate);
                self.gossip_op_directly(&op_to_propagate, "DnsOp").await?;
            }
            #[cfg(not(feature = "devnet"))]
            {
                log::info!("production mode: Queuing DNS Op ({:?}).", op_to_propagate);
                DataStore::write_to_queue(crate::datastore::DnsRequest::Op(op_to_propagate.clone()), 3).await?;
            }
            write_datastore(&DB_HANDLE, &self.clone())?;
        }
        Ok(())
    }

    pub async fn handle_dns_create(&mut self, create: FormDnsRecord) -> Result<(), Box<dyn std::error::Error>> {
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

    pub async fn handle_dns_update(&mut self, update: FormDnsRecord) -> Result<(), Box<dyn std::error::Error>> {
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

    pub async fn handle_dns_delete(&mut self, delete: String) -> Result<(), Box<dyn std::error::Error>> {
        let op = self.network_state.remove_dns_local(delete.clone());
        self.handle_dns_op(op).await?;

        Client::new()
            .post(format!("http://127.0.0.1:3005/record/{}/delete", delete))
            .send().await?
            .json::<DomainResponse>().await?;

        Ok(())
    }

    pub async fn handle_instance_request(&mut self, instance_request: InstanceRequest) -> Result<(), Box<dyn std::error::Error>> {
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

    pub async fn handle_add_cluster_member(
        &mut self,
        build_id: String,
        cluster_member: ClusterMember
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut instances = self.instance_state.get_instances_by_build_id(build_id);
        let mut iter_mut = instances.iter_mut();
        while let Some(instance) = iter_mut.next() {
            instance.cluster.insert(cluster_member.clone());
            let instance_op = self.instance_state.update_instance_local(instance.clone());
            self.handle_instance_op(instance_op).await?;
        }
        Ok(())
    }

    pub async fn handle_remove_cluster_member(
        &mut self,
        build_id: String,
        cluster_member_id: String
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut instances = self.instance_state.get_instances_by_build_id(build_id);
        let mut iter_mut = instances.iter_mut();
        while let Some(instance) = iter_mut.next() {
            instance.cluster.remove(&cluster_member_id);
            let instance_op = self.instance_state.update_instance_local(instance.clone());
            self.handle_instance_op(instance_op).await?;
        }
        Ok(())
    }

    pub async fn handle_instance_op(&mut self, instance_op: InstanceOp) -> Result<(), Box<dyn std::error::Error>> {
        let mut op_applied_successfully = false;
        let op_to_propagate = instance_op.clone(); // Clone for propagation

        match &instance_op {
            Op::Up { dot: _, key, op } => {
                self.instance_state.instance_op(instance_op.clone()); // Apply locally
                if let (true, _) = self.instance_state.instance_op_success(key.clone(), op.clone()) {
                    log::info!("Instance Op::Up successfully applied locally.");
                    op_applied_successfully = true;
                } else {
                    log::error!("Instance Op::Up failed to apply locally or was a no-op.");
                    return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Instance Op::Up failed local application")));
                }
            }
            Op::Rm { .. } => {
                self.instance_state.instance_op(instance_op); // Apply Rm locally
                log::info!("Instance Op::Rm applied locally.");
                op_applied_successfully = true;
            }
        }

        if op_applied_successfully {
            #[cfg(feature = "devnet")]
            {
                log::info!("devnet mode: Instance Op applied locally. Gossiping directly with op: {:?}", op_to_propagate);
                self.gossip_op_directly(&op_to_propagate, "InstanceOp").await?;
            }
            #[cfg(not(feature = "devnet"))]
            {
                log::info!("production mode: Queuing Instance Op ({:?}).", op_to_propagate);
                DataStore::write_to_queue(crate::datastore::InstanceRequest::Op(op_to_propagate.clone()), 4).await?;
            }
            write_datastore(&DB_HANDLE, &self.clone())?;
        }
        Ok(())
    }

    pub async fn handle_instance_create(&mut self, create: Instance) -> Result<(), Box<dyn std::error::Error>> {
        let op = self.instance_state.update_instance_local(create);
        self.handle_instance_op(op).await?;

        Ok(())
    }

    pub async fn handle_instance_update(&mut self, update: Instance) -> Result<(), Box<dyn std::error::Error>> {
        let op = self.instance_state.update_instance_local(update);
        self.handle_instance_op(op).await?;

        Ok(())
    }

    pub async fn handle_instance_delete(&mut self, delete: String) -> Result<(), Box<dyn std::error::Error>> {
        let op = self.instance_state.remove_instance_local(delete);
        self.handle_instance_op(op).await?;

        Ok(())
    }

    async fn handle_node_request(&mut self, node_request: NodeRequest) -> Result<(), Box<dyn std::error::Error>> {
        match node_request {
            NodeRequest::Op(op) => self.handle_node_op(op).await?,
            NodeRequest::Create(create) => self.handle_node_create(create).await?,
            NodeRequest::Update(update) => self.handle_node_update(update).await?,
            NodeRequest::Delete(id) => self.handle_node_delete(id).await?,
        }
        Ok(())
    }

    pub async fn handle_node_metrics_request(&mut self, node_metrics_request: NodeMetricsRequest) -> Result<(), Box<dyn std::error::Error>> {
        match node_metrics_request {
            NodeMetricsRequest::SetInitialMetrics { node_id, node_capabilities, node_capacity } => self.handle_node_initial_metrics(node_id, node_capabilities, node_capacity).await?,
            NodeMetricsRequest::Heartbeat { node_id, timestamp } => self.handle_node_heartbeat(node_id, timestamp).await?,
            NodeMetricsRequest::UpdateMetrics { node_id, node_capacity, node_metrics } => self.handle_node_update_metrics(node_id, node_capacity, node_metrics).await?,
        }
        Ok(())
    }

    pub async fn handle_node_heartbeat(&mut self, node_id: String, timestamp: i64) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(node_op) = self.node_state.update_node_heartbeat(node_id, timestamp) {
            self.handle_node_op(node_op).await?;
        }
        Ok(())
    }

    pub async fn handle_node_update_metrics(&mut self, node_id: String, node_capacity: NodeCapacity, node_metrics: NodeMetrics) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(node_op) = self.node_state.update_node_metrics(node_id, node_capacity, node_metrics) {
            self.handle_node_op(node_op).await?;
        }
        Ok(())
    }

    pub async fn handle_node_initial_metrics(&mut self, node_id: String, node_capabilities: NodeCapabilities, node_capacity: NodeCapacity) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(node_op) = self.node_state.set_initial_node_capabilities(node_id, node_capacity, node_capabilities) {
            self.handle_node_op(node_op).await?;
        }
        Ok(())
    }

    pub async fn handle_node_op(&mut self, node_op: NodeOp) -> Result<(), Box<dyn std::error::Error>> {
        let mut op_applied_successfully = false;
        let op_to_propagate = node_op.clone(); // Clone for propagation

        match &node_op {
            Op::Up { dot: _, key, op } => {
                self.node_state.node_op(node_op.clone()); // Apply locally
                if let (true, _) = self.node_state.node_op_success(key.clone(), op.clone()) {
                    log::info!("Node Op::Up successfully applied locally.");
                    op_applied_successfully = true;
                } else {
                    log::error!("Node Op::Up failed to apply locally or was a no-op.");
                    return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Node Op::Up failed local application")));
                }
            }
            Op::Rm { .. } => {
                self.node_state.node_op(node_op); // Apply Rm locally
                log::info!("Node Op::Rm applied locally.");
                op_applied_successfully = true;
            }
        }

        if op_applied_successfully {
            #[cfg(feature = "devnet")]
            {
                log::info!("devnet mode: Node Op applied locally. Gossiping directly with op: {:?}", op_to_propagate);
                self.gossip_op_directly(&op_to_propagate, "NodeOp").await?;
            }
            #[cfg(not(feature = "devnet"))]
            {
                log::info!("production mode: Queuing Node Op ({:?}).", op_to_propagate);
                DataStore::write_to_queue(crate::datastore::NodeRequest::Op(op_to_propagate.clone()), 5).await?;
            }
            write_datastore(&DB_HANDLE, &self.clone())?;
        }
        Ok(())
    }

    pub async fn handle_node_create(&mut self, create: Node) -> Result<(), Box<dyn std::error::Error>> {
        let op = self.node_state.update_node_local(create);
        self.handle_node_op(op).await?;

        Ok(())
    }

    pub async fn handle_node_update(&mut self, update: Node) -> Result<(), Box<dyn std::error::Error>> {
        let op = self.node_state.update_node_local(update);
        self.handle_node_op(op).await?;

        Ok(())
    }

    pub async fn handle_node_delete(&mut self, delete: String) -> Result<(), Box<dyn std::error::Error>> {
        let op = self.node_state.remove_node_local(delete);
        self.handle_node_op(op).await?;

        Ok(())
    }

    // Account handler methods
    pub async fn handle_account_request(&mut self, account_request: AccountRequest) -> Result<(), Box<dyn std::error::Error>> {
        match account_request {
            AccountRequest::Op(op) => {
                self.handle_account_op(op).await?;
            }
            AccountRequest::Create(create) => {
                self.handle_account_create(create).await?;
            }
            AccountRequest::Update(update) => {
                self.handle_account_update(update).await?;
            }
            AccountRequest::Delete(delete) => {
                self.handle_account_delete(delete).await?;
            }
            AccountRequest::AddOwnedInstance { address, instance_id } => {
                self.handle_add_owned_instance(address, instance_id).await?;
            }
            AccountRequest::RemoveOwnedInstance { address, instance_id } => {
                self.handle_remove_owned_instance(address, instance_id).await?;
            }
            AccountRequest::AddAuthorization { address, instance_id, level } => {
                self.handle_add_authorization(address, instance_id, level).await?;
            }
            AccountRequest::RemoveAuthorization { address, instance_id } => {
                self.handle_remove_authorization(address, instance_id).await?;
            }
            AccountRequest::TransferOwnership { from_address, to_address, instance_id } => {
                self.handle_transfer_ownership(from_address, to_address, instance_id).await?;
            }
            AccountRequest::AddOwnedAgent { address, agent_id } => {
                self.handle_add_owned_agent(address, agent_id).await?;
            }
            AccountRequest::RemoveOwnedAgent { address, agent_id } => {
                self.handle_remove_owned_agent(address, agent_id).await?;
            }
            AccountRequest::AddOwnedModel { address, model_id } => {
                self.handle_add_owned_model(address, model_id).await?;
            }
            AccountRequest::RemoveOwnedModel { address, model_id } => {
                self.handle_remove_owned_model(address, model_id).await?;
            }
        }

        Ok(())
    }

    pub async fn handle_account_op(&mut self, account_op: AccountOp) -> Result<(), Box<dyn std::error::Error>> {
        let mut op_applied_successfully = false;
        let op_to_propagate = account_op.clone(); // Clone for propagation

        match &account_op {
            Op::Up { dot: _, key, op } => {
                self.account_state.account_op(account_op.clone()); // Apply locally
                if let (true, _) = self.account_state.account_op_success(key.clone(), op.clone()) {
                    log::info!("Account Op::Up successfully applied locally.");
                    op_applied_successfully = true;
                } else {
                    log::error!("Account Op::Up failed to apply locally or was a no-op.");
                    return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Account Op::Up failed local application")));
                }
            }
            Op::Rm { .. } => {
                self.account_state.account_op(account_op); // Apply Rm locally
                log::info!("Account Op::Rm applied locally.");
                op_applied_successfully = true;
            }
        }

        if op_applied_successfully {
            #[cfg(feature = "devnet")]
            {
                log::info!("devnet mode: Account Op applied locally. Gossiping directly with op: {:?}", op_to_propagate);
                self.gossip_op_directly(&op_to_propagate, "AccountOp").await?;
            }
            #[cfg(not(feature = "devnet"))]
            {
                log::info!("production mode: Queuing Account Op ({:?}).", op_to_propagate);
                DataStore::write_to_queue(crate::datastore::AccountRequest::Op(op_to_propagate.clone()), 7).await?;
            }
            write_datastore(&DB_HANDLE, &self.clone())?;
        }
        Ok(())
    }

    pub async fn handle_account_create(&mut self, create: Account) -> Result<(), Box<dyn std::error::Error>> {
        let op = self.account_state.update_account_local(create);
        self.handle_account_op(op).await
    }

    pub async fn handle_account_update(&mut self, update: Account) -> Result<(), Box<dyn std::error::Error>> {
        let op = self.account_state.update_account_local(update);
        self.handle_account_op(op).await
    }

    pub async fn handle_account_delete(&mut self, delete: String) -> Result<(), Box<dyn std::error::Error>> {
        // Check if account exists
        match self.account_state.get_account(&delete) {
            None => return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Account with address {} does not exist", delete)
            ))),
            _ => {}
        };
        
        // Delete the account (remove it from the map)
        let op = self.account_state.remove_account_local(delete);
        // Apply the operation directly
        self.account_state.map.apply(op.clone());
        
        // Write to queue
        if let Err(e) = DataStore::write_to_queue(AccountRequest::Op(op), 7).await {
            log::error!("Error writing to queue: {}", e);
        }
        
        Ok(())
    }

    pub async fn handle_add_owned_instance(&mut self, address: String, instance_id: String) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(mut account) = self.account_state.get_account(&address) {
            account.add_owned_instance(instance_id);
            let op = self.account_state.update_account_local(account);
            self.handle_account_op(op).await?;
        }
        Ok(())
    }

    pub async fn handle_remove_owned_instance(&mut self, address: String, instance_id: String) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(mut account) = self.account_state.get_account(&address) {
            account.remove_owned_instance(&instance_id);
            let op = self.account_state.update_account_local(account);
            self.handle_account_op(op).await?;
        }
        Ok(())
    }

    pub async fn handle_add_authorization(&mut self, address: String, instance_id: String, level: AuthorizationLevel) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(mut account) = self.account_state.get_account(&address) {
            account.add_authorization(instance_id, level);
            let op = self.account_state.update_account_local(account);
            self.handle_account_op(op).await?;
        }
        Ok(())
    }

    pub async fn handle_remove_authorization(&mut self, address: String, instance_id: String) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(mut account) = self.account_state.get_account(&address) {
            account.remove_authorization(&instance_id);
            let op = self.account_state.update_account_local(account);
            self.handle_account_op(op).await?;
        }
        Ok(())
    }

    pub async fn handle_transfer_ownership(&mut self, from_address: String, to_address: String, instance_id: String) -> Result<(), Box<dyn std::error::Error>> {
        // Check and update the accounts
        if let Some(mut account) = self.account_state.get_account(&from_address) {
            account.remove_owned_instance(&instance_id);
            let op = self.account_state.update_account_local(account);
            self.handle_account_op(op).await?;
        }
        if let Some(mut account) = self.account_state.get_account(&to_address) {
            account.add_owned_instance(instance_id.clone());
            let op = self.account_state.update_account_local(account);
            self.handle_account_op(op).await?;
        }
        
        // Update the instance owner field
        if let Some(mut instance) = self.instance_state.get_instance(instance_id.clone()) {
            instance.instance_owner = to_address;
            let op = self.instance_state.update_instance_local(instance);
            self.handle_instance_op(op).await?;
        }
        
        Ok(())
    }

    pub async fn handle_add_owned_agent(&mut self, address: String, agent_id: String) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(mut account) = self.account_state.get_account(&address) {
            account.add_owned_agent(agent_id);
            let op = self.account_state.update_account_local(account);
            self.handle_account_op(op).await?;
        }
        Ok(())
    }

    pub async fn handle_remove_owned_agent(&mut self, address: String, agent_id: String) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(mut account) = self.account_state.get_account(&address) {
            account.remove_owned_agent(&agent_id);
            let op = self.account_state.update_account_local(account);
            self.handle_account_op(op).await?;
        }
        Ok(())
    }

    pub async fn handle_add_owned_model(&mut self, address: String, model_id: String) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(mut account) = self.account_state.get_account(&address) {
            account.add_owned_model(model_id);
            let op = self.account_state.update_account_local(account);
            self.handle_account_op(op).await?;
        }
        Ok(())
    }
    
    pub async fn handle_remove_owned_model(&mut self, address: String, model_id: String) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(mut account) = self.account_state.get_account(&address) {
            account.remove_owned_model(&model_id);
            let op = self.account_state.update_account_local(account);
            self.handle_account_op(op).await?;
        }
        Ok(())
    }

    pub async fn handle_agent_request(&mut self, account_request: AgentRequest) -> Result<(), Box<dyn std::error::Error>> {
        match account_request {
            AgentRequest::Op(op) => {
                self.handle_agent_op(op).await?;
            }
            AgentRequest::Create(create) => {
                self.handle_agent_create(create).await?;
            }
            AgentRequest::Update(update) => {
                self.handle_agent_update(update).await?;
            }
            AgentRequest::Delete(delete) => {
                self.handle_agent_delete(delete).await?;
            }
        }

        Ok(())
    }

    pub async fn handle_agent_create(&mut self, create: AIAgent) -> Result<(), Box<dyn std::error::Error>> {
        let op = self.agent_state.update_agent_local(create);
        self.handle_agent_op(op).await
    }

    pub async fn handle_agent_update(&mut self, update: AIAgent) -> Result<(), Box<dyn std::error::Error>> {
        let op = self.agent_state.update_agent_local(update);
        self.handle_agent_op(op).await
    }

    pub async fn handle_agent_delete(&mut self, delete: String) -> Result<(), Box<dyn std::error::Error>> {
        // Check if account exists
        match self.agent_state.get_agent(&delete) {
            None => return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Account with address {} does not exist", delete)
            ))),
            _ => {}
        };
        
        // Delete the account (remove it from the map)
        let op = self.agent_state.remove_agent_local(delete);
        // Apply the operation directly
        self.agent_state.map.apply(op.clone());
        
        // Write to queue
        if let Err(e) = DataStore::write_to_queue(AgentRequest::Op(op), 8).await {
            log::error!("Error writing to queue: {}", e);
        }

        Ok(())
    }

    pub async fn handle_agent_op(&mut self, agent_op: AgentOp) -> Result<(), Box<dyn std::error::Error>> {
        self.agent_state.map.apply(agent_op);
        Ok(())
    }

    pub async fn handle_model_request(&mut self, model_request: ModelRequest) -> Result<(), Box<dyn std::error::Error>> {
        match model_request {
            ModelRequest::Op(op) => {
                self.handle_model_op(op).await?;
            }
            ModelRequest::Create(create) => {
                self.handle_model_create(create).await?;
            }
            ModelRequest::Update(update) => {
                self.handle_model_update(update).await?;
            }
            ModelRequest::Delete(delete) => {
                self.handle_model_delete(delete).await?;
            }
        }

        Ok(())
    }

    pub async fn handle_model_create(&mut self, create: AIModel) -> Result<(), Box<dyn std::error::Error>> {
        let op = self.model_state.update_model_local(create);
        self.handle_model_op(op).await
    }

    pub async fn handle_model_update(&mut self, update: AIModel) -> Result<(), Box<dyn std::error::Error>> {
        let op = self.model_state.update_model_local(update);
        self.handle_model_op(op).await
    }

    pub async fn handle_model_delete(&mut self, delete: String) -> Result<(), Box<dyn std::error::Error>> {
        // Check if account exists
        match self.model_state.get_model(&delete) {
            None => return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Account with address {} does not exist", delete)
            ))),
            _ => {}
        };
        
        // Delete the account (remove it from the map)
        let op = self.model_state.remove_model_local(delete);
        // Apply the operation directly
        self.model_state.map.apply(op.clone());
        
        // Write to queue
        if let Err(e) = DataStore::write_to_queue(ModelRequest::Op(op), 9).await {
            log::error!("Error writing to queue: {}", e);
        }

        Ok(())
    }

    pub async fn handle_model_op(&mut self, model_op: ModelOp) -> Result<(), Box<dyn std::error::Error>> {
        self.model_state.map.apply(model_op);
        Ok(())
    }

    // Task handler methods
    pub async fn handle_task_request(&mut self, task_request: TaskRequest) -> Result<(), Box<dyn std::error::Error>> {
        match task_request {
            TaskRequest::Op(op) => self.handle_task_op(op).await?,
            TaskRequest::Create(mut task_to_create) => {
                // Ensure initial status for PoC tasks
                if matches!(task_to_create.task_variant, crate::tasks::TaskVariant::BuildImage(_) | crate::tasks::TaskVariant::LaunchInstance(_)) {
                    task_to_create.status = crate::tasks::TaskStatus::PendingPoCAssessment;
                }
                // TODO: task_id generation if not provided by client.
                // For now, assume task_to_create.task_id is unique and set.
                task_to_create.created_at = chrono::Utc::now().timestamp();
                task_to_create.updated_at = task_to_create.created_at;

                let op = self.task_state.update_task_local(task_to_create.clone()); // Create the task first
                self.handle_task_op(op).await?;

                // If it's a PoC eligible task, immediately determine responsible nodes and update it.
                if matches!(task_to_create.task_variant, crate::tasks::TaskVariant::BuildImage(_) | crate::tasks::TaskVariant::LaunchInstance(_)) {
                    // Fetch the just-created task to ensure we have its latest CRDT state (though task_to_create is what we just put in)
                    // It might be cleaner if update_task_local returned the created task state or if handle_task_op did.
                    // For now, let's re-fetch or use task_to_create if we trust its state is what was persisted before gossip.
                    // For simplicity, we use task_to_create, but a fetch would be safer if there were concurrent writes (unlikely for a new task ID).
                    
                    // We need all nodes from the datastore.
                    let all_nodes = self.node_state.list_nodes(); // Assuming list_nodes() exists and returns Vec<Node>
                    
                    let responsible_nodes = crate::tasks::determine_responsible_nodes(&task_to_create, &all_nodes, self);
                    
                    log::info!("PoC determined responsible nodes for task {}: {:?}", task_to_create.task_id, responsible_nodes);

                    // Now, update the task with responsible_nodes and new status
                    if let Some(mut task_to_update) = self.task_state.get_task(&task_to_create.task_id) { // Removed .cloned()
                        task_to_update.responsible_nodes = Some(responsible_nodes);
                        task_to_update.status = crate::tasks::TaskStatus::PoCAssigned;
                        task_to_update.updated_at = chrono::Utc::now().timestamp();
                        let update_op = self.task_state.update_task_local(task_to_update);
                        self.handle_task_op(update_op).await?;
                    } else {
                        log::error!("Failed to re-fetch task {} for PoC update.", task_to_create.task_id);
                        // Not returning error here, as initial task creation succeeded. PoC can be retried.
                    }
                }
            }
            TaskRequest::UpdateStatus{ task_id, status, progress, result_info } => {
                if let Some(mut task) = self.task_state.get_task(&task_id) {
                    task.status = status;
                    task.progress = progress;
                    task.result_info = result_info;
                    task.updated_at = chrono::Utc::now().timestamp();
                    let op = self.task_state.update_task_local(task);
                    self.handle_task_op(op).await?;
                } else {
                    return Err(Box::new(std::io::Error::new(std::io::ErrorKind::NotFound, format!("Task not found: {}", task_id))));
                }
            }
            TaskRequest::AssignNode{ task_id, node_id, status_after_assign } => {
                 if let Some(mut task) = self.task_state.get_task(&task_id) {
                    task.assigned_to_node_id = node_id;
                    if let Some(new_status) = status_after_assign {
                        task.status = new_status;
                    }
                    task.updated_at = chrono::Utc::now().timestamp();
                    let op = self.task_state.update_task_local(task);
                    self.handle_task_op(op).await?;
                } else {
                    return Err(Box::new(std::io::Error::new(std::io::ErrorKind::NotFound, format!("Task not found: {}", task_id))));
                }
            }
        }
        Ok(())
    }

    pub async fn handle_task_op(&mut self, task_op: TaskOp) -> Result<(), Box<dyn std::error::Error>> {
        let mut op_applied_successfully = false;
        let op_to_propagate = task_op.clone();

        match &task_op {
            Op::Up { dot: _, key, op } => {
                self.task_state.task_op(task_op.clone()); // Apply locally, clone task_op as it's borrowed in match
                // Using task_id from the successfully applied op for verification
                if let (true, Some(_)) = self.task_state.task_op_success(&op.op().value.task_id, op) {
                    log::info!("Task Op::Up successfully applied locally for task_id: {}", key);
                    op_applied_successfully = true;
                } else {
                    log::error!("Task Op::Up failed to apply locally or was a no-op for task_id: {}.", key);
                    return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Task Op::Up failed local application for task_id: {}", key))));
                }
            }
            Op::Rm { .. } => {
                self.task_state.task_op(task_op); // Apply Rm locally
                log::info!("Task Op::Rm applied locally.");
                op_applied_successfully = true;
            }
        }

        if op_applied_successfully {
            #[cfg(feature = "devnet")]
            {
                log::info!("devnet mode: Task Op applied locally. Gossiping directly with op: {:?}", op_to_propagate);
                self.gossip_op_directly(&op_to_propagate, "TaskOp").await?;
            }
            #[cfg(not(feature = "devnet"))]
            {
                log::info!("production mode: Queuing Task Op ({:?}).", op_to_propagate);
                DataStore::write_to_queue(crate::datastore::TaskRequest::Op(op_to_propagate.clone()), 10).await?;
            }
            write_datastore(&DB_HANDLE, &self.clone())?;
        }

        Ok(())
    }
    #[cfg(not(feature = "devnet"))]
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

    #[cfg(feature = "devnet")]
    pub async fn write_to_queue(
        message: impl Serialize + Clone,
        sub_topic: u8,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Log operation details without actually writing to the queue
        log::info!("DEVNET MODE: Skipping queue write for subtopic {}", sub_topic);
        log::debug!("DEVNET MODE: Would have written message: {:?}", serde_json::to_string(&message));
        
        // Success with no-op in devnet mode
        Ok(())
    }

    #[cfg(not(feature = "devnet"))]
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

    #[cfg(feature = "devnet")]
    pub async fn read_from_queue(
        _last: Option<usize>,
        _n: Option<usize>,
    ) -> Result<Vec<Vec<u8>>, Box<dyn std::error::Error>> {
        // In devnet mode, the queue reader doesn't process any messages
        // Return an empty list since there are no messages to process in devnet mode
        Ok(Vec::new())
    }

    pub async fn broadcast<R: DeserializeOwned>(
        &mut self,
        request: impl Serialize + Clone,
        endpoint: &str
    ) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Bradcasting Op to all active admins...");
        let peers = self.get_all_active_admin();
        for (id, peer) in peers {

            if id == self.node_state.node_id {
                continue
            }

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
            log::info!("Pulled assoc request from queue, processing...");
            let assoc_request: AssocRequest = serde_json::from_slice(payload)?;
            guard.handle_assoc_request(assoc_request).await?;
        },
        3 => {
            log::info!("Pulled dns request from queue, processing...");
            let dns_request: DnsRequest = serde_json::from_slice(payload)?;
            guard.handle_dns_request(dns_request).await?;
        },
        4 => {
            log::info!("Pulled instance request from queue, processing...");
            let instance_request: InstanceRequest = serde_json::from_slice(payload)?;
            guard.handle_instance_request(instance_request).await?;
        },
        5 => {
            log::info!("Pulled node request from queue, processing...");
            let node_request: NodeRequest = serde_json::from_slice(payload)?;
            guard.handle_node_request(node_request).await?;
        },
        6 => {
            log::info!("Pulled node metrics request from queue, processing...");
            let node_metrics_request: NodeMetricsRequest = serde_json::from_slice(payload)?;
            guard.handle_node_metrics_request(node_metrics_request).await?;
        },
        7 => {
            log::info!("Pulled account request from queue, processing...");
            let account_request: AccountRequest = serde_json::from_slice(payload)?;
            guard.handle_account_request(account_request).await?;
        },
        8 => {
            log::info!("Pulled agent request from queue, processing...");
            let agent_request: AgentRequest = serde_json::from_slice(payload)?;
            guard.handle_agent_request(agent_request).await?;
        },
        9 => {
            log::info!("Pulled model request from queue, processing...");
            let model_request: ModelRequest = serde_json::from_slice(payload)?;
            guard.handle_model_request(model_request).await?;
        }
        _ => unreachable!()
    }

    drop(guard);

    Ok(())
}

pub async fn pong() -> Json<Value> {
    log::info!("Received Ping Request, sending Pong...");
    Json(serde_json::json!({"ping":"pong"}))
}

pub async fn full_state(
    State(state): State<Arc<Mutex<DataStore>>>,
) -> Json<MergeableState> {
    log::info!("Received full state request, returning...");
    let datastore = state.lock().await.clone();
    Json(datastore.into())
}

pub async fn request_full_state(to_dial: &str) -> Result<MergeableState, Box<dyn std::error::Error>> {
    let resp = Client::new()
        .get(format!("http://{to_dial}:3004/bootstrap/full_state"))
        .send().await?.json().await?;

    println!("{:?}", resp);
    Ok(resp)
}

#[cfg(test)]
mod tests {
    use crate::instances::{InstanceAnnotations, InstanceCluster, InstanceEncryption, InstanceMetadata, InstanceMonitoring, InstanceResources, InstanceSecurity, InstanceStatus};
    use crate::nodes::{NodeAnnotations, NodeMetadata, NodeMonitoring};

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
                scaling_policy: None,
                template_instance_id: None,
                session_affinity_enabled: false,
                scaling_manager: None,
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
            capabilities: NodeCapabilities::default(),
            capacity: NodeCapacity::default(), 
            metrics: NodeMetrics::default(),
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
            host: Host::Domain("example.com".to_string()),
            operator_keys: vec![],
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
            accounts: Map::new(),
            agents: Map::new(),
            models: Map::new(),
        };

        assert!(serde_json::to_string(&mergeable_state.peers).is_ok());
        assert!(serde_json::to_string(&mergeable_state.cidrs).is_ok());
        assert!(serde_json::to_string(&mergeable_state.assocs).is_ok());
        assert!(serde_json::to_string(&mergeable_state.dns).is_ok());
        assert!(serde_json::to_string(&mergeable_state.instances).is_ok());
        assert!(serde_json::to_string(&mergeable_state.nodes).is_ok());
        assert!(serde_json::to_string(&mergeable_state.agents).is_ok());
        assert!(serde_json::to_string(&mergeable_state.models).is_ok());

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
