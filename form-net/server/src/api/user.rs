use std::collections::VecDeque;

use crate::{
    api::inject_endpoints,
    db::{DatabaseCidr, DatabasePeer},
    util::{form_body, json_response, status_response},
    Context, ServerError, Session,
};
use hyper::{Body, Method, Request, Response, StatusCode};
use shared::{EndpointContents, PeerContents, RedeemContents, State, REDEEM_TRANSITION_WAIT};
use wireguard_control::{DeviceUpdate, PeerConfigBuilder};

pub async fn routes(
    req: Request<Body>,
    mut components: VecDeque<String>,
    session: Session,
) -> Result<Response<Body>, ServerError> {
    match (req.method(), components.pop_front().as_deref()) {
        (&Method::GET, Some("state")) => {
            if !session.user_capable() {
                return Err(ServerError::Unauthorized);
            }
            handlers::state(session).await
        },
        (&Method::POST, Some("redeem")) => {
            if !session.redeemable() {
                return Err(ServerError::Unauthorized);
            }
            let form = form_body(req).await?;
            handlers::redeem(form, session).await
        },
        (&Method::PUT, Some("endpoint")) => {
            if !session.user_capable() {
                return Err(ServerError::Unauthorized);
            }
            let form = form_body(req).await?;
            handlers::endpoint(form, session).await
        },
        (&Method::PUT, Some("candidates")) => {
            if !session.user_capable() {
                return Err(ServerError::Unauthorized);
            }
            let form = form_body(req).await?;
            handlers::candidates(form, session).await
        },
        _ => Err(ServerError::NotFound),
    }
}

mod handlers {
    use shared::Endpoint;

    use super::*;

    /// Get the current state of the network, in the eyes of the current peer.
    ///
    /// This endpoint returns the visible CIDRs and Peers, providing all the necessary
    /// information for the peer to create connections to all of them.
    pub async fn state(session: Session) -> Result<Response<Body>, ServerError> {
        let conn = session.context.db.lock();
        let selected_peer = DatabasePeer::get(&conn, session.peer.id)?;

        let cidrs: Vec<_> = DatabaseCidr::list(&conn)?;

        let mut peers: Vec<_> = selected_peer
            .get_all_allowed_peers(&conn)?
            .into_iter()
            .map(|p| p.inner)
            .collect();
        inject_endpoints(&session, &mut peers);
        json_response(State { peers, cidrs })
    }

    /// Redeems an invitation. An invitation includes a WireGuard keypair generated by either the server
    /// or a peer with admin rights.
    ///
    /// Redemption is the process of an invitee generating their own keypair and exchanging their temporary
    /// key with their permanent one.
    ///
    /// Until this API endpoint is called, the invited peer will not show up to other peers, and once
    /// it is called and succeeds, it cannot be called again.
    pub async fn redeem(
        form: RedeemContents,
        session: Session,
    ) -> Result<Response<Body>, ServerError> {
        let conn = session.context.db.lock();
        let mut selected_peer = DatabasePeer::get(&conn, session.peer.id)?;

        let old_public_key = wireguard_control::Key::from_base64(&selected_peer.public_key)
            .map_err(|_| ServerError::WireGuard)?;

        selected_peer.redeem(&conn, &form.public_key)?;

        if cfg!(not(test)) {
            let Context {
                interface, backend, ..
            } = session.context;

            // If we were to modify the WireGuard interface immediately, the HTTP response wouldn't
            // get through. Instead, we need to wait a reasonable amount for the HTTP response to
            // flush, then update the interface.
            //
            // The client is also expected to wait the same amount of time after receiving a success
            // response from /redeem.
            //
            // This might be avoidable if we were able to run code after we were certain the response
            // had flushed over the TCP socket, but that isn't easily accessible from this high-level
            // web framework.
            //
            // Related: https://github.com/hyperium/hyper/issues/2181
            tokio::task::spawn(async move {
                tokio::time::sleep(REDEEM_TRANSITION_WAIT).await;
                log::info!(
                    "WireGuard: adding new peer {}, removing old pubkey {}",
                    &*selected_peer,
                    old_public_key.to_base64()
                );
                DeviceUpdate::new()
                    .remove_peer_by_key(&old_public_key)
                    .add_peer(PeerConfigBuilder::from(&*selected_peer))
                    .apply(&interface, backend)
                    .map_err(|e| log::error!("{:?}", e))
                    .ok();
            });
        }
        status_response(StatusCode::NO_CONTENT)
    }

