use crate::ServerError;
use form_p2p::queue::{QueueRequest, QueueResponse, QUEUE_PORT};
use form_state::datastore::CidrRequest;
use form_types::state::{Response, Success};
use ipnet::IpNet;
use rusqlite::{params, Connection};
use shared::{Cidr, CidrContents};
use tiny_keccak::{Hasher, Sha3};
use std::{fmt::Display, marker::PhantomData, ops::{Deref, DerefMut}};

use super::{CrdtMap, Sqlite};

pub static CREATE_TABLE_SQL: &str = "CREATE TABLE cidrs (
      id               INTEGER PRIMARY KEY,
      name             TEXT NOT NULL UNIQUE,
      ip               TEXT NOT NULL,
      prefix           INTEGER NOT NULL,
      parent           INTEGER REFERENCES cidrs,
      UNIQUE(ip, prefix),
      FOREIGN KEY (parent)
         REFERENCES cidrs (id)
            ON UPDATE RESTRICT
            ON DELETE RESTRICT
    )";

pub struct DatabaseCidr<T: Display + Clone + PartialEq, D> {
    inner: Cidr<T>,
    marker: PhantomData<D>
}

impl<T: Display + Clone + PartialEq> From<Cidr<T>> for DatabaseCidr<T, Sqlite> {
    fn from(inner: Cidr<T>) -> Self {
        Self { inner, marker: PhantomData }
    }
}

impl<T: Display + Clone + PartialEq> From<Cidr<T>> for DatabaseCidr<T, CrdtMap> {
    fn from(inner: Cidr<T>) -> Self {
        Self { inner, marker: PhantomData }
    }
}

impl<T: Display + Clone + PartialEq> Deref for DatabaseCidr<T, Sqlite> {
    type Target = Cidr<T>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T: Display + Clone + PartialEq> DerefMut for DatabaseCidr<T, Sqlite> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<T: Display + Clone + PartialEq> Deref for DatabaseCidr<T, CrdtMap> {
    type Target = Cidr<T>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T: Display + Clone + PartialEq> DerefMut for DatabaseCidr<T, CrdtMap> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}


impl DatabaseCidr<String, CrdtMap> {
    pub async fn create(contents: CidrContents<String>) -> Result<Cidr<String>, ServerError> {

        let client = reqwest::Client::new();

        if let Some(parent) = &contents.parent {
            let attached_peers = client 
                .get(format!("http://127.0.0.1:3004/user/{}/list", parent))
                .send()
                .await.map_err(|_| ServerError::NotFound)?
                .json::<Response<Cidr<String>>>()
                .await.map_err(|_| ServerError::NotFound)?;

            match attached_peers {
                Response::Success(Success::List(peers)) => {
                    if peers.len() > 0 {
                        log::warn!("tried to add a CIDR to a parent that has peers assigned to it.");
                        return Err(ServerError::InvalidQuery)
                    }
                }
                _ => {}
            }

            let cidrs = Self::list().await?;
            let closest_parent = cidrs.iter()
                .filter(|current| current.cidr.contains(&contents.cidr))
                .max_by_key(|current| current.cidr.prefix_len());

            if let Some(closest) = closest_parent {
                if closest.id != *parent {
                    log::warn!("tried to add a CIDR at the incrrect place in the tree (should be added to {}).", closest.name);
                    return Err(ServerError::InvalidQuery)
                }
            } else {
                log::warn!("tried to add a CIDR outside of the root network range.");
                return Err(ServerError::InvalidQuery);
            }
        }

        let overlapping_sibling = Self::list().await?
            .iter()
            .filter(|current| current.parent == contents.parent)
            .map(|sibling| sibling.cidr)
            .any(|sibling| {
                contents.cidr.contains(&sibling.network())
                    || contents.cidr.contains(&sibling.broadcast())
                    || sibling.contains(&contents.cidr.network())
                    || sibling.contains(&contents.cidr.broadcast())
            });

        if overlapping_sibling {
            log::warn!("tried to add a CIDR that overlaps with a sibling.");
            return Err(ServerError::InvalidQuery);
        }

        let request = Self::build_cidr_queue_request(CidrRequest::Create(contents.clone()))
            .map_err(|_| ServerError::InvalidQuery)?;

        let resp = client 
            .post(format!("http://127.0.0.1:{}/queue/write_local", QUEUE_PORT))
            .json(&request)
            .send()
            .await.map_err(|_| ServerError::InvalidQuery)?
            .json::<QueueResponse>()
            .await.map_err(|_| ServerError::NotFound)?;

        let db_cidr = DatabaseCidr {
            inner: Cidr {
                id: contents.name.clone(),
                contents: contents.clone()
            },
            marker: PhantomData::<String>
        };
        match resp {
            QueueResponse::OpSuccess => {
                return Ok(db_cidr.inner)
            }
            _ => return Err(ServerError::NotFound),
        }
    }

