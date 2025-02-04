use std::sync::Arc;
use form_dns::{resolvectl_domain, resolvectl_flush_cache, resolvectl_revert};
use tokio::sync::RwLock;
use form_dns::api::serve_api;
use form_dns::proxy::IntegratedProxy;
use form_dns::store::{DnsStore, SharedStore};
use form_dns::authority::FormAuthority;
use form_rplb::config::ProxyConfig;
use form_rplb::resolver::TlsManager;
use tokio::net::UdpSocket;
use trust_dns_client::client::AsyncClient;
use trust_dns_proto::rr::Name;
use trust_dns_proto::udp::{UdpClientConnect, UdpClientStream};
use trust_dns_server::authority::Catalog;
use trust_dns_server::ServerFuture;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    simple_logger::SimpleLogger::new().init().unwrap();
    resolvectl_revert().map_err(|e| anyhow::anyhow!(e.to_string()))?;
    resolvectl_flush_cache().map_err(|e| anyhow::anyhow!(e.to_string()))?;
    resolvectl_domain().map_err(|e| anyhow::anyhow!(e.to_string()))?;

    let (tx, rx) = tokio::sync::mpsc::channel(1024);
    let store: SharedStore = Arc::new(RwLock::new(DnsStore::new(tx.clone())));

    log::info!("Set up shared DNS store");

    let inner_store = store.clone();
    let tls_manager = TlsManager::new(vec![("hello.fog".to_string(), false)]).await.map_err(|e| {
        Box::new(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
    })?;
    let proxy_config = ProxyConfig::default();
    let mut reverse_proxy = IntegratedProxy::new(store.clone(), tls_manager, proxy_config).await.map_err(|e| anyhow::anyhow!(e.to_string()))?;
    let reverse_proxy_handle = tokio::spawn(async move {
        if let Err(e) = reverse_proxy.bind().await {
            eprintln!("Error attempting to bind http and/or https listener: {e}");
        };
        if let Err(e) = reverse_proxy.run(rx).await {
            eprintln!("Error in run process for Reverse Proxy: {e}");
        }
    });
    
    let dns_store_api_handle = tokio::spawn(async move {
        let _ = serve_api(inner_store).await;
    });

    {
        let mut guard = store.write().await;
        guard.add_server("10.0.0.1".parse()?).map_err(|e| anyhow::anyhow!(e.to_string()))?;
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
    let auth = FormAuthority::new(origin, store.clone(), fallback_client);

    log::debug!("Wrapping authority in an Atomic Reference Counter...");
    let auth_arc = Arc::new(auth);

    log::warn!("Building catalog...");
    let mut catalog = Catalog::new();
    catalog.upsert(Name::root().into(), Box::new(auth_arc.clone()));
    log::info!("Built catalog for FormAuthority with origin root...");

    let mut server_future = ServerFuture::new(catalog);
    log::warn!("Built server future for catalog...");
    let udp_socket = UdpSocket::bind("0.0.0.0:5453").await?;
    log::info!("Bound udp socket to port 5453 on all active interfaces...");
    server_future.register_socket(udp_socket);

    log::info!("DNS Server listening on port 53 (UDP)");

    server_future.block_until_done().await?;
    reverse_proxy_handle.await?;
    dns_store_api_handle.await?;

    Ok(())
}
