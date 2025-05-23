// form-state/src/tasks.rs
// This module will define Task related structs and enums for Proof of Claim tasks.

use serde::{Serialize, Deserialize};
use std::collections::BTreeSet;
use chrono;
use crdts::{map::Op, BFTReg, merkle_reg::Sha3Hash, CmRDT};
use crate::Actor;
use k256::ecdsa::SigningKey;
use hex;
use sha2::{Sha256, Digest as Sha2Digest};
use crate::nodes::Node;
use bincode;
use tiny_keccak::{Hasher, Sha3};
use log;

pub type TaskId = String; // Should ideally be a uuid::Uuid type for guaranteed uniqueness.

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TaskStatus {
    PendingPoCAssessment, // Newly created, Proof of Claim assessment pending
    PoCAssigned,          // Proof of Claim complete, responsible_nodes populated, awaiting pickup by a responsible node
    Claimed,              // A responsible node has claimed it (optional intermediate state)
    InProgress,           // Actively being worked on by an assigned node
    Completed,            // Successfully finished
    Failed,               // Execution failed
    Cancelled,            // Cancelled by user/system before completion
}

impl std::str::FromStr for TaskStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "PendingPoCAssessment" => Ok(TaskStatus::PendingPoCAssessment),
            "PoCAssigned" => Ok(TaskStatus::PoCAssigned),
            "Claimed" => Ok(TaskStatus::Claimed),
            "InProgress" => Ok(TaskStatus::InProgress),
            "Completed" => Ok(TaskStatus::Completed),
            "Failed" => Ok(TaskStatus::Failed),
            "Cancelled" => Ok(TaskStatus::Cancelled),
            _ => Err(format!("Unknown task status string: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BuildImageParams { // Parameters to build an image artifact (e.g., a rootfs)
    pub output_artifact_name: String, // e.g., "my-app-rootfs"
    pub output_artifact_tag: String,  // e.g., "v1.2"
    
    pub source_type: String, // e.g., "docker_context", "git_repo_with_script"
    pub source_url: String, // URL to the source (git repo, tarball etc.)
    pub source_ref: Option<String>, // e.g., git branch, tag, commit hash

    pub build_script_path: Option<String>, // Path to a build script within the source_url context
    pub build_args: Option<std::collections::BTreeMap<String, String>>, // Args for the build script or Docker build

    // Information about where the built artifact should be stored/registered
    pub target_registry_url: Option<String>, 
    // The result of this task should ideally be an ID or path to the artifact 
    // and a generated/updated Formfile snippet describing this artifact.
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LaunchInstanceParams {
    pub instance_name: String, 
    // The complete Formfile content as a string (JSON or YAML, to be parsed by VMM service)
    // This Formfile will specify the actual image paths (rootfs, kernel), resources, etc.
    pub formfile_content: String, 
    
    // Optional runtime overrides if the VMM service supports them beyond Formfile specs
    pub runtime_env_vars: Option<std::collections::BTreeMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TaskVariant {
    BuildImage(BuildImageParams),
    LaunchInstance(LaunchInstanceParams),
    // We can add RunModelInference(RunModelInferenceParams) here later if needed
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Task {
    pub task_id: TaskId,
    pub task_variant: TaskVariant,
    pub status: TaskStatus,
    #[serde(default)]
    pub required_capabilities: Vec<String>, // e.g., ["build_image", "gpu_enabled"]
    #[serde(default = "default_target_redundancy")]
    pub target_redundancy: u8, // How many nodes should ideally pick this up (e.g., for HA)
    #[serde(default)]
    pub responsible_nodes: Option<BTreeSet<String>>, // Nodes determined by PoC
    #[serde(default)]
    pub assigned_to_node_id: Option<String>, // Node ID that has claimed/is running the task
    #[serde(default = "current_timestamp")]
    pub created_at: i64,
    #[serde(default = "current_timestamp")]
    pub updated_at: i64,
    pub submitted_by: String, // ActorId of the submitter
    #[serde(default)]
    pub result_info: Option<String>, // JSON string for success/failure details
    #[serde(default)]
    pub progress: Option<u8>, // 0-100
}

fn default_target_redundancy() -> u8 {
    1
}

fn current_timestamp() -> i64 {
    chrono::Utc::now().timestamp()
}

impl crdts::merkle_reg::Sha3Hash for Task {
    fn hash(&self, hasher: &mut tiny_keccak::Sha3) {
        // Serialize the entire Task struct using bincode for a canonical representation
        match bincode::serialize(self) {
            Ok(encoded) => {
                hasher.update(&encoded);
            }
            Err(e) => {
                // If serialization fails, this is a critical issue for CRDT integrity.
                // Panic here, or hash a known error sentinel and log verbosely.
                // Hashing only task_id would be a very weak fallback.
                log::error!("Failed to serialize Task for hashing: {}. Task ID: {}", e, self.task_id);
                hasher.update(self.task_id.as_bytes()); // Minimal fallback, but problematic
            }
        }
    }
}

pub type TaskOp = Op<TaskId, BFTReg<Task, Actor>, Actor>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaskState {
    // ID of the node that owns this instance of TaskState (for CRDT actor ID)
    // This might be less relevant if tasks are created by various actors (users/admins/mcp)
    // but good for consistency with other state objects if this node applies ops.
    node_id: String, 
    pk: String, // Private key of this node_id, for signing ops originated by this TaskState instance.
    
    pub map: crdts::Map<TaskId, BFTReg<Task, Actor>, Actor>,
}

impl TaskState {
    pub fn new(node_id: String, pk: String) -> Self {
        Self {
            node_id,
            pk,
            map: crdts::Map::new(),
        }
    }

    /// Update (or add) a task record locally. This creates a signed op
    /// that will be merged into the CRDT map.
    pub fn update_task_local(&mut self, task: Task) -> TaskOp {
        let add_ctx = self.map.read_ctx().derive_add_ctx(self.node_id.clone());
        let signing_key = SigningKey::from_slice(
            &hex::decode(self.pk.clone())
                .expect("TaskState: Invalid SigningKey Hex in pk field")
        ).expect("TaskState: Invalid SigningKey bytes in pk field");
        
        self.map.update(task.task_id.clone(), add_ctx, |reg, _ctx| {
            reg.update(task, self.node_id.clone(), signing_key)
                .expect("TaskState: Unable to sign task update")
        })
    }

    /// Remove a task record locally.
    pub fn remove_task_local(&mut self, task_id: TaskId) -> TaskOp {
        let rm_ctx = self.map.read_ctx().derive_rm_ctx();
        self.map.rm(task_id, rm_ctx)
    }

    /// Apply an operation received from a peer.
    pub fn task_op(&mut self, op: TaskOp) -> Option<(Actor, TaskId)> { // Changed return to match others like node_op
        self.map.apply(op.clone());
        match op {
            Op::Up { dot, key, op: _ } => Some((dot.actor, key)),
            Op::Rm { .. } => None,
        }
    }

    // Method to check if a task_op was successfully applied (useful for the caller like DataStore)
    pub fn task_op_success(&self, task_id: &TaskId, update_op: &crdts::bft_reg::Update<Task, Actor>) -> (bool, Option<Task>) {
        if let Some(reg) = self.map.get(task_id).val {
            if let Some(v) = reg.val() {
                if v.value().task_id == update_op.op().value.task_id { // Basic check
                    return (true, Some(v.value()));
                } else if reg.dag_contains(&update_op.hash()) && reg.is_head(&update_op.hash()) {
                    return (true, Some(v.value()));
                } else if reg.is_orphaned(&update_op.hash()) {
                    return (true, Some(v.value()));
                }
                return (false, Some(v.value()));
            }
        }
        (false, None)
    }

    /// Retrieve a task by its ID.
    pub fn get_task(&self, task_id: &TaskId) -> Option<Task> {
        self.map.get(task_id).val.and_then(|reg| reg.val().map(|v| v.value()))
    }

    /// List all tasks.
    pub fn list_tasks(&self) -> Vec<Task> {
        self.map.iter().filter_map(|entry| {
            let (_key, val_reg) = entry.val; // Destructure the tuple from IterEntry.val
            val_reg.val().map(|v_ctx| v_ctx.value())
        }).collect()
    }
}

// --- Proof of Claim Utilities ---

/// Calculates a Proof of Claim score by XORing the SHA256 hashes of task_id and node_id.
/// Returns the first 8 bytes of the result interpreted as u64.
pub fn calculate_poc_score(task_id: &str, node_id: &str) -> u64 {
    let mut task_hasher = Sha256::new();
    task_hasher.update(task_id.as_bytes());
    let task_hash = task_hasher.finalize();

    let mut node_hasher = Sha256::new();
    node_hasher.update(node_id.as_bytes());
    let node_hash = node_hasher.finalize();

    let mut xor_result_bytes = [0u8; 32];
    for i in 0..32 {
        xor_result_bytes[i] = task_hash[i] ^ node_hash[i];
    }

    // Take the first 8 bytes and convert to u64 (little-endian is a common choice)
    // Ensure there are at least 8 bytes from the hash result for safety, though SHA256 gives 32.
    if xor_result_bytes.len() >= 8 {
        let mut arr = [0u8; 8];
        arr.copy_from_slice(&xor_result_bytes[0..8]);
        u64::from_le_bytes(arr)
    } else {
        // This case should ideally not be reached if inputs are always SHA256 hashes
        // Returning 0 or u64::MAX could be fallback strategies for an unexpected hash length.
        // For now, panic might be better to indicate an issue with hash generation.
        panic!("XOR result of hashes is unexpectedly short."); 
    }
}

/// Determines the set of node_ids responsible for a task based on Proof of Claim.
pub fn determine_responsible_nodes(
    task: &Task, 
    all_nodes: &[Node], 
    _datastore: &crate::datastore::DataStore, // Marked as unused for now if all_nodes is sufficient
) -> BTreeSet<String> {
    
    // 1. Filter nodes by required capabilities
    let capable_nodes: Vec<&Node> = all_nodes.iter().filter(|node| {
        // Check against node.metadata.annotations.roles()
        task.required_capabilities.iter().all(|cap| node.metadata.annotations().roles().contains(cap))
    }).collect();

    if capable_nodes.is_empty() {
        return BTreeSet::new(); // No capable nodes found
    }

    // 2. Calculate PoC score for each capable node
    let mut scored_nodes: Vec<(u64, &str)> = capable_nodes.iter().map(|node| {
        (calculate_poc_score(&task.task_id, &node.node_id), node.node_id.as_str())
    }).collect();

    // 3. Sort nodes by score (lowest first)
    scored_nodes.sort_by_key(|(score, _)| *score);

    // 4. Select the top `task.target_redundancy` nodes
    let responsible_node_ids: BTreeSet<String> = scored_nodes.iter()
        .take(task.target_redundancy as usize)
        .map(|(_, node_id_str)| node_id_str.to_string())
        .collect();

    responsible_node_ids
}

// Further definitions will go here based on subsequent sub-tasks. 