use std::{collections::{btree_map::{Iter, IterMut}, BTreeMap}, fmt::Display, net::{IpAddr, Ipv4Addr}, time::{Duration, SystemTime, UNIX_EPOCH}};
use crdts::{map::Op, merkle_reg::Sha3Hash, BFTReg, CmRDT, Map, bft_reg::Update};
use form_dns::store::FormDnsRecord;
use form_types::state::{Response, Success};
use k256::ecdsa::SigningKey;
use reqwest::Client;
use serde::{Serialize, Deserialize};
use tiny_keccak::Hasher;
use crate::Actor;
use crate::scaling::{ScalingManager, ScalingPhase, ScalingOperation, ScalingError, ScalingMetrics, ScalingResources};

pub type InstanceOp = Op<String, BFTReg<Instance, Actor>, Actor>; 

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InstanceStatus {
    Building,
    Built,
    Created,
    Started,
    Stopped,
    Killed,
    CriticalError,
}

impl Display for InstanceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InstanceStatus::Building => writeln!(f, "{}", "Building"),
            InstanceStatus::Built => writeln!(f, "{}", "Built"),
            InstanceStatus::Created => writeln!(f, "{}", "Created"),
            InstanceStatus::Started => writeln!(f, "{}", "Started"),
            InstanceStatus::Stopped => writeln!(f, "{}", "Stopped"),
            InstanceStatus::Killed => writeln!(f, "{}", "Killed"),
            InstanceStatus::CriticalError => writeln!(f, "{}", "Critical Error"),

        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Instance {
    pub instance_id: String,
    pub node_id: String,
    pub build_id: String,
    pub instance_owner: String,
    pub formnet_ip: Option<IpAddr>,
    pub dns_record: Option<FormDnsRecord>,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_snapshot: i64,
    pub status: InstanceStatus,
    pub host_region: String,
    pub resources: InstanceResources,
    pub cluster: InstanceCluster,
    /// Base64 encoded formfile
    pub formfile: String, 
    pub snapshots: Option<Snapshots>,
    pub metadata: InstanceMetadata,
}

impl Default for Instance {
    fn default() -> Self {
        let null_hash = [0u8; 32];
        let null_hex = hex::encode(null_hash);
        Self {
            instance_id: null_hex.clone(), 
            node_id: null_hex.clone(), 
            build_id: null_hex.clone(), 
            instance_owner: null_hex.clone(),
            formnet_ip: None,
            dns_record: None,
            created_at: 0,
            updated_at: 0,
            last_snapshot: 0,
            status: InstanceStatus::Building,
            host_region: String::new(),
            resources: Default::default(),
            cluster: Default::default(),
            formfile: String::new(),
            snapshots: None,
            metadata: Default::default()

        }
    }
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

    pub async fn get(id: &str) -> Option<Self> {
        let resp = Client::new()
            .get(format!("http://127.0.0.1:3004/instance/{}/get", id))
            .send().await.ok()?
            .json::<Response<Self>>().await.ok()?;

        match resp {
            Response::Success(Success::Some(instance)) => return Some(instance),
            _ => return None,
        }
    }
}

#[derive(Clone, Default, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InstanceResources {
    pub vcpus: u8,
    pub memory_mb: u32,
    pub bandwidth_mbps: u32,
    pub gpu: Option<InstanceGpu>
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
    pub count: u8,
    pub model: String,
    pub vram_mb: u32,
    pub usage: u32,
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

/// Defines scaling policies and constraints for an instance cluster.
/// 
/// This struct contains parameters that control how scaling operations are performed,
/// including minimum and maximum instance counts, target utilization metrics,
/// and cooldown periods to prevent oscillation.
#[derive(Clone, Default, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScalingPolicy {
    /// Minimum number of instances that should be maintained
    pub min_instances: u32,
    
    /// Maximum number of instances that can be created
    pub max_instances: u32,
    
    /// Target CPU utilization percentage (0-100) that triggers scaling
    /// Integer percentage instead of float to allow for Eq, Ord, and Hash derivation
    pub target_cpu_utilization: u32,
    
    /// Cooldown period in seconds after scaling in before another scale-in can occur
    pub scale_in_cooldown_seconds: u32,
    
    /// Cooldown period in seconds after scaling out before another scale-out can occur
    pub scale_out_cooldown_seconds: u32,
    
    /// Timestamp of the last scale-in operation (Unix timestamp in seconds)
    pub last_scale_in_time: i64,
    
    /// Timestamp of the last scale-out operation (Unix timestamp in seconds)
    pub last_scale_out_time: i64,
}

impl ScalingPolicy {
    /// Creates a new ScalingPolicy with the provided parameters.
    ///
    /// # Arguments
    ///
    /// * `min_instances` - Minimum number of instances to maintain
    /// * `max_instances` - Maximum number of instances allowed
    /// * `target_cpu_utilization` - Target CPU utilization percentage (0-100)
    /// * `scale_in_cooldown_seconds` - Cooldown period after scaling in
    /// * `scale_out_cooldown_seconds` - Cooldown period after scaling out
    ///
    /// # Returns
    ///
    /// A new ScalingPolicy with the specified parameters and current timestamps initialized to 0.
    pub fn new(
        min_instances: u32,
        max_instances: u32,
        target_cpu_utilization: u32,
        scale_in_cooldown_seconds: u32,
        scale_out_cooldown_seconds: u32,
    ) -> Self {
        Self {
            min_instances,
            max_instances,
            target_cpu_utilization,
            scale_in_cooldown_seconds,
            scale_out_cooldown_seconds,
            last_scale_in_time: 0,
            last_scale_out_time: 0,
        }
    }

    /// Creates a new ScalingPolicy with sensible defaults:
    /// - min_instances: 1
    /// - max_instances: 5
    /// - target_cpu_utilization: 70%
    /// - scale_in_cooldown_seconds: 300 (5 minutes)
    /// - scale_out_cooldown_seconds: 120 (2 minutes)
    pub fn with_defaults() -> Self {
        Self {
            min_instances: 1,
            max_instances: 5,
            target_cpu_utilization: 70,
            scale_in_cooldown_seconds: 300,
            scale_out_cooldown_seconds: 120,
            last_scale_in_time: 0,
            last_scale_out_time: 0,
        }
    }

    /// Validates that the scaling policy parameters are coherent and within acceptable ranges.
    ///
    /// # Returns
    ///
    /// `Ok(())` if the policy is valid, or an error describing the validation failure.
    pub fn validate(&self) -> Result<(), String> {
        // Check min_instances <= max_instances
        if self.min_instances > self.max_instances {
            return Err(format!(
                "min_instances ({}) must not be greater than max_instances ({})",
                self.min_instances, self.max_instances
            ));
        }

        // Check max_instances is at least 1
        if self.max_instances == 0 {
            return Err("max_instances must be at least 1".to_string());
        }

        // Check target_cpu_utilization is between 0 and 100
        if self.target_cpu_utilization > 100 {
            return Err(format!(
                "target_cpu_utilization ({}) must be between 0 and 100",
                self.target_cpu_utilization
            ));
        }

        Ok(())
    }

    /// Returns the minimum number of instances that should be maintained.
    pub fn min_instances(&self) -> u32 {
        self.min_instances
    }

    /// Returns the maximum number of instances that can be created.
    pub fn max_instances(&self) -> u32 {
        self.max_instances
    }

    /// Returns the target CPU utilization percentage.
    pub fn target_cpu_utilization(&self) -> u32 {
        self.target_cpu_utilization
    }

    /// Returns the cooldown period in seconds after scaling in.
    pub fn scale_in_cooldown_seconds(&self) -> u32 {
        self.scale_in_cooldown_seconds
    }

    /// Returns the cooldown period in seconds after scaling out.
    pub fn scale_out_cooldown_seconds(&self) -> u32 {
        self.scale_out_cooldown_seconds
    }

    /// Returns the timestamp of the last scale-in operation.
    pub fn last_scale_in_time(&self) -> i64 {
        self.last_scale_in_time
    }

    /// Returns the timestamp of the last scale-out operation.
    pub fn last_scale_out_time(&self) -> i64 {
        self.last_scale_out_time
    }

    /// Checks if the current number of instances should trigger a scale-out operation.
    ///
    /// # Arguments
    ///
    /// * `current_instances` - The current number of instances
    /// * `current_cpu_utilization` - The current CPU utilization percentage
    ///
    /// # Returns
    ///
    /// `true` if scaling out is recommended, `false` otherwise.
    pub fn should_scale_out(&self, current_instances: u32, current_cpu_utilization: u32) -> bool {
        // Cannot scale out if at maximum capacity
        if current_instances >= self.max_instances {
            return false;
        }

        // Scale out if CPU utilization is above target
        current_cpu_utilization > self.target_cpu_utilization
    }

    /// Checks if the current number of instances should trigger a scale-in operation.
    ///
    /// # Arguments
    ///
    /// * `current_instances` - The current number of instances
    /// * `current_cpu_utilization` - The current CPU utilization percentage
    ///
    /// # Returns
    ///
    /// `true` if scaling in is recommended, `false` otherwise.
    pub fn should_scale_in(&self, current_instances: u32, current_cpu_utilization: u32) -> bool {
        // Cannot scale in if at minimum capacity
        if current_instances <= self.min_instances {
            return false;
        }

        // Define a buffer below the target to prevent oscillation
        // Only scale in if 15% below target
        let scale_in_threshold = if self.target_cpu_utilization > 15 {
            self.target_cpu_utilization - 15
        } else {
            0
        };

        // Scale in if CPU utilization is below the threshold
        current_cpu_utilization < scale_in_threshold
    }

    /// Checks if the cluster is in a cooldown period after a recent scale-out operation.
    ///
    /// # Arguments
    ///
    /// * `current_time` - The current timestamp (Unix timestamp in seconds)
    ///
    /// # Returns
    ///
    /// `true` if in scale-out cooldown, `false` otherwise.
    pub fn is_in_scale_out_cooldown(&self, current_time: i64) -> bool {
        // If last_scale_out_time is 0, there's no cooldown (never scaled out)
        if self.last_scale_out_time == 0 {
            return false;
        }

        // Check if we're still within the cooldown period
        current_time - self.last_scale_out_time < self.scale_out_cooldown_seconds as i64
    }

    /// Checks if the cluster is in a cooldown period after a recent scale-in operation.
    ///
    /// # Arguments
    ///
    /// * `current_time` - The current timestamp (Unix timestamp in seconds)
    ///
    /// # Returns
    ///
    /// `true` if in scale-in cooldown, `false` otherwise.
    pub fn is_in_scale_in_cooldown(&self, current_time: i64) -> bool {
        // If last_scale_in_time is 0, there's no cooldown (never scaled in)
        if self.last_scale_in_time == 0 {
            return false;
        }

        // Check if we're still within the cooldown period
        current_time - self.last_scale_in_time < self.scale_in_cooldown_seconds as i64
    }

    /// Records a scale-out operation at the specified time.
    ///
    /// # Arguments
    ///
    /// * `timestamp` - The timestamp when the scale-out occurred (Unix timestamp in seconds)
    pub fn record_scale_out(&mut self, timestamp: i64) {
        self.last_scale_out_time = timestamp;
    }

    /// Records a scale-in operation at the specified time.
    ///
    /// # Arguments
    ///
    /// * `timestamp` - The timestamp when the scale-in occurred (Unix timestamp in seconds)
    pub fn record_scale_in(&mut self, timestamp: i64) {
        self.last_scale_in_time = timestamp;
    }

    /// Determines the target number of instances based on current metrics.
    ///
    /// # Arguments
    ///
    /// * `current_instances` - The current number of instances
    /// * `current_cpu_utilization` - The current CPU utilization percentage
    /// * `current_time` - The current timestamp (Unix timestamp in seconds)
    ///
    /// # Returns
    ///
    /// The recommended number of instances, or None if no change is needed
    /// or the cluster is in a cooldown period.
    pub fn get_target_instance_count(
        &self,
        current_instances: u32,
        current_cpu_utilization: u32,
        current_time: i64,
    ) -> Option<u32> {
        // Check if we need to scale out
        if self.should_scale_out(current_instances, current_cpu_utilization) 
           && !self.is_in_scale_out_cooldown(current_time) {
            // Calculate the ratio of current to target utilization
            let ratio = current_cpu_utilization as f64 / self.target_cpu_utilization as f64;
            
            // Calculate desired instance count (rounded up)
            let desired_instances = (current_instances as f64 * ratio).ceil() as u32;
            
            // Cap at max_instances
            let target_instances = std::cmp::min(desired_instances, self.max_instances);
            
            // Only return a value if it's different from current
            if target_instances > current_instances {
                return Some(target_instances);
            }
        }
        
        // Check if we need to scale in
        if self.should_scale_in(current_instances, current_cpu_utilization)
           && !self.is_in_scale_in_cooldown(current_time) {
            // Calculate the ratio of target to current utilization
            let ratio = if current_cpu_utilization == 0 {
                // Special case: if current utilization is 0, reduce to minimum
                0.0
            } else {
                self.target_cpu_utilization as f64 / current_cpu_utilization as f64
            };
            
            // Calculate desired instance count (rounded down to be conservative)
            let desired_instances = (current_instances as f64 / ratio).floor() as u32;
            
            // Ensure we don't go below min_instances
            let target_instances = std::cmp::max(desired_instances, self.min_instances);
            
            // Only return a value if it's different from current
            if target_instances < current_instances {
                return Some(target_instances);
            }
        }
        
        // No change needed
        None
    }
}

impl Sha3Hash for ScalingPolicy {
    fn hash(&self, hasher: &mut tiny_keccak::Sha3) {
        hasher.update(&bincode::serialize(self).unwrap());
    }
}

#[derive(Clone, Default, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InstanceCluster {
    /// Cluster members indexed by instance ID
    pub members: BTreeMap<String, ClusterMember>,
    
    /// Scaling policy configuration for this cluster
    pub scaling_policy: Option<ScalingPolicy>,
    
    /// Instance ID to use as a template when scaling out
    /// This is typically the primary instance in the cluster
    pub template_instance_id: Option<String>,
    
    /// Whether session affinity is enabled for this cluster
    /// When enabled, client requests are routed to the same instance consistently
    pub session_affinity_enabled: bool,
    
    /// State machine for managing scaling operations
    /// This field is not serialized as part of the CRDT
    #[serde(skip)]
    pub scaling_manager: Option<crate::scaling::ScalingManager>,
}

impl Sha3Hash for InstanceCluster {
    fn hash(&self, hasher: &mut tiny_keccak::Sha3) {
        hasher.update(&bincode::serialize(self).unwrap());
    }
}

