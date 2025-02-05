//! A table to describe which CIDRs another CIDR is allowed to peer with.
//!
//! A peer belongs to one parent CIDR, and can by default see all peers within that parent.

use crate::ServerError;
use form_p2p::queue::{QueueRequest, QueueResponse, QUEUE_PORT};
use form_types::state::{Response, Success};
use form_state::datastore::AssocRequest;
use rusqlite::{params, Connection};
use shared::{Cidr, Association, AssociationContents};
use tiny_keccak::{Hasher, Sha3};
use std::{fmt::Display, marker::PhantomData, ops::{Deref, DerefMut}};

use super::{CrdtMap, Sqlite};

pub static CREATE_TABLE_SQL: &str = "CREATE TABLE associations (
      id         INTEGER PRIMARY KEY,
      cidr_id_1  INTEGER NOT NULL,
      cidr_id_2  INTEGER NOT NULL,
      UNIQUE(cidr_id_1, cidr_id_2),
      FOREIGN KEY (cidr_id_1)
         REFERENCES cidrs (id) 
            ON UPDATE RESTRICT
            ON DELETE RESTRICT,
      FOREIGN KEY (cidr_id_2)
         REFERENCES cidrs (id) 
            ON UPDATE RESTRICT
            ON DELETE RESTRICT
    )";

#[derive(Debug)]
pub struct DatabaseAssociation<D, T, K> 
where 
    T: Display + Clone + PartialEq,
    K: Display + Clone + PartialEq
{
    pub inner: Association<T, K>,
    marker: PhantomData<D>
}

impl<D, T: Display + Clone + PartialEq, K: Display + Clone + PartialEq> From<Association<T, K>> for DatabaseAssociation<D, T, K> {
    fn from(inner: Association<T, K>) -> Self {
        Self { inner, marker: PhantomData }
    }
}

impl<D, T: Display + Clone + PartialEq, K: Display + Clone + PartialEq> Deref for DatabaseAssociation<D, T, K> {
    type Target = Association<T, K>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<D, T: Display + Clone + PartialEq, K: Display + Clone + PartialEq> DerefMut for DatabaseAssociation<D, T, K> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl DatabaseAssociation<CrdtMap, String, String> {
    pub async fn create(contents: AssociationContents<String>) -> Result<Association<String, String>, ServerError> {
        let cidr_1 = contents.cidr_id_1.clone();
        let cidr_2 = contents.cidr_id_2.clone();
        let _cidr_1_resp = reqwest::Client::new()
            .get(format!("http://127.0.0.1:3004/assoc/{cidr_1}/relationships"))
            .send()
            .await.map_err(|_| ServerError::InvalidQuery)?
            .json::<Response<Association<String, String>>>()
            .await.map_err(|_| ServerError::NotFound)?;

        let _cidr_2_resp = reqwest::Client::new()
            .get(format!("http://127.0.0.1:3004/assoc/{cidr_2}/relationships"))
            .send()
            .await.map_err(|_| ServerError::InvalidQuery)?
            .json::<Response<Association<String, String>>>()
            .await.map_err(|_| ServerError::NotFound)?;

        /*
        let existing = match (cidr_1_resp, cidr_2_resp) {
            todo!()
        };

        if !existing.is_empty() {
            return Err(ServerError::InvalidQuery);
        }
        */

        let cidr_1_resp = reqwest::Client::new()
            .get(format!("http://127.0.0.1:3004/cidr/{cidr_1}/get"))
            .send()
            .await.map_err(|_| ServerError::InvalidQuery)?
            .json::<Response<Cidr<String>>>()
            .await.map_err(|_| ServerError::NotFound)?;

        let cidr_2_resp = reqwest::Client::new()
            .get(format!("http://127.0.0.1:3004/cidr/{cidr_2}/get"))
            .send()
            .await.map_err(|_| ServerError::InvalidQuery)?
            .json::<Response<Cidr<String>>>()
            .await.map_err(|_| ServerError::NotFound)?;

        if let (Response::Failure { .. }, Response::Failure { .. }) = (cidr_1_resp, cidr_2_resp) {
            return Err(ServerError::InvalidQuery);
        }

        let request = Self::build_association_queue_request(AssocRequest::Create(contents.clone()))
            .map_err(|_| ServerError::InvalidQuery)?;

        let resp = reqwest::Client::new()
            .post(format!("http://127.0.0.1:{}/queue/write_local", QUEUE_PORT))
            .json(&request)
            .send()
            .await.map_err(|_| ServerError::InvalidQuery)?
            .json::<QueueResponse>()
            .await.map_err(|_| ServerError::NotFound)?;

        let assoc = Association {
            id: format!("{}-{}", contents.cidr_id_1.clone(), contents.cidr_id_2.clone()),
            contents,
        };

        match resp {
            QueueResponse::OpSuccess => {
                return Ok(assoc)
            }
            _ => return Err(ServerError::NotFound)
        }
    }

    pub fn build_association_queue_request(request: AssocRequest) -> Result<QueueRequest, Box<dyn std::error::Error>> {
        let mut message_code = vec![2];
        message_code.extend(serde_json::to_vec(&request)?);
        let topic = b"state";
        let mut hasher = Sha3::v256();
        let mut topic_hash = [0u8; 32];
        hasher.update(topic);
        hasher.finalize(&mut topic_hash);
        let queue_request = QueueRequest::Write { content: message_code, topic: topic_hash };
        Ok(queue_request)
    }

    pub async fn list() -> Result<Vec<Association<String, String>>, ServerError> {
        let resp = reqwest::Client::new()
            .get("http://127.0.0.1:3004/assoc/list")
            .send().await.map_err(|_| ServerError::InvalidQuery)?
            .json::<Response<Association<String, String>>>()
            .await.map_err(|_| ServerError::NotFound)?;

        match resp {
            Response::Success(Success::List(list)) => {
                return Ok(list)
            }
            _ => {
                return Err(ServerError::NotFound)
            }
        }

    }

    pub async fn delete(id: i64) -> Result<(), ServerError> {
        let request = Self::build_association_queue_request(AssocRequest::Delete((id.to_string(), id.to_string())))
            .map_err(|_| ServerError::InvalidQuery)?;
        let resp = reqwest::Client::new()
            .post(format!("http://127.0.0.1:{}/queue/write_local", QUEUE_PORT))
            .json(&request)
            .send()
            .await.map_err(|_| ServerError::InvalidQuery)?
            .json::<QueueResponse>()
            .await.map_err(|_| ServerError::NotFound)?;

        match resp {
            QueueResponse::OpSuccess => return Ok(()),
            _ => return Err(ServerError::NotFound)
        }
    }
}

impl DatabaseAssociation<Sqlite, i64, i64> {
    pub fn create(
        conn: &Connection,
        contents: AssociationContents<i64>,
    ) -> Result<Association<i64, i64>, ServerError> {
        let AssociationContents {
            cidr_id_1,
            cidr_id_2,
        } = &contents;

        // Verify an existing association doesn't currently exist
        let existing_associations: usize = conn.query_row(
            "SELECT COUNT(*)
            FROM associations
            WHERE (cidr_id_1 = ?1 AND cidr_id_2 = ?2) OR (cidr_id_1 = ?2 AND cidr_id_2 = ?1)",
            params![cidr_id_1, cidr_id_2],
            |r| r.get(0),
        )?;
        if existing_associations > 0 {
            return Err(ServerError::InvalidQuery);
        }

        // Verify both provided CIDR IDs exist
        let existing_cidrs: usize = conn.query_row(
            "SELECT COUNT(*)
            FROM cidrs
            WHERE id = ?1 OR id = ?2",
            params![cidr_id_1, cidr_id_2],
            |r| r.get(0),
        )?;
        if existing_cidrs != 2 {
            return Err(ServerError::InvalidQuery);
        }

        conn.execute(
            "INSERT INTO associations (cidr_id_1, cidr_id_2)
              VALUES (?1, ?2)",
            params![cidr_id_1, cidr_id_2],
        )?;
        let id = conn.last_insert_rowid();
        Ok(Association { id, contents })
    }

