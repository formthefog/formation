#[derive(Debug, Clone)]
pub struct Instance {
    pub id: String,
    pub name: String,
    pub owner: String,
    pub status: InstanceStatus,
    pub resources: Resources,
    pub ip: String,
    pub domain: String,
    pub tags: Vec<String>,
    pub authorized_users: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum InstanceStatus {
    Running,
    Stopped,
    Failed,
    Pending,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct Resources {
    pub memory: u64,
    pub vcpus: u32,
    pub disk_gb: u32,
    pub gpu: Option<GpuResources>,
}

#[derive(Debug, Clone)]
pub struct GpuResources {
    pub count: u32,
    pub model: String,
}

#[derive(Debug, Clone)]
pub struct Node {
    pub id: String,
    pub name: String,
    pub public_key: String,
    pub region: String,
    pub resources_available: Resources,
    pub resources_total: Resources,
    pub ip_addresses: Vec<String>,
    pub status: NodeStatus,
}

#[derive(Debug, Clone)]
pub enum NodeStatus {
    Online,
    Offline,
    Maintenance,
    Provisioning,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct Account {
    pub id: String,
    pub username: String,
    pub email: String,
    pub public_key: String,
    pub credit_balance: f64,
    pub instances: Vec<String>,
    pub created_at: u64,
}

#[derive(Debug, Clone)]
pub struct CrdtPeer {
    pub id: String,
    pub name: String,
    pub public_key: String,
    pub endpoint: String,
    pub last_seen: u64,
}

#[derive(Debug, Clone)]
pub struct CrdtCidr {
    pub cidr: String,
    pub owner: String,
    pub assigned_to: Option<String>,
    pub created_at: u64,
}

#[derive(Debug, Clone)]
pub struct CrdtAssociation {
    pub id: String,
    pub peer_id: String,
    pub cidr_id: String,
    pub created_at: u64,
}

#[derive(Debug, Clone)]
pub struct FormDnsRecord {
    pub domain: String,
    pub record_type: String,
    pub value: String,
    pub ttl: u32,
}

#[derive(Debug, Clone)]
pub struct CrdtDnsRecord {
    pub id: String,
    pub domain: String,
    pub record_type: String,
    pub value: String,
    pub ttl: u32,
    pub created_at: u64,
}

pub struct StateFuzzHarness {
    // You would typically have state storage and operation tracking here
}

impl StateFuzzHarness {
    pub fn new() -> Self {
        StateFuzzHarness {}
    }

    pub fn fuzz_instance(&self, instance: &Instance) {
        // In a real implementation, you would:
        // 1. Apply operations to the instance
        // 2. Verify state consistency
        // 3. Check for crashes or unexpected behavior
        println!("Fuzzing instance: {:?}", instance);
    }

    pub fn fuzz_node(&self, node: &Node) {
        println!("Fuzzing node: {:?}", node);
    }

    pub fn fuzz_account(&self, account: &Account) {
        println!("Fuzzing account: {:?}", account);
    }

    pub fn fuzz_crdt_peer(&self, peer: &CrdtPeer) {
        println!("Fuzzing CRDT peer: {:?}", peer);
    }

    pub fn fuzz_crdt_cidr(&self, cidr: &CrdtCidr) {
        println!("Fuzzing CRDT CIDR: {:?}", cidr);
    }

    pub fn fuzz_crdt_association(&self, association: &CrdtAssociation) {
        println!("Fuzzing CRDT association: {:?}", association);
    }

    pub fn fuzz_form_dns_record(&self, record: &FormDnsRecord) {
        println!("Fuzzing Form DNS record: {:?}", record);
    }

    pub fn fuzz_crdt_dns_record(&self, record: &CrdtDnsRecord) {
        println!("Fuzzing CRDT DNS record: {:?}", record);
    }
} 