use std::fmt::Display;

use shared::Peer;

use crate::{DatastoreContext, Session};

pub mod admin;
pub mod user;

/// Inject the collected endpoints from the WG interface into a list of peers.
/// This is essentially what adds NAT holepunching functionality. If a peer
/// already has an endpoint specified (by calling the override-endpoint) API,
/// the relatively recent wireguard endpoint will be added to the list of NAT
/// candidates, so other peers have a better chance of connecting.
pub fn inject_endpoints<C: DatastoreContext, T: Display + Clone + PartialEq, D>(session: &Session<C, T, D>, peers: &mut Vec<Peer<T>>) {
    for peer in peers {
        let endpoints = session.context.endpoints().clone();
        let reader = endpoints.read();
        if let Some(wg_endpoint) = reader.get(&peer.public_key) {
            if peer.contents.endpoint.is_none() {
                peer.contents.endpoint = Some(wg_endpoint.to_owned().into());
            } else {
                // The peer already has an endpoint specified, but it might be stale.
                // If there is an endpoint reported from wireguard, we should add it
                // to the list of candidates so others can try to connect using it.
                peer.contents.candidates.push(wg_endpoint.to_owned().into());
            }
        }
    }
}
