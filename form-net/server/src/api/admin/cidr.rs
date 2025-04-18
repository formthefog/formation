pub mod sqlite_routes {
    use std::collections::VecDeque;

    use crate::{
        db::{DatabaseCidr, Sqlite},
        util::{form_body, json_response, status_response},
        ServerError, Session, SqlContext,
    };
    use hyper::{Body, Method, Request, Response, StatusCode};
    use shared::CidrContents;

    pub async fn routes(
        req: Request<Body>,
        mut components: VecDeque<String>,
        session: Session<SqlContext, i64, Sqlite>,
    ) -> Result<Response<Body>, ServerError> {
        match (req.method(), components.pop_front().as_deref()) {
            (&Method::GET, None) => handlers::list(session).await,
            (&Method::POST, None) => {
                let form = form_body(req).await?;
                handlers::create(form, session).await
            },
            (&Method::PUT, Some(id)) => {
                let id: i64 = id.parse().map_err(|_| ServerError::NotFound)?;
                let form = form_body(req).await?;
                handlers::update(id, form, session).await
            },
            (&Method::DELETE, Some(id)) => {
                let id: i64 = id.parse().map_err(|_| ServerError::NotFound)?;
                handlers::delete(id, session).await
            },
            _ => Err(ServerError::NotFound),
        }
    }

    mod handlers {
        use crate::{db::Sqlite, util::json_status_response};

        use super::*;

        pub async fn create(
            contents: CidrContents<i64>,
            session: Session<SqlContext, i64, Sqlite>,
        ) -> Result<Response<Body>, ServerError> {
            let conn = session.context.db.lock();

            let cidr = DatabaseCidr::<i64, Sqlite>::create(&conn, contents)?;

            json_status_response(cidr, StatusCode::CREATED)
        }

        pub async fn update(
            id: i64,
            form: CidrContents<i64>,
            session: Session<SqlContext, i64, Sqlite>,
        ) -> Result<Response<Body>, ServerError> {
            let conn = session.context.db.lock();
            let cidr = DatabaseCidr::<i64, Sqlite>::get(&conn, id)?;
            DatabaseCidr::<i64, Sqlite>::from(cidr).update(&conn, form)?;

            status_response(StatusCode::NO_CONTENT)
        }

        pub async fn list(session: Session<SqlContext, i64, Sqlite>) -> Result<Response<Body>, ServerError> {
            let conn = session.context.db.lock();
            let cidrs = DatabaseCidr::<i64, Sqlite>::list(&conn)?;

            json_response(cidrs)
        }

        pub async fn delete(id: i64, session: Session<SqlContext, i64, Sqlite>) -> Result<Response<Body>, ServerError> {
            let conn = session.context.db.lock();
            DatabaseCidr::<i64, Sqlite>::delete(&conn, id)?;

            status_response(StatusCode::NO_CONTENT)
        }
    }
}


pub mod crdt_routes {
    use std::collections::VecDeque;

    use crate::{
        db::DatabaseCidr,
        util::{form_body, json_response, status_response},
        ServerError,
    };
    use hyper::{Body, Method, Request, Response, StatusCode};
    use shared::CidrContents;

    pub async fn routes(
        req: Request<Body>,
        mut components: VecDeque<String>,
    ) -> Result<Response<Body>, ServerError> {
        match (req.method(), components.pop_front().as_deref()) {
            (&Method::GET, None) => handlers::list().await,
            (&Method::POST, None) => {
                let form = form_body(req).await?;
                handlers::create(form).await
            },
            (&Method::PUT, Some(id)) => {
                let form = form_body(req).await?;
                handlers::update(id.to_string(), form).await
            },
            (&Method::DELETE, Some(id)) => {
                handlers::delete(id.to_string()).await
            },
            _ => Err(ServerError::NotFound),
        }
    }

    mod handlers {
        use crate::{db::CrdtMap, util::json_status_response};

        use super::*;

