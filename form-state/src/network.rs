use std::{net::{IpAddr, SocketAddr}, time::{Duration, SystemTime}};
use crdts::{bft_reg::Update, map::Op, merkle_reg::Sha3Hash, BFTReg, CmRDT, Map};
use ipnet::IpNet;
use k256::ecdsa::SigningKey;
use shared::{Association, AssociationContents, Cidr, CidrContents, Endpoint, Peer, PeerContents};
use serde::{Serialize, Deserialize};
use tiny_keccak::{Hasher, Sha3};
use trust_dns_proto::rr::RecordType;
use form_dns::store::FormDnsRecord;
use crate::Actor;

pub type PeerOp<T> = Op<String, BFTReg<CrdtPeer<T>, Actor>, Actor>; 
pub type CidrOp<T> = Op<String, BFTReg<CrdtCidr<T>, Actor>, Actor>;
pub type AssocOp<T> = Op<String, BFTReg<CrdtAssociation<T>, Actor>, Actor>;
pub type DnsOp = Op<String, BFTReg<CrdtDnsRecord, Actor>, Actor>;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct CrdtPeer<T: Clone> {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) ip: IpAddr,
    pub(crate) cidr_id: T, 
    pub(crate) public_key: String,
    pub(crate) endpoint: Option<Endpoint>,
    pub(crate) keepalive: Option<u16>,
    pub(crate) is_admin: bool,
    pub(crate) is_disabled: bool,
    pub(crate) is_redeemed: bool,
    pub(crate) invite_expires: Option<u64>,
    pub(crate) candidates: Vec<Endpoint>,
}

impl Sha3Hash for CrdtPeer<String> {
    fn hash(&self, hasher: &mut Sha3) {
        hasher.update(&serde_json::to_vec(self).unwrap())
    }
}

impl From<PeerContents<String>> for CrdtPeer<String> {
    fn from(value: PeerContents<String>) -> Self {
        Self {
            id: value.name.to_string().clone(),
            name: value.name.to_string(),
            ip: value.ip,
            cidr_id: value.cidr_id,
            public_key: value.public_key,
            endpoint: value.endpoint,
            keepalive: value.persistent_keepalive_interval,
            is_admin: value.is_admin,
            is_disabled: value.is_disabled,
            is_redeemed: value.is_redeemed,
            invite_expires: value.invite_expires
            .map(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
            .flatten()
            .map(|t| t.as_secs()),
            candidates: value.candidates,
        }
    }
}

impl From<CrdtPeer<String>> for Peer<String> {
    fn from(value: CrdtPeer<String>) -> Self {
        Peer{
            id: value.id,
            contents: PeerContents {
                name: value.name.into(),
                ip: value.ip,
                cidr_id: value.cidr_id,
                public_key: value.public_key,
                endpoint: value.endpoint,
                persistent_keepalive_interval: value.keepalive,
                is_admin: value.is_admin,
                is_disabled: value.is_disabled,
                is_redeemed: value.is_redeemed,
                invite_expires: value.invite_expires.map(|time| {
                SystemTime::UNIX_EPOCH + Duration::from_secs(time)}),
                candidates: value.candidates
            } 
        }
    }
}

impl From<CrdtCidr<String>> for Cidr<String> {
    fn from(value: CrdtCidr<String>) -> Self {
        Cidr {
            id: value.id,
            contents: CidrContents {
                name: value.name.clone(),
                cidr: value.cidr.clone(),
                parent: value.parent.clone(),
            }
        }
    }
}

impl From<CrdtAssociation<String>> for Association<String, (String, String)> {
    fn from(value: CrdtAssociation<String>) -> Self {
        Association {
            id: value.id,
            contents: AssociationContents {
                cidr_id_1: value.cidr_1,
                cidr_id_2: value.cidr_2
            }
        }
    }
}

impl<T: Clone> CrdtPeer<T> {
    pub fn id(&self) -> String {
        self.id.clone()
    }

    pub fn ip(&self) -> IpAddr {
        self.ip
    }

    pub fn is_admin(&self) -> bool {
        self.is_admin
    }

