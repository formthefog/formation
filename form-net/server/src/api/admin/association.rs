//! A table to describe which CIDRs another CIDR is allowed to peer with.
//!
//! A peer belongs to one parent CIDR, and can by default see all peers within that parent.

pub mod sqlite_routes {
    use std::collections::VecDeque;

    use crate::{
        db::Sqlite,
        util::form_body,
        ServerError, Session, SqlContext,
    };
    use hyper::{Body, Method, Request, Response};

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
            (&Method::DELETE, Some(id)) => {
                let id: i64 = id.parse().map_err(|_| ServerError::NotFound)?;
                handlers::delete(id, session).await
            },
            _ => Err(ServerError::NotFound),
        }
    }

    mod handlers {
        use hyper::{Body, Response, StatusCode};
        use shared::AssociationContents;

        use crate::{db::{DatabaseAssociation, Sqlite}, util::{json_response, status_response}, ServerError, Session, SqlContext};

        pub async fn create(
            contents: AssociationContents<i64>,
            session: Session<SqlContext, i64, Sqlite>,
        ) -> Result<Response<Body>, ServerError> {
            let conn = session.context.db.lock();

            DatabaseAssociation::<Sqlite, i64, i64>::create(&conn, contents)?;

            status_response(StatusCode::CREATED)
        }

        pub async fn list(session: Session<SqlContext, i64, Sqlite>) -> Result<Response<Body>, ServerError> {
            let conn = session.context.db.lock();
            let auths = DatabaseAssociation::<Sqlite, i64, i64>::list(&conn)?;

            json_response(auths)
        }

        pub async fn delete(id: i64, session: Session<SqlContext, i64, Sqlite>) -> Result<Response<Body>, ServerError> {
            let conn = session.context.db.lock();
            DatabaseAssociation::<Sqlite, i64, i64>::delete(&conn, id)?;

            status_response(StatusCode::NO_CONTENT)
        }
    }
}

pub mod crdt_routes {
    use std::collections::VecDeque;

    use crate::{
        util::form_body, ServerError
    };
    use hyper::{Body, Method, Request, Response};

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
            (&Method::DELETE, Some(id)) => {
                let id: i64 = id.parse().map_err(|_| ServerError::NotFound)?;
                handlers::delete(id).await
            },
            _ => Err(ServerError::NotFound),
        }
    }

    mod handlers {
        use hyper::{Body, Response, StatusCode};
        use shared::AssociationContents;

        use crate::{db::{CrdtMap, DatabaseAssociation}, util::{json_response, status_response}, ServerError};

        pub async fn create(
            contents: AssociationContents<String>,
        ) -> Result<Response<Body>, ServerError> {
            DatabaseAssociation::<CrdtMap, String, String>::create(contents).await?;
            status_response(StatusCode::CREATED)
        }

        pub async fn list() -> Result<Response<Body>, ServerError> {
            let auths = DatabaseAssociation::<CrdtMap, String, String>::list().await?;
            json_response(auths)
        }

        pub async fn delete(id: i64) -> Result<Response<Body>, ServerError> {
            DatabaseAssociation::<CrdtMap, String, String>::delete(id).await?;
            status_response(StatusCode::NO_CONTENT)
        }
    }
}