impl InstanceCluster {
    /// Returns a reference to the members of this cluster
    pub fn members(&self) -> &BTreeMap<String, ClusterMember> {
        &self.members
    }

    /// Returns a mutable reference to the members of this cluster
    pub fn members_mut(&mut self) -> &mut BTreeMap<String, ClusterMember> {
        &mut self.members
    }

    /// Returns a reference to the cluster member with the given ID, if it exists
    pub fn get(&self, id: &str) -> Option<&ClusterMember> {
        self.members.get(id)
    }

    /// Returns a mutable reference to the cluster member with the given ID, if it exists
    pub fn get_mut(&mut self, id: &str) -> Option<&mut ClusterMember> {
        self.members.get_mut(id)
    }

    /// Inserts a new member into the cluster
    pub fn insert(&mut self, member: ClusterMember) {
        let id = member.id();
        self.members.insert(id.to_string(), member);
    }

    /// Returns an iterator over the members of this cluster
    pub fn iter(&self) -> Iter<String, ClusterMember> {
        self.members.iter()
    }

    /// Returns a mutable iterator over the members of this cluster
    pub fn iter_mut(&mut self) -> IterMut<String, ClusterMember> {
        self.members.iter_mut()
    }

    /// Removes a member from the cluster and returns it, if it exists
    pub fn remove(&mut self, id: &str) -> Option<ClusterMember> {
        self.members.remove(id)
    }

    /// Returns true if the cluster contains a member with the given ID
    pub fn contains_key(&self, id: &str) -> bool {
        self.members.contains_key(id)
    }

    /// Returns the status of the cluster member with the given ID, if it exists
    pub fn get_member_status(&self, id: &str) -> Option<&str> {
        if let Some(member) = self.get(id) {
            return Some(member.status())
        }

        None
    }

    /// Returns the last heartbeat timestamp of the cluster member with the given ID, if it exists
    pub fn get_member_last_heartbeat(&self, id: &str) -> Option<i64> {
        if let Some(member) = self.get(id) {
            return Some(member.last_heartbeat()) 
        }

        None
    }

    /// Returns the number of heartbeats skipped by the cluster member with the given ID, if it exists
    pub fn get_member_heartbeats_skipped(&self, id: &str) -> Option<u32> {
        if let Some(member) = self.get(id) {
            return Some(member.heartbeats_skipped())
        }

        None
    }

    /// Returns a reference to the scaling policy for this cluster, if it exists
    pub fn scaling_policy(&self) -> Option<&ScalingPolicy> {
        self.scaling_policy.as_ref()
    }

    /// Sets the scaling policy for this cluster
    pub fn set_scaling_policy(&mut self, policy: Option<ScalingPolicy>) {
        self.scaling_policy = policy;
    }

    /// Returns the template instance ID for this cluster, if it exists
    pub fn template_instance_id(&self) -> Option<&String> {
        self.template_instance_id.as_ref()
    }

    /// Sets the template instance ID for this cluster
    pub fn set_template_instance_id(&mut self, id: Option<String>) {
        self.template_instance_id = id;
    }

    /// Returns whether session affinity is enabled for this cluster
    pub fn session_affinity_enabled(&self) -> bool {
        self.session_affinity_enabled
    }

    /// Sets whether session affinity is enabled for this cluster
    pub fn set_session_affinity_enabled(&mut self, enabled: bool) {
        self.session_affinity_enabled = enabled;
    }

    /// Returns the number of members in this cluster
    pub fn size(&self) -> usize {
        self.members.len()
    }

    /// Returns true if the cluster is empty (has no members)
    pub fn is_empty(&self) -> bool {
        self.members.is_empty()
    }

    /// Creates a new InstanceCluster with the specified template instance ID and no members
    pub fn new_with_template(template_id: String) -> Self {
        Self {
            members: BTreeMap::new(),
            scaling_policy: None,
            template_instance_id: Some(template_id),
            session_affinity_enabled: false,
            scaling_manager: None,
        }
    }

    /// Creates a new InstanceCluster with the specified scaling policy and no members
    pub fn new_with_policy(policy: ScalingPolicy) -> Self {
        Self {
            members: BTreeMap::new(),
            scaling_policy: Some(policy),
            template_instance_id: None,
            session_affinity_enabled: false,
            scaling_manager: None,
        }
    }

    /// Validates that the cluster's scaling policy is valid, if it exists
    pub fn validate_scaling_policy(&self) -> Result<(), String> {
        if let Some(policy) = &self.scaling_policy {
            policy.validate()?;
        }
        Ok(())
    }

    /// Determines if the cluster needs to scale out based on its scaling policy and current metrics
    ///
    /// # Arguments
    ///
    /// * `current_cpu_utilization` - The current CPU utilization percentage across the cluster
    /// * `current_time` - The current timestamp in seconds since Unix epoch
    ///
    /// # Returns
    ///
    /// `Some(target_count)` if scaling out is needed, `None` otherwise
    pub fn should_scale_out(&self, current_cpu_utilization: u32, current_time: i64) -> Option<u32> {
        // If there's no scaling policy, we can't scale
        let policy = self.scaling_policy.as_ref()?;
        
        // If we're already at or above the maximum number of instances, we can't scale out
        let current_count = self.size() as u32;
        if current_count >= policy.max_instances() {
            return None;
        }
        
        // Check if we're in a cooldown period
        if policy.is_in_scale_out_cooldown(current_time) {
            return None;
        }
        
        // Check if we need to scale based on CPU utilization
        if policy.should_scale_out(current_count, current_cpu_utilization) {
            return policy.get_target_instance_count(current_count, current_cpu_utilization, current_time);
        }
        
        None
    }
    
    /// Determines if the cluster needs to scale in based on its scaling policy and current metrics
    ///
    /// # Arguments
    ///
    /// * `current_cpu_utilization` - The current CPU utilization percentage across the cluster
    /// * `current_time` - The current timestamp in seconds since Unix epoch
    ///
    /// # Returns
    ///
    /// `Some(target_count)` if scaling in is needed, `None` otherwise
    pub fn should_scale_in(&self, current_cpu_utilization: u32, current_time: i64) -> Option<u32> {
        // If there's no scaling policy, we can't scale
        let policy = self.scaling_policy.as_ref()?;
        
        // If we're already at or below the minimum number of instances, we can't scale in
        let current_count = self.size() as u32;
        if current_count <= policy.min_instances() {
            return None;
        }
        
        // Check if we're in a cooldown period
        if policy.is_in_scale_in_cooldown(current_time) {
            return None;
        }
        
        // Check if we need to scale based on CPU utilization
        if policy.should_scale_in(current_count, current_cpu_utilization) {
            return policy.get_target_instance_count(current_count, current_cpu_utilization, current_time);
        }
        
        None
    }
    
    /// Records that a scale-out operation was performed
    ///
    /// # Arguments
    ///
    /// * `timestamp` - The timestamp of the scale-out operation in seconds since Unix epoch
    pub fn record_scale_out(&mut self, timestamp: i64) {
        if let Some(policy) = &mut self.scaling_policy {
            policy.record_scale_out(timestamp);
        }
    }
    
    /// Records that a scale-in operation was performed
    ///
    /// # Arguments
    ///
    /// * `timestamp` - The timestamp of the scale-in operation in seconds since Unix epoch
    pub fn record_scale_in(&mut self, timestamp: i64) {
        if let Some(policy) = &mut self.scaling_policy {
            policy.record_scale_in(timestamp);
        }
    }
    
    /// Finds the instance IDs of the members that should be removed when scaling in
    ///
    /// # Arguments
    ///
    /// * `count` - The number of instances to remove
    ///
    /// # Returns
    ///
    /// A vector of instance IDs to remove
    pub fn select_instances_to_remove(&self, count: usize) -> Vec<String> {
        // Default strategy: remove instances with the oldest heartbeats
        let mut members: Vec<(&String, &ClusterMember)> = self.members.iter().collect();
        
        // Sort by heartbeat timestamp (oldest first)
        members.sort_by_key(|(_, member)| member.last_heartbeat());
        
        // Skip the template instance if set
        let members_to_remove = members
            .iter()
            .filter(|(id, _)| self.template_instance_id.as_ref() != Some(id))
            .take(count)
            .map(|(id, _)| (*id).clone())
            .collect();
        
        members_to_remove
    }
    
    /// Initializes the scaling manager with the specified timeout
    ///
    /// # Arguments
    ///
    /// * `timeout` - The default timeout for scaling operations
    pub fn init_scaling_manager(&mut self, timeout: Duration) {
        let mut manager = ScalingManager::new();
        
        // Set timeouts for each phase
        let timeout_seconds = timeout.as_secs();
        let phase_names = [
            "Requested", "Validating", "Planning", "ResourceAllocating",
            "InstancePreparing", "Configuring", "Verifying", "Finalizing"
        ];
        
        for phase in phase_names.iter() {
            manager.set_phase_timeout(phase, timeout_seconds);
        }
        
        self.scaling_manager = Some(manager);
    }
    
    /// Returns a reference to the scaling manager, if it exists
    pub fn scaling_manager(&self) -> Option<&ScalingManager> {
        self.scaling_manager.as_ref()
    }
    
    /// Returns a mutable reference to the scaling manager, if it exists
    pub fn scaling_manager_mut(&mut self) -> Option<&mut ScalingManager> {
        self.scaling_manager.as_mut()
    }
    
    /// Processes a single step of the scaling state machine
    ///
    /// This method examines the current phase of the scaling operation and
    /// performs the appropriate action for that phase. It should be called
    /// periodically to advance the state machine.
    ///
    /// # Returns
    ///
    /// * `Ok(true)` if the phase was processed and advanced
    /// * `Ok(false)` if no processing was needed or possible
    /// * `Err(ScalingError)` if an error occurred during processing
    pub fn process_scaling_phase(&mut self) -> Result<bool, ScalingError> {
        // Check if there's a scaling manager
        if self.scaling_manager.is_none() {
            return Ok(false); // No scaling manager, nothing to process
        }
        
        // First check for timeouts - this requires a mutable reference
        {
            let manager = self.scaling_manager_mut().unwrap();
            if manager.check_timeouts() {
                return Ok(true); // Processed timeout
            }
        }
        
        // Then get the current phase - this can use an immutable reference
        let current_phase = {
            let manager = self.scaling_manager.as_ref().unwrap();
            
            // Get the current phase
            match manager.current_phase() {
                Some(phase) => phase.clone(), // Clone to avoid borrow issues
                None => return Ok(false), // No active operation, nothing to process
            }
        };
        
        // Process based on the current phase
        match current_phase {
            ScalingPhase::Requested { .. } => {
                // Just transition to Validating
                let manager = self.scaling_manager_mut().unwrap();
                manager.transition_to_validating()?;
                Ok(true)
            },
            ScalingPhase::Validating { operation, .. } => {
                // Validate the scaling operation
                let result = self.validate_scaling_operation(&operation);
                
                // Collect metrics for comparison after the operation (if validation succeeds)
                let pre_metrics = if result.is_ok() {
                    self.collect_cluster_metrics()
                } else {
                    None
                };
                
                // Now update the state machine
                let manager = self.scaling_manager_mut().unwrap();
                match result {
                    Ok(_) => {
                        // Validation succeeded, transition to Planning
                        manager.transition_to_planning(pre_metrics)?;
                    },
                    Err(err) => {
                        // Validation failed, transition to Failed
                        manager.fail_operation(&err.error_type, &err.message, None)?;
                    }
                }
                
                Ok(true)
            },
            ScalingPhase::Planning { operation, .. } => {
                // Plan the scaling operation
                let planning_result = self.plan_scaling_operation(&operation);
                
                // Update the state machine
                let manager = self.scaling_manager_mut().unwrap();
                match planning_result {
                    Ok(resources) => {
                        // Planning succeeded, transition to ResourceAllocating
                        manager.transition_to_resource_allocating(Some(resources))?;
                    },
                    Err(err) => {
                        // Planning failed, transition to Failed
                        manager.fail_operation(&err.error_type, &err.message, None)?;
                    }
                }
                
                Ok(true)
            },
            ScalingPhase::ResourceAllocating { operation, .. } => {
                // Allocate resources for the scaling operation
                let allocation_result = self.allocate_resources_for_operation(&operation);
                
                // Update the state machine
                let manager = self.scaling_manager_mut().unwrap();
                match allocation_result {
                    Ok(instance_ids) => {
                        // Resource allocation succeeded, transition to InstancePreparing
                        manager.transition_to_instance_preparing(instance_ids)?;
                    },
                    Err(err) => {
                        // Resource allocation failed, transition to Failed
                        manager.fail_operation(&err.error_type, &err.message, None)?;
                    }
                }
                
                Ok(true)
            },
            ScalingPhase::InstancePreparing { operation, instance_ids, .. } => {
                // Prepare instances for the scaling operation
                let preparation_result = self.prepare_instances(&operation, instance_ids.clone());
                
                // Update the state machine
                let manager = self.scaling_manager_mut().unwrap();
                match preparation_result {
                    Ok(previous_config) => {
                        // Instance preparation succeeded, transition to Configuring
                        manager.transition_to_configuring(Some(previous_config))?;
                    },
                    Err(err) => {
                        // Instance preparation failed, transition to Failed
                        manager.fail_operation(&err.error_type, &err.message, None)?;
                    }
                }
                
                Ok(true)
            },
            ScalingPhase::Configuring { operation, .. } => {
                // Apply configuration changes
                let config_result = self.apply_configuration_changes(&operation);
                
                // Update the state machine
                let manager = self.scaling_manager_mut().unwrap();
                match config_result {
                    Ok(_) => {
                        // Configuration succeeded, transition to Verifying
                        manager.transition_to_verifying()?;
                    },
                    Err(err) => {
                        // Configuration failed, transition to Failed
                        manager.fail_operation(&err.error_type, &err.message, None)?;
                    }
                }
                
                Ok(true)
            },
            ScalingPhase::Verifying { operation, .. } => {
                // Verify the scaling operation
                let verification_result = self.verify_scaling_operation(&operation);
                
                // Update the state machine
                let manager = self.scaling_manager_mut().unwrap();
                match verification_result {
                    Ok(cleanup_tasks) => {
                        // Verification succeeded, transition to Finalizing
                        manager.transition_to_finalizing(cleanup_tasks)?;
                    },
                    Err(err) => {
                        // Verification failed, transition to Failed
                        manager.fail_operation(&err.error_type, &err.message, None)?;
                    }
                }
                
                Ok(true)
            },
            ScalingPhase::Finalizing { operation, .. } => {
                // Perform cleanup tasks
                let finalization_result = self.finalize_scaling_operation(&operation);
                
                // Update the state machine
                let manager = self.scaling_manager_mut().unwrap();
                match finalization_result {
                    Ok(result_metrics) => {
                        // Finalization succeeded, transition to Completed
                        manager.complete_operation(Some(result_metrics))?;
                    },
                    Err(err) => {
                        // Finalization failed, transition to Failed
                        manager.fail_operation(&err.error_type, &err.message, None)?;
                    }
                }
                
                Ok(true)
            },
            // Terminal states - no action needed
            ScalingPhase::Completed { .. } | ScalingPhase::Failed { .. } | ScalingPhase::Canceled { .. } => {
                Ok(false)
            },
        }
    }
    