    pub fn build_cidr_queue_request(request: CidrRequest) -> Result<QueueRequest, Box<dyn std::error::Error>> {
        let mut message_code = vec![1];
        message_code.extend(serde_json::to_vec(&request)?);
        let topic = b"state";
        let mut hasher = Sha3::v256();
        let mut topic_hash = [0u8; 32];
        hasher.update(topic);
        hasher.finalize(&mut topic_hash);
        let queue_request = QueueRequest::Write { content: message_code, topic: topic_hash };
        Ok(queue_request)
    }

    pub async fn update(&mut self, contents: CidrContents<String>) -> Result<(), ServerError> {
        let new_contents = CidrContents {
            name: contents.name,
            ..self.contents.clone()
        };

        let request = Self::build_cidr_queue_request(CidrRequest::Update(new_contents.clone()))
            .map_err(|_| ServerError::InvalidQuery)?;

        let resp = reqwest::Client::new() 
            .post(format!("http://127.0.0.1:{}/queue/write_local", QUEUE_PORT))
            .json(&request)
            .send()
            .await.map_err(|_| ServerError::InvalidQuery)?
            .json::<QueueResponse>()
            .await.map_err(|_| ServerError::NotFound)?;

        match resp {
            QueueResponse::OpSuccess => {
                self.contents = new_contents;
                return Ok(())
            }
            QueueResponse::Failure { .. }=> {
                return Err(ServerError::NotFound)
            }
            _ => return Err(ServerError::Unauthorized)
        }
    }

    pub async fn delete(id: String) -> Result<(), ServerError> {
        let request = CidrRequest::Delete(id.to_string());
        let request = Self::build_cidr_queue_request(request)
            .map_err(|_| ServerError::InvalidQuery)?;

        let resp = reqwest::Client::new() 
            .post(format!("http://127.0.0.1:{}/queue/write_local", QUEUE_PORT))
            .json(&request)
            .send()
            .await.map_err(|_| ServerError::InvalidQuery)?
            .json::<QueueResponse>()
            .await.map_err(|_| ServerError::NotFound)?;

        match resp {
            QueueResponse::OpSuccess => {
                return Ok(())
            }
            QueueResponse::Failure { .. }=> {
                return Err(ServerError::NotFound)
            }
            _ => return Err(ServerError::Unauthorized)
        }
    }

    pub async fn get(id: String) -> Result<Cidr<String>, ServerError> {
        let resp = reqwest::Client::new()
            .get(format!("http://127.0.0.1:3004/cidr/{id}/get"))
            .send()
            .await.map_err(|_| ServerError::InvalidQuery)?
            .json::<Response<Cidr<String>>>()
            .await.map_err(|_| ServerError::NotFound)?;

        match resp {
            Response::Success(Success::Some(cidr)) => {
                let db_cidr: DatabaseCidr<String, CrdtMap> = cidr.into();
                Ok(db_cidr.inner)
            }
            _ => {
                return Err(ServerError::NotFound)
            }
        }
    }

