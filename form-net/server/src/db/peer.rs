use super::{CrdtMap, DatabaseCidr, Sqlite};
use crate::ServerError;
use form_state::{datastore::{DeleteExpiredResponse, GetPeerListResponse, GetPeerResponse, NewPeerRequest, PeerResponse, UpdatePeerRequest}, network::CrdtPeer};
use once_cell::sync::Lazy;
use regex::Regex;
use rusqlite::{params, types::Type, Connection};
use shared::{IpNetExt, Peer, PeerContents, PERSISTENT_KEEPALIVE_INTERVAL_SECS};
use std::{
    marker::PhantomData, net::IpAddr, ops::{Deref, DerefMut}, time::{Duration, SystemTime}
};

pub static CREATE_TABLE_SQL: &str = "CREATE TABLE peers (
      id              INTEGER PRIMARY KEY,
      name            TEXT NOT NULL UNIQUE,         /* The canonical name for the peer in canonical hostname(7) format. */
      ip              TEXT NOT NULL UNIQUE,         /* The WireGuard-internal IP address assigned to the peer.          */
      public_key      TEXT NOT NULL UNIQUE,         /* The WireGuard public key of the peer.                            */
      endpoint        TEXT,                         /* The optional external endpoint ([ip]:[port]) of the peer.        */
      cidr_id         INTEGER NOT NULL,             /* The ID of the peer's parent CIDR.                                */
      is_admin        INTEGER DEFAULT 0 NOT NULL,   /* Admin capabilities are per-peer, not per-CIDR.                   */
      is_disabled     INTEGER DEFAULT 0 NOT NULL,   /* Is the peer disabled? (peers cannot be deleted)                  */
      is_redeemed     INTEGER DEFAULT 0 NOT NULL,   /* Has the peer redeemed their invite yet?                          */
      invite_expires  INTEGER,                      /* The UNIX time that an invited peer can no longer redeem.         */
      candidates      TEXT,                         /* A list of additional endpoints that peers can use to connect.    */
      FOREIGN KEY (cidr_id)
         REFERENCES cidrs (id)
            ON UPDATE RESTRICT
            ON DELETE RESTRICT
    )";

pub static COLUMNS: &[&str] = &[
    "id",
    "name",
    "ip",
    "cidr_id",
    "public_key",
    "endpoint",
    "is_admin",
    "is_disabled",
    "is_redeemed",
    "invite_expires",
    "candidates",
];

/// Regex to match the requirements of hostname(7), needed to have peers also be reachable hostnames.
/// Note that the full length also must be maximum 63 characters, which this regex does not check.
static PEER_NAME_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^([a-z0-9]-?)*[a-z0-9]$").unwrap());

#[derive(Debug)]
pub struct DatabasePeer<D> {
    pub inner: Peer,
    marker: PhantomData<D>
}

impl From<Peer> for DatabasePeer<CrdtMap> {
    fn from(inner: Peer) -> Self {
        Self { inner, marker: PhantomData }
    }
}

impl TryFrom<CrdtPeer> for DatabasePeer<CrdtMap> {
    type Error = ServerError;
    fn try_from(value: CrdtPeer) -> Result<Self, Self::Error> {
        Ok(Self {
            inner: Peer::try_from(value).map_err(|_| ServerError::InvalidQuery)?,
            marker: PhantomData
        })
    }
}

impl Deref for DatabasePeer<CrdtMap> {
    type Target = Peer;

    fn deref(&self) -> &Self::Target {
        &self.inner
    } 
}

impl DerefMut for DatabasePeer<CrdtMap> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl From<Peer> for DatabasePeer<Sqlite> {
    fn from(inner: Peer) -> Self {
        Self { inner, marker: PhantomData }
     }
}