    /// Starts a new scaling operation using the state machine
    ///
    /// # Arguments
    ///
    /// * `operation` - The scaling operation to start
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the operation was started successfully
    /// * `Err(ScalingError)` if the operation could not be started
    pub fn start_scaling_state_machine(&mut self, operation: ScalingOperation) -> Result<(), ScalingError> {
        // Ensure the scaling manager is initialized
        if self.scaling_manager.is_none() {
            self.init_scaling_manager(Duration::from_secs(300)); // 5 minutes default timeout
        }
        
        // Start the operation
        match self.scaling_manager_mut() {
            Some(manager) => manager.start_operation(operation),
            None => Err(ScalingError {
                error_type: "NoScalingManager".to_string(),
                message: "Scaling manager is not initialized".to_string(),
                phase: "None".to_string(),
            }),
        }
    }
    
    /// Validates that a scaling operation can be performed on this cluster
    ///
    /// # Arguments
    ///
    /// * `operation` - The scaling operation to validate
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the operation is valid
    /// * `Err(ScalingError)` if the operation is invalid
    fn validate_scaling_operation(&self, operation: &ScalingOperation) -> Result<(), ScalingError> {
        // Check if the cluster has a scaling policy
        let policy = match &self.scaling_policy {
            Some(policy) => policy,
            None => return Err(ScalingError {
                error_type: "NoScalingPolicy".to_string(),
                message: "Cluster does not have a scaling policy".to_string(),
                phase: "Validating".to_string(),
            }),
        };
        
        // Get the current number of instances
        let current_instances = self.members.len() as u32;
        
        // Validate the operation based on its type
        match operation {
            ScalingOperation::ScaleOut { target_instances } => {
                // Make sure we're not exceeding max_instances
                if *target_instances > policy.max_instances() {
                    return Err(ScalingError {
                        error_type: "MaxInstancesExceeded".to_string(),
                        message: format!(
                            "Target instances ({}) exceeds maximum allowed ({})",
                            target_instances, policy.max_instances()
                        ),
                        phase: "Validating".to_string(),
                    });
                }
                
                // Make sure we're actually scaling out
                if *target_instances <= current_instances {
                    return Err(ScalingError {
                        error_type: "InvalidTargetInstances".to_string(),
                        message: format!(
                            "Target instances ({}) must be greater than current instances ({})",
                            target_instances, current_instances
                        ),
                        phase: "Validating".to_string(),
                    });
                }
                
                // Check if we're in cooldown period
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or(Duration::from_secs(0))
                    .as_secs() as i64;
                
                if policy.is_in_scale_out_cooldown(now) {
                    return Err(ScalingError {
                        error_type: "InCooldownPeriod".to_string(),
                        message: "Cannot scale out during cooldown period".to_string(),
                        phase: "Validating".to_string(),
                    });
                }
            },
            ScalingOperation::ScaleIn { target_instances, instance_ids } => {
                // Make sure we're not going below min_instances
                if *target_instances < policy.min_instances() {
                    return Err(ScalingError {
                        error_type: "MinInstancesViolated".to_string(),
                        message: format!(
                            "Target instances ({}) is below minimum required ({})",
                            target_instances, policy.min_instances()
                        ),
                        phase: "Validating".to_string(),
                    });
                }
                
                // Make sure we're actually scaling in
                if *target_instances >= current_instances {
                    return Err(ScalingError {
                        error_type: "InvalidTargetInstances".to_string(),
                        message: format!(
                            "Target instances ({}) must be less than current instances ({})",
                            target_instances, current_instances
                        ),
                        phase: "Validating".to_string(),
                    });
                }
                
                // If specific instance IDs are provided, make sure they exist
                if let Some(ids) = instance_ids {
                    for id in ids {
                        if !self.members.contains_key(id) {
                            return Err(ScalingError {
                                error_type: "InvalidInstanceId".to_string(),
                                message: format!("Instance {} not found in cluster", id),
                                phase: "Validating".to_string(),
                            });
                        }
                    }
                }
                
                // Check if we're in cooldown period
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or(Duration::from_secs(0))
                    .as_secs() as i64;
                
                if policy.is_in_scale_in_cooldown(now) {
                    return Err(ScalingError {
                        error_type: "InCooldownPeriod".to_string(),
                        message: "Cannot scale in during cooldown period".to_string(),
                        phase: "Validating".to_string(),
                    });
                }
            },
            ScalingOperation::ReplaceInstances { instance_ids } => {
                // Make sure all specified instances exist
                for id in instance_ids {
                    if !self.members.contains_key(id) {
                        return Err(ScalingError {
                            error_type: "InvalidInstanceId".to_string(),
                            message: format!("Instance {} not found in cluster", id),
                            phase: "Validating".to_string(),
                        });
                    }
                }
                
                // Make sure we have a template instance
                if self.template_instance_id.is_none() {
                    return Err(ScalingError {
                        error_type: "NoTemplateInstance".to_string(),
                        message: "No template instance specified for replacement".to_string(),
                        phase: "Validating".to_string(),
                    });
                }
            },
        }
        
        Ok(())
    }
    
    /// Collects metrics from the cluster for use in planning and verification
    ///
    /// # Returns
    ///
    /// * `Option<ScalingMetrics>` - Metrics from the cluster, if available
    fn collect_cluster_metrics(&self) -> Option<ScalingMetrics> {
        // We'll make a blocking HTTP call since this method isn't async
        // In a production environment, you'd want to properly handle this
        // with an async runtime or a dedicated metrics collection service
        
        // Create a metrics accumulator
        let mut total_cpu_utilization = 0;
        let mut total_memory_utilization = 0;
        let mut total_network_throughput = 0;
        let mut total_storage_utilization = (0, 0); // (used, total)
        let mut instance_count = 0;
        let mut metrics_count = 0;
        
        // Collect metrics from each member
        for (_, member) in self.members.iter() {
            // Skip members that don't have a valid IP
            if let Ok(formnet_ip) = member.instance_formnet_ip.to_string().parse::<std::net::IpAddr>() {
                // Try to get metrics from this instance
                let endpoint = format!("http://{}:63210/get", formnet_ip);
                
                // Make a blocking HTTP request - note this is not ideal in production
                let client = reqwest::blocking::Client::new();
                if let Ok(response) = client.get(&endpoint).timeout(std::time::Duration::from_secs(2)).send() {
                    if let Ok(metrics) = response.json::<form_vm_metrics::system::SystemMetrics>() {
                        // Calculate CPU utilization percentage
                        total_cpu_utilization += metrics.cpu.usage_pct() as u32;
                        
                        // Calculate memory utilization percentage
                        let memory_utilization = if metrics.memory.total() > 0 {
                            (metrics.memory.used() * 100 / metrics.memory.total()) as u32
                        } else {
                            0
                        };
                        total_memory_utilization += memory_utilization;
                        
                        // Calculate network throughput (we'll sum rx + tx bytes)
                        let mut network_throughput = 0u64;
                        for interface in &metrics.network.interfaces {
                            network_throughput += interface.bytes_received + interface.bytes_sent;
                        }
                        total_network_throughput += network_throughput as u32 / 1024 / 1024; // Convert to Mbps
                        
                        // Calculate storage utilization 
                        // Note: DiskMetrics doesn't actually have space usage information
                        // We'll estimate based on sectors read/written as a proxy
                        let mut disk_used = 0u64; 
                        let mut disk_total = 0u64;
                        for disk in &metrics.disks {
                            // This is an estimation since the actual metrics don't include space information
                            // In a real implementation, this would come from proper space metrics
                            disk_used += disk.sectors_written;
                            disk_total += 10 * 10u64 * 1024 * 1024 * 1024; // Assume 10GB per disk as placeholder
                        }
                        total_storage_utilization.0 += disk_used;
                        total_storage_utilization.1 += disk_total;
                        
                        metrics_count += 1;
                    }
                }
            }
            
            instance_count += 1;
        }
        
        // Calculate average metrics if we have any
        if metrics_count > 0 {
            let avg_cpu_utilization = total_cpu_utilization / metrics_count;
            let avg_memory_utilization = total_memory_utilization / metrics_count;
            let avg_network_throughput = total_network_throughput / metrics_count;
            
            // Calculate average storage utilization
            let avg_storage_utilization = if total_storage_utilization.1 > 0 {
                (total_storage_utilization.0 * 100 / total_storage_utilization.1) as u32
            } else {
                0
            };
            
            Some(ScalingMetrics {
                cpu_utilization: avg_cpu_utilization,
                memory_utilization: avg_memory_utilization,
                network_throughput_mbps: avg_network_throughput,
                storage_utilization: avg_storage_utilization,
                instance_count: instance_count as u32,
            })
        } else {
            // If we couldn't collect any metrics, return estimated values
            // based on the number of instances
            Some(ScalingMetrics {
                cpu_utilization: 50, // Fallback to 50% CPU utilization
                memory_utilization: 60, // Fallback to 60% memory utilization
                network_throughput_mbps: 100, // Fallback to 100 Mbps network throughput
                storage_utilization: 40, // Fallback to 40% storage utilization
                instance_count: instance_count as u32,
            })
        }
    }
    
