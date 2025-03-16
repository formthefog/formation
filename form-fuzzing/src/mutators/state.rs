// form-fuzzing/src/mutators/state.rs
//! Mutators for Form State components based on the actual form-state crate

use crate::harness::state::{
    Instance, InstanceStatus, Resources, GpuResources, Node, NodeStatus,
    Account, CrdtPeer, CrdtCidr, CrdtAssociation, FormDnsRecord, CrdtDnsRecord
};
use crate::mutators::Mutator;
use rand::{thread_rng, Rng, seq::SliceRandom};
use uuid::Uuid;

/// Instance status mutator
pub struct InstanceStatusMutator;

impl InstanceStatusMutator {
    pub fn new() -> Self {
        Self {}
    }
}

impl Mutator<InstanceStatus> for InstanceStatusMutator {
    fn mutate(&self, status: &mut InstanceStatus) {
        let mut rng = thread_rng();
        *status = match rng.gen_range(0..5) {
            0 => InstanceStatus::Running,
            1 => InstanceStatus::Stopped,
            2 => InstanceStatus::Failed,
            3 => InstanceStatus::Pending,
            _ => InstanceStatus::Unknown,
        };
    }
}

/// Resources mutator
pub struct ResourcesMutator;

impl ResourcesMutator {
    pub fn new() -> Self {
        Self {}
    }
}

impl Mutator<Resources> for ResourcesMutator {
    fn mutate(&self, resources: &mut Resources) {
        let mut rng = thread_rng();
        
        // Mutate memory, vcpus, and disk
        if rng.gen_bool(0.7) {
            resources.memory = rng.gen_range(1024..262144);
        }
        
        if rng.gen_bool(0.7) {
            resources.vcpus = rng.gen_range(1..64);
        }
        
        if rng.gen_bool(0.7) {
            resources.disk_gb = rng.gen_range(10..2000);
        }
        
        // Mutate GPU
        if rng.gen_bool(0.5) {
            if resources.gpu.is_some() && rng.gen_bool(0.3) {
                // Sometimes remove GPU
                resources.gpu = None;
            } else {
                // Add or modify GPU
                let gpu = GpuResources {
                    count: rng.gen_range(1..8),
                    model: ["nvidia-a100", "nvidia-t4", "nvidia-a10", "amd-mi100"]
                        .choose(&mut rng)
                        .unwrap()
                        .to_string(),
                };
                resources.gpu = Some(gpu);
            }
        }
    }
}

/// Instance mutator
pub struct InstanceMutator {
    status_mutator: InstanceStatusMutator,
    resources_mutator: ResourcesMutator,
}

impl InstanceMutator {
    pub fn new() -> Self {
        Self {
            status_mutator: InstanceStatusMutator::new(),
            resources_mutator: ResourcesMutator::new(),
        }
    }
}

impl Mutator<Instance> for InstanceMutator {
    fn mutate(&self, instance: &mut Instance) {
        let mut rng = thread_rng();
        
        // Mutate ID (10% chance)
        if rng.gen_bool(0.1) {
            instance.id = Uuid::new_v4().to_string();
        }
        
        // Mutate name (30% chance)
        if rng.gen_bool(0.3) {
            instance.name = format!("mutated-instance-{}", rng.gen::<u16>());
        }
        
        // Mutate owner (20% chance)
        if rng.gen_bool(0.2) {
            instance.owner = format!("mutated-user-{}", rng.gen::<u16>());
        }
        
        // Mutate status (50% chance)
        if rng.gen_bool(0.5) {
            self.status_mutator.mutate(&mut instance.status);
        }
        
        // Mutate resources (60% chance)
        if rng.gen_bool(0.6) {
            self.resources_mutator.mutate(&mut instance.resources);
        }
        
        // Mutate IP (40% chance)
        if rng.gen_bool(0.4) {
            instance.ip = format!("{}.{}.{}.{}", 
                rng.gen_range(1..255), 
                rng.gen_range(0..255), 
                rng.gen_range(0..255), 
                rng.gen_range(1..255)
            );
        }
        
        // Mutate domain (30% chance)
        if rng.gen_bool(0.3) {
            instance.domain = format!("{}.formtest.local", instance.name);
        }
        
        // Mutate tags (40% chance)
        if rng.gen_bool(0.4) {
            match rng.gen_range(0..3) {
                0 if !instance.tags.is_empty() => {
                    // Remove a random tag
                    let idx = rng.gen_range(0..instance.tags.len());
                    instance.tags.remove(idx);
                },
                1 => {
                    // Add a new tag
                    instance.tags.push(format!("tag-{}", rng.gen::<u16>()));
                },
                _ => {
                    // Replace all tags
                    let tag_count = rng.gen_range(0..5);
                    instance.tags = (0..tag_count)
                        .map(|_| format!("tag-{}", rng.gen::<u16>()))
                        .collect();
                }
            }
        }
        
        // Mutate authorized users (30% chance)
        if rng.gen_bool(0.3) {
            match rng.gen_range(0..3) {
                0 if !instance.authorized_users.is_empty() => {
                    // Remove a random user
                    let idx = rng.gen_range(0..instance.authorized_users.len());
                    instance.authorized_users.remove(idx);
                },
                1 => {
                    // Add a new user
                    instance.authorized_users.push(format!("user-{}", rng.gen::<u16>()));
                },
                _ => {
                    // Replace all users
                    let user_count = rng.gen_range(1..5);
                    instance.authorized_users = (0..user_count)
                        .map(|_| format!("user-{}", rng.gen::<u16>()))
                        .collect();
                }
            }
        }
    }
}