    pub fn delete(conn: &Connection, id: i64) -> Result<(), ServerError> {
        conn.execute("DELETE FROM associations WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn list(conn: &Connection) -> Result<Vec<Association<i64, i64>>, ServerError> {
        let mut stmt = conn.prepare_cached("SELECT id, cidr_id_1, cidr_id_2 FROM associations")?;
        let auth_iter = stmt.query_map(params![], |row| {
            let id = row.get(0)?;
            let cidr_id_1 = row.get(1)?;
            let cidr_id_2 = row.get(2)?;
            Ok(Association {
                id,
                contents: AssociationContents {
                    cidr_id_1,
                    cidr_id_2,
                },
            })
        })?;

        Ok(auth_iter.collect::<Result<Vec<_>, rusqlite::Error>>()?)
    }
}

#[cfg(test)]
mod tests {
    use crate::test;
    use shared::{CidrContents, Error};

    use super::*;

    #[tokio::test]
    async fn test_double_add() -> Result<(), Error> {
        let server = test::Server::new()?;

        let contents = AssociationContents {
            cidr_id_1: 1,
            cidr_id_2: 2,
        };
        let contents_flipped = AssociationContents {
            cidr_id_1: 2,
            cidr_id_2: 1,
        };
        let res = server
            .form_request(
                test::ADMIN_PEER_IP,
                "POST",
                "/v1/admin/associations",
                &contents,
            )
            .await;
        assert!(res.status().is_success());

        let res = server
            .form_request(
                test::ADMIN_PEER_IP,
                "POST",
                "/v1/admin/associations",
                &contents,
            )
            .await;
        assert!(res.status().is_client_error());

        let res = server
            .form_request(
                test::ADMIN_PEER_IP,
                "POST",
                "/v1/admin/associations",
                &contents_flipped,
            )
            .await;
        assert!(res.status().is_client_error());
        Ok(())
    }

    #[tokio::test]
    async fn test_nonexistent_cidr_id() -> Result<(), Error> {
        let server = test::Server::new()?;

        // Verify both provided CIDR IDs exist
        let last_cidr_id: i64 =
            server
                .db()
                .lock()
                .query_row("SELECT COUNT(*) FROM cidrs", params![], |r| r.get(0))?;
        let contents = AssociationContents {
            cidr_id_1: 1,
            cidr_id_2: last_cidr_id + 1,
        };
        let res = server
            .form_request(
                test::ADMIN_PEER_IP,
                "POST",
                "/v1/admin/associations",
                &contents,
            )
            .await;
        assert!(!res.status().is_success());

        let cidr = CidrContents {
            name: "experimental".to_string(),
            cidr: test::EXPERIMENTAL_CIDR.parse()?,
            parent: Some(test::ROOT_CIDR_ID),
        };

        let res = server
            .form_request(test::ADMIN_PEER_IP, "POST", "/v1/admin/cidrs", &cidr)
            .await;
        assert!(res.status().is_success());

        let res = server
            .form_request(
                test::ADMIN_PEER_IP,
                "POST",
                "/v1/admin/associations",
                &contents,
            )
            .await;
        assert!(res.status().is_success());

        Ok(())
    }
}