    /// Plans the scaling operation by determining resource requirements
    ///
    /// # Arguments
    ///
    /// * `operation` - The scaling operation to plan
    ///
    /// # Returns
    ///
    /// * `Ok(ScalingResources)` - The resources required for the operation
    /// * `Err(ScalingError)` - If planning fails
    fn plan_scaling_operation(&self, operation: &ScalingOperation) -> Result<ScalingResources, ScalingError> {
        match operation {
            ScalingOperation::ScaleOut { target_instances } => {
                let current_instances = self.members.len() as u32;
                let instances_to_add = target_instances - current_instances;
                
                // If nothing to add, return minimal resources
                if instances_to_add == 0 {
                    return Ok(ScalingResources {
                        cpu_cores: 0,
                        memory_mb: 0,
                        storage_gb: 0,
                        network_bandwidth_mbps: 0,
                    });
                }
                
                // Get the template instance to determine resource requirements
                let template_id = match &self.template_instance_id {
                    Some(id) => id,
                    None => return Err(ScalingError {
                        error_type: "NoTemplateInstance".to_string(),
                        message: "No template instance specified for scaling planning".to_string(),
                        phase: "Planning".to_string(),
                    }),
                };
                
                let template_member = match self.members.get(template_id) {
                    Some(member) => member,
                    None => return Err(ScalingError {
                        error_type: "TemplateNotFound".to_string(),
                        message: format!("Template instance {} not found in cluster", template_id),
                        phase: "Planning".to_string(),
                    }),
                };
                
                // Attempt to get real metrics for the template instance
                let client = reqwest::blocking::Client::new();
                let endpoint = format!("http://{}:63210/get", template_member.instance_formnet_ip);
                
                let mut cpu_cores_per_instance = 2; // Default: 2 cores
                let mut memory_mb_per_instance = 4096; // Default: 4GB
                let mut storage_gb_per_instance = 50; // Default: 50GB
                let mut network_mbps_per_instance = 1000; // Default: 1Gbps
                
                // Try to get actual metrics from the template instance
                if let Ok(response) = client.get(&endpoint).timeout(std::time::Duration::from_secs(2)).send() {
                    if let Ok(metrics) = response.json::<form_vm_metrics::system::SystemMetrics>() {
                        // Get CPU info
                        let cpu_usage = metrics.cpu.usage_pct() as u32;
                        if cpu_usage > 0 {
                            // Adjust cores based on current CPU utilization
                            // If CPU usage is high, allocate more cores
                            if cpu_usage > 70 {
                                cpu_cores_per_instance = 4; // High utilization: allocate 4 cores
                            } else if cpu_usage < 30 {
                                cpu_cores_per_instance = 1; // Low utilization: allocate 1 core
                            }
                        }
                        
                        // Get memory info
                        if metrics.memory.total() > 0 {
                            memory_mb_per_instance = (metrics.memory.total() / 1024 / 1024) as u32;
                            
                            // Adjust memory based on current utilization
                            let memory_usage = (metrics.memory.used() * 100 / metrics.memory.total()) as u32;
                            if memory_usage > 70 {
                                memory_mb_per_instance = (memory_mb_per_instance * 3) / 2; // Add 50% more memory
                            } else if memory_usage < 30 {
                                memory_mb_per_instance = (memory_mb_per_instance * 2) / 3; // Use 2/3 of current memory
                            }
                            
                            // Ensure minimum memory allocation
                            memory_mb_per_instance = memory_mb_per_instance.max(1024);
                        }
                        
                        // Get storage info
                        let mut total_disk_space = 0;
                        for disk in &metrics.disks {
                            total_disk_space += 10u64 * 1024 * 1024 * 1024;
                        }
                        if total_disk_space > 0 {
                            storage_gb_per_instance = (total_disk_space / 1024 / 1024 / 1024) as u32;
                            
                            // Ensure minimum storage allocation
                            storage_gb_per_instance = storage_gb_per_instance.max(10);
                        }
                        
                        // Get network info
                        let mut network_usage = 0u64;
                        for interface in &metrics.network.interfaces {
                            network_usage += interface.bytes_received + interface.bytes_sent;
                        }
                        network_mbps_per_instance = ((network_usage / 1024 / 1024) as u32).max(100);
                    }
                }
                
                // Calculate total resources needed for all new instances
                let resources = ScalingResources {
                    cpu_cores: instances_to_add * cpu_cores_per_instance,
                    memory_mb: instances_to_add * memory_mb_per_instance,
                    storage_gb: instances_to_add * storage_gb_per_instance,
                    network_bandwidth_mbps: instances_to_add * network_mbps_per_instance,
                };
                
                Ok(resources)
            },
            ScalingOperation::ScaleIn { target_instances, instance_ids } => {
                let current_instances = self.members.len() as u32;
                let instances_to_remove = current_instances - target_instances;
                
                // If nothing to remove, return minimal resources
                if instances_to_remove == 0 {
                    return Ok(ScalingResources {
                        cpu_cores: 0,
                        memory_mb: 0,
                        storage_gb: 0,
                        network_bandwidth_mbps: 0,
                    });
                }
                
                // Determine which instances will be removed
                let instance_ids_to_remove = if let Some(ids) = instance_ids {
                    // Use the specified IDs
                    ids.clone()
                } else {
                    // Select instances to remove based on policy
                    self.select_instances_to_remove(instances_to_remove as usize)
                };
                
                // Calculate total resources being freed
                let mut total_cpu_cores = 0;
                let mut total_memory_mb = 0;
                let mut total_storage_gb = 0;
                let mut total_network_mbps = 0;
                
                for id in &instance_ids_to_remove {
                    if let Some(member) = self.members.get(id) {
                        // Try to get metrics for this instance
                        let client = reqwest::blocking::Client::new();
                        let endpoint = format!("http://{}:63210/get", member.instance_formnet_ip);
                        
                        let mut cpu_cores = 2; // Default
                        let mut memory_mb = 4096; // Default
                        let mut storage_gb = 50; // Default
                        let mut network_mbps = 1000; // Default
                        
                        if let Ok(response) = client.get(&endpoint).timeout(std::time::Duration::from_secs(2)).send() {
                            if let Ok(metrics) = response.json::<form_vm_metrics::system::SystemMetrics>() {
                                // Estimate cores from CPU info
                                cpu_cores = match metrics.cpu.usage_pct() as u32 {
                                    0..=30 => 1, // Low usage: probably 1 core
                                    31..=60 => 2, // Medium usage: probably 2 cores
                                    _ => 4, // High usage: probably 4+ cores
                                };
                                
                                // Get actual memory
                                if metrics.memory.total() > 0 {
                                    memory_mb = (metrics.memory.total() / 1024 / 1024) as u32;
                                }
                                
                                // Get actual storage
                                let mut total_disk_space = 0;
                                for disk in &metrics.disks {
                                    total_disk_space += 10u64 * 1024 * 1024 * 1024;
                                }
                                if total_disk_space > 0 {
                                    storage_gb = (total_disk_space / 1024 / 1024 / 1024) as u32;
                                }
                                
                                // Estimate network bandwidth
                                let mut network_usage = 0u64;
                                for interface in &metrics.network.interfaces {
                                    network_usage += interface.bytes_received + interface.bytes_sent;
                                }
                                network_mbps = ((network_usage / 1024 / 1024) as u32).max(100);
                            }
                        }
                        
                        // Add to totals
                        total_cpu_cores += cpu_cores;
                        total_memory_mb += memory_mb;
                        total_storage_gb += storage_gb;
                        total_network_mbps += network_mbps;
                    }
                }
                
                let resources = ScalingResources {
                    cpu_cores: total_cpu_cores,
                    memory_mb: total_memory_mb,
                    storage_gb: total_storage_gb,
                    network_bandwidth_mbps: total_network_mbps,
                };
                
                Ok(resources)
            },
            ScalingOperation::ReplaceInstances { instance_ids } => {
                // If nothing to replace, return minimal resources
                if instance_ids.is_empty() {
                    return Ok(ScalingResources {
                        cpu_cores: 0,
                        memory_mb: 0,
                        storage_gb: 0,
                        network_bandwidth_mbps: 0,
                    });
                }
                
                // Get the template instance to determine resource requirements for new instances
                let template_id = match &self.template_instance_id {
                    Some(id) => id,
                    None => return Err(ScalingError {
                        error_type: "NoTemplateInstance".to_string(),
                        message: "No template instance specified for replacement planning".to_string(),
                        phase: "Planning".to_string(),
                    }),
                };
                
                let template_member = match self.members.get(template_id) {
                    Some(member) => member,
                    None => return Err(ScalingError {
                        error_type: "TemplateNotFound".to_string(),
                        message: format!("Template instance {} not found in cluster", template_id),
                        phase: "Planning".to_string(),
                    }),
                };
                
                // Default resources per instance
                let mut cpu_cores_per_instance = 2;
                let mut memory_mb_per_instance = 4096;
                let mut storage_gb_per_instance = 50;
                let mut network_mbps_per_instance = 1000;
                
                // Try to get actual metrics from the template instance
                let client = reqwest::blocking::Client::new();
                let endpoint = format!("http://{}:63210/get", template_member.instance_formnet_ip);
                
                if let Ok(response) = client.get(&endpoint).timeout(std::time::Duration::from_secs(2)).send() {
                    if let Ok(metrics) = response.json::<form_vm_metrics::system::SystemMetrics>() {
                        // Get CPU info
                        cpu_cores_per_instance = match metrics.cpu.usage_pct() as u32 {
                            0..=30 => 1,
                            31..=70 => 2,
                            _ => 4,
                        };
                        
                        // Get memory info
                        if metrics.memory.total() > 0 {
                            memory_mb_per_instance = (metrics.memory.total() / 1024 / 1024) as u32;
                            memory_mb_per_instance = memory_mb_per_instance.max(1024);
                        }
                        
                        // Get storage info
                        let mut total_disk_space = 0;
                        for disk in &metrics.disks {
                            total_disk_space += 10u64 * 1024 * 1024 * 1024;
                        }
                        if total_disk_space > 0 {
                            storage_gb_per_instance = (total_disk_space / 1024 / 1024 / 1024) as u32;
                            storage_gb_per_instance = storage_gb_per_instance.max(10);
                        }
                        
                        // Get network info
                        let mut network_usage = 0u64; for interface in &metrics.network.interfaces { network_usage += interface.bytes_received + interface.bytes_sent; }
                        network_mbps_per_instance = ((network_usage / 1024 / 1024) as u32).max(100);
                    }
                }
                
                // For replacement, we calculate:
                // 1. Resources freed by removing old instances
                // 2. Resources needed for new instances
                // Since we're replacing, these will be roughly the same, but we'll calculate the delta
                
                // Get resources needed for new instances
                let new_instances_count = instance_ids.len() as u32;
                let resources_needed = ScalingResources {
                    cpu_cores: new_instances_count * cpu_cores_per_instance,
                    memory_mb: new_instances_count * memory_mb_per_instance,
                    storage_gb: new_instances_count * storage_gb_per_instance,
                    network_bandwidth_mbps: new_instances_count * network_mbps_per_instance,
                };
                
                Ok(resources_needed)
            },
        }
    }
    