/// Node status mutator
pub struct NodeStatusMutator;

impl NodeStatusMutator {
    pub fn new() -> Self {
        Self {}
    }
}

impl Mutator<NodeStatus> for NodeStatusMutator {
    fn mutate(&self, status: &mut NodeStatus) {
        let mut rng = thread_rng();
        *status = match rng.gen_range(0..5) {
            0 => NodeStatus::Online,
            1 => NodeStatus::Offline,
            2 => NodeStatus::Maintenance,
            3 => NodeStatus::Provisioning,
            _ => NodeStatus::Unknown,
        };
    }
}

/// Node mutator
pub struct NodeMutator {
    resources_mutator: ResourcesMutator,
    status_mutator: NodeStatusMutator,
}

impl NodeMutator {
    pub fn new() -> Self {
        Self {
            resources_mutator: ResourcesMutator::new(),
            status_mutator: NodeStatusMutator::new(),
        }
    }
}

impl Mutator<Node> for NodeMutator {
    fn mutate(&self, node: &mut Node) {
        let mut rng = thread_rng();
        
        // Mutate ID (10% chance)
        if rng.gen_bool(0.1) {
            node.id = Uuid::new_v4().to_string();
        }
        
        // Mutate name (30% chance)
        if rng.gen_bool(0.3) {
            node.name = format!("mutated-node-{}", rng.gen::<u16>());
        }
        
        // Mutate public key (20% chance)
        if rng.gen_bool(0.2) {
            node.public_key = format!("ssh-rsa MUTATED{}", rng.gen::<u32>());
        }
        
        // Mutate region (20% chance)
        if rng.gen_bool(0.2) {
            node.region = ["us-east", "us-west", "eu-central", "ap-south", "sa-east"]
                .choose(&mut rng)
                .unwrap()
                .to_string();
        }
        
        // Mutate resources (60% chance)
        if rng.gen_bool(0.6) {
            self.resources_mutator.mutate(&mut node.resources_total);
            self.resources_mutator.mutate(&mut node.resources_available);
            
            // Ensure available resources are a subset of total resources
            node.resources_available.memory = rng.gen_range(0..=node.resources_total.memory);
            node.resources_available.vcpus = rng.gen_range(0..=node.resources_total.vcpus);
            node.resources_available.disk_gb = rng.gen_range(0..=node.resources_total.disk_gb);
            
            if let Some(gpu) = &node.resources_total.gpu {
                let count_available = rng.gen_range(0..=gpu.count);
                if count_available > 0 {
                    node.resources_available.gpu = Some(GpuResources {
                        count: count_available,
                        model: gpu.model.clone(),
                    });
                } else {
                    node.resources_available.gpu = None;
                }
            } else {
                node.resources_available.gpu = None;
            }
        }
        
        // Mutate IP addresses (40% chance)
        if rng.gen_bool(0.4) {
            match rng.gen_range(0..3) {
                0 if !node.ip_addresses.is_empty() => {
                    // Remove a random IP
                    let idx = rng.gen_range(0..node.ip_addresses.len());
                    node.ip_addresses.remove(idx);
                },
                1 => {
                    // Add a new IP
                    node.ip_addresses.push(format!("{}.{}.{}.{}", 
                        rng.gen_range(1..255), 
                        rng.gen_range(0..255), 
                        rng.gen_range(0..255), 
                        rng.gen_range(1..255)
                    ));
                },
                _ => {
                    // Replace all IPs
                    let ip_count = rng.gen_range(1..4);
                    node.ip_addresses = (0..ip_count)
                        .map(|_| format!("{}.{}.{}.{}", 
                            rng.gen_range(1..255), 
                            rng.gen_range(0..255), 
                            rng.gen_range(0..255), 
                            rng.gen_range(1..255)
                        ))
                        .collect();
                }
            }
        }
        
        // Mutate status (50% chance)
        if rng.gen_bool(0.5) {
            self.status_mutator.mutate(&mut node.status);
        }
    }
}

