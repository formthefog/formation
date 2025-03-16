// form-fuzzing/src/generators/state.rs
//! Generators for Form State components based on the actual form-state crate

use crate::generators::Generator;
use std::collections::{HashMap, HashSet};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use rand::{Rng, thread_rng, seq::SliceRandom};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::time::{SystemTime, UNIX_EPOCH};
use crdts::{BFTReg, Map, CvRDT};
use crate::harness::state::{
    Instance, InstanceStatus, Resources, GpuResources, Node, NodeStatus,
    Account, CrdtPeer, CrdtCidr, CrdtAssociation, FormDnsRecord, CrdtDnsRecord
};

/// Generate a random node ID
fn generate_node_id() -> String {
    format!("node-{}", Uuid::new_v4())
}

/// Generate a random instance ID
fn generate_instance_id() -> String {
    format!("i-{}", Uuid::new_v4().simple())
}

/// Generate a random account ID
fn generate_account_id() -> String {
    format!("account-{}", Uuid::new_v4().simple())
}

/// Generate a random IPv4 address
fn generate_ipv4() -> IpAddr {
    let mut rng = thread_rng();
    IpAddr::V4(Ipv4Addr::new(
        rng.gen_range(1..255),
        rng.gen_range(0..255),
        rng.gen_range(0..255),
        rng.gen_range(1..255),
    ))
}

/// Generate a random domain name
fn generate_domain() -> String {
    let prefixes = ["app", "api", "dev", "test", "prod", "staging", "demo", "beta"];
    let domains = ["formation.network", "form-net.com", "formnet.io", "formation.dev", "form-cluster.net"];
    
    let mut rng = thread_rng();
    format!("{}.{}", 
        prefixes.choose(&mut rng).unwrap(),
        domains.choose(&mut rng).unwrap()
    )
}

/// Instance Status generator
pub struct InstanceStatusGenerator;

impl InstanceStatusGenerator {
    pub fn new() -> Self {
        Self {}
    }
}

// Since we're using the harness types and custom generator types separately,
// we'll define our own enum for our internal generator uses

/// Generator's own instance status enum
#[derive(Debug, Clone)]
pub enum GenInstanceStatus {
    Running,
    Stopped,
    Starting,
    Stopping,
    Failed,
    Creating,
    Deleting,
}

impl Generator<GenInstanceStatus> for InstanceStatusGenerator {
    fn generate(&self) -> GenInstanceStatus {
        let mut rng = thread_rng();
        match rng.gen_range(0..=5) {
            0 => GenInstanceStatus::Running,
            1 => GenInstanceStatus::Stopped,
            2 => GenInstanceStatus::Starting,
            3 => GenInstanceStatus::Stopping,
            4 => GenInstanceStatus::Failed,
            5 => GenInstanceStatus::Creating,
            _ => GenInstanceStatus::Deleting,
        }
    }
}

impl Generator<InstanceStatus> for InstanceStatusGenerator {
    fn generate(&self) -> InstanceStatus {
        let mut rng = thread_rng();
        match rng.gen_range(0..=4) {
            0 => InstanceStatus::Running,
            1 => InstanceStatus::Stopped,
            2 => InstanceStatus::Failed,
            3 => InstanceStatus::Pending,
            _ => InstanceStatus::Unknown,
        }
    }
}

/// Instance generator
pub struct InstanceGenerator {
    status_generator: InstanceStatusGenerator,
    resources_generator: ResourcesGenerator,
}

impl InstanceGenerator {
    pub fn new() -> Self {
        Self {
            status_generator: InstanceStatusGenerator::new(),
            resources_generator: ResourcesGenerator::new(),
        }
    }
}

impl Generator<Instance> for InstanceGenerator {
    fn generate(&self) -> Instance {
        let mut rng = thread_rng();
        
        let status = self.status_generator.generate();
        let resources = self.resources_generator.generate();
        
        Instance {
            id: generate_instance_id(),
            name: format!("instance-{}", rng.gen::<u16>()),
            owner: format!("user-{}", rng.gen::<u16>()),
            status,
            resources,
            ip: format!("{}.{}.{}.{}", 
                rng.gen_range(1..255), 
                rng.gen_range(0..255), 
                rng.gen_range(0..255), 
                rng.gen_range(1..255)),
            domain: generate_domain(),
            tags: (0..rng.gen_range(0..5))
                .map(|_| format!("tag-{}", rng.gen::<u8>()))
                .collect(),
            authorized_users: (0..rng.gen_range(0..3))
                .map(|_| format!("user-{}", rng.gen::<u16>()))
                .collect(),
        }
    }
}