impl Deref for DatabasePeer<Sqlite> {
    type Target = Peer;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for DatabasePeer<Sqlite> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<D> DatabasePeer<D> {
    fn is_valid_name(name: &str) -> bool {
        name.len() < 64 && PEER_NAME_REGEX.is_match(name)
    }
}

impl DatabasePeer<CrdtMap> {
    pub async fn create(contents: PeerContents) -> Result<Self, ServerError> {
        if !Self::is_valid_name(&contents.name) {
            log::warn!("Peer name is invalid, must confirm to hostname(7) requirements");
            return Err(ServerError::InvalidQuery);
        }

        let cidr = DatabaseCidr::<CrdtMap>::get(contents.cidr_id).await?;
        if !cidr.cidr.contains(&contents.ip) {
            log::warn!("Tried to add peer with IP outside of parent CIDR range.");
            return Err(ServerError::InvalidQuery);
        }

        if !cidr.cidr.is_assignable(&contents.ip) {
            log::warn!("Peer IP {} is not unicast assignable in CIDR {}", contents.ip, cidr.cidr);
            return Err(ServerError::InvalidQuery);
        }

        let request = NewPeerRequest::Join(contents);

        let resp = reqwest::Client::new()
            .post("http://127.0.0.1:3004/user/create")
            .json(&request)
            .send()
            .await.map_err(|_| ServerError::NotFound)?
            .json::<PeerResponse>()
            .await.map_err(|_| ServerError::NotFound)?;

        match resp {
            PeerResponse::Success(Some(peer)) => Ok(peer.into()),
            PeerResponse::Success(None) => Err(ServerError::NotFound),
            PeerResponse::Failure => Err(ServerError::NotFound)
        }
    }

    pub async fn update(&mut self, contents: PeerContents) -> Result<(), ServerError> {
        if !Self::is_valid_name(&contents.name) {
            log::warn!("peer name is invalid, must conform to hostname(7) requirements.");
            return Err(ServerError::InvalidQuery);
        }

        // We will only allow updates of certain fields at this point, disregarding any requests
        // for changes of IP address, public key, or parent CIDR, for security reasons.
        //
        // In the future, we may allow re-assignments of peers to new CIDRs, but it's easiest to
        // disregard that case for now to prevent possible attacks.
        let new_contents = PeerContents {
            name: contents.name,
            endpoint: contents.endpoint,
            is_admin: contents.is_admin,
            is_disabled: contents.is_disabled,
            candidates: contents.candidates,
            ..self.contents.clone()
        };

        let request = UpdatePeerRequest::Update(new_contents.clone());

        let resp = reqwest::Client::new()
            .post("http://127.0.0.1:3004/user/update")
            .json(&request)
            .send()
            .await.map_err(|_| ServerError::NotFound)?
            .json::<PeerResponse>()
            .await.map_err(|_| ServerError::NotFound)?;

        match resp {
            PeerResponse::Success(_) => {
                self.contents = new_contents;
                return Ok(())
            }
            PeerResponse::Failure => Err(ServerError::NotFound)
        }

    }

    pub async fn disable(id: i64) -> Result<(), ServerError> {
        let resp = reqwest::Client::new()
            .get(format!("http://127.0.0.1:3004/user/{id}/get"))
            .send()
            .await.map_err(|_| ServerError::InvalidQuery)?
            .json()
            .await.map_err(|_| ServerError::NotFound)?;
        
        let peer_contents = match resp {
            GetPeerResponse::Success(peer) => {
                let db_peer = DatabasePeer::<CrdtMap>::try_from(peer)?;
                db_peer.contents.clone()
            }
            GetPeerResponse::Failure => {
                return Err(ServerError::NotFound)
            }
        };

        let new_contents = PeerContents {
            is_disabled: true,
            ..peer_contents.clone()
        };

        let request = UpdatePeerRequest::Update(new_contents.clone()); 
        let resp = reqwest::Client::new()
            .post("http://127.0.0.1:3004/user/disable")
            .json(&request)
            .send()
            .await.map_err(|_| ServerError::NotFound)?
            .json::<PeerResponse>()
            .await.map_err(|_| ServerError::NotFound)?;

        match resp {
            PeerResponse::Success(_) => Ok(()),
            PeerResponse::Failure => Err(ServerError::NotFound)
        }
    }

    pub async fn redeem(&self) -> Result<(), ServerError> {
        let new_contents = PeerContents {
            is_redeemed: true,
            ..self.contents.clone()
        };
        let request = UpdatePeerRequest::Update(new_contents.clone());
        let resp = reqwest::Client::new()
            .post("http://127.0.0.1:3004/user/update")
            .json(&request)
            .send()
            .await.map_err(|_| ServerError::NotFound)?
            .json::<PeerResponse>()
            .await.map_err(|_| ServerError::NotFound)?;

        match resp {
            PeerResponse::Success(_) => Ok(()),
            PeerResponse::Failure => Err(ServerError::NotFound)
        }
    }