    pub async fn list() -> Result<Vec<Cidr<String>>, ServerError> {
        let resp = reqwest::Client::new()
            .get("http://127.0.0.1:3004/cidr/list")
            .send()
            .await.map_err(|_| ServerError::InvalidQuery)?
            .json::<Response<Cidr<String>>>()
            .await.map_err(|_| ServerError::NotFound)?;

        match resp {
            Response::Success(Success::List(list)) => {
                let cidr_list = list.iter().map(|cidr| {
                    let db_cidr = DatabaseCidr::<String, CrdtMap>::from(cidr.clone());
                    db_cidr.inner
                }).collect();
                return Ok(cidr_list)
            }
            _ => {
                return Err(ServerError::NotFound)
            }
        }
    }
}

impl DatabaseCidr<i64, Sqlite> {
    pub fn create(conn: &Connection, contents: CidrContents<i64>) -> Result<Cidr<i64>, ServerError> {
        let CidrContents { name, cidr, parent } = &contents;

        log::debug!("creating {:?}", contents);

        let attached_peers = conn.query_row(
            "SELECT COUNT(*) FROM peers WHERE cidr_id = ?1",
            params![parent],
            |row| row.get::<_, u32>(0),
        )?;
        if attached_peers > 0 {
            log::warn!("tried to add a CIDR to a parent that has peers assigned to it.");
            return Err(ServerError::InvalidQuery);
        }

        if let Some(parent_id) = parent {
            let cidrs = Self::list(conn)?;

            let closest_parent = cidrs
                .iter()
                .filter(|current| current.cidr.contains(cidr))
                .max_by_key(|current| current.cidr.prefix_len());

            if let Some(closest_parent) = closest_parent {
                if closest_parent.id != *parent_id {
                    log::warn!("tried to add a CIDR at the incorrect place in the tree (should be added to {}).", closest_parent.name);
                    return Err(ServerError::InvalidQuery);
                }
            } else {
                log::warn!("tried to add a CIDR outside of the root network range.");
                return Err(ServerError::InvalidQuery);
            }

            let parent_cidr = Self::get(conn, *parent_id)?.cidr;
            if !parent_cidr.contains(&cidr.network()) || !parent_cidr.contains(&cidr.broadcast()) {
                log::warn!("tried to add a CIDR with a network range outside of its parent.");
                return Err(ServerError::InvalidQuery);
            }
        }

        let overlapping_sibling = Self::list(conn)?
            .iter()
            .filter(|current| current.parent == *parent)
            .map(|sibling| sibling.cidr)
            .any(|sibling| {
                cidr.contains(&sibling.network())
                    || cidr.contains(&sibling.broadcast())
                    || sibling.contains(&cidr.network())
                    || sibling.contains(&cidr.broadcast())
            });

        if overlapping_sibling {
            log::warn!("tried to add a CIDR that overlaps with a sibling.");
            return Err(ServerError::InvalidQuery);
        }

        conn.execute(
            "INSERT INTO cidrs (name, ip, prefix, parent)
              VALUES (?1, ?2, ?3, ?4)",
            params![
                name,
                cidr.addr().to_string(),
                cidr.prefix_len() as i32,
                parent
            ],
        )?;
        let id = conn.last_insert_rowid();
        Ok(Cidr { id, contents })
    }

    /// Update self with new contents, validating them and updating the backend in the process.
    /// Currently this only supports updating the name and ignores changes to any other field.
    pub fn update(&mut self, conn: &Connection, contents: CidrContents<i64>) -> Result<(), ServerError> {
        let new_contents = CidrContents {
            name: contents.name,
            ..self.contents.clone()
        };

        conn.execute(
            "UPDATE cidrs SET name = ?2 WHERE id = ?1",
            params![self.id, &*new_contents.name,],
        )?;

        self.contents = new_contents;
        Ok(())
    }

    pub fn delete(conn: &Connection, id: i64) -> Result<(), ServerError> {
        conn.execute("DELETE FROM cidrs WHERE id = ?1", params![id])?;
        Ok(())
    }

    fn from_row(row: &rusqlite::Row) -> Result<Cidr<i64>, rusqlite::Error> {
        let id = row.get(0)?;
        let name = row.get(1)?;
        let ip_str: String = row.get(2)?;
        let prefix = row.get(3)?;
        let ip = ip_str
            .parse()
            .map_err(|_| rusqlite::Error::ExecuteReturnedResults)?;
        let cidr = IpNet::new(ip, prefix).map_err(|_| rusqlite::Error::ExecuteReturnedResults)?;
        let parent = row.get(4)?;
        Ok(Cidr {
            id,
            contents: CidrContents { name, cidr, parent },
        })
    }

    pub fn get(conn: &Connection, id: i64) -> Result<Cidr<i64>, ServerError> {
        Ok(conn.query_row(
            "SELECT id, name, ip, prefix, parent FROM cidrs WHERE id = ?1",
            params![id],
            Self::from_row,
        )?)
    }

    pub fn list(conn: &Connection) -> Result<Vec<Cidr<i64>>, ServerError> {
        let mut stmt = conn.prepare_cached("SELECT id, name, ip, prefix, parent FROM cidrs")?;
        let cidr_iter = stmt.query_map(params![], Self::from_row)?;

        Ok(cidr_iter.collect::<Result<Vec<_>, rusqlite::Error>>()?)
    }
}