/// Account mutator
pub struct AccountMutator;

impl AccountMutator {
    pub fn new() -> Self {
        Self {}
    }
}

impl Mutator<Account> for AccountMutator {
    fn mutate(&self, account: &mut Account) {
        let mut rng = thread_rng();
        
        // Mutate ID (10% chance)
        if rng.gen_bool(0.1) {
            account.id = Uuid::new_v4().to_string();
        }
        
        // Mutate username (30% chance)
        if rng.gen_bool(0.3) {
            account.username = format!("mutated_user{}", rng.gen::<u16>());
            account.email = format!("{}@example.com", account.username);
        }
        
        // Mutate email only (20% chance)
        if rng.gen_bool(0.2) {
            account.email = format!("{}@mutated.com", account.username);
        }
        
        // Mutate public key (20% chance)
        if rng.gen_bool(0.2) {
            account.public_key = format!("ssh-rsa MUTATED{}", rng.gen::<u32>());
        }
        
        // Mutate credit balance (60% chance)
        if rng.gen_bool(0.6) {
            account.credit_balance = rng.gen_range(0.0..1000.0);
        }
        
        // Mutate instances (40% chance)
        if rng.gen_bool(0.4) {
            match rng.gen_range(0..3) {
                0 if !account.instances.is_empty() => {
                    // Remove a random instance
                    let idx = rng.gen_range(0..account.instances.len());
                    account.instances.remove(idx);
                },
                1 => {
                    // Add a new instance
                    account.instances.push(Uuid::new_v4().to_string());
                },
                _ => {
                    // Replace all instances
                    let instance_count = rng.gen_range(0..10);
                    account.instances = (0..instance_count)
                        .map(|_| Uuid::new_v4().to_string())
                        .collect();
                }
            }
        }
        
        // Mutate created_at (20% chance)
        if rng.gen_bool(0.2) {
            account.created_at = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
                - rng.gen_range(0..31536000); // Up to a year ago
        }
    }
}

/// CrdtPeer mutator
pub struct CrdtPeerMutator;

impl CrdtPeerMutator {
    pub fn new() -> Self {
        Self {}
    }
}

impl Mutator<CrdtPeer> for CrdtPeerMutator {
    fn mutate(&self, peer: &mut CrdtPeer) {
        let mut rng = thread_rng();
        
        // Mutate ID (10% chance)
        if rng.gen_bool(0.1) {
            peer.id = Uuid::new_v4().to_string();
        }
        
        // Mutate name (30% chance)
        if rng.gen_bool(0.3) {
            peer.name = format!("mutated-peer-{}", rng.gen::<u16>());
        }
        
        // Mutate public key (20% chance)
        if rng.gen_bool(0.2) {
            peer.public_key = format!("ssh-rsa MUTATED{}", rng.gen::<u32>());
        }
        
        // Mutate endpoint (40% chance)
        if rng.gen_bool(0.4) {
            peer.endpoint = format!("{}:{}", 
                format!("{}.{}.{}.{}", 
                    rng.gen_range(1..255), 
                    rng.gen_range(0..255), 
                    rng.gen_range(0..255), 
                    rng.gen_range(1..255)
                ),
                rng.gen_range(1024..65535)
            );
        }
        
        // Mutate last_seen (50% chance)
        if rng.gen_bool(0.5) {
            peer.last_seen = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
                - rng.gen_range(0..86400); // Up to a day ago
        }
    }
}

/// CrdtCidr mutator
pub struct CrdtCidrMutator;

impl CrdtCidrMutator {
    pub fn new() -> Self {
        Self {}
    }
}

impl Mutator<CrdtCidr> for CrdtCidrMutator {
    fn mutate(&self, cidr: &mut CrdtCidr) {
        let mut rng = thread_rng();
        
        // Mutate CIDR range (30% chance)
        if rng.gen_bool(0.3) {
            let first_octet = rng.gen_range(10..192);
            let second_octet = rng.gen_range(0..255);
            let prefix_length = rng.gen_range(16..30);
            cidr.cidr = format!("{}.{}.0.0/{}", first_octet, second_octet, prefix_length);
        }
        
        // Mutate owner (20% chance)
        if rng.gen_bool(0.2) {
            cidr.owner = format!("mutated-user-{}", rng.gen::<u16>());
        }
        
        // Mutate assigned_to (40% chance)
        if rng.gen_bool(0.4) {
            cidr.assigned_to = if rng.gen_bool(0.5) {
                Some(format!("mutated-instance-{}", rng.gen::<u16>()))
            } else {
                None
            };
        }
        
        // Mutate created_at (20% chance)
        if rng.gen_bool(0.2) {
            cidr.created_at = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
                - rng.gen_range(0..31536000); // Up to a year ago
        }
    }
}

