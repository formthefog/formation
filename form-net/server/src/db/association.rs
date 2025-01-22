//! A table to describe which CIDRs another CIDR is allowed to peer with.
//!
//! A peer belongs to one parent CIDR, and can by default see all peers within that parent.

use crate::ServerError;
use form_state::{datastore::{AssociationResponse, CreateAssociationRequest, DeleteAssociationRequest, GetCidrResponse, ListAssociationResponse}, network::CrdtAssociation};
use rusqlite::{params, Connection};
use shared::{Association, AssociationContents};
use std::{marker::PhantomData, ops::{Deref, DerefMut}};

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
pub struct DatabaseAssociation<D> {
    pub inner: Association,
    marker: PhantomData<D>
}

impl From<Association> for DatabaseAssociation<Sqlite> {
    fn from(inner: Association) -> Self {
        Self { inner, marker: PhantomData }
    }
}

impl From<Association> for DatabaseAssociation<CrdtMap> {
    fn from(inner: Association) -> Self {
        Self { inner, marker: PhantomData }
    }
}

impl From<CrdtAssociation> for DatabaseAssociation<CrdtMap> {
    fn from(value: CrdtAssociation) -> Self {
        Self {
            inner: Association { 
                id: value.id(), 
                contents: AssociationContents {
                    cidr_id_1: value.cidr_1(),
                    cidr_id_2: value.cidr_2(),
                }
            },
            marker: PhantomData
        }
    }
}

impl Deref for DatabaseAssociation<Sqlite> {
    type Target = Association;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Deref for DatabaseAssociation<CrdtMap> {
    type Target = Association;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for DatabaseAssociation<Sqlite> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl DatabaseAssociation<CrdtMap> {
    pub async fn create(contents: AssociationContents) -> Result<Association, ServerError> {
        let cidr_1 = contents.cidr_id_1;
        let cidr_2 = contents.cidr_id_2;
        let cidr_1_resp = reqwest::Client::new()
            .get(format!("http://127.0.0.1:3004/assoc/{cidr_1}/relationships"))
            .send()
            .await.map_err(|_| ServerError::InvalidQuery)?
            .json::<ListAssociationResponse>()
            .await.map_err(|_| ServerError::NotFound)?;

        let cidr_2_resp = reqwest::Client::new()
            .get(format!("http://127.0.0.1:3004/assoc/{cidr_2}/relationships"))
            .send()
            .await.map_err(|_| ServerError::InvalidQuery)?
            .json::<ListAssociationResponse>()
            .await.map_err(|_| ServerError::NotFound)?;

        let existing = match (cidr_1_resp, cidr_2_resp) {
            (ListAssociationResponse::Success(mut list_1), ListAssociationResponse::Success(list_2)) => {
                list_1.extend(list_2);
                list_1
            },
            (ListAssociationResponse::Success(list_1), ListAssociationResponse::Failure) => {
                list_1
            },
            (ListAssociationResponse::Failure, ListAssociationResponse::Success(list_2)) => {
                list_2
            }
            _ => vec![]
        };

        if !existing.is_empty() {
            return Err(ServerError::InvalidQuery);
        }

        let cidr_1_resp = reqwest::Client::new()
            .get(format!("http://127.0.0.1:3004/cidr/{cidr_1}/get"))
            .send()
            .await.map_err(|_| ServerError::InvalidQuery)?
            .json::<GetCidrResponse>()
            .await.map_err(|_| ServerError::NotFound)?;

        let cidr_2_resp = reqwest::Client::new()
            .get(format!("http://127.0.0.1:3004/cidr/{cidr_2}/get"))
            .send()
            .await.map_err(|_| ServerError::InvalidQuery)?
            .json::<GetCidrResponse>()
            .await.map_err(|_| ServerError::NotFound)?;

        if let (GetCidrResponse::Failure, GetCidrResponse::Failure) = (cidr_1_resp, cidr_2_resp) {
            return Err(ServerError::InvalidQuery);
        }

        let request = CreateAssociationRequest::Create(contents);

        let resp = reqwest::Client::new()
            .post("http://127.0.0.1:3004/assoc/create")
            .json(&request)
            .send()
            .await.map_err(|_| ServerError::InvalidQuery)?
            .json::<AssociationResponse>()
            .await.map_err(|_| ServerError::NotFound)?;

        match resp {
            AssociationResponse::Success(Some(assoc)) => {
                let db_assoc = DatabaseAssociation::from(assoc.clone());
                return Ok(db_assoc.deref().clone())
            }
            _ => return Err(ServerError::NotFound)
        }
    }

    pub async fn list() -> Result<Vec<Association>, ServerError> {
        let resp = reqwest::Client::new()
            .get("http://127.0.0.1:3004/assoc/list")
            .send().await.map_err(|_| ServerError::InvalidQuery)?
            .json::<ListAssociationResponse>()
            .await.map_err(|_| ServerError::NotFound)?;

        match resp {
            ListAssociationResponse::Success(list) => {
                let list = list.iter().map(|assoc| {
                    let db_assoc = DatabaseAssociation::from(assoc.clone()); 
                    db_assoc.deref().clone()
                }).collect();

                return Ok(list)
            }
            ListAssociationResponse::Failure => {
                return Err(ServerError::NotFound)
            }
        }

    }

    pub async fn delete(id: i64) -> Result<(), ServerError> {
        let request = DeleteAssociationRequest::Delete(id.to_string());
        let resp = reqwest::Client::new()
            .post("http://127.0.0.1:3004/assoc/delete")
            .json(&request)
            .send()
            .await.map_err(|_| ServerError::InvalidQuery)?
            .json::<AssociationResponse>()
            .await.map_err(|_| ServerError::NotFound)?;

        match resp {
            AssociationResponse::Success(_) => return Ok(()),
            AssociationResponse::Failure => return Err(ServerError::NotFound),
        }
    }
}

impl DatabaseAssociation<Sqlite> {
    pub fn create(
        conn: &Connection,
        contents: AssociationContents,
    ) -> Result<Association, ServerError> {
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

    pub fn list(conn: &Connection) -> Result<Vec<Association>, ServerError> {
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
