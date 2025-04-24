use std::{net::{IpAddr, Ipv4Addr, SocketAddr}, path::PathBuf, str::FromStr, time::Duration};
use reqwest::Client;
use serde_json::Value;
use shared::{CidrContents, IpNetExt, PeerContents, PERSISTENT_KEEPALIVE_INTERVAL_SECS};
use shared::{wg, NetworkOpts};
use formnet_server::{db::CrdtMap, initialize::DbInitData, ConfigFile, DatabaseCidr, DatabasePeer};
use ipnet::IpNet;
use publicip::Preference;
use shared::{Endpoint, Interface};
use wireguard_control::{InterfaceName, KeyPair};

use crate::{CONFIG_DIR, DATA_DIR};


pub async fn init(address: String) -> Result<IpAddr, Box<dyn std::error::Error>> {
    let config_dir = PathBuf::from(CONFIG_DIR);
    let data_dir = PathBuf::from(DATA_DIR);
    shared::ensure_dirs_exist(&[&config_dir, &data_dir]).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            e.to_string()
        )
    })?;

    let name: Interface = InterfaceName::from_str("formnet")?.into();

    let root_cidr: IpNet = IpNet::new(
        IpAddr::V4(Ipv4Addr::new(10,0,0,0)),
        8
    )?;

    let listen_port: u16 = 51820;

    log::info!("listen port: {}", listen_port);

    let endpoint: Endpoint = {
        let ip = publicip::get_any(Preference::Ipv4)
            .ok_or_else(|| Box::new(std::io::Error::new(std::io::ErrorKind::Other, "couldn't get external IP")))?;
        SocketAddr::new(ip, listen_port).into()
    }; 

    let our_ip = root_cidr
        .hosts()
        .find(|ip| root_cidr.is_assignable(ip))
        .unwrap();
    let config_path = config_dir.join(&name.to_string()).with_extension("conf");
    let our_keypair = KeyPair::generate();

    let config = ConfigFile {
        private_key: our_keypair.private.to_base64(),
        listen_port: Some(listen_port),
        address: our_ip.clone(),
        network_cidr_prefix: root_cidr.prefix_len(),
        bootstrap: None, 
    };
    config.write_to_path(config_path)?;

    let db_init_data = DbInitData {
        network_name: name.to_string(),
        network_cidr: root_cidr,
        server_cidr: IpNet::new(our_ip.clone(), root_cidr.max_prefix_len())?,
        our_ip,
        public_key_base64: our_keypair.public.to_base64(),
        endpoint,
    };

    let database_path = data_dir.join(&name.to_string()).with_extension("db");
    let _ = tokio::time::sleep(Duration::from_secs(1)).await;
    ensure_crdt_datastore().await?;
    log::info!("Populating CRDT datastore with: server_name: {}", address);
    populate_crdt_datastore(
        db_init_data,
        address
    ).await?;

    // After creating config and database, actually create the WireGuard interface
    log::info!("Creating WireGuard interface for bootstrap node");
    
    // For the bootstrap node, we don't have any peers yet since we are the first node
    // We'll create the interface without peers initially
    wg::up(
        &InterfaceName::from_str("formnet")?,
        &our_keypair.private.to_base64(),
        IpNet::new(our_ip.clone(), root_cidr.prefix_len())?,
        Some(listen_port),
        None, // No peers yet for bootstrap node
        NetworkOpts::default(),
    )?;
    
    log::info!("WireGuard interface successfully created");

    println!(
        "{} Created database at {}\n",
        "[*]",
        database_path.to_string_lossy()
    );
    print!(
        r#"
        {star} Setup finished.

            Network {interface} has been {created}, but it's not started yet!

            Your new network starts with only one peer: this innernet server. Next,
            you'll want to create additional CIDRs and peers using the commands:

                {wg_manage_server} {add_cidr} {interface}, and
                {wg_manage_server} {add_peer} {interface}
            
            See https://github.com/tonarino/innernet for more detailed instruction
            on designing your network.
        
            When you're ready to start the network, you can auto-start the server:
            
                {systemctl_enable}{interface}

    "#,
        star = "[*]",
        interface = name.to_string(),
        created = "created",
        wg_manage_server = "formnet-server",
        add_cidr = "add-cidr",
        add_peer = "add-peer",
        systemctl_enable = "systemctl enable --now innernet-server@",
    );

    Ok(our_ip)
}

pub async fn ensure_crdt_datastore() -> Result<(), Box<dyn std::error::Error>> {
    match Client::new()
        .get("http://127.0.0.1:3004/ping")
        .send()
        .await?
        .json::<Value>()
        .await {
            Ok(_) => return Ok(()),
            Err(e) => return Err(Box::new(e)),
    };
}

async fn populate_crdt_datastore(
    db_init_data: DbInitData,
    server_name: String
) -> Result<(), Box<dyn std::error::Error>> {
    log::info!("Creating root cidr");
    let root_cidr = DatabaseCidr::<String, CrdtMap>::create(
        CidrContents {
            name: db_init_data.network_name.clone(),
            cidr: db_init_data.network_cidr,
            parent: None,
        },
    ).await?;

    log::info!("Succesfully created root cidr");

    tokio::time::sleep(Duration::from_millis(100)).await;

    tokio::time::sleep(Duration::from_millis(100)).await;

    log::info!("Creating local peer");

    let _me = DatabasePeer::<String, CrdtMap>::create(
        PeerContents {
            name: server_name.into(),
            ip: db_init_data.our_ip,
            cidr_id: root_cidr.id,
            public_key: db_init_data.public_key_base64,
            endpoint: Some(db_init_data.endpoint),
            is_admin: true,
            is_disabled: false,
            is_redeemed: true,
            persistent_keepalive_interval: Some(PERSISTENT_KEEPALIVE_INTERVAL_SECS),
            invite_expires: None,
            candidates: vec![],
        }
    ).await?;

    tokio::time::sleep(Duration::from_millis(100)).await;

    Ok(())
}
