use std::{collections::{btree_map::{Iter, IterMut}, BTreeMap}, fmt::Display, net::IpAddr};
use crdts::{map::Op, merkle_reg::Sha3Hash, BFTReg, CmRDT, Map, bft_reg::Update};
use form_dns::store::FormDnsRecord;
use form_types::state::{Response, Success};
use k256::ecdsa::SigningKey;
use reqwest::Client;
use serde::{Serialize, Deserialize};
use tiny_keccak::Hasher;
use crate::Actor;

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
    pub session_affinity_enabled: bool
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
        }
    }

    /// Creates a new InstanceCluster with the specified scaling policy and no members
    pub fn new_with_policy(policy: ScalingPolicy) -> Self {
        Self {
            members: BTreeMap::new(),
            scaling_policy: Some(policy),
            template_instance_id: None,
            session_affinity_enabled: false,
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
}