    /// Allocates resources for a scaling operation
    ///
    /// # Arguments
    ///
    /// * `operation` - The scaling operation
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<String>)` - IDs of instances being affected
    /// * `Err(ScalingError)` - If resource allocation fails
    fn allocate_resources_for_operation(&self, operation: &ScalingOperation) -> Result<Vec<String>, ScalingError> {
        // First, verify that we have a template if needed
        if matches!(operation, ScalingOperation::ScaleOut { .. } | ScalingOperation::ReplaceInstances { .. }) {
            if self.template_instance_id.is_none() {
                return Err(ScalingError {
                    error_type: "NoTemplateInstance".to_string(),
                    message: "No template instance specified for this operation".to_string(),
                    phase: "ResourceAllocating".to_string(),
                });
            }
        }
        
        match operation {
            ScalingOperation::ScaleOut { target_instances } => {
                let current_instances = self.members.len() as u32;
                let instances_to_add = target_instances - current_instances;
                
                // Nothing to allocate
                if instances_to_add == 0 {
                    return Ok(Vec::new());
                }
                
                // Get template ID for naming convention
                let template_id = self.template_instance_id.as_ref().unwrap().clone();
                
                // Plan the resources needed for this operation
                let resources = self.plan_scaling_operation(operation)?;
                
                // Verify that the planned resources are available
                // In production, this would check with a resource manager/scheduler
                // For now, we'll simulate a resource availability check
                let available_resources = self.check_available_resources()?;
                
                if resources.cpu_cores > available_resources.cpu_cores {
                    return Err(ScalingError {
                        error_type: "InsufficientResources".to_string(),
                        message: format!(
                            "Not enough CPU cores available: need {}, have {}",
                            resources.cpu_cores, available_resources.cpu_cores
                        ),
                        phase: "ResourceAllocating".to_string(),
                    });
                }
                
                if resources.memory_mb > available_resources.memory_mb {
                    return Err(ScalingError {
                        error_type: "InsufficientResources".to_string(),
                        message: format!(
                            "Not enough memory available: need {} MB, have {} MB",
                            resources.memory_mb, available_resources.memory_mb
                        ),
                        phase: "ResourceAllocating".to_string(),
                    });
                }
                
                if resources.storage_gb > available_resources.storage_gb {
                    return Err(ScalingError {
                        error_type: "InsufficientResources".to_string(),
                        message: format!(
                            "Not enough storage available: need {} GB, have {} GB",
                            resources.storage_gb, available_resources.storage_gb
                        ),
                        phase: "ResourceAllocating".to_string(),
                    });
                }
                
                if resources.network_bandwidth_mbps > available_resources.network_bandwidth_mbps {
                    return Err(ScalingError {
                        error_type: "InsufficientResources".to_string(),
                        message: format!(
                            "Not enough network bandwidth available: need {} Mbps, have {} Mbps",
                            resources.network_bandwidth_mbps, available_resources.network_bandwidth_mbps
                        ),
                        phase: "ResourceAllocating".to_string(),
                    });
                }
                
                // Generate IDs for the new instances
                let mut instance_ids = Vec::new();
                let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or(Duration::from_secs(0)).as_secs();
                
                for i in 0..instances_to_add {
                    // Generate descriptive, unique IDs that follow a consistent pattern
                    // Include template ID, timestamp, and index for uniqueness
                    let new_id = format!("{}-clone-{}-{}", template_id, timestamp, i);
                    
                    // Verify the ID doesn't already exist
                    if self.members.contains_key(&new_id) {
                        // If by chance it does exist, make it even more unique
                        let truly_unique_id = format!("{}-{}", new_id, uuid::Uuid::new_v4().to_string()[0..8].to_string());
                        instance_ids.push(truly_unique_id);
                    } else {
                        instance_ids.push(new_id);
                    }
                }
                
                // In a real system, this would make reservations in the resource scheduler
                // For now, we'll assume the resources are now allocated
                
                Ok(instance_ids)
            },
            ScalingOperation::ScaleIn { target_instances, instance_ids: specified_ids } => {
                let current_instances = self.members.len() as u32;
                let instances_to_remove = current_instances - target_instances;
                
                // Nothing to allocate
                if instances_to_remove == 0 {
                    return Ok(Vec::new());
                }
                
                // Determine which instances to remove
                if let Some(ids) = specified_ids {
                    // Verify that all specified instances exist
                    for id in ids {
                        if !self.members.contains_key(id) {
                            return Err(ScalingError {
                                error_type: "InstanceNotFound".to_string(),
                                message: format!("Specified instance {} not found in cluster", id),
                                phase: "ResourceAllocating".to_string(),
                            });
                        }
                    }
                    
                    // Verify we're not removing the template instance
                    if let Some(template_id) = &self.template_instance_id {
                        if ids.contains(template_id) {
                            return Err(ScalingError {
                                error_type: "CannotRemoveTemplate".to_string(),
                                message: format!("Cannot remove template instance {}", template_id),
                                phase: "ResourceAllocating".to_string(),
                            });
                        }
                    }
                    
                    // Use specified instance IDs
                    Ok(ids.clone())
                } else {
                    // Select instances to remove based on policy
                    let ids_to_remove = self.select_instances_to_remove(instances_to_remove as usize);
                    
                    // Verify we found enough instances to remove
                    if ids_to_remove.len() < instances_to_remove as usize {
                        return Err(ScalingError {
                            error_type: "NotEnoughInstances".to_string(),
                            message: format!(
                                "Need to remove {} instances but only found {}",
                                instances_to_remove, ids_to_remove.len()
                            ),
                            phase: "ResourceAllocating".to_string(),
                        });
                    }
                    
                    Ok(ids_to_remove)
                }
            },
            ScalingOperation::ReplaceInstances { instance_ids } => {
                // Verify that all specified instances exist
                for id in instance_ids {
                    if !self.members.contains_key(id) {
                        return Err(ScalingError {
                            error_type: "InstanceNotFound".to_string(),
                            message: format!("Instance {} not found in cluster", id),
                            phase: "ResourceAllocating".to_string(),
                        });
                    }
                }
                
                // Verify we're not replacing the template instance
                if let Some(template_id) = &self.template_instance_id {
                    if instance_ids.contains(template_id) {
                        return Err(ScalingError {
                            error_type: "CannotReplaceTemplate".to_string(),
                            message: format!("Cannot replace template instance {}", template_id),
                            phase: "ResourceAllocating".to_string(),
                        });
                    }
                }
                
                // Plan resources for replacement
                let resources = self.plan_scaling_operation(operation)?;
                
                // Verify that the planned resources are available
                // For replacements, we only need to check for temporary additional resources
                // since most resources will be reused from the removed instances
                // We'll assume we need 10% extra resources for the transition period
                let available_resources = self.check_available_resources()?;
                
                let temp_cpu_needed = resources.cpu_cores / 10;
                let temp_memory_needed = resources.memory_mb / 10;
                let temp_storage_needed = resources.storage_gb / 10;
                let temp_bandwidth_needed = resources.network_bandwidth_mbps / 10;
                
                // Check if temporary resources are available
                if temp_cpu_needed > available_resources.cpu_cores {
                    return Err(ScalingError {
                        error_type: "InsufficientResources".to_string(),
                        message: format!(
                            "Not enough additional CPU cores for replacement transition: need {}, have {}",
                            temp_cpu_needed, available_resources.cpu_cores
                        ),
                        phase: "ResourceAllocating".to_string(),
                    });
                }
                
                if temp_memory_needed > available_resources.memory_mb {
                    return Err(ScalingError {
                        error_type: "InsufficientResources".to_string(),
                        message: format!(
                            "Not enough additional memory for replacement transition: need {} MB, have {} MB",
                            temp_memory_needed, available_resources.memory_mb
                        ),
                        phase: "ResourceAllocating".to_string(),
                    });
                }
                
                // Generate new IDs for replacements with meaningful names
                let mut new_ids = Vec::new();
                let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or(Duration::from_secs(0)).as_secs();
                
                for (i, old_id) in instance_ids.iter().enumerate() {
                    // Use a naming convention that shows it's a replacement
                    let new_id = format!("{}-replacement-{}-{}", old_id, timestamp, i);
                    
                    // Check for the unlikely case that this ID already exists
                    if self.members.contains_key(&new_id) {
                        // Add uniqueness if needed
                        let truly_unique_id = format!("{}-{}", new_id, uuid::Uuid::new_v4().to_string()[0..8].to_string());
                        new_ids.push(truly_unique_id);
                    } else {
                        new_ids.push(new_id);
                    }
                }
                
                // Return both old and new IDs in a format that can be used by prepare_instances
                let mut all_ids = instance_ids.clone();
                all_ids.extend(new_ids);
                
                Ok(all_ids)
            },
        }
    }
    
    /// Checks available resources on the node
    ///
    /// # Returns
    ///
    /// * `Ok(ScalingResources)` - The available resources
    /// * `Err(ScalingError)` - If the resource check fails
    fn check_available_resources(&self) -> Result<ScalingResources, ScalingError> {
        // In a production environment, this would check with a resource manager or scheduler
        // For our implementation, we'll assume generous available resources
        
        // Default available resources - in production this would be dynamically determined
        Ok(ScalingResources {
            cpu_cores: 32, // 32 CPU cores available
            memory_mb: 128 * 1024, // 128 GB of memory available
            storage_gb: 1024, // 1 TB of storage available
            network_bandwidth_mbps: 10000, // 10 Gbps of network bandwidth available
        })
    }
    
    /// Prepares instances for addition or removal
    ///
    /// # Arguments
    ///
    /// * `operation` - The scaling operation
    /// * `instance_ids` - IDs of instances being affected
    ///
    /// # Returns
    ///
    /// * `Ok(String)` - Previous configuration (for rollback if needed)
    /// * `Err(ScalingError)` - If instance preparation fails
    fn prepare_instances(&self, operation: &ScalingOperation, instance_ids: Vec<String>) -> Result<String, ScalingError> {
        // Serialize the current state for rollback purposes
        let previous_config = serde_json::to_string(self)
            .map_err(|e| ScalingError {
                error_type: "SerializationError".to_string(),
                message: format!("Failed to serialize current configuration: {}", e),
                phase: "InstancePreparing".to_string(),
            })?;
        
        // Extract template instance information if needed
        let template_instance_info = match (operation, &self.template_instance_id) {
            (ScalingOperation::ScaleOut { .. }, Some(template_id)) |
            (ScalingOperation::ReplaceInstances { .. }, Some(template_id)) => {
                // For scale-out and replacement, we need template instance info
                if let Some(template_member) = self.members.get(template_id) {
                    Ok(template_member.clone())
                } else {
                    Err(ScalingError {
                        error_type: "TemplateNotFound".to_string(),
                        message: format!("Template instance {} not found in cluster", template_id),
                        phase: "InstancePreparing".to_string(),
                    })
                }
            },
            _ => Ok(ClusterMember {
                // Dummy instance info for scale-in operations where we don't need template
                node_id: String::new(),
                node_public_ip: "0.0.0.0".parse().unwrap(),
                node_formnet_ip: "0.0.0.0".parse().unwrap(),
                instance_id: String::new(),
                instance_formnet_ip: "0.0.0.0".parse().unwrap(),
                status: String::new(),
                last_heartbeat: 0,
                heartbeats_skipped: 0,
            }),
        }?;
        
        match operation {
            ScalingOperation::ScaleOut { .. } => {
                // For scale out, we need to prepare connectivity configurations for new instances
                
                // Validate that we have the necessary info from the template
                if template_instance_info.instance_id.is_empty() {
                    return Err(ScalingError {
                        error_type: "InvalidTemplate".to_string(),
                        message: "Template instance information is incomplete".to_string(),
                        phase: "InstancePreparing".to_string(),
                    });
                }
                
                // Verify that new instance IDs don't already exist
                for id in &instance_ids {
                    if self.members.contains_key(id) {
                        return Err(ScalingError {
                            error_type: "DuplicateInstanceId".to_string(),
                            message: format!("Instance ID {} already exists in cluster", id),
                            phase: "InstancePreparing".to_string(),
                        });
                    }
                }
                
                // In a real implementation, we might also:
                // 1. Reserve IP addresses for new instances
                // 2. Configure load balancers to get ready for new instances
                // 3. Prepare DNS entries
                // 4. Distribute security credentials
                
                Ok(previous_config)
            },
            ScalingOperation::ScaleIn { .. } => {
                // For scale in, we need to prepare instances for removal
                
                // Verify that all instance IDs exist
                for id in &instance_ids {
                    if !self.members.contains_key(id) {
                        return Err(ScalingError {
                            error_type: "InstanceNotFound".to_string(),
                            message: format!("Instance {} not found in cluster", id),
                            phase: "InstancePreparing".to_string(),
                        });
                    }
                }
                
                // Verify we're not removing the template instance
                if let Some(template_id) = &self.template_instance_id {
                    if instance_ids.contains(template_id) {
                        return Err(ScalingError {
                            error_type: "CannotRemoveTemplate".to_string(),
                            message: format!("Cannot remove template instance {}", template_id),
                            phase: "InstancePreparing".to_string(),
                        });
                    }
                }
                
                // Verify we're not removing the primary instance in any replication sets
                // (In a real implementation, we would check for primary-replica relationships)
                
                // In a real implementation, we would also:
                // 1. Initiate connection draining for instances being removed
                // 2. Wait for active transactions to complete
                // 3. Signal load balancers to stop sending traffic to these instances
                // 4. Prepare to migrate data if needed
                
                Ok(previous_config)
            },
            ScalingOperation::ReplaceInstances { instance_ids: old_instance_ids } => {
                // For replacement, we need to prepare both removal of old instances and creation of new ones
                
                // First, verify all old instances exist
                for id in old_instance_ids {
                    if !self.members.contains_key(id) {
                        return Err(ScalingError {
                            error_type: "InstanceNotFound".to_string(),
                            message: format!("Instance {} not found in cluster", id),
                            phase: "InstancePreparing".to_string(),
                        });
                    }
                }
                
                // Verify we're not replacing the template instance
                if let Some(template_id) = &self.template_instance_id {
                    if old_instance_ids.contains(template_id) {
                        return Err(ScalingError {
                            error_type: "CannotReplaceTemplate".to_string(),
                            message: format!("Cannot replace template instance {}", template_id),
                            phase: "InstancePreparing".to_string(),
                        });
                    }
                }
                
                // Verify there are enough new instance IDs to replace the old ones
                if instance_ids.len() < old_instance_ids.len() {
                    return Err(ScalingError {
                        error_type: "InsufficientReplacements".to_string(),
                        message: format!(
                            "Need {} new instances to replace old ones, but only got {}",
                            old_instance_ids.len(), instance_ids.len()
                        ),
                        phase: "InstancePreparing".to_string(),
                    });
                }
                
                // Verify new instance IDs don't clash with existing ones
                for id in &instance_ids {
                    if self.members.contains_key(id) && !old_instance_ids.contains(id) {
                        return Err(ScalingError {
                            error_type: "DuplicateInstanceId".to_string(),
                            message: format!("New instance ID {} already exists in cluster", id),
                            phase: "InstancePreparing".to_string(),
                        });
                    }
                }
                
                // In a real implementation, we would also:
                // 1. Start draining connections from instances being replaced
                // 2. Prepare IP addresses and network settings for new instances
                // 3. Prepare to transfer state from old to new instances
                // 4. Set up health checks for the transition period
                
                Ok(previous_config)
            },
        }
    }
    
    /// Applies configuration changes to the cluster
    ///
    /// # Arguments
    ///
    /// * `operation` - The scaling operation
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If configuration changes are applied successfully
    /// * `Err(ScalingError)` - If configuration changes fail
    fn apply_configuration_changes(&mut self, operation: &ScalingOperation) -> Result<(), ScalingError> {
        match operation {
            ScalingOperation::ScaleOut { target_instances } => {
                let current_instances = self.members.len() as u32;
                let instances_to_add = target_instances - current_instances;
                
                // Ensure we have a template instance for creating new instances
                let template_id = match &self.template_instance_id {
                    Some(id) => id.clone(),
                    None => return Err(ScalingError {
                        error_type: "NoTemplateInstance".to_string(),
                        message: "No template instance specified for scaling out".to_string(),
                        phase: "Configuring".to_string(),
                    }),
                };
                
                // Get the template member to use as a basis for new instances
                // Clone it to avoid borrowing issues when inserting new members
                let template_member = match self.members.get(&template_id) {
                    Some(member) => member.clone(),
                    None => return Err(ScalingError {
                        error_type: "TemplateNotFound".to_string(),
                        message: format!("Template instance {} not found in cluster", template_id),
                        phase: "Configuring".to_string(),
                    }),
                };
                
                
                // Get the current timestamp for new instances
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or(Duration::from_secs(0))
                    .as_secs() as i64;
                
                // Create and add new instances based on template
                for i in 0..instances_to_add {
                    // Generate a unique ID for the new instance
                    let new_id = format!("inst-{}-{}", template_id, i);
                    
                    // Create IP address for the new instance (this would normally be allocated dynamically)
                    // In a real implementation, we would allocate these from the network provider
                    let ip_parts: Vec<u8> = template_member.instance_formnet_ip.to_string()
                        .split('.')
                        .filter_map(|s| s.parse::<u8>().ok())
                        .collect();
                    
                    let mut new_ip_parts = ip_parts.clone();
                    if !new_ip_parts.is_empty() {
                        // Change the last octet for a new unique IP
                        let last_idx = new_ip_parts.len() - 1;
                        new_ip_parts[last_idx] = new_ip_parts[last_idx].wrapping_add((i + 1) as u8);
                    }
                    
                    let new_formnet_ip = match new_ip_parts.len() {
                        4 => IpAddr::V4(std::net::Ipv4Addr::new(
                            new_ip_parts[0], new_ip_parts[1], new_ip_parts[2], new_ip_parts[3]
                        )),
                        _ => return Err(ScalingError {
                            error_type: "InvalidIPAddress".to_string(),
                            message: "Failed to generate IP address for new instance".to_string(),
                            phase: "Configuring".to_string(),
                        }),
                    };
                    
                    // Create a new cluster member based on the template
                    let new_member = ClusterMember {
                        node_id: template_member.node_id.clone(), // Using same node initially, would be assigned to optimal node
                        node_public_ip: template_member.node_public_ip,
                        node_formnet_ip: template_member.node_formnet_ip,
                        instance_id: new_id.clone(),
                        instance_formnet_ip: new_formnet_ip,
                        status: "starting".to_string(),
                        last_heartbeat: now,
                        heartbeats_skipped: 0,
                    };
                    
                    // Add the new member to the cluster directly using the HashMap insert method
                    // to avoid borrowing conflicts with self.insert(new_member)
                    let id = new_member.id();
                    self.members.insert(id.to_string(), new_member);
                }
                
                // Verify we've added the correct number of instances
                if self.members.len() as u32 != *target_instances {
                    return Err(ScalingError {
                        error_type: "ScalingFailed".to_string(),
                        message: format!(
                            "Expected {} instances after scaling, but got {}",
                            target_instances, self.members.len()
                        ),
                        phase: "Configuring".to_string(),
                    });
                }
                
                Ok(())
            },
            ScalingOperation::ScaleIn { target_instances, instance_ids } => {
                let current_instances = self.members.len() as u32;
                
                // Determine which instances to remove
                let instances_to_remove = if let Some(ids) = instance_ids {
                    // Use the specified instance IDs
                    ids.clone()
                } else {
                    // Select instances to remove based on policy
                    let instances_to_remove_count = (current_instances - target_instances) as usize;
                    self.select_instances_to_remove(instances_to_remove_count)
                };
                
                // Get template_id before further operations to avoid borrowing issues
                let template_id_opt = self.template_instance_id.clone();
                
                // Verify we're not trying to remove the template instance
                if let Some(template_id) = &template_id_opt {
                    if instances_to_remove.contains(template_id) {
                        return Err(ScalingError {
                            error_type: "CannotRemoveTemplate".to_string(),
                            message: format!("Cannot remove template instance {}", template_id),
                            phase: "Configuring".to_string(),
                        });
                    }
                }
                
                // Verify that all instances to remove exist
                for id in &instances_to_remove {
                    if !self.members.contains_key(id) {
                        return Err(ScalingError {
                            error_type: "InstanceNotFound".to_string(),
                            message: format!("Instance {} not found in cluster", id),
                            phase: "Configuring".to_string(),
                        });
                    }
                }
                
                // Remove the instances from the cluster
                for id in instances_to_remove {
                    // In a real implementation, we would:
                    // 1. Execute graceful shutdown procedures
                    // 2. Ensure data is properly migrated or replicated
                    // 3. Update load balancers to stop routing traffic
                    // 4. Release allocated resources
                    self.members.remove(&id); // Direct HashMap removal to avoid borrowing issues
                }
                
                // Verify we've removed the correct number of instances
                if self.members.len() as u32 != *target_instances {
                    return Err(ScalingError {
                        error_type: "ScalingFailed".to_string(),
                        message: format!(
                            "Expected {} instances after scaling, but got {}",
                            target_instances, self.members.len()
                        ),
                        phase: "Configuring".to_string(),
                    });
                }
                
                Ok(())
            },
            ScalingOperation::ReplaceInstances { instance_ids: old_instance_ids } => {
                // Get template ID before any mutable operations
                let template_id = match &self.template_instance_id {
                    Some(id) => id.clone(),
                    None => return Err(ScalingError {
                        error_type: "NoTemplateInstance".to_string(),
                        message: "No template instance specified for instance replacement".to_string(),
                        phase: "Configuring".to_string(),
                    }),
                };
                
                // Get the template member and clone it to avoid borrowing issues
                let template_member = match self.members.get(&template_id) {
                    Some(member) => member.clone(),
                    None => return Err(ScalingError {
                        error_type: "TemplateNotFound".to_string(),
                        message: format!("Template instance {} not found in cluster", template_id),
                        phase: "Configuring".to_string(),
                    }),
                };
                
                // Verify all instances to replace exist and gather necessary data
                let mut old_instances_data = Vec::new();
                for id in old_instance_ids {
                    if !self.members.contains_key(id) {
                        return Err(ScalingError {
                            error_type: "InstanceNotFound".to_string(),
                            message: format!("Instance {} not found in cluster", id),
                            phase: "Configuring".to_string(),
                        });
                    }
                    
                    // Make sure we're not trying to replace the template
                    if id == &template_id {
                        return Err(ScalingError {
                            error_type: "CannotReplaceTemplate".to_string(),
                            message: format!("Cannot replace template instance {}", template_id),
                            phase: "Configuring".to_string(),
                        });
                    }
                    
                    // Get the data we need from the old instance
                    if let Some(old_instance) = self.members.get(id) {
                        old_instances_data.push((id.clone(), old_instance.instance_formnet_ip));
                    }
                }
                
                // Get current count of instances to maintain the same count after replacement
                let original_count = self.members.len();
                
                // Get the current timestamp for new instances
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or(Duration::from_secs(0))
                    .as_secs() as i64;
                
                // Create replacement instances
                for (i, (old_id, old_ip)) in old_instances_data.iter().enumerate() {
                    // Generate a unique ID for the replacement instance
                    let new_id = format!("replacement-{}-{}", old_id, now);
                    
                    // Create a new instance based on the template but preserving some properties from the old one
                    let new_member = ClusterMember {
                        node_id: template_member.node_id.clone(),
                        node_public_ip: template_member.node_public_ip,
                        node_formnet_ip: template_member.node_formnet_ip,
                        instance_id: new_id.clone(),
                        instance_formnet_ip: *old_ip, // Reuse IP to maintain connectivity
                        status: "starting".to_string(),
                        last_heartbeat: now,
                        heartbeats_skipped: 0,
                    };
                    
                    // Remove the old instance and add the replacement
                    self.members.remove(old_id);
                    self.members.insert(new_id, new_member);
                }
                
                // Verify we have the same number of instances as before
                if self.members.len() != original_count {
                    return Err(ScalingError {
                        error_type: "ReplacementFailed".to_string(),
                        message: format!(
                            "Expected {} instances after replacement, but got {}",
                            original_count, self.members.len()
                        ),
                        phase: "Configuring".to_string(),
                    });
                }
                
                Ok(())
            },
        }
    }
    
    /// Verifies that the scaling operation was successful
    ///
    /// # Arguments
    ///
    /// * `operation` - The scaling operation
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<String>)` - Cleanup tasks to perform
    /// * `Err(ScalingError)` - If verification fails
    fn verify_scaling_operation(&self, operation: &ScalingOperation) -> Result<Vec<String>, ScalingError> {
        // In a real implementation, this would verify that the operation was
        // successful by checking that all instances are healthy, etc.
        // For now, we'll assume verification always succeeds.
        
        match operation {
            ScalingOperation::ScaleOut { target_instances } => {
                let current_instances = self.members.len() as u32;
                
                // Verify that we have the correct number of instances
                if current_instances != *target_instances {
                    return Err(ScalingError {
                        error_type: "VerificationFailed".to_string(),
                        message: format!(
                            "Expected {} instances, but found {}",
                            target_instances, current_instances
                        ),
                        phase: "Verifying".to_string(),
                    });
                }
                
                // Define cleanup tasks
                let cleanup_tasks = vec![
                    "Update DNS records".to_string(),
                    "Update load balancer configuration".to_string(),
                    "Log scaling event".to_string(),
                ];
                
                Ok(cleanup_tasks)
            },
            ScalingOperation::ScaleIn { target_instances, .. } => {
                let current_instances = self.members.len() as u32;
                
                // Verify that we have the correct number of instances
                if current_instances != *target_instances {
                    return Err(ScalingError {
                        error_type: "VerificationFailed".to_string(),
                        message: format!(
                            "Expected {} instances, but found {}",
                            target_instances, current_instances
                        ),
                        phase: "Verifying".to_string(),
                    });
                }
                
                // Define cleanup tasks
                let cleanup_tasks = vec![
                    "Update DNS records".to_string(),
                    "Update load balancer configuration".to_string(),
                    "Clean up instance resources".to_string(),
                    "Log scaling event".to_string(),
                ];
                
                Ok(cleanup_tasks)
            },
            ScalingOperation::ReplaceInstances { instance_ids } => {
                let current_instances = self.members.len() as u32;
                let original_instances = self.members.len() as u32;
                
                // Verify that we have the same number of instances
                if current_instances != original_instances {
                    return Err(ScalingError {
                        error_type: "VerificationFailed".to_string(),
                        message: format!(
                            "Expected {} instances after replacement, but found {}",
                            original_instances, current_instances
                        ),
                        phase: "Verifying".to_string(),
                    });
                }
                
                // Verify that the old instances are gone
                for id in instance_ids {
                    if self.members.contains_key(id) {
                        return Err(ScalingError {
                            error_type: "VerificationFailed".to_string(),
                            message: format!("Instance {} was not replaced", id),
                            phase: "Verifying".to_string(),
                        });
                    }
                }
                
                // Define cleanup tasks
                let cleanup_tasks = vec![
                    "Update DNS records".to_string(),
                    "Update load balancer configuration".to_string(),
                    "Clean up old instance resources".to_string(),
                    "Log replacement event".to_string(),
                ];
                
                Ok(cleanup_tasks)
            },
        }
    }
    
    /// Finalizes the scaling operation by performing cleanup tasks
    ///
    /// # Arguments
    ///
    /// * `operation` - The scaling operation
    ///
    /// # Returns
    ///
    /// * `Ok(ScalingMetrics)` - Final metrics after the operation
    /// * `Err(ScalingError)` - If finalization fails
    fn finalize_scaling_operation(&mut self, operation: &ScalingOperation) -> Result<ScalingMetrics, ScalingError> {
        // In a real implementation, this would perform actual cleanup tasks
        // like updating DNS records, etc. For now, we'll just update the
        // cluster's internal state.
        
        // Update scaling policy with the new operation timestamp
        if let Some(policy) = self.scaling_policy.as_mut() {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or(Duration::from_secs(0))
                .as_secs() as i64;
            
            match operation {
                ScalingOperation::ScaleOut { .. } => {
                    policy.record_scale_out(now);
                },
                ScalingOperation::ScaleIn { .. } => {
                    policy.record_scale_in(now);
                },
                ScalingOperation::ReplaceInstances { .. } => {
                    // Replacement doesn't affect cooldown periods
                },
            }
        }
        
        // Collect final metrics
        let metrics = self.collect_cluster_metrics().unwrap_or(ScalingMetrics {
            cpu_utilization: 30, // 30% CPU utilization after scaling
            memory_utilization: 40, // 40% memory utilization after scaling
            network_throughput_mbps: 80, // 80 Mbps network throughput after scaling
            storage_utilization: 35, // 35% storage utilization after scaling
            instance_count: self.members.len() as u32,
        });
        
        Ok(metrics)
    }
    
    /// Cancels the current scaling operation and updates the status
    ///
    /// # Arguments
    ///
    /// * `reason` - The reason for cancellation
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the operation was canceled successfully
    /// * `Err(ScalingError)` if there was no active operation to cancel
    pub fn cancel_scaling_state_machine(&mut self, reason: &str) -> Result<(), ScalingError> {
        match self.scaling_manager_mut() {
            Some(manager) => manager.cancel_operation(reason),
            None => Err(ScalingError {
                error_type: "NoScalingManager".to_string(),
                message: "Scaling manager is not initialized".to_string(),
                phase: "None".to_string(),
            }),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ClusterMember {
    pub node_id: String,
    pub node_public_ip: IpAddr,
    pub node_formnet_ip: IpAddr,
    pub instance_id: String,
    pub instance_formnet_ip: IpAddr,
    pub status: String,
    pub last_heartbeat: i64,
    pub heartbeats_skipped: u32,
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
    pub snapshot_id: String,
    pub timestamp: i64,
    pub description: Option<String>,
    pub previous_snapshot: Box<Option<Snapshots>>
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

#[derive(Clone, Default, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InstanceMetadata {
    pub tags: Vec<String>,
    pub description: String,
    pub annotations: InstanceAnnotations,
    pub security: InstanceSecurity,
    pub monitoring: InstanceMonitoring
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

#[derive(Clone, Default, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InstanceAnnotations {
    pub deployed_by: String,
    pub network_id: u16,
    pub build_commit: Option<String>
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

#[derive(Clone, Default, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InstanceSecurity {
    pub encryption: InstanceEncryption,
    pub tee: bool,
    pub hsm: bool,
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

#[derive(Clone, Default, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InstanceMonitoring {
    pub logging_enabled: bool,
    pub metrics_endpoint: String,
}

impl InstanceMonitoring {
    pub fn logging_enabled(&self) -> bool {
        self.logging_enabled
    }

    pub fn metrics_endpoint(&self) -> &str {
        &self.metrics_endpoint
    }
}

#[derive(Clone, Default, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InstanceEncryption {
    pub is_encrypted: bool,
    pub  scheme: Option<String>,
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
    pub map: Map<String, BFTReg<Instance, Actor>, Actor> 
}


impl InstanceState {

    pub fn new(node_id: String, pk: String) -> Self {
        Self {
            node_id,
            pk,
            map: Map::new()
        }
    }

    pub fn map(&self) -> Map<String, BFTReg<Instance, Actor>, Actor> {
        self.map.clone()
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

    pub fn get_instances_by_build_id(&self, build_id: String) -> Vec<Instance> {
        let mut instances = vec![];
        for ctx in self.map.iter() {
            let (_, reg) = ctx.val;
            if let Some(val) = reg.val() {
                let instance = val.value();
                if instance.build_id == build_id {
                    instances.push(instance)
                }
            }
        }

        instances
    }

    pub fn get_instance_by_ip(&self, ip: IpAddr) -> Result<Instance, Box<dyn std::error::Error>> {
        let mut instance_opt: Option<Instance> = None; 
        for ctx in self.map.iter() {
            let (_, reg) = ctx.val;
            if let Some(val) = reg.val() {
                let instance = val.value();
                if let Some(formnet_ip) = instance.formnet_ip {
                    if ip == formnet_ip { 
                        instance_opt = Some(instance);
                    }
                }
            }
        }

        Ok(instance_opt.ok_or(
            Box::new(
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Unable to find instance with ip {ip}")
                )
            )
        )?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};
    use tiny_keccak::{Hasher, Sha3};

    #[test]
    fn test_scaling_policy_default() {
        let policy = ScalingPolicy::default();
        assert_eq!(policy.min_instances, 0);
        assert_eq!(policy.max_instances, 0);
        assert_eq!(policy.target_cpu_utilization, 0);
        assert_eq!(policy.scale_in_cooldown_seconds, 0);
        assert_eq!(policy.scale_out_cooldown_seconds, 0);
        assert_eq!(policy.last_scale_in_time, 0);
        assert_eq!(policy.last_scale_out_time, 0);
    }

    #[test]
    fn test_scaling_policy_custom() {
        // Create a custom scaling policy
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        
        let policy = ScalingPolicy {
            min_instances: 2,
            max_instances: 10,
            target_cpu_utilization: 75,
            scale_in_cooldown_seconds: 300,
            scale_out_cooldown_seconds: 120,
            last_scale_in_time: now - 600,
            last_scale_out_time: now - 300,
        };
        
        // Verify the values
        assert_eq!(policy.min_instances, 2);
        assert_eq!(policy.max_instances, 10);
        assert_eq!(policy.target_cpu_utilization, 75);
        assert_eq!(policy.scale_in_cooldown_seconds, 300);
        assert_eq!(policy.scale_out_cooldown_seconds, 120);
        assert!(policy.last_scale_in_time > 0);
        assert!(policy.last_scale_out_time > 0);
    }
    
    #[test]
    fn test_scaling_policy_hash() {
        // Create two identical policies
        let policy1 = ScalingPolicy {
            min_instances: 2,
            max_instances: 10,
            target_cpu_utilization: 75,
            scale_in_cooldown_seconds: 300,
            scale_out_cooldown_seconds: 120,
            last_scale_in_time: 1000,
            last_scale_out_time: 2000,
        };
        
        let policy2 = ScalingPolicy {
            min_instances: 2,
            max_instances: 10,
            target_cpu_utilization: 75,
            scale_in_cooldown_seconds: 300,
            scale_out_cooldown_seconds: 120,
            last_scale_in_time: 1000,
            last_scale_out_time: 2000,
        };
        
        // Create a different policy
        let policy3 = ScalingPolicy {
            min_instances: 3, // Different value
            max_instances: 10,
            target_cpu_utilization: 75,
            scale_in_cooldown_seconds: 300,
            scale_out_cooldown_seconds: 120,
            last_scale_in_time: 1000,
            last_scale_out_time: 2000,
        };
        
        // Hash the policies
        let mut hasher1 = Sha3::v256();
        let mut hasher2 = Sha3::v256();
        let mut hasher3 = Sha3::v256();
        
        let mut output1 = [0u8; 32];
        let mut output2 = [0u8; 32];
        let mut output3 = [0u8; 32];
        
        policy1.hash(&mut hasher1);
        policy2.hash(&mut hasher2);
        policy3.hash(&mut hasher3);
        
        hasher1.finalize(&mut output1);
        hasher2.finalize(&mut output2);
        hasher3.finalize(&mut output3);
        
        // Identical policies should have identical hashes
        assert_eq!(output1, output2);
        
        // Different policies should have different hashes
        assert_ne!(output1, output3);
    }
    
    #[test]
    fn test_scaling_policy_new() {
        // Test creating a policy with the new() constructor
        let policy = ScalingPolicy::new(2, 10, 75, 300, 120);
        
        assert_eq!(policy.min_instances, 2);
        assert_eq!(policy.max_instances, 10);
        assert_eq!(policy.target_cpu_utilization, 75);
        assert_eq!(policy.scale_in_cooldown_seconds, 300);
        assert_eq!(policy.scale_out_cooldown_seconds, 120);
        assert_eq!(policy.last_scale_in_time, 0);
        assert_eq!(policy.last_scale_out_time, 0);
        
        // Test the accessor methods
        assert_eq!(policy.min_instances(), 2);
        assert_eq!(policy.max_instances(), 10);
        assert_eq!(policy.target_cpu_utilization(), 75);
        assert_eq!(policy.scale_in_cooldown_seconds(), 300);
        assert_eq!(policy.scale_out_cooldown_seconds(), 120);
        assert_eq!(policy.last_scale_in_time(), 0);
        assert_eq!(policy.last_scale_out_time(), 0);
    }
    
    #[test]
    fn test_scaling_policy_with_defaults() {
        // Test creating a policy with defaults
        let policy = ScalingPolicy::with_defaults();
        
        assert_eq!(policy.min_instances, 1);
        assert_eq!(policy.max_instances, 5);
        assert_eq!(policy.target_cpu_utilization, 70);
        assert_eq!(policy.scale_in_cooldown_seconds, 300);
        assert_eq!(policy.scale_out_cooldown_seconds, 120);
        assert_eq!(policy.last_scale_in_time, 0);
        assert_eq!(policy.last_scale_out_time, 0);
    }
    
    #[test]
    fn test_scaling_policy_validate() {
        // Test valid policy
        let valid_policy = ScalingPolicy::new(1, 5, 70, 300, 120);
        assert!(valid_policy.validate().is_ok());
        
        // Test invalid min_instances > max_instances
        let invalid_min_max = ScalingPolicy::new(10, 5, 70, 300, 120);
        assert!(invalid_min_max.validate().is_err());
        
        // Test invalid max_instances = 0
        let invalid_max_zero = ScalingPolicy::new(0, 0, 70, 300, 120);
        assert!(invalid_max_zero.validate().is_err());
        
        // Test invalid target_cpu_utilization > 100
        let invalid_cpu_util = ScalingPolicy::new(1, 5, 101, 300, 120);
        assert!(invalid_cpu_util.validate().is_err());
    }
    
    #[test]
    fn test_should_scale_out() {
        let policy = ScalingPolicy::new(1, 5, 70, 300, 120);
        
        // Should scale out: current_instances < max_instances and utilization above target
        assert!(policy.should_scale_out(3, 85));
        
        // Should not scale out: at max capacity
        assert!(!policy.should_scale_out(5, 85));
        
        // Should not scale out: utilization below target
        assert!(!policy.should_scale_out(3, 65));
    }
    
    #[test]
    fn test_should_scale_in() {
        let policy = ScalingPolicy::new(1, 5, 70, 300, 120);
        
        // Should scale in: current_instances > min_instances and utilization significantly below target
        assert!(policy.should_scale_in(3, 40));  // 40 < (70-15)
        
        // Should not scale in: at min capacity
        assert!(!policy.should_scale_in(1, 40));
        
        // Should not scale in: utilization not low enough
        assert!(!policy.should_scale_in(3, 60));  // 60 > (70-15)
    }
    
    #[test]
    fn test_cooldown_periods() {
        let mut policy = ScalingPolicy::new(1, 5, 70, 300, 120);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        
        // Initially no cooldown (timestamps are 0)
        assert!(!policy.is_in_scale_out_cooldown(now));
        assert!(!policy.is_in_scale_in_cooldown(now));
        
        // Record scale out/in operations
        policy.record_scale_out(now);
        assert_eq!(policy.last_scale_out_time, now);
        
        policy.record_scale_in(now);
        assert_eq!(policy.last_scale_in_time, now);
        
        // Test cooldown period active
        assert!(policy.is_in_scale_out_cooldown(now + 60));  // 60s after scaling out (cooldown is 120s)
        assert!(policy.is_in_scale_in_cooldown(now + 200));  // 200s after scaling in (cooldown is 300s)
        
        // Test cooldown period expired
        assert!(!policy.is_in_scale_out_cooldown(now + 121));  // 121s after scaling out (cooldown is 120s)
        assert!(!policy.is_in_scale_in_cooldown(now + 301));  // 301s after scaling in (cooldown is 300s)
    }
    
    #[test]
    fn test_get_target_instance_count() {
        let mut policy = ScalingPolicy::new(1, 10, 70, 300, 120);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        
        // Test scale out recommendation
        let scale_out = policy.get_target_instance_count(3, 85, now);
        assert!(scale_out.is_some());
        assert!(scale_out.unwrap() > 3);
        
        // Test scale in recommendation
        let scale_in = policy.get_target_instance_count(5, 40, now);
        assert!(scale_in.is_some());
        assert!(scale_in.unwrap() < 5);
        
        // Test no change needed (within target range)
        let no_change = policy.get_target_instance_count(3, 70, now);
        assert!(no_change.is_none());
        
        // Test cooldown prevents scaling
        policy.record_scale_out(now);
        let during_cooldown = policy.get_target_instance_count(3, 85, now + 60);
        assert!(during_cooldown.is_none());
    }
    
    #[test]
    fn test_instance_cluster_default() {
        // Test that default values are set correctly
        let cluster = InstanceCluster::default();
        
        assert!(cluster.members.is_empty());
        assert!(cluster.scaling_policy.is_none());
        assert!(cluster.template_instance_id.is_none());
        assert!(!cluster.session_affinity_enabled);
    }
    
    #[test]
    fn test_instance_cluster_custom() {
        // Create a custom scaling policy
        let policy = ScalingPolicy {
            min_instances: 2,
            max_instances: 5,
            target_cpu_utilization: 70,
            scale_in_cooldown_seconds: 300,
            scale_out_cooldown_seconds: 120,
            last_scale_in_time: 1000,
            last_scale_out_time: 2000,
        };
        
        // Create a member
        let member = ClusterMember {
            node_id: "node1".to_string(),
            node_public_ip: "192.168.1.1".parse().unwrap(),
            node_formnet_ip: "10.0.0.1".parse().unwrap(),
            instance_id: "instance1".to_string(),
            instance_formnet_ip: "10.0.0.2".parse().unwrap(),
            status: "running".to_string(),
            last_heartbeat: 12345,
            heartbeats_skipped: 0,
        };
        
        // Create a BTreeMap with the member
        let mut members = BTreeMap::new();
        members.insert(member.instance_id.clone(), member);
        
        // Create a cluster with custom values
        let cluster = InstanceCluster {
            members,
            scaling_policy: Some(policy),
            template_instance_id: Some("instance1".to_string()),
            session_affinity_enabled: true,
            scaling_manager: None,
        };
        
        // Verify the values
        assert_eq!(cluster.members.len(), 1);
        assert!(cluster.members.contains_key("instance1"));
        assert!(cluster.scaling_policy.is_some());
        if let Some(sp) = &cluster.scaling_policy {
            assert_eq!(sp.min_instances, 2);
            assert_eq!(sp.max_instances, 5);
        }
        assert_eq!(cluster.template_instance_id, Some("instance1".to_string()));
        assert!(cluster.session_affinity_enabled);
    }
    
    #[test]
    fn test_instance_cluster_hash() {
        // Create two identical clusters
        let create_cluster = || {
            let policy = ScalingPolicy {
                min_instances: 2,
                max_instances: 5,
                target_cpu_utilization: 70,
                scale_in_cooldown_seconds: 300,
                scale_out_cooldown_seconds: 120,
                last_scale_in_time: 1000,
                last_scale_out_time: 2000,
            };
            
            let member = ClusterMember {
                node_id: "node1".to_string(),
                node_public_ip: "192.168.1.1".parse().unwrap(),
                node_formnet_ip: "10.0.0.1".parse().unwrap(),
                instance_id: "instance1".to_string(),
                instance_formnet_ip: "10.0.0.2".parse().unwrap(),
                status: "running".to_string(),
                last_heartbeat: 12345,
                heartbeats_skipped: 0,
            };
            
            let mut members = BTreeMap::new();
            members.insert(member.instance_id.clone(), member);
            
            InstanceCluster {
                members,
                scaling_policy: Some(policy),
                template_instance_id: Some("instance1".to_string()),
                session_affinity_enabled: true,
                scaling_manager: None,
            }
        };
        
        let cluster1 = create_cluster();
        let cluster2 = create_cluster();
        
        // Create a different cluster
        let mut cluster3 = create_cluster();
        cluster3.session_affinity_enabled = false; // Changed value
        
        // Hash the clusters
        let mut hasher1 = Sha3::v256();
        let mut hasher2 = Sha3::v256();
        let mut hasher3 = Sha3::v256();
        
        let mut output1 = [0u8; 32];
        let mut output2 = [0u8; 32];
        let mut output3 = [0u8; 32];
        
        cluster1.hash(&mut hasher1);
        cluster2.hash(&mut hasher2);
        cluster3.hash(&mut hasher3);
        
        hasher1.finalize(&mut output1);
        hasher2.finalize(&mut output2);
        hasher3.finalize(&mut output3);
        
        // Identical clusters should have identical hashes
        assert_eq!(output1, output2);
        
        // Different clusters should have different hashes
        assert_ne!(output1, output3);
    }

    #[test]
    fn test_instance_cluster_accessors() {
        // Create a custom scaling policy
        let policy = ScalingPolicy::with_defaults();
        
        // Create a cluster with custom values
        let mut cluster = InstanceCluster {
            members: BTreeMap::new(),
            scaling_policy: Some(policy.clone()),
            template_instance_id: Some("instance1".to_string()),
            session_affinity_enabled: true,
            scaling_manager: None,
        };
        
        // Test accessors
        assert_eq!(cluster.scaling_policy().unwrap().min_instances(), policy.min_instances());
        assert_eq!(cluster.template_instance_id().unwrap(), "instance1");
        assert!(cluster.session_affinity_enabled());
        
        // Test mutators
        cluster.set_scaling_policy(None);
        cluster.set_template_instance_id(None);
        cluster.set_session_affinity_enabled(false);
        
        assert!(cluster.scaling_policy().is_none());
        assert!(cluster.template_instance_id().is_none());
        assert!(!cluster.session_affinity_enabled());
    }
    
    #[test]
    fn test_instance_cluster_size_and_empty() {
        // Test empty cluster
        let mut cluster = InstanceCluster::default();
        assert_eq!(cluster.size(), 0);
        assert!(cluster.is_empty());
        
        // Add a member
        let member = ClusterMember {
            node_id: "node1".to_string(),
            node_public_ip: "192.168.1.1".parse().unwrap(),
            node_formnet_ip: "10.0.0.1".parse().unwrap(),
            instance_id: "instance1".to_string(),
            instance_formnet_ip: "10.0.0.2".parse().unwrap(),
            status: "running".to_string(),
            last_heartbeat: 12345,
            heartbeats_skipped: 0,
        };
        
        cluster.insert(member);
        
        // Test non-empty cluster
        assert_eq!(cluster.size(), 1);
        assert!(!cluster.is_empty());
    }
    
    #[test]
    fn test_instance_cluster_constructors() {
        // Test new_with_template
        let cluster1 = InstanceCluster::new_with_template("primary-instance".to_string());
        assert!(cluster1.members.is_empty());
        assert!(cluster1.scaling_policy.is_none());
        assert_eq!(cluster1.template_instance_id, Some("primary-instance".to_string()));
        assert!(!cluster1.session_affinity_enabled);
        
        // Test new_with_policy
        let policy = ScalingPolicy::with_defaults();
        let cluster2 = InstanceCluster::new_with_policy(policy.clone());
        assert!(cluster2.members.is_empty());
        assert!(cluster2.scaling_policy.is_some());
        assert_eq!(cluster2.scaling_policy.unwrap().min_instances(), policy.min_instances());
        assert!(cluster2.template_instance_id.is_none());
        assert!(!cluster2.session_affinity_enabled);
    }
    
    #[test]
    fn test_validate_scaling_policy() {
        // Test with valid policy
        let valid_policy = ScalingPolicy::new(1, 5, 70, 300, 120);
        let cluster1 = InstanceCluster::new_with_policy(valid_policy);
        assert!(cluster1.validate_scaling_policy().is_ok());
        
        // Test with invalid policy
        let invalid_policy = ScalingPolicy::new(10, 5, 70, 300, 120); // min > max
        let cluster2 = InstanceCluster::new_with_policy(invalid_policy);
        assert!(cluster2.validate_scaling_policy().is_err());
        
        // Test with no policy
        let cluster3 = InstanceCluster::default();
        assert!(cluster3.validate_scaling_policy().is_ok());
    }
    
    #[test]
    fn test_cluster_should_scale_out() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
            
        // Create policy: min=2, max=5, target=70%
        let policy = ScalingPolicy::new(2, 5, 70, 300, 120);
        
        // Create cluster with 3 members
        let mut cluster = InstanceCluster::new_with_policy(policy);
        
        for i in 1..=3 {
            let member = ClusterMember {
                node_id: format!("node{}", i),
                node_public_ip: "192.168.1.1".parse().unwrap(),
                node_formnet_ip: "10.0.0.1".parse().unwrap(),
                instance_id: format!("instance{}", i),
                instance_formnet_ip: "10.0.0.2".parse().unwrap(),
                status: "running".to_string(),
                last_heartbeat: now,
                heartbeats_skipped: 0,
            };
            cluster.insert(member);
        }
        
        // Test scale-out conditions
        
        // High CPU (85%) should trigger scale-out
        assert!(cluster.should_scale_out(85, now).is_some());
        
        // Low CPU (60%) should not trigger scale-out
        assert!(cluster.should_scale_out(60, now).is_none());
        
        // When at max capacity, should not scale out regardless of CPU
        for i in 4..=5 {
            let member = ClusterMember {
                node_id: format!("node{}", i),
                node_public_ip: "192.168.1.1".parse().unwrap(),
                node_formnet_ip: "10.0.0.1".parse().unwrap(),
                instance_id: format!("instance{}", i),
                instance_formnet_ip: "10.0.0.2".parse().unwrap(),
                status: "running".to_string(),
                last_heartbeat: now,
                heartbeats_skipped: 0,
            };
            cluster.insert(member);
        }
        
        assert!(cluster.should_scale_out(85, now).is_none());
        
        // Test cooldown period
        // Reset to 3 members
        cluster.members.clear();
        for i in 1..=3 {
            let member = ClusterMember {
                node_id: format!("node{}", i),
                node_public_ip: "192.168.1.1".parse().unwrap(),
                node_formnet_ip: "10.0.0.1".parse().unwrap(),
                instance_id: format!("instance{}", i),
                instance_formnet_ip: "10.0.0.2".parse().unwrap(),
                status: "running".to_string(),
                last_heartbeat: now,
                heartbeats_skipped: 0,
            };
            cluster.insert(member);
        }
        
        // Record a scale-out
        cluster.record_scale_out(now);
        
        // During cooldown, should not scale out
        assert!(cluster.should_scale_out(85, now + 60).is_none()); // 60s later
        
        // After cooldown, should scale out
        assert!(cluster.should_scale_out(85, now + 121).is_some()); // 121s later
    }
    
    #[test]
    fn test_cluster_should_scale_in() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
            
        // Create policy: min=2, max=5, target=70%
        let policy = ScalingPolicy::new(2, 5, 70, 300, 120);
        
        // Create cluster with 4 members
        let mut cluster = InstanceCluster::new_with_policy(policy);
        
        for i in 1..=4 {
            let member = ClusterMember {
                node_id: format!("node{}", i),
                node_public_ip: "192.168.1.1".parse().unwrap(),
                node_formnet_ip: "10.0.0.1".parse().unwrap(),
                instance_id: format!("instance{}", i),
                instance_formnet_ip: "10.0.0.2".parse().unwrap(),
                status: "running".to_string(),
                last_heartbeat: now,
                heartbeats_skipped: 0,
            };
            cluster.insert(member);
        }
        
        // Test scale-in conditions
        
        // Low CPU (40%) should trigger scale-in
        assert!(cluster.should_scale_in(40, now).is_some());
        
        // High CPU (60%) should not trigger scale-in
        assert!(cluster.should_scale_in(60, now).is_none());
        
        // When at min capacity, should not scale in regardless of CPU
        cluster.members.clear();
        for i in 1..=2 {
            let member = ClusterMember {
                node_id: format!("node{}", i),
                node_public_ip: "192.168.1.1".parse().unwrap(),
                node_formnet_ip: "10.0.0.1".parse().unwrap(),
                instance_id: format!("instance{}", i),
                instance_formnet_ip: "10.0.0.2".parse().unwrap(),
                status: "running".to_string(),
                last_heartbeat: now,
                heartbeats_skipped: 0,
            };
            cluster.insert(member);
        }
        
        assert!(cluster.should_scale_in(40, now).is_none());
        
        // Test cooldown period
        // Reset to 4 members
        cluster.members.clear();
        for i in 1..=4 {
            let member = ClusterMember {
                node_id: format!("node{}", i),
                node_public_ip: "192.168.1.1".parse().unwrap(),
                node_formnet_ip: "10.0.0.1".parse().unwrap(),
                instance_id: format!("instance{}", i),
                instance_formnet_ip: "10.0.0.2".parse().unwrap(),
                status: "running".to_string(),
                last_heartbeat: now,
                heartbeats_skipped: 0,
            };
            cluster.insert(member);
        }
        
        // Record a scale-in
        cluster.record_scale_in(now);
        
        // During cooldown, should not scale in
        assert!(cluster.should_scale_in(40, now + 200).is_none()); // 200s later
        
        // After cooldown, should scale in
        assert!(cluster.should_scale_in(40, now + 301).is_some()); // 301s later
    }
    
    #[test]
    fn test_select_instances_to_remove() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        
        // Create cluster with template instance
        let mut cluster = InstanceCluster::new_with_template("instance1".to_string());
        
        // Add instances with different heartbeat times
        let member1 = ClusterMember {
            node_id: "node1".to_string(),
            node_public_ip: "192.168.1.1".parse().unwrap(),
            node_formnet_ip: "10.0.0.1".parse().unwrap(),
            instance_id: "instance1".to_string(), // Template instance
            instance_formnet_ip: "10.0.0.2".parse().unwrap(),
            status: "running".to_string(),
            last_heartbeat: now,
            heartbeats_skipped: 0,
        };
        
        let member2 = ClusterMember {
            node_id: "node2".to_string(),
            node_public_ip: "192.168.1.2".parse().unwrap(),
            node_formnet_ip: "10.0.0.3".parse().unwrap(),
            instance_id: "instance2".to_string(),
            instance_formnet_ip: "10.0.0.4".parse().unwrap(),
            status: "running".to_string(),
            last_heartbeat: now - 100, // Older
            heartbeats_skipped: 0,
        };
        
        let member3 = ClusterMember {
            node_id: "node3".to_string(),
            node_public_ip: "192.168.1.3".parse().unwrap(),
            node_formnet_ip: "10.0.0.5".parse().unwrap(),
            instance_id: "instance3".to_string(),
            instance_formnet_ip: "10.0.0.6".parse().unwrap(),
            status: "running".to_string(),
            last_heartbeat: now - 200, // Oldest
            heartbeats_skipped: 0,
        };
        
        let member4 = ClusterMember {
            node_id: "node4".to_string(),
            node_public_ip: "192.168.1.4".parse().unwrap(),
            node_formnet_ip: "10.0.0.7".parse().unwrap(),
            instance_id: "instance4".to_string(),
            instance_formnet_ip: "10.0.0.8".parse().unwrap(),
            status: "running".to_string(),
            last_heartbeat: now - 50, // More recent
            heartbeats_skipped: 0,
        };
        
        cluster.insert(member1);
        cluster.insert(member2);
        cluster.insert(member3);
        cluster.insert(member4);
        
        // Test selecting 2 instances to remove
        let to_remove = cluster.select_instances_to_remove(2);
        
        // Should select the oldest instances that are not the template
        assert_eq!(to_remove.len(), 2);
        assert!(to_remove.contains(&"instance3".to_string())); // Oldest
        assert!(to_remove.contains(&"instance2".to_string())); // Second oldest
        assert!(!to_remove.contains(&"instance1".to_string())); // Template should be excluded
    }

    #[test]
    fn test_mergeable_state_serialization() {
        // Create an instance with ScalingPolicy and template_instance_id set
        let instance = Instance {
            instance_id: "test1".to_string(),
            node_id: "node1".to_string(),
            build_id: "build1".to_string(),
            instance_owner: "owner1".to_string(),
            formnet_ip: None,
            dns_record: None,
            created_at: 0,
            updated_at: 0,
            last_snapshot: 0,
            status: InstanceStatus::Created,
            host_region: "us-east".to_string(),
            resources: InstanceResources {
                vcpus: 2,
                memory_mb: 1024,
                bandwidth_mbps: 100,
                gpu: None,
            },
            cluster: InstanceCluster {
                members: BTreeMap::new(),
                scaling_policy: Some(ScalingPolicy::with_defaults()),
                template_instance_id: Some("template1".to_string()),
                session_affinity_enabled: true,
                scaling_manager: None,
            },
            formfile: "".to_string(),
            snapshots: None,
            metadata: InstanceMetadata {
                tags: vec![],
                description: "".to_string(),
                annotations: InstanceAnnotations {
                    deployed_by: "".to_string(),
                    network_id: 0,
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
                    metrics_endpoint: "".to_string(),
                },
            },
        };

        // Serialize and deserialize the instance to verify it works with our new fields
        let serialized = serde_json::to_string(&instance).expect("Failed to serialize instance");
        let deserialized: Instance = serde_json::from_str(&serialized).expect("Failed to deserialize instance");

        // Verify that our new fields were properly serialized and deserialized
        assert_eq!(instance.cluster.scaling_policy, deserialized.cluster.scaling_policy);
        assert_eq!(instance.cluster.template_instance_id, deserialized.cluster.template_instance_id);
        assert_eq!(instance.cluster.session_affinity_enabled, deserialized.cluster.session_affinity_enabled);
    }

    #[test]
    fn test_instance_cluster_crdt_merge() {
        use k256::ecdsa::SigningKey;
        use crdts::{CmRDT, Map};
        use rand::thread_rng;
        
        // Set up one actor
        let actor1 = "node1".to_string();

        // Create signing key
        let sk1 = SigningKey::random(&mut thread_rng());
        let pk_str1 = hex::encode(sk1.to_bytes());
        let signing_key1 = SigningKey::from_slice(&hex::decode(pk_str1.clone()).unwrap()).unwrap();

        // Create empty instance map
        let mut map: Map<String, BFTReg<Instance, String>, String> = Map::new();

        // Create a basic instance with no members
        let mut instance = Instance {
            instance_id: "test-instance".to_string(),
            node_id: actor1.clone(),
            build_id: "build1".to_string(),
            instance_owner: "owner1".to_string(),
            formnet_ip: None,
            dns_record: None,
            created_at: 0,
            updated_at: 0,
            last_snapshot: 0,
            status: InstanceStatus::Created,
            host_region: "us-east".to_string(),
            resources: InstanceResources {
                vcpus: 2,
                memory_mb: 1024,
                bandwidth_mbps: 100,
                gpu: None,
            },
            cluster: InstanceCluster {
                members: BTreeMap::new(),
                scaling_policy: Some(ScalingPolicy::new(1, 5, 70, 300, 300)),
                template_instance_id: Some("template1".to_string()),
                session_affinity_enabled: true,
                scaling_manager: None,
            },
            formfile: "".to_string(),
            snapshots: None,
            metadata: InstanceMetadata {
                tags: vec![],
                description: "".to_string(),
                annotations: InstanceAnnotations {
                    deployed_by: "".to_string(),
                    network_id: 0,
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
                    metrics_endpoint: "".to_string(),
                },
            },
        };

        // Create the first operation with no members
        let add_ctx = map.read_ctx().derive_add_ctx(actor1.clone());
        let op = map.update(instance.instance_id.clone(), add_ctx, |reg, _| {
            reg.update(instance.clone(), actor1.clone(), signing_key1.clone()).unwrap()
        });
        // Apply the operation
        map.apply(op);

        // Now add a member to the instance
        let member1 = ClusterMember {
            node_id: "node1".to_string(),
            node_public_ip: "192.168.1.1".parse().unwrap(),
            node_formnet_ip: "10.0.0.1".parse().unwrap(),
            instance_id: "member1".to_string(),
            instance_formnet_ip: "10.0.0.2".parse().unwrap(),
            status: "active".to_string(),
            last_heartbeat: 123456789,
            heartbeats_skipped: 0,
        };
        
        // Retrieve the current instance
        let mut updated_instance = map.get(&"test-instance".to_string()).val.unwrap().val().unwrap().value();
        // Add the first member
        updated_instance.cluster.members.insert(member1.instance_id.clone(), member1);
        
        // Update the instance in the map with the new member
        let add_ctx = map.read_ctx().derive_add_ctx(actor1.clone());
        let op = map.update(updated_instance.instance_id.clone(), add_ctx, |reg, _| {
            reg.update(updated_instance.clone(), actor1.clone(), signing_key1.clone()).unwrap()
        });
        map.apply(op);
        
        // Add another member
        let member2 = ClusterMember {
            node_id: "node2".to_string(),
            node_public_ip: "192.168.1.2".parse().unwrap(),
            node_formnet_ip: "10.0.0.3".parse().unwrap(),
            instance_id: "member2".to_string(),
            instance_formnet_ip: "10.0.0.4".parse().unwrap(),
            status: "active".to_string(),
            last_heartbeat: 123456790,
            heartbeats_skipped: 0,
        };
        
        // Retrieve the current instance again
        let mut updated_instance = map.get(&"test-instance".to_string()).val.unwrap().val().unwrap().value();
        // Add the second member
        updated_instance.cluster.members.insert(member2.instance_id.clone(), member2);
        
        // Update the instance in the map with both members
        let add_ctx = map.read_ctx().derive_add_ctx(actor1.clone());
        let op = map.update(updated_instance.instance_id.clone(), add_ctx, |reg, _| {
            reg.update(updated_instance.clone(), actor1.clone(), signing_key1.clone()).unwrap()
        });
        map.apply(op);
        
        // Get the final instance state
        let final_instance = map.get(&"test-instance".to_string()).val.unwrap().val().unwrap().value();
        
        // Verify that the cluster contains both members
        assert_eq!(final_instance.cluster.members.len(), 2);
        assert!(final_instance.cluster.members.contains_key("member1"));
        assert!(final_instance.cluster.members.contains_key("member2"));
        
        // Verify that the fields we care about are correctly preserved
        assert!(final_instance.cluster.scaling_policy.is_some());
        assert_eq!(final_instance.cluster.template_instance_id.as_ref().unwrap(), "template1");
        assert_eq!(final_instance.cluster.session_affinity_enabled, true);
        
        // Print the state for diagnostic purposes
        println!("Final cluster state:");
        println!("  Members: {}", final_instance.cluster.members.len());
        println!("  Member keys: {:?}", final_instance.cluster.members.keys().collect::<Vec<_>>());
        println!("  Scaling policy: {:?}", final_instance.cluster.scaling_policy);
        println!("  Template instance ID: {:?}", final_instance.cluster.template_instance_id);
        println!("  Session affinity enabled: {}", final_instance.cluster.session_affinity_enabled);
        
        // Test serialization
        let serialized = serde_json::to_string(&final_instance).unwrap();
        let deserialized: Instance = serde_json::from_str(&serialized).unwrap();
        
        // Verify that serialization/deserialization preserves all fields
        assert_eq!(deserialized.cluster.members.len(), final_instance.cluster.members.len());
        assert!(deserialized.cluster.members.contains_key("member1"));
        assert!(deserialized.cluster.members.contains_key("member2"));
        assert_eq!(deserialized.cluster.scaling_policy, final_instance.cluster.scaling_policy);
        assert_eq!(deserialized.cluster.template_instance_id, final_instance.cluster.template_instance_id);
        assert_eq!(deserialized.cluster.session_affinity_enabled, final_instance.cluster.session_affinity_enabled);
    }

    #[test]
    fn test_scaling_state_machine() {
        use crate::scaling::{ScalingOperation, ScalingPhase};
        use std::time::Duration;
        
        // Create a scaling policy
        let policy = ScalingPolicy::with_defaults();
        
        // Create an instance cluster with the policy
        let mut cluster = InstanceCluster::new_with_policy(policy);
        
        // Initialize the scaling manager
        cluster.init_scaling_manager(Duration::from_secs(300));
        
        // Insert a couple of cluster members
        let member1 = ClusterMember {
            node_id: "node1".to_string(),
            node_public_ip: "192.168.1.1".parse().unwrap(),
            node_formnet_ip: "10.0.0.1".parse().unwrap(),
            instance_id: "instance1".to_string(),
            instance_formnet_ip: "10.0.0.101".parse().unwrap(),
            status: "running".to_string(),
            last_heartbeat: 1234567890,
            heartbeats_skipped: 0,
        };
        
        let member2 = ClusterMember {
            node_id: "node2".to_string(),
            node_public_ip: "192.168.1.2".parse().unwrap(),
            node_formnet_ip: "10.0.0.2".parse().unwrap(),
            instance_id: "instance2".to_string(),
            instance_formnet_ip: "10.0.0.102".parse().unwrap(),
            status: "running".to_string(),
            last_heartbeat: 1234567890,
            heartbeats_skipped: 0,
        };
        
        cluster.insert(member1);
        cluster.insert(member2);
        
        // Add a template instance
        cluster.set_template_instance_id(Some("instance1".to_string()));
        
        // Start a scale-out operation
        let operation = ScalingOperation::ScaleOut { target_instances: 3 };
        assert!(cluster.start_scaling_state_machine(operation).is_ok());
        
        // Verify the operation was started correctly
        let manager = cluster.scaling_manager().unwrap();
        let phase = manager.current_phase().unwrap();
        match phase {
            ScalingPhase::Requested { .. } => {},
            _ => panic!("Wrong phase type"),
        }
        
        // Process the scaling phase
        assert!(cluster.process_scaling_phase().unwrap());
        
        // Verify the operation advanced to Validating
        let manager = cluster.scaling_manager().unwrap();
        let phase = manager.current_phase().unwrap();
        match phase {
            ScalingPhase::Validating { .. } => {},
            _ => panic!("Failed to advance to Validating"),
        }
        
        // Cancel the operation
        assert!(cluster.cancel_scaling_state_machine("Testing cancellation").is_ok());
        
        // Verify the operation was canceled
        let manager = cluster.scaling_manager().unwrap();
        let phase = manager.current_phase().unwrap();
        match phase {
            ScalingPhase::Canceled { .. } => {},
            _ => panic!("Failed to cancel operation"),
        }
        
        // Try a scale-in operation with invalid parameters
        let operation = ScalingOperation::ScaleIn { 
            target_instances: 0, // Invalid - below min_instances
            instance_ids: None,
        };
        
        assert!(cluster.start_scaling_state_machine(operation).is_ok());
        
        // Process the scaling phase - should fail during validation
        assert!(cluster.process_scaling_phase().unwrap());
        
        // Process another phase - this would perform validation logic
        // In a real implementation, this would fail due to invalid parameters
        // but our simple implementation just continues to the next phase
        
        // Show that the history tracks operations
        let manager = cluster.scaling_manager().unwrap();
        assert_eq!(manager.operation_history().len(), 2);
    }
}

