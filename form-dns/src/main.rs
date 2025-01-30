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
    let store: SharedStore = Arc::new(RwLock::new(DnsStore::new()));

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
        }
    }

    let fallback = "8.8.8.8:53".parse().unwrap();
    let stream: UdpClientConnect<UdpSocket> = UdpClientStream::new(fallback);
    let (fallback_client, bg) = AsyncClient::connect(stream).await?;
    tokio::spawn(bg);
    
    let origin = Name::root();
    let auth = FormAuthority::new(origin, store, fallback_client);

    let auth_arc = Arc::new(auth);

    let mut catalog = Catalog::new();
    catalog.upsert(Name::root().into(), Box::new(auth_arc.clone()));

    let mut server_future = ServerFuture::new(catalog);
    let udp_socket = UdpSocket::bind("0.0.0.0:5354").await?;
    println!("DNS Server listening on port 5354 (UDP)");
    server_future.register_socket(udp_socket);

    server_future.block_until_done().await?;

    Ok(())
}
