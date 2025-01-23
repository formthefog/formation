pub mod association;
pub mod cidr;
pub mod peer;

pub mod sqlite_routes {
    use super::*;
    use std::collections::VecDeque;

    use hyper::{Body, Request, Response};

    use crate::{db::Sqlite, ServerError, Session, SqlContext};

    pub async fn routes(
        req: Request<Body>,
        mut components: VecDeque<String>,
        session: Session<SqlContext, Sqlite>,
    ) -> Result<Response<Body>, ServerError> {
        if !session.admin_capable() {
            return Err(ServerError::Unauthorized);
        }

        match components.pop_front().as_deref() {
            Some("associations") => association::sqlite_routes::routes(req, components, session).await,
            Some("cidrs") => cidr::sqlite_routes::routes(req, components, session).await,
            Some("peers") => peer::sqlite_routes::routes(req, components, session).await,
            _ => Err(ServerError::NotFound),
        }
    }
}

pub mod crdt_routes {
    use super::*;
    use std::collections::VecDeque;

    use hyper::{Body, Request, Response};

    use crate::{db::CrdtMap, CrdtContext, ServerError, Session};

    pub async fn routes(
        req: Request<Body>,
        mut components: VecDeque<String>,
        session: Session<CrdtContext, CrdtMap>,
    ) -> Result<Response<Body>, ServerError> {
        if !session.admin_capable() {
            return Err(ServerError::Unauthorized);
        }

        match components.pop_front().as_deref() {
            Some("associations") => association::crdt_routes::routes(req, components).await,
            Some("cidrs") => cidr::crdt_routes::routes(req, components).await,
            Some("peers") => peer::crdt_routes::routes(req, components, session).await,
            _ => Err(ServerError::NotFound),
        }
    }
}
