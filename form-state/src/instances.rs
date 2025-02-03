use std::collections::{btree_map::{Iter, IterMut}, BTreeMap};
use crdts::{map::Op, merkle_reg::Sha3Hash, BFTReg, CmRDT, Map, Update};
use k256::ecdsa::SigningKey;
use serde::{Serialize, Deserialize};
use tiny_keccak::Hasher;
use crate::Actor;

pub type InstanceOp = Op<String, BFTReg<Instance, Actor>, Actor>; 

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Instance {
    instance_id: String,
    instance_owner: String,
    created_at: i64,
    updated_at: i64,
    last_snapshot: i64,
    host_region: String,
    resources: InstanceResources,
    cluster: InstanceCluster,
    /// Base64 encoded formfile
    formfile: String, 
    snapshots: Option<Snapshots>,
    metadata: InstanceMetadata,
}

impl Sha3Hash for Instance {
    fn hash(&self, hasher: &mut tiny_keccak::Sha3) {
        hasher.update(&bincode::serialize(self).unwrap());
    }
}

impl Instance {
    pub fn instance_id(&self) -> &str {
        &self.instance_id
    }

    pub fn instance_owner(&self) -> &str {
        &self.instance_owner
    }

    pub fn created_at(&self) -> i64 {
        self.created_at
    }

    pub fn updated_at(&self) -> i64 {
        self.updated_at
    }

    pub fn last_snapshot(&self) -> i64 {
        self.last_snapshot
    }

    pub fn host_region(&self) -> &str {
        &self.host_region
    }

    pub fn resources(&self) -> &InstanceResources {
        &self.resources
    }

    pub fn cluster(&self) -> &InstanceCluster {
        &self.cluster
    }

    pub fn formfile(&self) -> &str {
        &self.formfile
    }

    pub fn snapshots(&self) -> &Option<Snapshots> {
        &self.snapshots
    }

    pub fn metadata(&self) -> &InstanceMetadata {
        &self.metadata
    }

    pub fn vcpus(&self) -> u8 {
        self.resources.vcpus()
    }

    pub fn memory_mb(&self) -> u32 {
        self.resources.memory_mb() 
    }

    pub fn bandwidth_mbps(&self) -> u32 {
        self.resources.bandwidth_mbps()
    }

    pub fn gpu(&self) -> Option<InstanceGpu> {
        self.resources.gpu()
    }

    pub fn gpu_count(&self) -> Option<u8> {
        self.resources.gpu_count()
    }

    pub fn gpu_model(&self) -> Option<&str> {
        self.resources.gpu_model()
    }

    pub fn gpu_vram_mp(&self) -> Option<u32> {
        self.resources.gpu_vram_mb()
    }

    pub fn gpu_usage(&self) -> Option<u32> {
        self.resources.gpu_usage()
    }

    pub fn cluster_members(&self) -> &BTreeMap<String, ClusterMember> {
        self.cluster().members()
    }

    pub fn is_cluster_member(&self, id: &str) -> bool {
        self.cluster().contains_key(id)
    }

    pub fn get_cluster_member(&self, id: &str) -> Option<&ClusterMember> {
        self.cluster().get(id)
    }

    pub fn get_cluster_member_status(&self, id: &str) -> Option<&str> {
        self.cluster().get_member_status(id)
    }

    pub fn get_cluster_member_last_heartbeat(&self, id: &str) -> Option<i64> {
        self.cluster().get_member_last_heartbeat(id)
    }

    pub fn get_cluster_member_heartbeats_skipped(&self, id: &str) -> Option<u32> {
        self.cluster().get_member_heartbeats_skipped(id)
    }

    pub fn insert_cluster_member(&mut self, member: ClusterMember) {
        self.cluster.members_mut().insert(member.id().to_string(), member.clone());
    }

    pub fn remove_cluster_member(&mut self, id: &str) -> Option<ClusterMember> {
        self.cluster.remove(id)
    }

    pub fn cluster_member_iter(&self) -> Iter<String, ClusterMember> {
        self.cluster.iter()
    }

    pub fn cluster_member_iter_mut(&mut self) -> IterMut<String, ClusterMember> {
        self.cluster.iter_mut()
    }

    pub fn formfile_b64(&self) -> &str {
        &self.formfile
    }

    pub fn n_snapshots_ago(&self, mut n: u32) -> (Option<Snapshots>, u32) {
        let mut current: Option<Snapshots> = self.snapshots().clone();
        while n > 0 {
            if let Some(ref c) = &current {
                let next = *c.previous_snapshot.clone(); 
                if !next.is_none() {
                    current = next;
                    n -= 1;
                } else {
                    return (current.clone(), n);
                }
            } else {
                return (current, n)
            }
        }

        return (current, n);
    }

    pub fn tags(&self) -> Vec<String> {
        self.metadata().tags()
    }

    pub fn description(&self) -> &str {
        self.metadata().description()
    }