    pub fn is_disabled(&self) -> bool {
        self.is_disabled
    }

    pub fn is_redeemed(&self) -> bool {
        self.is_redeemed
    }

    pub fn cidr(&self) -> T {
        self.cidr_id.clone()
    }

    pub fn invite_expires(&self) -> Option<u64> {
        self.invite_expires
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CrdtCidr<T: Clone> {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) cidr: IpNet,
    pub(crate) parent: Option<T>
}

impl Sha3Hash for CrdtCidr<String> {
    fn hash(&self, hasher: &mut Sha3) {
        hasher.update(&serde_json::to_vec(self).unwrap())
    }
}


impl<T: Clone> CrdtCidr<T> {
    pub fn id(&self) -> String {
        self.id.clone()
    }
    
    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn cidr(&self) -> IpNet {
        self.cidr.clone()
    }

    pub fn parent(&self) -> Option<T> {
        self.parent.clone()
    }
}

impl From<CidrContents<String>> for CrdtCidr<String> {
    fn from(value: CidrContents<String>) -> Self { 
        Self {
            id:  value.name.clone(),
            name: value.name,
            cidr: value.cidr,
            parent: value.parent,
        }
    }

}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CrdtAssociation<T: Clone> {
    pub(crate) id: (T, T),
    pub(crate) cidr_1: T,
    pub(crate) cidr_2: T 
}

impl Sha3Hash for CrdtAssociation<String> {
    fn hash(&self, hasher: &mut Sha3) {
        hasher.update(&serde_json::to_vec(self).unwrap())
    }
}


impl<T: Clone> CrdtAssociation<T> {
    pub fn id(&self) -> (T, T) {
        self.id.clone()
    }

    pub fn cidr_1(&self) -> T {
        self.cidr_1.clone()
    }

