use crate::formfile::Formfile;
use form_state::nodes::Node;
use form_types::state::{Response as StateResponse, Success};
use reqwest::Client;
use std::collections::{HashMap, BTreeMap};
use std::error::Error;
use log::{info, warn, error, debug};

/// A utility for matching workload requirements against node capabilities and capacity
/// and determining which node is responsible for a workload
pub struct CapabilityMatcher {
    form_state_url: String,
    http_client: Client,
}

impl CapabilityMatcher {
    /// Create a new CapabilityMatcher with the given form-state URL
    pub fn new(form_state_url: Option<String>) -> Self {
        let form_state_url = form_state_url.unwrap_or_else(|| 
            std::env::var("FORM_STATE_URL").unwrap_or_else(|_| "http://127.0.0.1:63210".to_string())
        );
        
        Self {
            form_state_url,
            http_client: Client::new(),
        }
    }

    /// Check if the local node has the capability and capacity to handle the workload
    pub async fn is_local_node_capable(&self, formfile: &Formfile, node_id: &str) -> Result<bool, Box<dyn Error + Send + Sync>> {
        let node = self.get_node(node_id).await?;
        
        match node {
            Some(node) => {
                let (is_capable, reason) = self.check_node_capability(&node, formfile);
                if !is_capable {
                    info!("Local node is not capable of handling workload: {}", reason);
                }
                Ok(is_capable)
            },
            None => {
                warn!("Could not find local node with ID {} in form-state", node_id);
                Ok(false)
            }
        }
    }

    /// Check if the local node is responsible for handling this workload
    /// - First filters nodes based on capability/capacity
    /// - Then determines the responsible node using XOR of build_id and node_id
    pub async fn is_local_node_responsible(
        &self, 
        formfile: &Formfile, 
        local_node_id: &str, 
        build_id: &str
    ) -> Result<bool, Box<dyn Error + Send + Sync>> {
        // First check if the local node is capable
        if !self.is_local_node_capable(formfile, local_node_id).await? {
            return Ok(false);
        }
        
        // Get all capable nodes
        let capable_nodes = self.get_capable_nodes(formfile).await?;
        
        if capable_nodes.is_empty() {
            warn!("No nodes capable of handling this workload");
            return Ok(false);
        }
        
        let responsible_nodes = self.select_responsible_nodes(capable_nodes, build_id, 1);
        if responsible_nodes.is_empty() {
            warn!("No responsible nodes determined for build {}", build_id);
            return Ok(false);
        }
        
        let is_responsible = responsible_nodes[0].node_id == local_node_id;
        if is_responsible {
            info!("Local node {} is responsible for build {}", local_node_id, build_id);
        } else {
            info!("Local node {} is NOT responsible for build {}", local_node_id, build_id);
            info!("Responsible node is: {}", responsible_nodes[0].node_id);
        }
        
        Ok(is_responsible)
    }
    
    /// Determine if the local node is part of a cluster for this workload
    /// For clustering (future implementation), returns true if the node is one of the N lowest XOR values
    pub async fn is_local_node_in_cluster(
        &self, 
        formfile: &Formfile, 
        local_node_id: &str, 
        build_id: &str,
        cluster_size: usize
    ) -> Result<bool, Box<dyn Error + Send + Sync>> {
        // First check if the local node is capable
        if !self.is_local_node_capable(formfile, local_node_id).await? {
            return Ok(false);
        }
        
        // Get all capable nodes
        let capable_nodes = self.get_capable_nodes(formfile).await?;
        
        if capable_nodes.is_empty() {
            warn!("No nodes capable of handling this workload");
            return Ok(false);
        }
        
        // If we have fewer nodes than the requested cluster size, all capable nodes form the cluster
        let effective_cluster_size = if capable_nodes.len() < cluster_size {
            capable_nodes.len()
        } else {
            cluster_size
        };
        
        let responsible_nodes = self.select_responsible_nodes(capable_nodes, build_id, effective_cluster_size);
        if responsible_nodes.is_empty() {
            warn!("No responsible nodes determined for build {}", build_id);
            return Ok(false);
        }
        
        // Check if the local node is in the cluster
        let is_in_cluster = responsible_nodes.iter().any(|node| node.node_id == local_node_id);
        
        if is_in_cluster {
            info!("Local node {} is part of the cluster for build {}", local_node_id, build_id);
        } else {
            info!("Local node {} is NOT part of the cluster for build {}", local_node_id, build_id);
            let cluster_nodes: Vec<String> = responsible_nodes.iter().map(|n| n.node_id.clone()).collect();
            info!("Cluster nodes are: {:?}", cluster_nodes);
        }
        
        Ok(is_in_cluster)
    }
    
