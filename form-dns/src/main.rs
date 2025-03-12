use std::sync::Arc;
use std::time::Duration;
use form_dns::{resolvectl_domain, resolvectl_flush_cache, resolvectl_revert};
use tokio::sync::RwLock;
use form_dns::api::serve_api;
use form_dns::proxy::IntegratedProxy;
use form_dns::store::{DnsStore, SharedStore};
use form_dns::authority::FormAuthority;
use form_dns::health_tracker;
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
    let dns_store = DnsStore::new(tx.clone());
    
    log::info!("Set up DNS store");

    // Initialize health tracker service
    log::info!("Initializing health tracker service");
    let health_repo = health_tracker::start_health_tracker(
        "http://localhost:3004".to_string(),  // Form-state API endpoint
        Some(Duration::from_secs(60)),        // Heartbeat timeout
        Some(Duration::from_secs(10)),        // Check interval
        Some(Duration::from_secs(300)),       // Stale timeout
    ).await;
    log::info!("Health tracker service initialized");
    
    // Connect health repository to DNS store
    let dns_store_with_health = dns_store.with_health_repository(health_repo.clone());
    let store: SharedStore = Arc::new(RwLock::new(dns_store_with_health));
    
    log::info!("Connected health repository to DNS store");

    // Add bootstrap domain configuration
    {
        log::info!("Configuring bootstrap domain...");
        let mut guard = store.write().await;
        
        // Create the bootstrap domain record
        let bootstrap_domain = "bootstrap.formation.cloud";
        let bootstrap_record = form_dns::store::FormDnsRecord {
            domain: bootstrap_domain.to_string(),
            record_type: trust_dns_proto::rr::RecordType::A,
            public_ip: vec![], // Will be populated later with actual bootstrap nodes
            formnet_ip: vec![],
            cname_target: None,
            ssl_cert: false,
            ttl: 60, // Lower TTL for bootstrap domain to allow faster failover
            verification_status: Some(form_dns::store::VerificationStatus::Verified),
            verification_timestamp: Some(std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_secs())),
        };
        
        // Add the bootstrap domain to the DNS store
        guard.insert(bootstrap_domain, bootstrap_record).await;
        log::info!("Bootstrap domain configured successfully");
    }

    let inner_store = store.clone();
    log::info!("Cloned DNS store for TLS Manager store");
    let tls_manager = TlsManager::new(vec![("hello.fog".to_string(), false)]).await.map_err(|e| {
        Box::new(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
    })?;

    log::info!("Built TlsManager...");
    log::info!("Building ProxyConfig...");
    let proxy_config = ProxyConfig::default();
    log::info!("Building IntegratedProxy...");
    let mut reverse_proxy = IntegratedProxy::new(store.clone(), tls_manager, proxy_config).await.map_err(|e| anyhow::anyhow!(e.to_string()))?;
    log::info!("Launching IntegratedProxy...");
    let reverse_proxy_handle = tokio::spawn(async move {
        if let Err(e) = reverse_proxy.bind().await {
            eprintln!("Error attempting to bind http and/or https listener: {e}");
        };
        if let Err(e) = reverse_proxy.run(rx).await {
            eprintln!("Error in run process for Reverse Proxy: {e}");
        }
    });
    
    log::info!("Launching DNS Store API Server...");
    let dns_store_api_handle = tokio::spawn(async move {
        let _ = serve_api(inner_store).await;
    });

    {
        log::info!("Adding DNS server to DNS Store...");
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
    
    // Create the authority with health repository integration
    let auth = FormAuthority::new(origin, store.clone(), fallback_client)
        .with_health_repository(health_repo);

    log::info!("Created FormAuthority with health repository integration");
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