    pub fn cidr_2(&self) -> T {
        self.cidr_2.clone()
    }
}

impl From<AssociationContents<String>> for CrdtAssociation<String> {
    fn from(value: AssociationContents<String>) -> Self {
        Self {
            id: (value.cidr_id_1.clone(), value.cidr_id_2.clone()),
            cidr_1: value.cidr_id_1,
            cidr_2: value.cidr_id_2
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CrdtDnsRecord {
    pub(crate) domain: String,
    pub(crate) record_type: RecordType,
    pub(crate) formnet_ip: Vec<SocketAddr>,
    pub(crate) public_ip: Vec<SocketAddr>,
    pub(crate) cname_target: Option<String>,
    pub(crate) ttl: u32,
    pub(crate) ssl_cert: bool,
}

impl CrdtDnsRecord {
    pub fn domain(&self) -> String {
        self.domain.clone()
    }

    pub fn record_type(&self) -> RecordType {
        self.record_type
    }

    pub fn formnet_ip(&self) -> Vec<SocketAddr> {
        self.formnet_ip.clone()
    }

    pub fn public_ip(&self) -> Vec<SocketAddr> {
        self.public_ip.clone()
    }

    pub fn cname_target(&self) -> Option<String> {
        self.cname_target.clone()
    }

    pub fn ttl(&self) -> u32 {
        self.ttl
    }

    pub fn ssl_cert(&self) -> bool {
        self.ssl_cert
    }

}

impl From<FormDnsRecord> for CrdtDnsRecord {
    fn from(value: FormDnsRecord) -> Self {
        CrdtDnsRecord { 
            domain: value.domain, 
            record_type: value.record_type, 
            formnet_ip: value.formnet_ip, 
            public_ip: value.public_ip, 
            cname_target: value.cname_target, 
            ttl: value.ttl,
            ssl_cert: value.ssl_cert
        }
    }
}

impl From<CrdtDnsRecord> for FormDnsRecord {
    fn from(value: CrdtDnsRecord) -> Self {
        FormDnsRecord { 
            domain: value.domain, 
            record_type: value.record_type, 
            formnet_ip: value.formnet_ip, 
            public_ip: value.public_ip, 
            cname_target: value.cname_target, 
            ttl: value.ttl,
            ssl_cert: value.ssl_cert
        }
    }
}

impl From<&CrdtDnsRecord> for FormDnsRecord {
    fn from(value: &CrdtDnsRecord) -> Self {
        FormDnsRecord { 
            domain: value.domain.clone(), 
            record_type: value.record_type, 
            formnet_ip: value.formnet_ip.clone(), 
            public_ip: value.public_ip.clone(), 
            cname_target: value.cname_target.clone(), 
            ttl: value.ttl,
            ssl_cert: value.ssl_cert
        }
    }
}

impl Sha3Hash for CrdtDnsRecord {
    fn hash(&self, hasher: &mut Sha3) {
        hasher.update(&serde_json::to_vec(self).unwrap())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DnsState {
    pub zones: Map<String, BFTReg<CrdtDnsRecord, Actor>, Actor>
}

impl DnsState {
    pub fn new() -> Self {
        Self {
            zones: Map::new()
        }
    }

    pub fn apply(&mut self, op: DnsOp) {
        self.zones.apply(op);
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetworkState {
    pub pk: String,
    pub node_id: Actor,
    pub peers: Map<String, BFTReg<CrdtPeer<String>, Actor>, Actor>,
    pub cidrs: Map<String, BFTReg<CrdtCidr<String>, Actor>, Actor>,
    pub associations: Map<String, BFTReg<CrdtAssociation<String>, Actor>, Actor>,
    pub dns_state: DnsState
}

#[allow(dead_code, unused)]
impl NetworkState {
    pub fn new(node_id: Actor, pk: String) -> Self {
        let peer_map = Map::new();
        let cidr_map = Map::new();
        let associations_map = Map::new();
        let dns_map = DnsState::new();

        Self {
            pk,
            node_id,
            peers: peer_map,
            cidrs: cidr_map,
            associations: associations_map,
            dns_state: dns_map 
        }
    }

    pub fn update_peer_local(&mut self, peer: PeerContents<String>) -> PeerOp<String> {
        log::info!("Acquiring add ctx...");
        let add_ctx = self.peers.read_ctx().derive_add_ctx(self.node_id.clone());
        log::info!("Decoding our private key...");
        let signing_key = SigningKey::from_slice(
            &hex::decode(self.pk.clone())
                .expect("PANIC: Invalid SigningKey Cannot Decode from Hex"))
                .expect("PANIC: Invalid SigningKey cannot recover ffrom Bytes");
        log::info!("Creating op...");
        let op = self.peers.update(peer.name.to_string(), add_ctx, |reg, ctx| {
            let op = reg.update(peer.into(), self.node_id.clone(), signing_key).expect("PANIC: Unable to sign updates");
            op
        });
        log::info!("Op created, returning...");
        op
    }

    pub fn remove_peer_local(&mut self, id: String) -> PeerOp<String> {
        log::info!("Acquiring remove context...");
        let rm_ctx = self.peers.read_ctx().derive_rm_ctx();
        log::info!("Building Rm Op...");
        self.peers.rm(id, rm_ctx)
    }

    pub fn update_cidr_local(&mut self, cidr: CidrContents<String>) ->  CidrOp<String> { 
        log::info!("Acquiring add ctx...");
        let add_ctx = self.cidrs.read_ctx().derive_add_ctx(self.node_id.clone());
        log::info!("Decoding our private key...");
        let signing_key = SigningKey::from_slice(
            &hex::decode(self.pk.clone())
                .expect("PANIC: Invalid SigningKey Cannot Decode from Hex"))
                .expect("PANIC: Invalid SigningKey cannot recover ffrom Bytes");
        log::info!("Creating op...");
        let op = self.cidrs.update(cidr.name.to_string(), add_ctx, |reg, ctx| {
            let op = reg.update(cidr.into(), self.node_id.clone(), signing_key).expect("PANIC: Unable to sign updates");
            op
        });
        log::info!("Op created, returning...");
        op
    }

    pub fn remove_cidr_local(&mut self, id: String) -> CidrOp<String> { 
        let rm_ctx = self.cidrs.read_ctx().derive_rm_ctx();
        self.cidrs.rm(id, rm_ctx)
    }

    pub fn update_association_local(&mut self, association: AssociationContents<String>) -> AssocOp<String> { 
        log::info!("Acquiring add ctx...");
        let add_ctx = self.associations.read_ctx().derive_add_ctx(self.node_id.clone());
        log::info!("Decoding our private key...");
        let signing_key = SigningKey::from_slice(
            &hex::decode(self.pk.clone())
                .expect("PANIC: Invalid SigningKey Cannot Decode from Hex"))
                .expect("PANIC: Invalid SigningKey cannot recover ffrom Bytes");
        log::info!("Creating op...");
        let op = self.associations.update(format!("{}-{}",association.cidr_id_1.to_string(), association.cidr_id_2.to_string()), add_ctx, |reg, ctx| {
            let op = reg.update(association.into(), self.node_id.clone(), signing_key).expect("PANIC: Unable to sign updates");
            op
        });
        log::info!("Op created, returning...");
        op
    }

    pub fn remove_association_local(&mut self, id: (String, String)) -> AssocOp<String> { 
        let rm_ctx = self.associations.read_ctx().derive_rm_ctx();
        let id = format!("{}-{}", id.0, id.1);
        self.associations.rm(id, rm_ctx)
    }


    pub fn peer_op(&mut self, op: PeerOp<String>) -> Option<(String, String)> {
        log::info!("Applying peer op");
        self.peers.apply(op.clone());
        match op {
            Op::Up { dot, key, op } => Some((dot.actor, key)),
            Op::Rm { clock, keyset } => None
        }
    }

    pub fn peer_op_success(&self, key: String, update: Update<CrdtPeer<String>, String>) -> (bool, CrdtPeer<String>) {
        if let Some(reg) = self.peers.get(&key).val {
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

    pub fn get_peer_by_ip(&self, ip: String) -> Option<CrdtPeer<String>> {
        if let Some(ctx) = self.peers.values().find(|ctx| {
            match ctx.val.val() {
                None => false,
                Some(node) => {
                    node.value().ip().to_string() == ip
                }
            }
        }) {
            if let Some(node) = ctx.val.val() {
                return Some(node.value())
            }
        }

        None
    }

    pub fn cidr_op(&mut self, op: CidrOp<String>) {
        self.cidrs.apply(op);
    }

    pub fn cidr_op_success(&self, key: String, update: Update<CrdtCidr<String>, String>) -> (bool, CrdtCidr<String>) {
        if let Some(reg) = self.cidrs.get(&key).val {
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

    pub fn associations_op(&mut self, op: AssocOp<String>) {
        self.associations.apply(op);
    }

    pub fn associations_op_success(&self, key: String, update: Update<CrdtAssociation<String>, String>) -> (bool, CrdtAssociation<String>) {
        if let Some(reg) = self.associations.get(&key).val {
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

    pub fn update_dns_local(&mut self, dns: FormDnsRecord) -> DnsOp { 
        log::info!("Acquiring add ctx...");
        let add_ctx = self.dns_state.zones.read_ctx().derive_add_ctx(self.node_id.clone());
        log::info!("Decoding our private key...");
        let signing_key = SigningKey::from_slice(
            &hex::decode(self.pk.clone())
                .expect("PANIC: Invalid SigningKey Cannot Decode from Hex"))
                .expect("PANIC: Invalid SigningKey cannot recover ffrom Bytes");
        log::info!("Creating op...");
        let op = self.dns_state.zones.update(dns.domain.clone(), add_ctx, |reg, ctx| {
            let op = reg.update(dns.into(), self.node_id.clone(), signing_key).expect("PANIC: Unable to sign updates");
            op
        });
        log::info!("Op created, returning...");
        op
    }

    pub fn remove_dns_local(&mut self, domain: String) -> DnsOp { 
        log::info!("Acquiring remove context...");
        let rm_ctx = self.dns_state.zones.read_ctx().derive_rm_ctx();
        log::info!("Building Rm Op...");
        self.dns_state.zones.rm(domain, rm_ctx)
    }

    pub fn dns_op(&mut self, op: DnsOp) {
        self.dns_state.apply(op);
    }

    pub fn dns_op_success(&self, domain: String, update: Update<CrdtDnsRecord, String>) -> (bool, CrdtDnsRecord) {
        if let Some(reg) = self.dns_state.zones.get(&domain).val {
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

    fn handle_cached_peer_ops(&mut self, ops: Vec<PeerOp<String>>) {
        for op in ops {
            self.peer_op(op);
        }
    }

    fn handle_cached_cidr_ops(&mut self, ops: Vec<CidrOp<String>>) {
        for op in ops {
            self.cidr_op(op);
        }
    }

    fn handle_cached_assoc_ops(&mut self, ops: Vec<AssocOp<String>>) {
        for op in ops {
            self.associations_op(op);
        }
    }

    fn handle_cached_dns_ops(&mut self, ops: Vec<DnsOp>) {
        for op in ops {
            self.dns_op(op);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::{PeerContents, Hostname};
    use std::str::FromStr;
    use std::net::IpAddr;
    use k256::ecdsa::SigningKey;
    use alloy_primitives::Address;
    use hex;

    #[test]
    fn test_insert_peer() -> Result<(), Box<dyn std::error::Error>> {
        let sk = SigningKey::random(&mut rand::thread_rng());
        let address = hex::encode(&Address::from_private_key(&sk));
        let pk = hex::encode(SigningKey::random(&mut rand::thread_rng()).to_bytes());
        let mut state = NetworkState::new(address.clone(), pk.clone());

        // Create a new peer
        let peer_contents = PeerContents {
            name: Hostname::from_str("peer-1")?,
            ip: IpAddr::from([192, 168, 1, 1]),
            cidr_id: "cidr-1".to_string(),
            public_key: "public-key".to_string(),
            endpoint: None,
            persistent_keepalive_interval: Some(15),
            is_admin: false,
            is_disabled: false,
            is_redeemed: true,
            invite_expires: None,
            candidates: vec![],
        };

        // Insert the peer
        let op = state.update_peer_local(peer_contents.clone());
        println!("{op:?}\n\n");
        state.peer_op(op);
        println!("{:?}\n\n", state.peers);

        // Validate the peer exists
        let peer = state.get_peer_by_ip("192.168.1.1".to_string());
        assert!(peer.is_some());
        let peer = peer.unwrap();
        assert_eq!(peer.id(), "peer-1");
        assert_eq!(peer.ip(), IpAddr::from([192, 168, 1, 1]));
        assert!(peer.is_redeemed());

        Ok(())
    }

    #[test]
    fn test_update_peer() -> Result<(), Box<dyn std::error::Error>> {
        let sk = SigningKey::random(&mut rand::thread_rng());
        let address = hex::encode(&Address::from_private_key(&sk));
        let pk = hex::encode(SigningKey::random(&mut rand::thread_rng()).to_bytes());
        let mut state = NetworkState::new(address.clone(), pk.clone());

        // Insert the peer
        let peer_contents = PeerContents {
            name: Hostname::from_str("peer-1")?,
            ip: IpAddr::from([192, 168, 1, 1]),
            cidr_id: "cidr-1".to_string(),
            public_key: "public-key".to_string(),
            endpoint: None,
            persistent_keepalive_interval: Some(15),
            is_admin: false,
            is_disabled: false,
            is_redeemed: true,
            invite_expires: None,
            candidates: vec![],
        };

        let op = state.update_peer_local(peer_contents.clone());
        state.peer_op(op);

        // Update the peer
        let mut updated_peer_contents = peer_contents.clone();
        updated_peer_contents.is_admin = true;
        let update_op = state.update_peer_local(updated_peer_contents.clone());
        println!("{update_op:?}\n\n");
        state.peer_op(update_op);
        println!("{:?}\n\n", state.peers);

        // Validate the peer was updated
        let peer = state.get_peer_by_ip("192.168.1.1".to_string());
        assert!(peer.is_some());
        let peer = peer.unwrap();
        assert!(peer.is_admin());
        Ok(())
    }
}