    /// Report any other endpoint candidates that can be tried by peers to connect.
    /// Currently limited to 10 candidates max.
    pub async fn candidates(
        contents: Vec<Endpoint>,
        session: Session,
    ) -> Result<Response<Body>, ServerError> {
        if contents.len() > 10 {
            return status_response(StatusCode::PAYLOAD_TOO_LARGE);
        }
        let conn = session.context.db.lock();
        let mut selected_peer = DatabasePeer::get(&conn, session.peer.id)?;
        selected_peer.update(
            &conn,
            PeerContents {
                candidates: contents,
                ..selected_peer.contents.clone()
            },
        )?;

        status_response(StatusCode::NO_CONTENT)
    }

    /// Force a specific endpoint to be reported by the server.
    pub async fn endpoint(
        contents: EndpointContents,
        session: Session,
    ) -> Result<Response<Body>, ServerError> {
        let conn = session.context.db.lock();
        let mut selected_peer = DatabasePeer::get(&conn, session.peer.id)?;
        selected_peer.update(
            &conn,
            PeerContents {
                endpoint: contents.into(),
                ..selected_peer.contents.clone()
            },
        )?;

        status_response(StatusCode::NO_CONTENT)
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime};

    use super::*;
    use crate::{db::DatabaseAssociation, test};
    use bytes::Buf;
    use shared::{AssociationContents, CidrContents, Endpoint, EndpointContents, Error};