    pub async fn get(id: i64) -> Result<Self, ServerError> {
        let resp = reqwest::Client::new()
            .get(format!("http://127.0.0.1:3004/user/{id}/get"))
            .send()
            .await.map_err(|_| ServerError::NotFound)?
            .json::<GetPeerResponse>()
            .await.map_err(|_| ServerError::NotFound)?;

        match resp {
            GetPeerResponse::Success(peer) => {
                let peer: Peer = peer.try_into().map_err(|_| ServerError::NotFound)?;
                Ok(peer.into())
            }
            GetPeerResponse::Failure => Err(ServerError::NotFound)
        }
    }

    pub async fn get_from_ip(ip: IpAddr) -> Result<Self, ServerError> {
        let resp = reqwest::Client::new()
            .get(format!("http://127.0.0.1:3004/user/{ip}/get_from_ip"))
            .send()
            .await.map_err(|_| ServerError::NotFound)?
            .json::<GetPeerResponse>()
            .await.map_err(|_| ServerError::NotFound)?;

        match resp {
            GetPeerResponse::Success(peer) => {
                let peer: Peer = peer.try_into().map_err(|_| ServerError::NotFound)?;
                Ok(peer.into())
            }
            GetPeerResponse::Failure => Err(ServerError::NotFound)
        }
    }

    pub async fn get_all_allowed_peers(&self) -> Result<Vec<Self>, ServerError> {
        let id = self.inner.id;
        let resp = reqwest::Client::new()
            .get(format!("http://127.0.0.1:3004/user/{id}/get_all_allowed"))
            .send()
            .await.map_err(|_| ServerError::NotFound)?
            .json::<GetPeerListResponse>()
            .await.map_err(|_| ServerError::NotFound)?;

        match resp {
            GetPeerListResponse::Success(peers) => {
                let peers = peers.iter().filter_map(|p| {
                    match Peer::try_from(p.clone()) {
                        Ok(peer) => Some(peer.into()),
                        Err(_) => None
                    }
                }).collect();

                Ok(peers)
            }
            GetPeerListResponse::Failure => Err(ServerError::NotFound)
        }
    }

    pub async fn list() -> Result<Vec<Self>, ServerError> {
        let resp = reqwest::Client::new()
            .get("http://127.0.0.1:3004/user/list")
            .send()
            .await.map_err(|_| ServerError::NotFound)?
            .json::<GetPeerListResponse>()
            .await.map_err(|_| ServerError::NotFound)?;

        match resp {
            GetPeerListResponse::Success(peers) => {
                let peers = peers.iter().filter_map(|p| {
                    match Peer::try_from(p.clone()) {
                        Ok(peer) => Some(peer.into()),
                        Err(_) => None
                    }
                }).collect();

                Ok(peers)
            }
            GetPeerListResponse::Failure => Err(ServerError::NotFound)
        }
    }

