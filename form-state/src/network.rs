use std::{net::{IpAddr, SocketAddr}, time::{Duration, SystemTime}};
use crdts::{map::Op, Map, BFTReg, merkle_reg::Sha3Hash, CmRDT};
use ipnet::IpNet;
use k256::ecdsa::SigningKey;
use shared::{AssociationContents, CidrContents, Endpoint, Peer, PeerContents};
use serde::{Serialize, Deserialize};
use tiny_keccak::{Hasher, Sha3};
use crate::Actor;

pub type PeerOp<T> = Op<String, BFTReg<CrdtPeer<T>, Actor>, Actor>; 
pub type CidrOp<T> = Op<String, BFTReg<CrdtCidr<T>, Actor>, Actor>;
pub type AssocOp<T> = Op<(String, String), BFTReg<CrdtAssociation<T>, Actor>, Actor>;
pub type DnsOp = Op<String, BFTReg<DnsZone, Actor>, Actor>;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct CrdtPeer<T: Clone> {
    id: String,
    name: String,
    ip: IpAddr,
    cidr_id: T, 
    public_key: String,
    endpoint: Option<Endpoint>,
    keepalive: Option<u16>,
    is_admin: bool,
    is_disabled: bool,
    is_redeemed: bool,
    invite_expires: Option<u64>,
    candidates: Vec<Endpoint>,
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

impl TryFrom<CrdtPeer<String>> for Peer<String> {
    type Error = Box<dyn std::error::Error>;
    fn try_from(value: CrdtPeer<String>) -> Result<Self, Self::Error> {
        Ok(Peer{
            id: value.id.parse()?,
            contents: PeerContents {
                name: value.name.parse()?,
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
        })
    }
}

impl<T: Clone> CrdtPeer<T> {
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
    id: String,
    name: String,
    cidr: IpNet,
    parent: Option<T>
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
    id: (T, T),
    cidr_1: T,
    cidr_2: T 
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
pub enum EndpointProtocol {
    Http,
    Https,
    Tcp,
    Udp,
    Quic
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PortMap {
    public_port: u16,
    private_port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq,)]
pub enum RecordType {
    A,
    AAAA,
    CNAME,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Entrypoint {
    addr: SocketAddr,
    protocol: EndpointProtocol,
    gateway_id: String,
    ports: Option<PortMap>
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CrdtDnsRecord {
    name: String,
    ip: IpAddr,
    entrypoints: Vec<Entrypoint>,
    record_type: RecordType,
    owner: String,
    ttl: u32,
    last_updated: u64,
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ZoneEntry {
    Record(CrdtDnsRecord),
    Subdomain(DnsZone),
}

impl Sha3Hash for ZoneEntry {
    fn hash(&self, hasher: &mut Sha3) {
        hasher.update(&serde_json::to_vec(self).unwrap())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DnsZone {
    records: Map<Actor, BFTReg<ZoneEntry, Actor>, Actor>,
    owner: String,
}

impl Sha3Hash for DnsZone {
    fn hash(&self, hasher: &mut Sha3) {
        hasher.update(&serde_json::to_vec(self).unwrap())
    }
}

impl DnsZone {
    pub fn new(owner: String) -> Self {
        Self { owner, records: Map::new() }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DnsState {
    pub zones: Map<Actor, BFTReg<DnsZone, Actor>, Actor>
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
    pub associations: Map<(String, String), BFTReg<CrdtAssociation<String>, Actor>, Actor>,
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
        let add_ctx = self.peers.read_ctx().derive_add_ctx(self.node_id.clone());
        let signing_key = SigningKey::from_slice(
            &hex::decode(self.pk.clone())
                .expect("PANIC: Invalid SigningKey Cannot Decode from Hex"))
                .expect("PANIC: Invalid SigningKey cannot recover ffrom Bytes");
        let op = self.peers.update(peer.name.to_string(), add_ctx, |reg, ctx| {
            let op = reg.update(peer.into(), self.node_id.clone(), signing_key).expect("PANIC: Unable to sign updates");
            op
        });
        op
    }

    pub fn remove_peer_local(&mut self, id: String) -> PeerOp<String> {
        let rm_ctx = self.peers.read_ctx().derive_rm_ctx();
        self.peers.rm(id, rm_ctx)
    }

    pub fn update_cidr_local(&mut self, cidr: CidrContents<String>) ->  CidrOp<String> { 
        let add_ctx = self.cidrs.read_ctx().derive_add_ctx(self.node_id.clone());
        let signing_key = SigningKey::from_slice(
            &hex::decode(self.pk.clone())
                .expect("PANIC: Invalid SigningKey Cannot Decode from Hex"))
                .expect("PANIC: Invalid SigningKey cannot recover ffrom Bytes");
        let op = self.cidrs.update(cidr.name.to_string(), add_ctx, |reg, ctx| {
            let op = reg.update(cidr.into(), self.node_id.clone(), signing_key).expect("PANIC: Unable to sign updates");
            op
        });
        op
    }

    pub fn remove_cidr_local(&mut self, id: String) -> CidrOp<String> { 
        let rm_ctx = self.cidrs.read_ctx().derive_rm_ctx();
        self.cidrs.rm(id, rm_ctx)
    }

    pub fn add_association_local(&mut self, association: AssociationContents<String>) -> AssocOp<String> { 
        let add_ctx = self.associations.read_ctx().derive_add_ctx(self.node_id.clone());
        let signing_key = SigningKey::from_slice(
            &hex::decode(self.pk.clone())
                .expect("PANIC: Invalid SigningKey Cannot Decode from Hex"))
                .expect("PANIC: Invalid SigningKey cannot recover ffrom Bytes");
        let op = self.associations.update((association.cidr_id_1.to_string(), association.cidr_id_2.to_string()), add_ctx, |reg, ctx| {
            let op = reg.update(association.into(), self.node_id.clone(), signing_key).expect("PANIC: Unable to sign updates");
            op
        });
        op
    }

    pub fn remove_association_local(&mut self, id: (String, String)) -> AssocOp<String> { 
        let rm_ctx = self.associations.read_ctx().derive_rm_ctx();
        self.associations.rm(id, rm_ctx)
    }

    pub fn add_dns_local(&mut self, _dns: ZoneEntry) { 
        todo!()
    }

    pub fn remove_dns_local(&mut self, _id: String) { 
        todo!()
    }

    pub fn peer_op(&mut self, op: PeerOp<String>) {
        self.peers.apply(op);
    }

    pub fn cidr_op(&mut self, op: CidrOp<String>) {
        self.cidrs.apply(op);
    }

    pub fn associations_op(&mut self, op: AssocOp<String>) {
        self.associations.apply(op);
    }

    pub fn dns_op(&mut self, op: DnsOp) {
        self.dns_state.apply(op);
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
