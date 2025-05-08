use std::time::{SystemTime, UNIX_EPOCH};
use std::collections::BTreeMap;
use uuid::Uuid;
use crate::formfile::Formfile;
use form_state::instances::{Instance, InstanceResources, InstanceStatus};
use form_state::agent::{AIAgent, AgentResourceRequirements};

pub fn build_instance_id(node_id: String, build_id: String) -> Result<String, Box<dyn std::error::Error + Send + Sync + 'static>> {
    println!("Deriving instance id from node_id: {node_id} and build_id: {build_id}");
    let node_id_vec = &hex::decode(node_id)?[..20];
    let vm_name_bytes = &hex::decode(build_id.clone())?[..20];

    let instance_id = hex::encode(&vm_name_bytes.iter().zip(node_id_vec.iter()).map(|(&x, &y)| x ^ y).collect::<Vec<u8>>());

    Ok(instance_id)
}

pub(crate) fn is_gzip(data: &[u8]) -> bool {
    data.starts_with(&[0x1F, 0x8B]) // Gzip magic number
}

pub(crate) fn get_host_bridge_ip() -> Result<String, Box<dyn std::error::Error + Send + Sync +'static>> {
    let addrs = get_if_addrs::get_if_addrs()?;
    let pub_addr: Vec<_> = addrs.iter().filter_map(|iface| {
        if iface.name == "br0" {
            match &iface.addr {
                get_if_addrs::IfAddr::V4(ifv4) => {
                    Some(ifv4.ip.to_string())
                }
                _ => None
            }
        } else {
            None
        }
    }).collect::<Vec<_>>();

    let first = pub_addr.first()
        .ok_or(
            Box::new(
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Unable to find IP for br0, host is not set up to host instances"
                )
            )
        )?; 

    Ok(first.to_string())
}

pub fn create_new_instance_entry(
    instance_id: String,
    node_id: String,
    build_id: String,
    signer_address: String,
    formfile: Formfile
) -> Result<Instance, Box<dyn std::error::Error + Send + Sync>> {
    Ok(Instance {
        instance_id: instance_id.clone(),
        node_id: node_id.clone(),
        build_id: hex::encode(build_id),
        instance_owner: signer_address.clone(),
        updated_at: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64,
        formfile: serde_json::to_string(&formfile)?,
        snapshots: None,
        resources: InstanceResources {
            vcpus: formfile.get_vcpus(),
            memory_mb: formfile.get_memory() as u32,
            bandwidth_mbps: 1000,
            gpu: None,
        },
        ..Default::default()
    })
}

pub fn create_new_agent_entry(
    formfile: Formfile,
    build_id: String,
    signer_address: String
) -> Result<AIAgent, Box<dyn std::error::Error + Send + Sync>> {
    let mut metadata = BTreeMap::new();
    metadata.insert("build_id".to_string(), hex::encode(&build_id));
    
    Ok(AIAgent {
        agent_id: build_id,
        owner_id: signer_address,
        name: formfile.name.clone(),
        description: formfile.get_description().unwrap_or("").to_string(),
        requires_specific_model: formfile.is_model_required(),
        required_model_id: formfile.get_model_id().map(|s| s.to_string()),
        formfile_template: base64::encode(serde_json::to_string(&formfile)?),
        resource_requirements: AgentResourceRequirements {
            min_vcpus: formfile.get_vcpus(),
            recommended_vcpus: formfile.get_vcpus(),
            min_memory_mb: formfile.get_memory() as u64,
            recommended_memory_mb: formfile.get_memory() as u64,
            min_disk_gb: formfile.get_storage().unwrap_or(5) as u64,
            recommended_disk_gb: formfile.get_storage().unwrap_or(5) as u64,
            requires_gpu: formfile.get_gpu_devices().is_some(),
        },
        metadata,
        ..Default::default()
    })
}