    pub async fn delete_expired_invites() -> Result<(), ServerError> {
        let resp = reqwest::Client::new()
            .get("http://127.0.0.1:3004/user/delete_expired")
            .send()
            .await.map_err(|_| ServerError::NotFound)?
            .json::<DeleteExpiredResponse>()
            .await.map_err(|_| ServerError::NotFound)?;

        match resp {
            DeleteExpiredResponse::Success => Ok(()),
            DeleteExpiredResponse::Failure => Err(ServerError::NotFound)
        }
    }
}

impl DatabasePeer<Sqlite> {
    pub fn create(conn: &Connection, contents: PeerContents) -> Result<Self, ServerError> {
        let PeerContents {
            name,
            ip,
            cidr_id,
            public_key,
            endpoint,
            is_admin,
            is_disabled,
            is_redeemed,
            invite_expires,
            candidates,
            ..
        } = &contents;
        log::info!("creating peer {:?}", contents);
        println!("creating peer {:?}", contents);

        if !Self::is_valid_name(name) {
            log::warn!("peer name is invalid, must conform to hostname(7) requirements.");
            println!("peer name is invalid, must conform to hostname(7) requirements.");
            return Err(ServerError::InvalidQuery);
        }

        let cidr = DatabaseCidr::<Sqlite>::get(conn, *cidr_id)?;
        if !cidr.cidr.contains(ip) {
            log::warn!("tried to add peer with IP outside of parent CIDR range.");
            println!("tried to add peer with IP outside of parent CIDR range.");
            return Err(ServerError::InvalidQuery);
        }

        if !cidr.cidr.is_assignable(ip) {
            println!(
                "Peer IP {} is not unicast assignable in CIDR {}",
                ip, cidr.cidr
            );
            return Err(ServerError::InvalidQuery);
        }

        let invite_expires = invite_expires
            .map(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
            .flatten()
            .map(|t| t.as_secs());

        let candidates = serde_json::to_string(candidates)?;

        println!("Executing SQL insert...");
        let params = params![
                &**name,
                ip.to_string(),
                cidr_id,
                &public_key,
                endpoint.as_ref().map(|endpoint| endpoint.to_string()),
                is_admin,
                is_disabled,
                is_redeemed,
                invite_expires,
                candidates,
            ];
        conn.execute(
            &format!(
                "INSERT INTO peers ({}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                COLUMNS[1..].join(", ")
            ),
            params
        )?;
        println!("Executed SQL insert...");
        let id = conn.last_insert_rowid();
        Ok(Peer { id, contents }.into())
    }


    /// Update self with new contents, validating them and updating the backend in the process.
    pub fn update(&mut self, conn: &Connection, contents: PeerContents) -> Result<(), ServerError> {
        if !Self::is_valid_name(&contents.name) {
            log::warn!("peer name is invalid, must conform to hostname(7) requirements.");
            return Err(ServerError::InvalidQuery);
        }

        // We will only allow updates of certain fields at this point, disregarding any requests
        // for changes of IP address, public key, or parent CIDR, for security reasons.
        //
        // In the future, we may allow re-assignments of peers to new CIDRs, but it's easiest to
        // disregard that case for now to prevent possible attacks.
        let new_contents = PeerContents {
            name: contents.name,
            endpoint: contents.endpoint,
            is_admin: contents.is_admin,
            is_disabled: contents.is_disabled,
            candidates: contents.candidates,
            ..self.contents.clone()
        };

        let new_candidates = serde_json::to_string(&new_contents.candidates)?;
        conn.execute(
            "UPDATE peers SET
                name = ?2,
                endpoint = ?3,
                is_admin = ?4,
                is_disabled = ?5,
                candidates = ?6
            WHERE id = ?1",
            params![
                self.id,
                &*new_contents.name,
                new_contents
                    .endpoint
                    .as_ref()
                    .map(|endpoint| endpoint.to_string()),
                new_contents.is_admin,
                new_contents.is_disabled,
                new_candidates,
            ],
        )?;

        self.contents = new_contents;
        Ok(())
    }

    pub fn disable(conn: &Connection, id: i64) -> Result<(), ServerError> {
        match conn.execute(
            "UPDATE peers SET is_disabled = 1 WHERE id = ?1",
            params![id],
        )? {
            0 => Err(ServerError::NotFound),
            _ => Ok(()),
        }
    }

    pub fn redeem(&mut self, conn: &Connection, pubkey: &str) -> Result<(), ServerError> {
        if self.is_redeemed {
            return Err(ServerError::Gone);
        }

        if matches!(self.invite_expires, Some(time) if time < SystemTime::now()) {
            return Err(ServerError::Unauthorized);
        }

        match conn.execute(
            "UPDATE peers SET is_redeemed = 1, public_key = ?1 WHERE id = ?2 AND is_redeemed = 0",
            params![pubkey, self.id],
        )? {
            0 => Err(ServerError::NotFound),
            _ => {
                self.contents.public_key = pubkey.into();
                self.contents.is_redeemed = true;
                Ok(())
            },
        }
    }

    fn from_row(row: &rusqlite::Row) -> Result<Self, rusqlite::Error> {
        let id = row.get(0)?;
        let name = row
            .get::<_, String>(1)?
            .parse()
            .map_err(|_| rusqlite::Error::InvalidColumnType(1, "hostname".into(), Type::Text))?;
        let ip: IpAddr = row
            .get::<_, String>(2)?
            .parse()
            .map_err(|_| rusqlite::Error::InvalidColumnType(2, "ip".into(), Type::Text))?;
        let cidr_id = row.get(3)?;
        let public_key = row.get(4)?;
        let endpoint = row
            .get::<_, Option<String>>(5)?
            .and_then(|endpoint| endpoint.parse().ok());
        let is_admin = row.get(6)?;
        let is_disabled = row.get(7)?;
        let is_redeemed = row.get(8)?;
        let invite_expires = row
            .get::<_, Option<u64>>(9)?
            .map(|unixtime| SystemTime::UNIX_EPOCH + Duration::from_secs(unixtime));

        let candidates = if let Some(candidates) = row.get::<_, Option<String>>(10)? {
            serde_json::from_str(&candidates).map_err(|_| {
                rusqlite::Error::InvalidColumnType(10, "candidates (json)".into(), Type::Text)
            })?
        } else {
            vec![]
        };

        let persistent_keepalive_interval = Some(PERSISTENT_KEEPALIVE_INTERVAL_SECS);

        Ok(Peer {
            id,
            contents: PeerContents {
                name,
                ip,
                cidr_id,
                public_key,
                endpoint,
                persistent_keepalive_interval,
                is_admin,
                is_disabled,
                is_redeemed,
                invite_expires,
                candidates,
            },
        }
        .into())
    }

    pub fn get(conn: &Connection, id: i64) -> Result<Self, ServerError> {
        let result = conn.query_row(
            &format!("SELECT {} FROM peers WHERE id = ?1", COLUMNS.join(", ")),
            params![id],
            Self::from_row,
        )?;

        Ok(result)
    }

    pub fn get_from_ip(conn: &Connection, ip: IpAddr) -> Result<Self, rusqlite::Error> {
        let result = conn.query_row(
            &format!("SELECT {} FROM peers WHERE ip = ?1", COLUMNS.join(", ")),
            params![ip.to_string()],
            Self::from_row,
        )?;

        Ok(result)
    }

    pub fn get_all_allowed_peers(&self, conn: &Connection) -> Result<Vec<Self>, ServerError> {
        // This query is a handful, so an explanation of what's happening, and what each CTE does (https://sqlite.org/lang_with.html):
        //
        // 1. parent_of: Enumerate all ancestor CIDRs of the CIDR associated with peer.
        // 2. associated: Enumerate all auth associations between any of the above enumerated CIDRs.
        // 3. associated_subcidrs: For each association, list all peers by enumerating down each
        //    associated CIDR's children and listing any peers belonging to them.
        //
        // NOTE that a forced association is created with the special "infra" CIDR with id 2 (1 being the root).
        let mut stmt = conn.prepare_cached(
            &format!("WITH
                parent_of(id, parent) AS (
                    SELECT id, parent FROM cidrs WHERE id = ?1
                    UNION ALL
                    SELECT cidrs.id, cidrs.parent FROM cidrs JOIN parent_of ON parent_of.parent = cidrs.id
                ),
                associated(cidr_id) as (
                    SELECT associations.cidr_id_2 FROM associations, parent_of WHERE associations.cidr_id_1 = parent_of.id
                    UNION
                    SELECT associations.cidr_id_1 FROM associations, parent_of WHERE associations.cidr_id_2 = parent_of.id
                ),
                associated_subcidrs(cidr_id) AS (
                    VALUES(?1), (2)
                    UNION
                    SELECT cidr_id FROM associated
                    UNION
                    SELECT id FROM cidrs, associated_subcidrs WHERE cidrs.parent=associated_subcidrs.cidr_id
                )
                SELECT DISTINCT {}
                FROM peers
                JOIN associated_subcidrs ON peers.cidr_id=associated_subcidrs.cidr_id
                WHERE peers.is_disabled = 0 AND peers.is_redeemed = 1;",
                COLUMNS.iter().map(|col| format!("peers.{col}")).collect::<Vec<_>>().join(", ")
            ),
        )?;
        let peers = stmt
            .query_map(params![self.cidr_id], Self::from_row)?
            .collect::<Result<_, _>>()?;
        Ok(peers)
    }

    pub fn list(conn: &Connection) -> Result<Vec<Self>, ServerError> {
        let mut stmt = conn.prepare_cached(&format!("SELECT {} FROM peers", COLUMNS.join(", ")))?;
        let peer_iter = stmt.query_map(params![], Self::from_row)?;

        Ok(peer_iter.collect::<Result<_, _>>()?)
    }

    pub fn delete_expired_invites(conn: &Connection) -> Result<usize, ServerError> {
        let unix_now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("Something is horribly wrong with system time.");
        let deleted = conn.execute(
            "DELETE FROM peers
            WHERE is_redeemed = 0 AND invite_expires < ?1",
            params![unix_now.as_secs()],
        )?;

        Ok(deleted)
    }
}
