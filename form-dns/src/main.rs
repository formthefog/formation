use std::sync::{Arc, RwLock};

use form_dns::api::serve_api;
use form_dns::store::{DnsStore, SharedStore, FormDnsRecord};
use form_dns::authority::FormAuthority;
use tokio::net::UdpSocket;
use trust_dns_client::client::AsyncClient;
use trust_dns_proto::rr::{Name, RecordType};
use trust_dns_proto::udp::{UdpClientConnect, UdpClientStream};
use trust_dns_server::authority::Catalog;
use trust_dns_server::ServerFuture;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    simple_logger::SimpleLogger::new().init().unwrap();
    let store: SharedStore = Arc::new(RwLock::new(DnsStore::new()));
    log::info!("Set up shared DNS store");

    let inner_store = store.clone();
    tokio::spawn(async move {
        let _ = serve_api(inner_store).await;
    });

    {
        if let Ok(mut guard) = store.write() {
            let record = FormDnsRecord {
                domain: "hello.fog".to_lowercase().to_string(),
                record_type: RecordType::A,
                formnet_ip: Some("10.0.0.42".parse().unwrap()),
                public_ip: None,
                cname_target: None,
                ttl: 3600
            };
            guard.insert(&record.domain.clone(), record);
            log::info!("Inserted hello.fog as test record...");
        }
    }

    log::warn!("Setting 8.8.8.8 as fallback for DNS lookup...");
    let fallback = "8.8.8.8:53".parse().unwrap();
    let stream: UdpClientConnect<UdpSocket> = UdpClientStream::new(fallback);
    log::warn!("Built UDP Client Stream for communication with fallback...");
    let (fallback_client, bg) = AsyncClient::connect(stream).await?;
    tokio::spawn(bg);
    log::warn!("Spawned AsyncClient in background...");
    
    log::warn!("Setting authority origin to root...");
    let origin = Name::root();
    let auth = FormAuthority::new(origin, store, fallback_client);

    log::debug!("Wrapping authority in an Atomic Reference Counter...");
    let auth_arc = Arc::new(auth);

    log::warn!("Building catalog...");
    let mut catalog = Catalog::new();
    catalog.upsert(Name::root().into(), Box::new(auth_arc.clone()));
    log::info!("Built catalog for FormAuthority with origin root...");

    let mut server_future = ServerFuture::new(catalog);
    log::warn!("Built server future for catalog...");
    let udp_socket = UdpSocket::bind("0.0.0.0:5354").await?;
    log::info!("Bound udp socket to port 5354 on all active interfaces...");
    server_future.register_socket(udp_socket);

    log::info!("DNS Server listening on port 5354 (UDP)");

    server_future.block_until_done().await?;

    Ok(())
}