        pub async fn create(
            contents: CidrContents<String>,
        ) -> Result<Response<Body>, ServerError> {
            let cidr = DatabaseCidr::<String, CrdtMap>::create(contents).await?;
            json_status_response(cidr, StatusCode::CREATED)
        }

        pub async fn update(
            id: String,
            form: CidrContents<String>,
        ) -> Result<Response<Body>, ServerError> {
            let cidr = DatabaseCidr::<String, CrdtMap>::get(id).await?;
            DatabaseCidr::<String, CrdtMap>::from(cidr).update(form).await?;
            status_response(StatusCode::NO_CONTENT)
        }

        pub async fn list() -> Result<Response<Body>, ServerError> {
            let cidrs = DatabaseCidr::<String, CrdtMap>::list().await?;
            json_response(cidrs)
        }

        pub async fn delete(id: String) -> Result<Response<Body>, ServerError> {
            DatabaseCidr::<String,CrdtMap>::delete(id).await?;

            status_response(StatusCode::NO_CONTENT)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{db::Sqlite, test, DatabaseCidr, DatabasePeer};
    use anyhow::Result;
    use bytes::Buf;
    use hyper::StatusCode;
    use shared::{Cidr, CidrContents, Error};

    #[tokio::test]
    async fn test_cidr_add() -> Result<(), Error> {
        let server = test::Server::new()?;

        let old_cidrs = DatabaseCidr::<i64, Sqlite>::list(&server.db().lock())?;

        let contents = CidrContents {
            name: "experimental".to_string(),
            cidr: test::EXPERIMENTAL_CIDR.parse()?,
            parent: Some(test::ROOT_CIDR_ID),
        };

        let res = server
            .form_request(test::ADMIN_PEER_IP, "POST", "/v1/admin/cidrs", &contents)
            .await;

        assert_eq!(res.status(), 201);

        let whole_body = hyper::body::aggregate(res).await?;
        let cidr_res: Cidr<i64> = serde_json::from_reader(whole_body.reader())?;
        assert_eq!(contents, cidr_res.contents);

        let new_cidrs = DatabaseCidr::<i64, Sqlite>::list(&server.db().lock())?;
        assert_eq!(old_cidrs.len() + 1, new_cidrs.len());

        Ok(())
    }

    #[tokio::test]
    async fn test_cidr_name_uniqueness() -> Result<(), Error> {
        let server = test::Server::new()?;

        let contents = CidrContents {
            name: "experimental".to_string(),
            cidr: test::EXPERIMENTAL_CIDR.parse()?,
            parent: Some(test::ROOT_CIDR_ID),
        };

        let res = server
            .form_request(test::ADMIN_PEER_IP, "POST", "/v1/admin/cidrs", &contents)
            .await;
        assert!(res.status().is_success());
        let whole_body = hyper::body::aggregate(res).await?;
        let cidr_res: Cidr<i64> = serde_json::from_reader(whole_body.reader())?;

        let contents = CidrContents {
            name: "experimental".to_string(),
            cidr: test::EXPERIMENTAL_SUBCIDR.parse()?,
            parent: Some(cidr_res.id),
        };
        let res = server
            .form_request(test::ADMIN_PEER_IP, "POST", "/v1/admin/cidrs", &contents)
            .await;
        assert!(!res.status().is_success());

        Ok(())
    }

    #[tokio::test]
    async fn test_cidr_create_auth() -> Result<(), Error> {
        let server = test::Server::new()?;

        let contents = CidrContents {
            name: "experimental".to_string(),
            cidr: test::EXPERIMENTAL_CIDR.parse()?,
            parent: Some(test::ROOT_CIDR_ID),
        };

        let res = server
            .form_request(test::USER1_PEER_IP, "POST", "/v1/admin/cidrs", &contents)
            .await;
        assert!(!res.status().is_success());

        Ok(())
    }

    #[tokio::test]
    async fn test_cidr_bad_parent() -> Result<(), Error> {
        let server = test::Server::new()?;

        let contents = CidrContents {
            name: "experimental".to_string(),
            cidr: test::EXPERIMENTAL_CIDR.parse()?,
            parent: Some(test::ROOT_CIDR_ID),
        };
        let res = server
            .form_request(test::ADMIN_PEER_IP, "POST", "/v1/admin/cidrs", &contents)
            .await;
        assert!(res.status().is_success());

        let contents = CidrContents {
            name: "experimental".to_string(),
            cidr: test::EXPERIMENTAL_SUBCIDR.parse()?,
            parent: Some(test::ROOT_CIDR_ID),
        };

        let res = server
            .form_request(test::ADMIN_PEER_IP, "POST", "/v1/admin/cidrs", &contents)
            .await;
        assert!(!res.status().is_success());

        Ok(())
    }

    #[tokio::test]
    async fn test_cidr_overlap() -> Result<(), Error> {
        let server = test::Server::new()?;

        let contents = CidrContents {
            name: "experimental".to_string(),
            cidr: "10.80.1.0/21".parse()?,
            parent: Some(test::ROOT_CIDR_ID),
        };
        let res = server
            .form_request(test::ADMIN_PEER_IP, "POST", "/v1/admin/cidrs", &contents)
            .await;
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);

        Ok(())
    }