    /// Select the responsible nodes for a workload using XOR of build_id and node_id
    /// Returns the top N nodes with the lowest XOR values
    fn select_responsible_nodes(&self, nodes: Vec<Node>, build_id: &str, count: usize) -> Vec<Node> {
        if nodes.is_empty() || count == 0 {
            return Vec::new();
        }
        
        // Calculate XOR values for each node
        let mut node_xor_values: BTreeMap<u64, Node> = BTreeMap::new();
        
        for node in nodes {
            let xor_value = self.calculate_xor_value(&node.node_id, build_id);
            node_xor_values.insert(xor_value, node);
        }
        
        // Return the N nodes with the lowest XOR values
        node_xor_values.into_iter()
            .take(count)
            .map(|(_, node)| node)
            .collect()
    }
    
    /// Calculate the XOR value between a node_id and build_id
    /// This is used to deterministically select the responsible node
    fn calculate_xor_value(&self, node_id: &str, build_id: &str) -> u64 {
        // Convert the node_id and build_id to u64 values that can be XORed
        let node_hash = self.hash_string(node_id);
        let build_hash = self.hash_string(build_id);
        
        // XOR the hash values
        node_hash ^ build_hash
    }
    
    /// Simple hash function to convert a string to a u64
    fn hash_string(&self, s: &str) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        s.hash(&mut hasher);
        hasher.finish()
    }

    /// Get a list of all nodes from form-state that are capable of handling the workload
    pub async fn get_capable_nodes(&self, formfile: &Formfile) -> Result<Vec<Node>, Box<dyn Error + Send + Sync>> {
        let nodes = self.get_all_nodes().await?;
        
        let capable_nodes: Vec<Node> = nodes.into_iter()
            .filter(|node| {
                let (is_capable, reason) = self.check_node_capability(node, formfile);
                if !is_capable {
                    debug!("Node {} is not capable: {}", node.node_id, reason);
                }
                is_capable
            })
            .collect();
            
        info!("Found {} nodes capable of handling workload", capable_nodes.len());
        Ok(capable_nodes)
    }

    /// Check if a node can handle the workload defined in the formfile
    /// Returns (is_capable, reason) where reason is a string explaining why the node is not capable (if applicable)
    fn check_node_capability(&self, node: &Node, formfile: &Formfile) -> (bool, String) {
        // Check CPU requirements
        let vcpus = formfile.get_vcpus() as usize;
        if node.capabilities.cpu_cores < vcpus {
            return (false, format!("Node has {} CPU cores, but workload requires {}", 
                node.capabilities.cpu_cores, vcpus));
        }
        
        // Check if the node has enough available CPU capacity
        if ((node.capacity.cpu_available_cores / 1000) as usize) < vcpus {
            return (false, format!("Node only has {} available CPU cores, but workload requires {}", 
                node.capacity.cpu_available_cores / 1000, vcpus));
        }
        
        // Check memory requirements
        let memory_mb = formfile.get_memory();
        let memory_bytes = memory_mb as u64 * 1024 * 1024; // Convert MB to bytes
        if node.capacity.memory_available_bytes < memory_bytes {
            return (false, format!("Node only has {} MB available memory, but workload requires {} MB",
                node.capacity.memory_available_bytes / (1024 * 1024), memory_mb));
        }
        
        // Check storage requirements
        if let Some(storage_gb) = formfile.get_storage() {
            let storage_bytes = storage_gb as u64 * 1024 * 1024 * 1024; // Convert GB to bytes
            if node.capacity.storage_available_bytes < storage_bytes {
                return (false, format!("Node only has {} GB available storage, but workload requires {} GB",
                    node.capacity.storage_available_bytes / (1024 * 1024 * 1024), storage_gb));
            }
        }
        
        // Check GPU requirements
        if let Some(gpu_devices) = formfile.get_gpu_devices() {
            if gpu_devices.is_empty() {
                // No GPU required
                return (true, String::new());
            }
            
            if node.capabilities.gpu_models.is_empty() {
                return (false, "Workload requires GPU but node has no GPUs".to_string());
            }
            
            // Parse GPU requirements from the formfile
            let mut required_gpus: HashMap<String, u8> = HashMap::new();
            for gpu_req in gpu_devices {
                let parts: Vec<&str> = gpu_req.split(':').collect();
                let model = parts[0].to_string();
                let count: u8 = if parts.len() > 1 {
                    parts[1].parse().unwrap_or(1)
                } else {
                    1
                };
                
                *required_gpus.entry(model).or_insert(0) += count;
            }
            
            // Check if the node has the required GPUs
            let mut available_gpus: HashMap<String, u8> = HashMap::new();
            for gpu in &node.capabilities.gpu_models {
                if let Some(model) = &gpu.model {
                    *available_gpus.entry(model.clone()).or_insert(0) += gpu.count as u8;
                }
            }
            
            for (model, count) in required_gpus {
                match available_gpus.get(&model) {
                    Some(available_count) if *available_count >= count => {
                        // GPU requirement satisfied
                    },
                    Some(available_count) => {
                        return (false, format!("Node has {} of GPU {}, but workload requires {}", 
                            available_count, model, count));
                    },
                    None => {
                        return (false, format!("Node has no GPU of model {}", model));
                    }
                }
            }
        }
        
        // All requirements are satisfied
        (true, String::new())
    }

    /// Get a specific node from form-state
    async fn get_node(&self, node_id: &str) -> Result<Option<Node>, Box<dyn Error + Send + Sync>> {
        let url = format!("{}/nodes/{}", self.form_state_url, node_id);
        
        match self.http_client.get(&url).send().await {
            Ok(response) => {
                match response.json::<StateResponse<Node>>().await {
                    Ok(StateResponse::Success(Success::Some(node))) => {
                        Ok(Some(node))
                    },
                    Ok(_) => {
                        warn!("Unexpected response format when getting node {}", node_id);
                        Ok(None)
                    },
                    Err(e) => {
                        error!("Error deserializing response for node {}: {}", node_id, e);
                        Err(Box::new(e))
                    }
                }
            },
            Err(e) => {
                error!("Error fetching node {}: {}", node_id, e);
                Err(Box::new(e))
            }
        }
    }

    /// Get all nodes from form-state
    async fn get_all_nodes(&self) -> Result<Vec<Node>, Box<dyn Error + Send + Sync>> {
        let url = format!("{}/nodes/list", self.form_state_url);
        
        match self.http_client.get(&url).send().await {
            Ok(response) => {
                match response.json::<StateResponse<Node>>().await {
                    Ok(StateResponse::Success(Success::List(nodes))) => {
                        Ok(nodes)
                    },
                    Ok(_) => {
                        warn!("Unexpected response format when listing nodes");
                        Ok(Vec::new())
                    },
                    Err(e) => {
                        error!("Error deserializing node list response: {}", e);
                        Err(Box::new(e))
                    }
                }
            },
            Err(e) => {
                error!("Error fetching node list: {}", e);
                Err(Box::new(e))
            }
        }
    }
} 