/// Node status generator
pub struct NodeStatusGenerator;

impl NodeStatusGenerator {
    pub fn new() -> Self {
        Self {}
    }
}

/// Generator's own node status enum
#[derive(Debug, Clone)]
pub enum GenNodeStatus {
    Online,
    Offline,
    Maintenance,
    Provisioning,
    Unknown,
}

impl Generator<GenNodeStatus> for NodeStatusGenerator {
    fn generate(&self) -> GenNodeStatus {
        let mut rng = thread_rng();
        match rng.gen_range(0..5) {
            0 => GenNodeStatus::Online,
            1 => GenNodeStatus::Offline,
            2 => GenNodeStatus::Maintenance,
            3 => GenNodeStatus::Provisioning,
            _ => GenNodeStatus::Unknown,
        }
    }
}

impl Generator<NodeStatus> for NodeStatusGenerator {
    fn generate(&self) -> NodeStatus {
        let mut rng = thread_rng();
        match rng.gen_range(0..5) {
            0 => NodeStatus::Online,
            1 => NodeStatus::Offline,
            2 => NodeStatus::Maintenance,
            3 => NodeStatus::Provisioning,
            _ => NodeStatus::Unknown,
        }
    }
}

/// Node generator
pub struct NodeGenerator {
    resources_generator: ResourcesGenerator,
    status_generator: NodeStatusGenerator,
}

impl NodeGenerator {
    pub fn new() -> Self {
        Self {
            resources_generator: ResourcesGenerator::new(),
            status_generator: NodeStatusGenerator::new(),
        }
    }
}

impl Generator<Node> for NodeGenerator {
    fn generate(&self) -> Node {
        let mut rng = thread_rng();
        
        let status = self.status_generator.generate();
        let resources_available = self.resources_generator.generate();
        let resources_total = self.resources_generator.generate();
        
        Node {
            id: generate_node_id(),
            name: format!("node-{}", rng.gen::<u16>()),
            public_key: format!("ssh-rsa AAAAB3NzaC1yc2E{}", rng.gen::<u32>()),
            region: match rng.gen_range(0..4) {
                0 => "us-east".to_string(),
                1 => "us-west".to_string(),
                2 => "eu-central".to_string(),
                _ => "ap-southeast".to_string(),
            },
            resources_available,
            resources_total,
            ip_addresses: (0..rng.gen_range(1..3))
                .map(|_| format!("{}.{}.{}.{}", 
                    rng.gen_range(1..255), 
                    rng.gen_range(0..255), 
                    rng.gen_range(0..255), 
                    rng.gen_range(1..255)))
                .collect(),
            status,
        }
    }
}

/// Account generator
pub struct AccountGenerator;

impl AccountGenerator {
    pub fn new() -> Self {
        Self {}
    }
}

