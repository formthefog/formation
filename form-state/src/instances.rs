use std::collections::HashSet;

use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
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
    snapshots: Snapshots,
    metadata: InstanceMetadata,
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InstanceResources {
    vcpus: u8,
    memory_mb: u32,
    bandwidth_mbps: u32,
    gpu: Option<InstanceGpu>
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InstanceGpu {
    count: u8,
    model: String,
    vram_mb: u32,
    usage: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InstanceCluster {
    members: HashSet<ClusterMember>
}

#[derive(Clone, Debug, Serialize, Deserialize, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ClusterMember {
    instance_id: String,
    status: String,
    last_heartbeat: i64,
    heartbeats_skipped: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Snapshots {
    snapshot_id: String,
    timestamp: i64,
    description: Option<String>
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InstanceMetadata {
    tags: Vec<String>,
    description: String,
    annotations: InstanceAnnotations,
    security: InstanceSecurity,
    monitoring: InstanceMonitoring
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InstanceAnnotations {
    deployed_by: String,
    network_id: u16,
    build_commit: Option<String>
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InstanceSecurity {
    encryption: bool,
    tee: bool,
    hsm: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InstanceMonitoring {
    logging_enabled: bool,
    metrics_endpoint: String,
}