    #[tokio::test]
    async fn test_cidr_delete_fail_with_child_cidr() -> Result<(), Error> {
        let server = test::Server::new()?;

        let experimental_cidr = DatabaseCidr::<i64, Sqlite>::create(
            &server.db().lock(),
            CidrContents {
                name: "experimental".to_string(),
                cidr: test::EXPERIMENTAL_CIDR.parse()?,
                parent: Some(test::ROOT_CIDR_ID),
            },
        )?;
        let experimental_subcidr = DatabaseCidr::<i64, Sqlite>::create(
            &server.db().lock(),
            CidrContents {
                name: "experimental subcidr".to_string(),
                cidr: test::EXPERIMENTAL_SUBCIDR.parse()?,
                parent: Some(experimental_cidr.id),
            },
        )?;

        let res = server
            .request(
                test::ADMIN_PEER_IP,
                "DELETE",
                &format!("/v1/admin/cidrs/{}", experimental_cidr.id),
            )
            .await;
        // Should fail because child CIDR exists.
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);

        let res = server
            .request(
                test::ADMIN_PEER_IP,
                "DELETE",
                &format!("/v1/admin/cidrs/{}", experimental_subcidr.id),
            )
            .await;
        // Deleting child "leaf" CIDR should fail because peer exists inside it.
        assert_eq!(res.status(), StatusCode::NO_CONTENT);

        let res = server
            .request(
                test::ADMIN_PEER_IP,
                "DELETE",
                &format!("/v1/admin/cidrs/{}", experimental_cidr.id),
            )
            .await;
        // Now deleting parent CIDR should work because child is gone.
        assert_eq!(res.status(), StatusCode::NO_CONTENT);
        Ok(())
    }

    #[tokio::test]
    async fn test_cidr_delete_fail_with_peer_inside() -> Result<(), Error> {
        let server = test::Server::new()?;

        let experimental_cidr = DatabaseCidr::<i64, Sqlite>::create(
            &server.db().lock(),
            CidrContents {
                name: "experimental".to_string(),
                cidr: test::EXPERIMENTAL_CIDR.parse()?,
                parent: Some(test::ROOT_CIDR_ID),
            },
        )?;

        let _experiment_peer = DatabasePeer::<i64, Sqlite>::create(
            &server.db().lock(),
            test::peer_contents(
                "experiment-peer",
                test::EXPERIMENT_SUBCIDR_PEER_IP,
                experimental_cidr.id,
                false,
            )?,
        )?;

        let res = server
            .request(
                test::ADMIN_PEER_IP,
                "DELETE",
                &format!("/v1/admin/cidrs/{}", experimental_cidr.id),
            )
            .await;
        // Deleting CIDR should fail because peer exists inside it.
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);

        Ok(())
    }
}