impl Generator<Account> for AccountGenerator {
    fn generate(&self) -> Account {
        let mut rng = thread_rng();
        
        Account {
            id: generate_account_id(),
            username: format!("user{}", rng.gen::<u16>()),
            email: format!("user{}@example.com", rng.gen::<u16>()),
            public_key: format!("ssh-rsa AAAAB3NzaC1yc2E{}", rng.gen::<u32>()),
            credit_balance: rng.gen_range(0.0..10000.0),
            instances: (0..rng.gen_range(0..5))
                .map(|_| generate_instance_id())
                .collect(),
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }
}

/// Network Peer generator
pub struct PeerGenerator;

impl PeerGenerator {
    pub fn new() -> Self {
        Self {}
    }
}

impl Generator<CrdtPeer> for PeerGenerator {
    fn generate(&self) -> CrdtPeer {
        let mut rng = thread_rng();
        
        CrdtPeer {
            id: Uuid::new_v4().to_string(),
            name: format!("peer-{}", rng.gen::<u16>()),
            public_key: format!("ssh-rsa AAAAB3NzaC1yc2E{}", rng.gen::<u32>()),
            endpoint: format!("{}:{}", 
                format!("{}.{}.{}.{}", 
                    rng.gen_range(1..255), 
                    rng.gen_range(0..255), 
                    rng.gen_range(0..255), 
                    rng.gen_range(1..255)
                ),
                rng.gen_range(1024..65535)
            ),
            last_seen: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                - rng.gen_range(0..86400), // Up to a day ago
        }
    }
}

/// CIDR generator
pub struct CidrGenerator;

impl CidrGenerator {
    pub fn new() -> Self {
        Self {}
    }
}

impl Generator<CrdtCidr> for CidrGenerator {
    fn generate(&self) -> CrdtCidr {
        let mut rng = thread_rng();
        
        let first_octet = rng.gen_range(10..192);
        let second_octet = rng.gen_range(0..255);
        let prefix_length = rng.gen_range(16..30);
        let cidr = format!("{}.{}.0.0/{}", first_octet, second_octet, prefix_length);
        
        let owner = format!("user-{}", rng.gen::<u16>());
        let assigned_to = if rng.gen_bool(0.5) {
            Some(format!("instance-{}", rng.gen::<u16>()))
        } else {
            None
        };
        
        CrdtCidr {
            cidr,
            owner,
            assigned_to,
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                - rng.gen_range(0..31536000), // Up to a year ago
        }
    }
}

/// Association generator
pub struct AssociationGenerator;

impl AssociationGenerator {
    pub fn new() -> Self {
        Self {}
    }
}

impl Generator<CrdtAssociation> for AssociationGenerator {
    fn generate(&self) -> CrdtAssociation {
        let mut rng = thread_rng();
        
        CrdtAssociation {
            id: Uuid::new_v4().to_string(),
            peer_id: Uuid::new_v4().to_string(),
            cidr_id: Uuid::new_v4().to_string(),
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                - rng.gen_range(0..31536000), // Up to a year ago
        }
    }
}

/// DNS Record generator
pub struct DnsRecordGenerator;

impl DnsRecordGenerator {
    pub fn new() -> Self {
        Self {}
    }
}

impl Generator<FormDnsRecord> for DnsRecordGenerator {
    fn generate(&self) -> FormDnsRecord {
        let mut rng = thread_rng();
        
        let domain = generate_domain();
        let record_type = match rng.gen_range(0..3) {
            0 => "A".to_string(),
            1 => "CNAME".to_string(),
            _ => "TXT".to_string(),
        };
        
        let value = match record_type.as_str() {
            "A" => format!("{}.{}.{}.{}", 
                rng.gen_range(1..255), 
                rng.gen_range(0..255), 
                rng.gen_range(0..255), 
                rng.gen_range(1..255)),
            "CNAME" => generate_domain(),
            _ => format!("v=spf1 ip4:{}.{}.0.0/16 ~all", 
                rng.gen_range(10..192), 
                rng.gen_range(0..255)),
        };
        
        FormDnsRecord {
            domain,
            record_type,
            value,
            ttl: rng.gen_range(60..86400),
        }
    }
}

/// DNS CRDT Record generator
pub struct CRDTDnsRecordGenerator {
    form_dns_record_generator: DnsRecordGenerator,
}

impl CRDTDnsRecordGenerator {
    pub fn new() -> Self {
        Self {
            form_dns_record_generator: DnsRecordGenerator::new(),
        }
    }
}

impl Generator<CrdtDnsRecord> for CRDTDnsRecordGenerator {
    fn generate(&self) -> CrdtDnsRecord {
        let mut rng = thread_rng();
        let form_record = self.form_dns_record_generator.generate();
        
        CrdtDnsRecord {
            id: Uuid::new_v4().to_string(),
            domain: form_record.domain,
            record_type: form_record.record_type,
            value: form_record.value,
            ttl: form_record.ttl,
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                - rng.gen_range(0..31536000), // Up to a year ago
        }
    }
}

/// Resources generator  
pub struct ResourcesGenerator;

impl ResourcesGenerator {
    pub fn new() -> Self {
        Self {}
    }
}

impl Generator<Resources> for ResourcesGenerator {
    fn generate(&self) -> Resources {
        let mut rng = thread_rng();
        let gpu = if rng.gen_bool(0.3) {
            Some(GpuResources {
                count: rng.gen_range(1..8),
                model: ["nvidia-a100", "nvidia-t4", "nvidia-a10", "amd-mi100"]
                    .choose(&mut rng)
                    .unwrap()
                    .to_string(),
            })
        } else {
            None
        };
        
        Resources {
            memory: rng.gen_range(1024..262144),
            vcpus: rng.gen_range(1..64),
            disk_gb: rng.gen_range(10..2000),
            gpu,
        }
    }
} 