    pub fn annotations(&self) -> &InstanceAnnotations {
        self.metadata().annotations()
    }

    pub fn security(&self) -> &InstanceSecurity {
        self.metadata().security()
    }

    pub fn monitoring(&self) -> &InstanceMonitoring {
        self.metadata().monitoring()
    }

    pub fn encryption(&self) -> &InstanceEncryption {
        self.security().encryption()
    }

    pub fn tee(&self) -> bool {
        self.security().tee()
    }

    pub fn hsm(&self) -> bool {
        self.security().hsm()
    }

    pub fn is_encrypted(&self) -> bool {
        self.encryption().is_encrypted()
    }

    pub fn scheme(&self) -> Option<String> {
        self.encryption().scheme()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InstanceResources {
    vcpus: u8,
    memory_mb: u32,
    bandwidth_mbps: u32,
    gpu: Option<InstanceGpu>
}

impl InstanceResources {
    pub fn vcpus(&self) -> u8 {
        self.vcpus
    }

    pub fn memory_mb(&self) -> u32 {
        self.memory_mb
    }

    pub fn bandwidth_mbps(&self) -> u32 {
        self.bandwidth_mbps
    }

    pub fn gpu(&self) -> Option<InstanceGpu> {
        self.gpu.clone()
    }

    pub fn gpu_count(&self) -> Option<u8> {
        if let Some(gpu) = &self.gpu {
            return Some(gpu.count())
        }
        None
    }

    pub fn gpu_model(&self) -> Option<&str> {
        if let Some(gpu) = &self.gpu {
            return Some(gpu.model())
        }
        None
    }

    pub fn gpu_vram_mb(&self) -> Option<u32> {
        if let Some(gpu) = &self.gpu {
            return Some(gpu.vram_mb())
        }
        None
    }

    pub fn gpu_usage(&self) -> Option<u32> {
        if let Some(gpu) = &self.gpu {
            return Some(gpu.usage())
        }
        None
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InstanceGpu {
    count: u8,
    model: String,
    vram_mb: u32,
    usage: u32,
}

impl InstanceGpu {
    pub fn count(&self) -> u8 {
        self.count
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    pub fn vram_mb(&self) -> u32 {
        self.vram_mb
    } 

    pub fn usage(&self) -> u32 {
        self.usage
    }

}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InstanceCluster {
    members: BTreeMap<String, ClusterMember>
}

impl InstanceCluster {
    pub fn members(&self) -> &BTreeMap<String, ClusterMember> {
        &self.members
    }

    pub fn members_mut(&mut self) -> &mut BTreeMap<String, ClusterMember> {
        &mut self.members
    }

    pub fn get(&self, id: &str) -> Option<&ClusterMember> {
        self.members.get(id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut ClusterMember> {
        self.members.get_mut(id)
    }

    pub fn insert(&mut self, member: ClusterMember) {
        let id = member.id();
        self.members.insert(id.to_string(), member);
    }

    pub fn iter(&self) -> Iter<String, ClusterMember> {
        self.members.iter()
    }

    pub fn iter_mut(&mut self) -> IterMut<String, ClusterMember> {
        self.members.iter_mut()
    }

    pub fn remove(&mut self, id: &str) -> Option<ClusterMember> {
        self.members.remove(id)
    }

    pub fn contains_key(&self, id: &str) -> bool {
        self.members.contains_key(id)
    }

    pub fn get_member_status(&self, id: &str) -> Option<&str> {
        if let Some(member) = self.get(id) {
            return Some(member.status())
        }

        None
    }

    pub fn get_member_last_heartbeat(&self, id: &str) -> Option<i64> {
        if let Some(member) = self.get(id) {
            return Some(member.last_heartbeat()) 
        }

        None
    }

    pub fn get_member_heartbeats_skipped(&self, id: &str) -> Option<u32> {
        if let Some(member) = self.get(id) {
            return Some(member.heartbeats_skipped())
        }

        None
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ClusterMember {
    instance_id: String,
    status: String,
    last_heartbeat: i64,
    heartbeats_skipped: u32,
}

impl ClusterMember {
    pub fn id(&self) -> &str {
        &self.instance_id
    }

    pub fn status(&self) -> &str {
        &self.status
    }

    pub fn last_heartbeat(&self) -> i64 {
        self.last_heartbeat
    }

    pub fn heartbeats_skipped(&self) -> u32 {
        self.heartbeats_skipped
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Snapshots {
    snapshot_id: String,
    timestamp: i64,
    description: Option<String>,
    previous_snapshot: Box<Option<Snapshots>>
}

impl Snapshots {
    pub fn id(&self) -> &str {
        &self.snapshot_id
    }

    pub fn timestamp(&self) -> i64 {
        self.timestamp
    }

    pub fn description(&self) -> Option<String> {
        self.description.clone()
    }

    pub fn previous_snapshot(&self) -> Box<Option<Snapshots>> {
        self.previous_snapshot.clone()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InstanceMetadata {
    tags: Vec<String>,
    description: String,
    annotations: InstanceAnnotations,
    security: InstanceSecurity,
    monitoring: InstanceMonitoring
}

impl InstanceMetadata {
    pub fn tags(&self) -> Vec<String> {
        self.tags.clone()
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn annotations(&self) -> &InstanceAnnotations {
        &self.annotations
    }

    pub fn security(&self) -> &InstanceSecurity {
        &self.security
    }

    pub fn monitoring(&self) -> &InstanceMonitoring {
        &self.monitoring
    }

    pub fn deployed_by(&self) -> &str {
        &self.annotations.deployed_by()
    }

    pub fn network_id(&self) -> u16 {
        self.annotations.network_id()
    }

    pub fn build_commit(&self) -> Option<String> {
        self.annotations.build_commit.clone()
    }

    pub fn is_encrypted(&self) -> bool {
        self.security.encryption().is_encrypted()
    }

    pub fn encryption_scheme(&self) -> Option<String> {
        self.security.encryption().scheme()
    }

    pub fn is_tee_enabled(&self) -> bool {
        self.security.tee()
    }

    pub fn is_hsm_enabled(&self) -> bool {
        self.security.hsm()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InstanceAnnotations {
    deployed_by: String,
    network_id: u16,
    build_commit: Option<String>
}

impl InstanceAnnotations {
    pub fn deployed_by(&self) -> &str {
        &self.deployed_by
    }

    pub fn network_id(&self) -> u16 {
        self.network_id
    }

    pub fn build_commit(&self) -> Option<String> {
        self.build_commit.clone()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InstanceSecurity {
    encryption: InstanceEncryption,
    tee: bool,
    hsm: bool,
}

impl InstanceSecurity {
    pub fn encryption(&self) -> &InstanceEncryption {
        &self.encryption
    }

    pub fn tee(&self) -> bool {
        self.tee
    }

    pub fn hsm(&self) -> bool {
        self.hsm
    }

    pub fn is_encrypted(&self) -> bool {
        self.encryption.is_encrypted()
    }

    pub fn scheme(&self) -> Option<String> {
        self.encryption.scheme()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InstanceMonitoring {
    logging_enabled: bool,
    metrics_endpoint: String,
}

impl InstanceMonitoring {
    pub fn logging_enabled(&self) -> bool {
        self.logging_enabled
    }

    pub fn metrics_endpoint(&self) -> &str {
        &self.metrics_endpoint
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InstanceEncryption {
    is_encrypted: bool,
    scheme: Option<String>,
}

impl InstanceEncryption {
    pub fn is_encrypted(&self) -> bool {
        self.is_encrypted
    }

    pub fn scheme(&self) -> Option<String> {
        self.scheme.clone()
    }
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InstanceState {
    node_id: String,
    pk: String,
    map: Map<String, BFTReg<Instance, Actor>, Actor> 
}


impl InstanceState {

    pub fn new(node_id: String, pk: String) -> Self {
        Self {
            node_id,
            pk,
            map: Map::new()
        }
    }

    pub fn update_instance_local(&mut self, instance: Instance) -> InstanceOp {
        log::info!("Acquiring add ctx...");
        let add_ctx = self.map.read_ctx().derive_add_ctx(self.node_id.clone());
        log::info!("Decoding our private key...");
        let signing_key = SigningKey::from_slice(
            &hex::decode(self.pk.clone())
                .expect("PANIC: Invalid SigningKey Cannot Decode from Hex"))
                .expect("PANIC: Invalid SigningKey cannot recover ffrom Bytes");
        log::info!("Creating op...");
        let op = self.map.update(instance.instance_id().to_string(), add_ctx, |reg, _ctx| {
            let op = reg.update(instance.into(), self.node_id.clone(), signing_key).expect("PANIC: Unable to sign updates");
            op
        });
        log::info!("Op created, returning...");
        op
    }

    pub fn remove_instance_local(&mut self, id: String) -> InstanceOp {
        log::info!("Acquiring remove context...");
        let rm_ctx = self.map.read_ctx().derive_rm_ctx();
        log::info!("Building Rm Op...");
        self.map.rm(id, rm_ctx)
    }

    pub fn instance_op(&mut self, op: InstanceOp) -> Option<(String, String)> {
        log::info!("Applying peer op");
        self.map.apply(op.clone());
        match op {
            Op::Up { dot, key, op: _ } => Some((dot.actor, key)),
            Op::Rm { .. } => None
        }
    }

    pub fn instance_op_success(&self, key: String, update: Update<Instance, String>) -> (bool, Instance) {
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

    pub fn get_instance(&self, key: String) -> Option<Instance> {
        if let Some(reg) = self.map.get(&key).val {
            if let Some(v) = reg.val() {
                return Some(v.value())
            }
        }

        return None
    }
}
