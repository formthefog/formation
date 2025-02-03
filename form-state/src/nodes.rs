use crdts::{map::Op, merkle_reg::Sha3Hash, BFTReg, Map, CmRDT};
use k256::ecdsa::SigningKey;
use tiny_keccak::Hasher;
use crate::Actor;
use serde::{Serialize, Deserialize};

pub type NodeOp = Op<String, BFTReg<Node, Actor>, Actor>;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Node {
    node_id: String,
    node_owner: String,
    created_at: i64,
    updated_at: i64,
    last_heartbeat: i64,
    host_region: String,
    capacity: NodeCapacity,
    availability: NodeAvailability,
    metadata: NodeMetadata,
}

impl Sha3Hash for Node {
    fn hash(&self, hasher: &mut tiny_keccak::Sha3) {
        // Serialize the node and feed it to the hasher.
        hasher.update(&bincode::serialize(self).unwrap());
    }
}

impl Node {
    // Getters for Node fields.
    pub fn node_id(&self) -> &str {
        &self.node_id
    }

    pub fn node_owner(&self) -> &str {
        &self.node_owner
    }

    pub fn created_at(&self) -> i64 {
        self.created_at
    }

    pub fn updated_at(&self) -> i64 {
        self.updated_at
    }

    pub fn last_heartbeat(&self) -> i64 {
        self.last_heartbeat
    }

    pub fn host_region(&self) -> &str {
        &self.host_region
    }

    pub fn capacity(&self) -> &NodeCapacity {
        &self.capacity
    }

    pub fn availability(&self) -> &NodeAvailability {
        &self.availability
    }

    pub fn metadata(&self) -> &NodeMetadata {
        &self.metadata
    }
}

/// Describes the resource capacity of the node.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeCapacity {
    vcpus: u8,
    memory_mb: u32,
    bandwidth_mbps: u32,
    gpu: Option<NodeGpu>,
}

impl NodeCapacity {
    pub fn vcpus(&self) -> u8 {
        self.vcpus
    }

    pub fn memory_mb(&self) -> u32 {
        self.memory_mb
    }

    pub fn bandwidth_mbps(&self) -> u32 {
        self.bandwidth_mbps
    }

    pub fn gpu(&self) -> Option<NodeGpu> {
        self.gpu.clone()
    }
}

/// Describes available GPU capacity on the node.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeGpu {
    count: u8,
    model: String,
    vram_mb: u32,
}

impl NodeGpu {
    pub fn count(&self) -> u8 {
        self.count
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    pub fn vram_mb(&self) -> u32 {
        self.vram_mb
    }
}

/// Contains real-time availability information of the node.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeAvailability {
    uptime_seconds: u64,
    load_average: u32,
    status: String, // e.g. "online", "offline", "maintenance"
}

impl NodeAvailability {
    pub fn uptime_seconds(&self) -> u64 {
        self.uptime_seconds
    }

    pub fn load_average(&self) -> f64 {
        self.load_average as f64 / 100.0
    }

    pub fn status(&self) -> &str {
        &self.status
    }
}

/// Additional metadata for operational context.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeMetadata {
    tags: Vec<String>,
    description: String,
    annotations: NodeAnnotations,
    monitoring: NodeMonitoring,
}

impl NodeMetadata {
    pub fn tags(&self) -> Vec<String> {
        self.tags.clone()
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn annotations(&self) -> &NodeAnnotations {
        &self.annotations
    }

    pub fn monitoring(&self) -> &NodeMonitoring {
        &self.monitoring
    }
}

/// Additional annotations, such as roles and datacenter info.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeAnnotations {
    roles: Vec<String>,     // e.g. ["compute", "storage"]
    datacenter: String,     // Which datacenter the node belongs to.
}

impl NodeAnnotations {
    pub fn roles(&self) -> Vec<String> {
        self.roles.clone()
    }

    pub fn datacenter(&self) -> &str {
        &self.datacenter
    }
}

/// Monitoring settings specific to the node.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeMonitoring {
    logging_enabled: bool,
    metrics_endpoint: String,
}

impl NodeMonitoring {
    pub fn logging_enabled(&self) -> bool {
        self.logging_enabled
    }

    pub fn metrics_endpoint(&self) -> &str {
        &self.metrics_endpoint
    }
}

/// A NodeState wraps a CRDT map that holds all node records,
/// enabling you to update, remove, and query nodes in a BFT CRDT fashion.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeState {
    node_id: String,
    pk: String,
    map: Map<String, BFTReg<Node, Actor>, Actor>,
}

impl NodeState {
    pub fn new(node_id: String, pk: String) -> Self {
        Self {
            node_id,
            pk,
            map: Map::new(),
        }
    }

    pub fn map(&self) -> Map<String, BFTReg<Node, Actor>, Actor> {
        self.map.clone()
    }

    /// Update (or add) a node record locally. This creates a signed op
    /// that will be merged into the CRDT map.
    pub fn update_node_local(&mut self, node: Node) -> NodeOp {
        log::info!("Acquiring add context...");
        let add_ctx = self.map.read_ctx().derive_add_ctx(self.node_id.clone());
        log::info!("Decoding our private key...");
        let signing_key = SigningKey::from_slice(
            &hex::decode(self.pk.clone())
                .expect("Invalid SigningKey: Cannot decode from hex")
        ).expect("Invalid SigningKey: Cannot recover from bytes");
        log::info!("Creating node op...");
        let op = self.map.update(node.node_id().to_string(), add_ctx, |reg, _ctx| {
            let op = reg.update(node.into(), self.node_id.clone(), signing_key)
                .expect("Unable to sign node update");
            op
        });
        log::info!("Node op created, returning...");
        op
    }

    /// Remove a node record locally.
    pub fn remove_node_local(&mut self, id: String) -> NodeOp {
        log::info!("Acquiring remove context...");
        let rm_ctx = self.map.read_ctx().derive_rm_ctx();
        log::info!("Building remove op...");
        self.map.rm(id, rm_ctx)
    }

    /// Apply an operation received from a peer.
    pub fn node_op(&mut self, op: NodeOp) -> Option<(String, String)> {
        log::info!("Applying peer node op");
        self.map.apply(op.clone());
        match op {
            Op::Up { dot, key, op: _ } => Some((dot.actor, key)),
            Op::Rm { .. } => None,
        }
    }

    /// Retrieve a node by its key.
    pub fn get_node(&self, key: String) -> Option<Node> {
        if let Some(reg) = self.map.get(&key).val {
            if let Some(v) = reg.val() {
                return Some(v.value());
            }
        }
        None
    }
}
