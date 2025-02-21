use crdts::{map::Op, merkle_reg::Sha3Hash, BFTReg, CmRDT, Map, bft_reg::Update};
use k256::ecdsa::SigningKey;
use tiny_keccak::Hasher;
use url::Host;
use crate::Actor;
use serde::{Serialize, Deserialize};

pub type NodeOp = Op<String, BFTReg<Node, Actor>, Actor>;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Node {
    pub node_id: String,
    pub node_owner: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_heartbeat: i64,
    pub host_region: String,
    pub capacity: NodeCapacity,
    pub availability: NodeAvailability,
    pub metadata: NodeMetadata,
    pub host: Host
}

impl Default for Node {
    fn default() -> Self {
        let null_hex = hex::encode(&[0u8; 32]);
        Self {
            node_id: null_hex.clone(),
            node_owner: null_hex.clone(),
            created_at: 0,
            updated_at: 0,
            last_heartbeat: 0,
            host_region: Default::default(),
            capacity: Default::default(),
            availability: Default::default(),
            metadata: Default::default(),
            host: Host::Domain(Default::default())
        }
    }
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
#[derive(Clone, Default, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeCapacity {
    pub(crate) vcpus: u8,
    pub(crate) memory_mb: u32,
    pub(crate) bandwidth_mbps: u32,
    pub(crate) gpu: Option<NodeGpu>,
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
    pub(crate) count: u8,
    pub(crate) model: String,
    pub(crate) vram_mb: u32,
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
#[derive(Clone, Default, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeAvailability {
    pub(crate) uptime_seconds: u64,
    pub(crate) load_average: u32,
    pub(crate) status: String, // e.g. "online", "offline", "maintenance"
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
#[derive(Clone, Default, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeMetadata {
    pub(crate) tags: Vec<String>,
    pub(crate) description: String,
    pub(crate) annotations: NodeAnnotations,
    pub(crate) monitoring: NodeMonitoring,
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
#[derive(Clone, Default, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeAnnotations {
    pub(crate) roles: Vec<String>,     // e.g. ["compute", "storage"]
    pub(crate) datacenter: String,     // Which datacenter the node belongs to.
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
#[derive(Clone, Default, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeMonitoring {
    pub(crate) logging_enabled: bool,
    pub(crate) metrics_endpoint: String,
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
    pub map: Map<String, BFTReg<Node, Actor>, Actor>,
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

    pub fn node_op_success(&self, key: String, update: Update<Node, String>) -> (bool, Node) {
        if let Some(reg) = self.map.get(&key).val {
            if let Some(v) = reg.val() {
                // If the in the updated register equals the value in the Op it
                // succeeded
                if v.value() == update.op().value {
                    return (true, v.value()) 
                // Otherwise, it could be that it's a concurrent update and was added
                // to the DAG as a head
                } else if reg.dag_contains(&update.hash()) && reg.is_head(&update.hash()) {
                    return (true, v.value()) 
                // Otherwise, we could be missing a child, and this particular update
                // is orphaned, if so we should requst the child we are missing from
                // the actor who shared this update
                } else if reg.is_orphaned(&update.hash()) {
                    return (true, v.value())
                // Otherwise it was a no-op for some reason
                } else {
                    return (false, v.value()) 
                }
            } else {
                return (false, update.op().value) 
            }
        } else {
            return (false, update.op().value);
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