    #[tokio::test]
    async fn test_get_state_from_developer1() -> Result<(), Error> {
        let server = test::Server::new()?;
        let res = server
            .request(test::DEVELOPER1_PEER_IP, "GET", "/v1/user/state")
            .await;

        assert_eq!(res.status(), StatusCode::OK);

        let whole_body = hyper::body::aggregate(res).await?;
        let State { peers, .. } = serde_json::from_reader(whole_body.reader())?;
        let mut peer_names = peers.iter().map(|p| &*p.contents.name).collect::<Vec<_>>();
        peer_names.sort_unstable();
        // Developers should see only peers in infra CIDR and developer CIDR.
        assert_eq!(
            &["developer1", "developer2", "innernet-server"],
            &peer_names[..]
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_override_endpoint() -> Result<(), Error> {
        let server = test::Server::new()?;
        assert_eq!(
            server
                .form_request(
                    test::DEVELOPER1_PEER_IP,
                    "PUT",
                    "/v1/user/endpoint",
                    &EndpointContents::Set("1.1.1.1:51820".parse().unwrap())
                )
                .await
                .status(),
            StatusCode::NO_CONTENT
        );

        println!("{}", serde_json::to_string(&EndpointContents::Unset)?);
        assert_eq!(
            server
                .form_request(
                    test::DEVELOPER1_PEER_IP,
                    "PUT",
                    "/v1/user/endpoint",
                    &EndpointContents::Unset,
                )
                .await
                .status(),
            StatusCode::NO_CONTENT
        );

        assert_eq!(
            server
                .form_request(
                    test::DEVELOPER1_PEER_IP,
                    "PUT",
                    "/v1/user/endpoint",
                    "endpoint=blah",
                )
                .await
                .status(),
            StatusCode::BAD_REQUEST
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_list_peers_from_unknown_ip() -> Result<(), Error> {
        let server = test::Server::new()?;

        // Request comes from an unknown IP.
        let res = server.request("10.80.80.80", "GET", "/v1/user/state").await;

        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

        Ok(())
    }

    #[tokio::test]
    async fn test_list_peers_for_developer_subcidr() -> Result<(), Error> {
        let server = test::Server::new()?;
        {
            let db = server.db.lock();
            let cidr = DatabaseCidr::create(
                &db,
                CidrContents {
                    name: "experiment cidr".to_string(),
                    cidr: test::EXPERIMENTAL_CIDR.parse()?,
                    parent: Some(test::ROOT_CIDR_ID),
                },
            )?;
            let subcidr = DatabaseCidr::create(
                &db,
                CidrContents {
                    name: "experiment subcidr".to_string(),
                    cidr: test::EXPERIMENTAL_SUBCIDR.parse()?,
                    parent: Some(cidr.id),
                },
            )?;
            DatabasePeer::create(
                &db,
                test::peer_contents(
                    "experiment-peer",
                    test::EXPERIMENT_SUBCIDR_PEER_IP,
                    subcidr.id,
                    false,
                )?,
            )?;

            // Add a peering between the developer's CIDR and the experimental *parent* cidr.
            DatabaseAssociation::create(
                &db,
                AssociationContents {
                    cidr_id_1: test::DEVELOPER_CIDR_ID,
                    cidr_id_2: cidr.id,
                },
            )?;
            DatabaseAssociation::create(
                &db,
                AssociationContents {
                    cidr_id_1: test::INFRA_CIDR_ID,
                    cidr_id_2: cidr.id,
                },
            )?;
        }

        for ip in &[test::DEVELOPER1_PEER_IP, test::EXPERIMENT_SUBCIDR_PEER_IP] {
            let res = server.request(ip, "GET", "/v1/user/state").await;
            assert_eq!(res.status(), StatusCode::OK);
            let whole_body = hyper::body::aggregate(res).await?;
            let State { peers, .. } = serde_json::from_reader(whole_body.reader())?;
            let mut peer_names = peers.iter().map(|p| &*p.contents.name).collect::<Vec<_>>();
            peer_names.sort_unstable();
            // Developers should see only peers in infra CIDR and developer CIDR.
            assert_eq!(
                &[
                    "developer1",
                    "developer2",
                    "experiment-peer",
                    "innernet-server"
                ],
                &peer_names[..]
            );
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_redeem() -> Result<(), Error> {
        let server = test::Server::new()?;

        let experimental_cidr = DatabaseCidr::create(
            &server.db().lock(),
            CidrContents {
                name: "experimental".to_string(),
                cidr: test::EXPERIMENTAL_CIDR.parse()?,
                parent: Some(test::ROOT_CIDR_ID),
            },
        )?;

        let mut peer_contents = test::peer_contents(
            "experiment-peer",
            test::EXPERIMENT_SUBCIDR_PEER_IP,
            experimental_cidr.id,
            false,
        )?;
        peer_contents.is_redeemed = false;
        peer_contents.invite_expires = Some(SystemTime::now() + Duration::from_secs(100));
        let _experiment_peer = DatabasePeer::create(&server.db().lock(), peer_contents)?;

        // Step 1: Ensure that before redeeming, other endpoints aren't yet accessible.
        let res = server
            .request(test::EXPERIMENT_SUBCIDR_PEER_IP, "GET", "/v1/user/state")
            .await;
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

        // Step 2: Ensure that redemption works.
        let body = RedeemContents {
            public_key: "YBVIgpfLbi/knrMCTEb0L6eVy0daiZnJJQkxBK9s+2I=".into(),
        };
        let res = server
            .form_request(
                test::EXPERIMENT_SUBCIDR_PEER_IP,
                "POST",
                "/v1/user/redeem",
                &body,
            )
            .await;
        assert!(res.status().is_success());

        // Step 3: Ensure that a second attempt at redemption DOESN'T work.
        let res = server
            .form_request(
                test::EXPERIMENT_SUBCIDR_PEER_IP,
                "POST",
                "/v1/user/redeem",
                &body,
            )
            .await;
        assert!(res.status().is_client_error());

        // Step 3: Ensure that after redemption, fetching state works.
        let res = server
            .request(test::EXPERIMENT_SUBCIDR_PEER_IP, "GET", "/v1/user/state")
            .await;
        assert_eq!(res.status(), StatusCode::OK);
        Ok(())
    }

    #[tokio::test]
    async fn test_redeem_expired() -> Result<(), Error> {
        let server = test::Server::new()?;

        let experimental_cidr = DatabaseCidr::create(
            &server.db().lock(),
            CidrContents {
                name: "experimental".to_string(),
                cidr: test::EXPERIMENTAL_CIDR.parse()?,
                parent: Some(test::ROOT_CIDR_ID),
            },
        )?;

        let mut peer_contents = test::peer_contents(
            "experiment-peer",
            test::EXPERIMENT_SUBCIDR_PEER_IP,
            experimental_cidr.id,
            false,
        )?;
        peer_contents.is_redeemed = false;
        peer_contents.invite_expires = Some(SystemTime::now() - Duration::from_secs(1));
        let _experiment_peer = DatabasePeer::create(&server.db().lock(), peer_contents)?;

        // Step 1: Ensure that before redeeming, other endpoints aren't yet accessible.
        let res = server
            .request(test::EXPERIMENT_SUBCIDR_PEER_IP, "GET", "/v1/user/state")
            .await;
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

        // Step 2: Ensure that redemption works.
        let body = RedeemContents {
            public_key: "YBVIgpfLbi/knrMCTEb0L6eVy0daiZnJJQkxBK9s+2I=".into(),
        };
        let res = server
            .form_request(
                test::EXPERIMENT_SUBCIDR_PEER_IP,
                "POST",
                "/v1/user/redeem",
                &body,
            )
            .await;
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        Ok(())
    }

    #[tokio::test]
    async fn test_candidates() -> Result<(), Error> {
        let server = test::Server::new()?;

        let peer = DatabasePeer::get(&server.db().lock(), test::DEVELOPER1_PEER_ID)?;
        assert_eq!(peer.candidates, vec![]);

        let candidates = vec!["1.1.1.1:51820".parse::<Endpoint>().unwrap()];
        assert_eq!(
            server
                .form_request(
                    test::DEVELOPER1_PEER_IP,
                    "PUT",
                    "/v1/user/candidates",
                    &candidates
                )
                .await
                .status(),
            StatusCode::NO_CONTENT
        );

        let res = server
            .request(test::DEVELOPER1_PEER_IP, "GET", "/v1/user/state")
            .await;

        assert_eq!(res.status(), StatusCode::OK);

        let peer = DatabasePeer::get(&server.db().lock(), test::DEVELOPER1_PEER_ID)?;
        assert_eq!(peer.candidates, candidates);
        Ok(())
    }

    #[tokio::test]
    async fn test_endpoint_in_candidates() -> Result<(), Error> {
        // We want to verify that the current wireguard endpoint always shows up
        // either in the peer.endpoint field, or the peer.candidates field (in the
        // case that the peer has specified an endpoint override).
        let server = test::Server::new()?;

        let peer = DatabasePeer::get(&server.db().lock(), test::DEVELOPER1_PEER_ID)?;
        assert_eq!(peer.candidates, vec![]);

        // Specify one NAT candidate. At this point, we have an unspecified
        // endpoint and one NAT candidate specified.
        let candidates = vec!["1.1.1.1:51820".parse::<Endpoint>().unwrap()];
        assert_eq!(
            server
                .form_request(
                    test::DEVELOPER1_PEER_IP,
                    "PUT",
                    "/v1/user/candidates",
                    &candidates
                )
                .await
                .status(),
            StatusCode::NO_CONTENT
        );

        let res = server
            .request(test::DEVELOPER1_PEER_IP, "GET", "/v1/user/state")
            .await;

        assert_eq!(res.status(), StatusCode::OK);

        let whole_body = hyper::body::aggregate(res).await?;
        let State { peers, .. } = serde_json::from_reader(whole_body.reader())?;

        let developer_1 = peers
            .into_iter()
            .find(|p| p.id == test::DEVELOPER1_PEER_ID)
            .unwrap();
        assert_eq!(
            developer_1.endpoint,
            Some(test::DEVELOPER1_PEER_ENDPOINT.parse().unwrap())
        );
        assert_eq!(developer_1.candidates, candidates);

        // Now, explicitly set an endpoint with the override-endpoint API
        // and check that the original wireguard endpoint still shows up
        // in the list of NAT candidates.
        assert_eq!(
            server
                .form_request(
                    test::DEVELOPER1_PEER_IP,
                    "PUT",
                    "/v1/user/endpoint",
                    &EndpointContents::Set("1.2.3.4:51820".parse().unwrap())
                )
                .await
                .status(),
            StatusCode::NO_CONTENT
        );

        let res = server
            .request(test::DEVELOPER1_PEER_IP, "GET", "/v1/user/state")
            .await;

        assert_eq!(res.status(), StatusCode::OK);

        let whole_body = hyper::body::aggregate(res).await?;
        let State { peers, .. } = serde_json::from_reader(whole_body.reader())?;

        let developer_1 = peers
            .into_iter()
            .find(|p| p.id == test::DEVELOPER1_PEER_ID)
            .unwrap();

        // The peer endpoint should be the one we just specified in the override-endpoint request.
        assert_eq!(developer_1.endpoint, Some("1.2.3.4:51820".parse().unwrap()));

        // The list of candidates should now contain the one we specified at the beginning of the
        // test, and the wireguard-reported endpoint of the peer.
        let nat_candidate_1 = "1.1.1.1:51820".parse().unwrap();
        let nat_candidate_2 = test::DEVELOPER1_PEER_ENDPOINT.parse().unwrap();
        assert_eq!(developer_1.candidates.len(), 2);
        assert!(developer_1.candidates.contains(&nat_candidate_1));
        assert!(developer_1.candidates.contains(&nat_candidate_2));

        Ok(())
    }
}