/// CrdtAssociation mutator
pub struct CrdtAssociationMutator;

impl CrdtAssociationMutator {
    pub fn new() -> Self {
        Self {}
    }
}

impl Mutator<CrdtAssociation> for CrdtAssociationMutator {
    fn mutate(&self, association: &mut CrdtAssociation) {
        let mut rng = thread_rng();
        
        // Mutate ID (10% chance)
        if rng.gen_bool(0.1) {
            association.id = Uuid::new_v4().to_string();
        }
        
        // Mutate peer_id (30% chance)
        if rng.gen_bool(0.3) {
            association.peer_id = Uuid::new_v4().to_string();
        }
        
        // Mutate cidr_id (30% chance)
        if rng.gen_bool(0.3) {
            association.cidr_id = Uuid::new_v4().to_string();
        }
        
        // Mutate created_at (20% chance)
        if rng.gen_bool(0.2) {
            association.created_at = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
                - rng.gen_range(0..31536000); // Up to a year ago
        }
    }
}

/// FormDnsRecord mutator
pub struct FormDnsRecordMutator;

impl FormDnsRecordMutator {
    pub fn new() -> Self {
        Self {}
    }
}

impl Mutator<FormDnsRecord> for FormDnsRecordMutator {
    fn mutate(&self, record: &mut FormDnsRecord) {
        let mut rng = thread_rng();
        
        // Mutate domain (30% chance)
        if rng.gen_bool(0.3) {
            let name = format!("mutated-record-{}", rng.gen::<u16>());
            record.domain = format!("{}.formtest.local", name);
        }
        
        // Mutate record_type (30% chance)
        if rng.gen_bool(0.3) {
            record.record_type = ["A", "AAAA", "CNAME", "MX", "TXT"]
                .choose(&mut rng)
                .unwrap()
                .to_string();
        }
        
        // Mutate value (50% chance)
        if rng.gen_bool(0.5) {
            record.value = match record.record_type.as_str() {
                "A" => format!("{}.{}.{}.{}", 
                    rng.gen_range(1..255), 
                    rng.gen_range(0..255), 
                    rng.gen_range(0..255), 
                    rng.gen_range(1..255)
                ),
                "AAAA" => format!("2001:db8:85a3::{:x}:{:x}:{:x}:{:x}", 
                    rng.gen::<u16>(), 
                    rng.gen::<u16>(),
                    rng.gen::<u16>(),
                    rng.gen::<u16>()
                ),
                "CNAME" => format!("mutated-cname-{}.formtest.local", rng.gen::<u16>()),
                "MX" => format!("{} mutated-mail-{}.formtest.local", rng.gen_range(1..20), rng.gen::<u16>()),
                "TXT" => format!("\"v=spf1 include:_spf.mutated.local ~all\""),
                _ => "".to_string(),
            };
        }
        
        // Mutate TTL (40% chance)
        if rng.gen_bool(0.4) {
            record.ttl = rng.gen_range(60..86400);
        }
    }
}

/// CrdtDnsRecord mutator
pub struct CrdtDnsRecordMutator {
    form_dns_record_mutator: FormDnsRecordMutator,
}

impl CrdtDnsRecordMutator {
    pub fn new() -> Self {
        Self {
            form_dns_record_mutator: FormDnsRecordMutator::new(),
        }
    }
}

impl Mutator<CrdtDnsRecord> for CrdtDnsRecordMutator {
    fn mutate(&self, record: &mut CrdtDnsRecord) {
        let mut rng = thread_rng();
        
        // Mutate ID (10% chance)
        if rng.gen_bool(0.1) {
            record.id = Uuid::new_v4().to_string();
        }
        
        // Mutate DNS record fields using the form DNS record mutator
        let mut form_record = FormDnsRecord {
            domain: record.domain.clone(),
            record_type: record.record_type.clone(),
            value: record.value.clone(),
            ttl: record.ttl,
        };
        
        self.form_dns_record_mutator.mutate(&mut form_record);
        
        record.domain = form_record.domain;
        record.record_type = form_record.record_type;
        record.value = form_record.value;
        record.ttl = form_record.ttl;
        
        // Mutate created_at (20% chance)
        if rng.gen_bool(0.2) {
            record.created_at = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
                - rng.gen_range(0..31536000); // Up to a year ago
        }
    }
} 