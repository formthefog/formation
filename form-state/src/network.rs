use std::{net::{IpAddr, SocketAddr}, time::{Duration, SystemTime}};
use ditto::{map::{LocalOp, Op}, Map};
use ipnet::IpNet;
use shared::{Association, AssociationContents, Cidr, CidrContents, Endpoint, Peer, PeerContents};
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CrdtPeer {
    id: String,
    name: String,
    ip: IpAddr,
    cidr_id: i64,
    public_key: String,
    endpoint: Option<Endpoint>,
    keepalive: Option<u16>,
    is_admin: bool,
    is_disabled: bool,
    is_redeemed: bool,
    invite_expires: Option<u64>,
    candidates: Vec<Endpoint>,
}

impl From<Peer> for CrdtPeer {
    fn from(value: Peer) -> Self {
        Self {
            id: value.id.to_string(),
            name: value.contents.name.to_string(),
            ip: value.contents.ip,
            cidr_id: value.contents.cidr_id,
            public_key: value.contents.public_key,
            endpoint: value.contents.endpoint,
            keepalive: value.contents.persistent_keepalive_interval,
            is_admin: value.contents.is_admin,
            is_disabled: value.contents.is_disabled,
            is_redeemed: value.contents.is_redeemed,
            invite_expires: value.contents.invite_expires
            .map(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
            .flatten()
            .map(|t| t.as_secs()),
            candidates: value.contents.candidates,
        }
    }
}

impl TryFrom<CrdtPeer> for Peer {
    type Error = Box<dyn std::error::Error>;
    fn try_from(value: CrdtPeer) -> Result<Self, Self::Error> {
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

impl CrdtPeer {
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

    pub fn cidr(&self) -> i64 {
        self.cidr_id
    }

    pub fn invite_expires(&self) -> Option<u64> {
        self.invite_expires
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CrdtCidr {
    id: i64,
    name: String,
    cidr: IpNet,
    parent: Option<i64>
}

impl CrdtCidr {
    pub fn id(&self) -> i64 {
        self.id
    }
    
    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn cidr(&self) -> IpNet {
        self.cidr.clone()
    }

    pub fn parent(&self) -> Option<i64> {
        self.parent.clone()
    }
}

impl From<Cidr> for CrdtCidr {
    fn from(value: Cidr) -> Self { 
        Self {
            id:  value.id,
            name: value.contents.name,
            cidr: value.contents.cidr,
            parent: value.contents.parent,
        }
    }

}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CrdtAssociation {
    id: i64,
    cidr_1: i64,
    cidr_2: i64
}

impl CrdtAssociation {
    pub fn id(&self) -> i64 {
        self.id
    }

    pub fn cidr_1(&self) -> i64 {
        self.cidr_1
    }

    pub fn cidr_2(&self) -> i64 {
        self.cidr_2
    }
}

impl From<Association> for CrdtAssociation{
    fn from(value: Association) -> Self {
        Self {
            id: value.id,
            cidr_1: value.contents.cidr_id_1,
            cidr_2: value.contents.cidr_id_2
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


#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ZoneEntry {
    Record(CrdtDnsRecord),
    Subdomain(DnsZone),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DnsZone {
    records: Map<String, ZoneEntry>,
    owner: String,
}

impl DnsZone {
    pub fn new(owner: String) -> Self {
        Self { owner, records: Map::new() }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DnsState {
    zones: Map<String, DnsZone>
}

impl DnsState {
    pub fn new() -> Self {
        Self {
            zones: Map::new()
        }
    }

    pub fn add_site_id(&mut self, site_id: u32) -> Result<Vec<Op<String, DnsZone>>, ditto::Error> {
        self.zones.add_site_id(site_id)
    }

    pub fn execute_op(&mut self, op: Op<String, DnsZone>) -> LocalOp<String, DnsZone> {
        self.zones.execute_op(op)
    }
}


pub struct NetworkState {
    pub peers: Map<String, CrdtPeer>,
    pub cidrs: Map<String, CrdtCidr>,
    pub associations: Map<String, CrdtAssociation>,
    pub dns_state: DnsState
}

impl NetworkState {
    pub fn new(site_id: u32) -> Result<Self, ditto::Error> {
        let mut peer_map = Map::new();
        let mut cidr_map = Map::new();
        let mut associations_map = Map::new();
        let mut dns_map = DnsState::new();

        let cached_peer_ops = peer_map.add_site_id(site_id)?;
        let cached_cidr_ops = cidr_map.add_site_id(site_id)?;
        let cached_assoc_ops = associations_map.add_site_id(site_id)?;
        let cached_dns_ops = dns_map.add_site_id(site_id)?;

        let mut ns = Self {
            peers: peer_map,
            cidrs: cidr_map,
            associations: associations_map,
            dns_state: dns_map 
        };

        ns.handle_cached_peer_ops(cached_peer_ops)?;
        ns.handle_cached_cidr_ops(cached_cidr_ops)?;
        ns.handle_cached_assoc_ops(cached_assoc_ops)?;
        ns.handle_cached_dns_ops(cached_dns_ops)?;

        Ok(ns)

    }

    pub fn add_peer_local(&mut self, peer: PeerContents) -> Result<Op<String, CrdtPeer>, ditto::Error> {
        let peer_id = self.peers.local_value().len() as i64;
        let p = Peer {
            id: peer_id,
            contents: peer.clone() 
        };
        let op = self.peers.insert(peer_id.to_string(), p.into())?;
        Ok(op)
    }

    pub fn update_peer_local(&mut self, peer: PeerContents) -> Result<Op<String, CrdtPeer>, ditto::Error> {
        let peer_id = self.peers.local_value().len() as i64;
        let p = Peer {
            id: peer_id,
            contents: peer.clone() 
        };
        let op = self.peers.insert(peer_id.to_string(), p.into())?;
        Ok(op)
    }

    pub fn remove_peer_local(&mut self, id: String) -> Option<Result<Op<String, CrdtPeer>, ditto::Error>> {
        let op = self.peers.remove(&id)?;
        Some(op)
    }

    pub fn add_cidr_local(&mut self, cidr: CidrContents) -> Result<Op<String, CrdtCidr>, ditto::Error> {
        let cidr_id = self.cidrs.local_value().len() as i64;
        let c = Cidr {
            id: cidr_id,
            contents: cidr
        };
        let op = self.cidrs.insert(cidr_id.to_string(), c.into())?;
        Ok(op)
    }

    pub fn update_cidr_local(&mut self, cidr: CidrContents) -> Result<Op<String, CrdtCidr>, ditto::Error> {
        let cidr_id = self.cidrs.local_value().len() as i64;
        let c = Cidr {
            id: cidr_id,
            contents: cidr
        };
        let op = self.cidrs.insert(cidr_id.to_string(), c.into())?;
        Ok(op)
    }

    pub fn remove_cidr_local(&mut self, id: String) -> Option<Result<Op<String, CrdtCidr>, ditto::Error>> {
        let op = self.cidrs.remove(&id)?;
        Some(op)
    }

    pub fn add_association_local(&mut self, association: AssociationContents) -> Result<Op<String, CrdtAssociation>, ditto::Error> {
        let assoc_id = self.associations.local_value().len() as i64;
        let a = Association {
            id: assoc_id,
            contents: association
        };
        let op = self.associations.insert(assoc_id.to_string(), a.into())?;
        Ok(op)
    }

    pub fn remove_association_local(&mut self, id: String) -> Option<Result<Op<String, CrdtAssociation>, ditto::Error>> {
        let op = self.associations.remove(&id)?;
        Some(op)
    }

    pub fn add_dns_local(&mut self, _dns: ZoneEntry) -> Result<Op<String, ZoneEntry>, ditto::Error> {
        todo!()
    }

    pub fn remove_dns_local(&mut self, _id: String) -> Option<Result<Op<String, ZoneEntry>, ditto::Error>> {
        todo!()
    }

    pub fn peer_op(&mut self, op: Op<String, CrdtPeer>, site_id: u32)  -> Result<(), ditto::Error> {
        self.peers.validate_and_execute_op(op, site_id)?;
        Ok(())
    }

    pub fn cidr_op(&mut self, op: Op<String, CrdtCidr>, site_id: u32) -> Result<(), ditto::Error> {
        self.cidrs.validate_and_execute_op(op, site_id)?;
        Ok(())
    }

    pub fn associations_op(&mut self, op: Op<String, CrdtAssociation>, site_id: u32) -> Result<(), ditto::Error> {
        self.associations.validate_and_execute_op(op, site_id)?;
        Ok(())
    }

    fn handle_cached_peer_ops(&mut self, ops: Vec<Op<String, CrdtPeer>>) -> Result<(), ditto::Error> {
        for op in ops {
            self.peers.execute_op(op);
        }

        Ok(())
    }

    fn handle_cached_cidr_ops(&mut self, ops: Vec<Op<String, CrdtCidr>>) -> Result<(), ditto::Error> {
        for op in ops {
            self.cidrs.execute_op(op);
        }
        Ok(())
    }

    fn handle_cached_assoc_ops(&mut self, ops: Vec<Op<String, CrdtAssociation>>) -> Result<(), ditto::Error> {
        for op in ops {
            self.associations.execute_op(op);
        }

        Ok(())
    }

    fn handle_cached_dns_ops(&mut self, ops: Vec<Op<String, DnsZone>>) -> Result<(), ditto::Error> {
        for op in ops {
            self.dns_state.execute_op(op);
        }

        Ok(())
    }
}
