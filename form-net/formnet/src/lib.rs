use std::{net::{IpAddr, Ipv4Addr, SocketAddr}, path::PathBuf, str::FromStr, time::Duration};
use axum::{http::StatusCode, Json};
use hostsfile::HostsBuilder;
use ipnet::IpNet;
use publicip::Preference;
use shared::{get_local_addrs, interface_config::{InterfaceConfig, InterfaceInfo}, wg, AddCidrOpts, Endpoint, Interface, IpNetExt, NatOpts, NetworkOpts, RedeemContents, State, PERSISTENT_KEEPALIVE_INTERVAL_SECS, REDEEM_TRANSITION_WAIT};
use shared::{interface_config::ServerInfo, Cidr, CidrTree, Hostname, Peer, PeerContents};
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, sync::broadcast::Receiver};
use wireguard_control::{Backend, Device, DeviceUpdate, InterfaceName, KeyPair, PeerConfigBuilder};
use client::{data_store::DataStore, nat::{self, NatTraverse}, util::{self, all_installed, Api}};
use innernet_server::{
    add_cidr, initialize::{create_database, populate_database, DbInitData, InitializeOpts}, open_database_connection, ConfigFile, DatabaseCidr, DatabasePeer, ServerConfig
};
use shared::wg::DeviceExt; 

use conductor::{
    HEADER_SIZE,
    TOPIC_SIZE_OFFSET,
    util::{
        try_get_topic_len, try_get_message_len, parse_next_message
    },
    subscriber::SubStream,
};
use form_types::FormnetMessage;
use tokio::net::TcpStream;
use serde::{Serialize, Deserialize};

pub const CONFIG_DIR: &'static str = "/etc/formnet";
pub const DATA_DIR: &'static str = "/var/lib/formnet";
pub const SERVER_CONFIG_DIR: &'static str = "/etc/formnet";
pub const SERVER_DATA_DIR: &'static str = "/var/lib/formnet";

pub async fn add_peer<'a>(
    peers: &[Peer],
    cidr_tree: &CidrTree<'a>,
    peer_type: &PeerType,
    peer_id: &str
) -> Result<(PeerContents, KeyPair), Box<dyn std::error::Error + Send + Sync + 'static>> {
    log::info!("Attempting to add peer {peer_id} of {peer_type:?} to formnet");
    let leaves = cidr_tree.leaves();
    log::info!("Converted CIDRs into leaves");
    let cidr = leaves.iter().filter(|cidr| cidr.name == "peers-1")
        .collect::<Vec<_>>()
        .first()
        .cloned()
        .ok_or(
            {
                log::error!("Unable to get CIDR");
                Box::new(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "CIDRs are not properly set up"
                    )
                )
            }
        )?;

    log::info!("Assigned CIDR to {peer_id}: {cidr}");

    let mut available_ip = None;
    let candidate_ips = cidr.hosts().filter(|ip| cidr.is_assignable(ip));
    for ip in candidate_ips {
        if !peers.iter().any(|peer| peer.ip == ip) {
            available_ip = Some(ip);
            break;
        }
    }
    let ip = available_ip.ok_or(
        Box::new(
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "No IPs in this CIDR are available"
            )
        )
    )?;
    log::info!("Assigned IP: {ip:?} to {peer_id}");

    let default_keypair = KeyPair::generate();

    log::info!("Generated Keypair");

    /*
    let invite_expires: Timestring = "1d".parse().map_err(|e| {
        Box::new(
            std::io::Error::new(
                std::io::ErrorKind::Other,
                e
            )
        )
    })?; 
    */
    log::info!("Generated expiration");

    let name = Hostname::from_str(&peer_id.split("_").collect::<Vec<_>>().join("-"))?;
    log::info!("Generated Hostname");

    let peer_request = PeerContents {
        name,
        ip,
        cidr_id: cidr.id,
        public_key: default_keypair.public.to_base64(),
        endpoint: None,
        is_admin: match peer_type {
            PeerType::Operator => true,
            _ => false
        },
        is_disabled: false,
        is_redeemed: false,
        persistent_keepalive_interval: Some(PERSISTENT_KEEPALIVE_INTERVAL_SECS),
        invite_expires: None, 
        candidates: vec![],
    };

    Ok((peer_request, default_keypair))
}

pub async fn server_add_peer(
    inet: &InterfaceName, 
    conf: &ServerConfig,
    peer_type: &PeerType,
    peer_id: &str,
) -> Result<InterfaceConfig, Box<dyn std::error::Error + Send + Sync + 'static>> {
    log::info!("Reading config file into ConfigFile...");
    let config = ConfigFile::from_file(conf.config_path(inet))?;
    log::info!("Opening database connection...");
    let conn = open_database_connection(inet, conf)?;
    log::info!("Collecting peers...");
    let peers = DatabasePeer::list(&conn)?
        .into_iter().map(|dp| dp.inner)
        .collect::<Vec<_>>();

    log::info!("Collecting CIDRs...");
    let cidrs = DatabaseCidr::list(&conn)?;
    let cidr_tree = CidrTree::new(&cidrs[..]);

    log::info!("calling add peer to get key pair and contents...");
    let (contents, keypair) = match add_peer(&peers, &cidr_tree, peer_type, peer_id).await {
        Ok((contents, keypair)) => (contents, keypair),
        Err(e) => {
            log::error!("Error while attempting to add peer: {e}");
            return Err(
                Box::new(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e
                    )
                )
            )
        },
    };

    log::info!("Getting Server Peer...");
    let server_peer = DatabasePeer::get(&conn, 1)?;

    log::info!("Creating peer...");
    let peer = DatabasePeer::create(&conn, contents)?;

    if Device::get(
        inet, Backend::Kernel 
    ).is_ok() {
        DeviceUpdate::new()
            .add_peer(PeerConfigBuilder::from(&*peer))
            .apply(inet, Backend::Kernel)?;
    }

    log::info!("building invitation...");
    let peer_invitation = InterfaceConfig {
        interface: InterfaceInfo {
            network_name: inet.to_string(),
            private_key: keypair.private.to_base64(),
            address: IpNet::new(peer.ip, cidr_tree.prefix_len())?,
            listen_port: None,
        },
        server: ServerInfo {
            external_endpoint: server_peer
                .endpoint
                .clone()
                .expect("The formnet server should have a WireGuard endpoint"),
            internal_endpoint: SocketAddr::new(config.address, config.listen_port),
            public_key: server_peer.public_key.clone(),
        },
    };

    log::info!("returning invitation...");
    Ok(peer_invitation)
}

pub async fn respond_with_peer_invitation<'a>(
    peer: &Peer,
    server: ServerInfo,
    root_cidr: &CidrTree<'a>,
    keypair: KeyPair,
    callback: SocketAddr,
) -> Result<(), Box<dyn std::error::Error>> {
    let invite = InterfaceConfig {
        interface: InterfaceInfo {
            network_name: "formnet".to_string(),
            private_key: keypair.private.to_base64(),
            address: IpNet::new(peer.ip, root_cidr.prefix_len())?,
            listen_port: None,
        },
        server
    };

    let mut stream = TcpStream::connect(callback).await?;
    stream.write_all(
        &serde_json::to_vec(&invite)?
    ).await?;

    Ok(())
}

pub async fn server_respond_with_peer_invitation(
    invitation: InterfaceConfig,
    callback: SocketAddr
) -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = TcpStream::connect(callback).await?;
    stream.write_all(
        &serde_json::to_vec(&invitation)?
    ).await?;

    Ok(())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FormnetEvent {
    AddPeer {
        peer_type: PeerType,
        peer_id: String,
        callback: SocketAddr
    },
    DisablePeer,
    EnablePeer,
    SetListenPort,
    OverrideEndpoint,
}

impl FormnetEvent {
    #[cfg(not(any(feature = "integration", test)))]
    pub const INTERFACE_NAME: &'static str = "formnet";
    #[cfg(any(feature = "integration", test))]
    pub const INTERFACE_NAME: &'static str = "test-net";
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PeerType {
    Operator,
    User,
    Instance,
}

impl From<form_types::PeerType> for PeerType {
    fn from(value: form_types::PeerType) -> Self {
        match value {
            form_types::PeerType::User => PeerType::User,
            form_types::PeerType::Operator => PeerType::Operator,
            form_types::PeerType::Instance => PeerType::Instance,
        }
    }
}

impl From<&form_types::PeerType> for PeerType {
    fn from(value: &form_types::PeerType) -> Self {
        match value {
            form_types::PeerType::User => PeerType::User,
            form_types::PeerType::Operator => PeerType::Operator,
            form_types::PeerType::Instance => PeerType::Instance,
        }
    }
}

impl From<PeerType> for form_types::PeerType {
    fn from(value: PeerType) -> Self {
        match value {
            PeerType::User => form_types::PeerType::User, 
            PeerType::Operator => form_types::PeerType::Operator,
            PeerType::Instance => form_types::PeerType::Instance ,
        }
    }
}

impl From<&PeerType> for form_types::PeerType {
    fn from(value: &PeerType) -> Self {
        match value {
            PeerType::User => form_types::PeerType::User, 
            PeerType::Operator => form_types::PeerType::Operator,
            PeerType::Instance => form_types::PeerType::Instance ,
        }
    }
}

pub struct FormnetSubscriber {
    stream: TcpStream
}

impl FormnetSubscriber {
    pub async fn new(uri: &str, topics: Vec<String>) -> std::io::Result<Self> {
        log::info!("Attempting to connect to broker: {uri}");
        let mut stream = TcpStream::connect(uri).await?;
        log::info!("Created TCP stream to broker: {uri}");
        let topic_str = topics.join(",");
        stream.write_all(topic_str.as_bytes()).await?;
        log::info!("Successfully subscribed to broker: {uri}");
        Ok(Self { stream })
    }
}

#[async_trait::async_trait]
impl SubStream for FormnetSubscriber {
    type Message = Vec<FormnetMessage>;

    async fn receive(&mut self) -> std::io::Result<Self::Message> {
        let mut buffer = Vec::new();
        loop {
            let mut read_buffer = [0; 4096];
            match self.stream.read(&mut read_buffer).await {
                Err(e) => log::error!("Error reading stream to buffer: {e}..."),
                Ok(n) => {
                    if n == 0 {
                        break;
                    }

                    buffer.extend_from_slice(&read_buffer[..n]);
                    let results = Self::parse_messages(&mut buffer).await?;
                    if !results.is_empty() {
                        return Ok(results);
                    }
                }
            }
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "No complete messages received",
        ))
    }

    async fn parse_messages(msg: &mut Vec<u8>) -> std::io::Result<Self::Message> {
        let mut results = Vec::new();
        while msg.len() >= HEADER_SIZE {
            let total_len = try_get_message_len(msg)?;
            if msg.len() >= total_len {
                let topic_len = try_get_topic_len(msg)?;
                let (_, message) = parse_next_message(total_len, topic_len, msg).await;
                let message_offset = TOPIC_SIZE_OFFSET + topic_len;
                let msg = &message[message_offset..message_offset + total_len];
                results.push(msg.to_vec());
            }
        }

        let msg_results = results
            .iter()
            .filter_map(|m| serde_json::from_slice(&m).ok())
            .collect();

        Ok(msg_results)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum JoinRequest {
    UserJoinRequest(UserJoinRequest),
    OperatorJoinRequest(OperatorJoinRequest),
    InstanceJoinRequest(VmJoinRequest),
}

impl JoinRequest {
    pub fn id(&self) -> String {
        match self {
            Self::UserJoinRequest(req) => req.user_id.clone(),
            Self::OperatorJoinRequest(req) => req.operator_id.clone(),
            Self::InstanceJoinRequest(req) => req.vm_id.clone(),
        }
    }

    pub fn peer_type(&self) -> PeerType {
        match self {
            Self::UserJoinRequest(_) => PeerType::User,
            Self::OperatorJoinRequest(_) => PeerType::Operator,
            Self::InstanceJoinRequest(_) => PeerType::Instance,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VmJoinRequest {
    pub vm_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OperatorJoinRequest {
    pub operator_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserJoinRequest {
    pub user_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum JoinResponse {
    Success {
        #[serde(flatten)]
        invitation: InterfaceConfig,
    },
    Error(String) 
}

pub fn create_router() -> axum::Router {
    axum::Router::new().route("/join", axum::routing::post(handle_join_request))
}

pub fn is_server() -> bool {
    PathBuf::from(SERVER_CONFIG_DIR).join(
        format!("{}.conf", FormnetMessage::INTERFACE_NAME)
    ).exists()
}

async fn handle_join_request_from_server(
    join_request: JoinRequest,
    inet: InterfaceName
) -> (StatusCode, axum::Json<JoinResponse>) {
    match server_add_peer(
        &inet,
        &ServerConfig { config_dir: SERVER_CONFIG_DIR.into(), data_dir: SERVER_DATA_DIR.into() },
        &join_request.peer_type(),
        &join_request.id()
    ).await {
        Ok(invitation) => {
            let resp = JoinResponse::Success { invitation };
            log::info!("SUCCESS! Sending Response: {resp:?}");
            return (
                StatusCode::OK,
                Json(resp)
            )
        },
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(JoinResponse::Error(e.to_string()))
            )
        }
    }
}

async fn handle_join_request_from_admin_client(
    join_request: JoinRequest,
    inet: InterfaceName
) -> (StatusCode, axum::Json<JoinResponse>) {
    let InterfaceConfig { server, ..} = {
        match InterfaceConfig::from_interface(
            &PathBuf::from(CONFIG_DIR).as_path(),
            &inet
        ) {
            Ok(config) => config,
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(JoinResponse::Error(format!("Failed to acquire config for innernet server: {}", e)))
                )
            }
        }
    };

    let api = Api::new(&server);

    log::info!("Fetching CIDRs...");
    let cidrs: Vec<Cidr> = match api.http("GET", "/admin/cidrs") {
        Ok(cidr_list) => cidr_list,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(JoinResponse::Error(format!("Failed to acquire CIDR list for innernet network: {e}")))
            )
        }
    };
    log::info!("Fetching Peers...");
    let peers: Vec<Peer> = match api.http("GET", "/admin/peers") {
        Ok(peer_list) => peer_list,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(JoinResponse::Error(format!("Failed to acquire Peer list for innernet network: {e}")))
            )
        }
    };
    log::info!("Creating CIDR Tree...");
    let cidr_tree = CidrTree::new(&cidrs[..]);

    match add_peer(
        &peers, &cidr_tree, &join_request.peer_type(), &join_request.id() 
    ).await {
        Ok((content, keypair)) => {
            log::info!("Creating peer...");
            let peer: Peer = match api.http_form("POST", "/admin/peers", content) {
                Ok(peer) => peer,
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(JoinResponse::Error(format!("Failed to create peer: {e}")))
                    )
                }
            };

            match api_respond_with_peer_invitation(&peer, server, &cidr_tree, keypair).await {
                Ok(resp) => {
                    return (
                        StatusCode::OK,
                        Json(resp)
                    )
                }
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(JoinResponse::Error(format!("Unable to build peer invitation: {e}")))
                    )
                }
            }
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(JoinResponse::Error(format!("Failed to add peer to innernet: {e}")))
            )
        }
    }
}

pub async fn handle_join_request(axum::Json(join_request): axum::Json<JoinRequest>) -> impl axum::response::IntoResponse {
    let inet = match InterfaceName::from_str(
        FormnetMessage::INTERFACE_NAME
    ) {
        Ok(inet) => inet,
        Err(e) => return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(JoinResponse::Error(format!("Failed to convert {} into InterfaceName: {e}", FormnetMessage::INTERFACE_NAME)))
        )
    };

    if is_server() {
        return handle_join_request_from_server(join_request, inet).await;
    } else {
        return handle_join_request_from_admin_client(join_request, inet).await;
    }
}

async fn api_respond_with_peer_invitation<'a>(
    peer: &Peer,
    server: ServerInfo,
    root_cidr: &CidrTree<'a>,
    keypair: KeyPair,
) -> Result<JoinResponse, Box<dyn std::error::Error>> {
    Ok(JoinResponse::Success {
        invitation: InterfaceConfig {
            interface: InterfaceInfo {
                network_name: "formnet".to_string(),
                private_key: keypair.private.to_base64(),
                address: IpNet::new(peer.ip, root_cidr.prefix_len())?,
                listen_port: None,
            },
            server
        }
    })
}

pub async fn api_shutdown_handler(
    mut rx: Receiver<()>
) {
    tokio::select! {
        res = rx.recv() => {
            log::info!("Received shutdown signal for api server: {res:?}");
        }
    }
}

pub fn redeem_invite(
    iface: &InterfaceName,
    mut config: InterfaceConfig,
    target_conf: PathBuf,
    network: NetworkOpts,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let resolved_endpoint = config
        .server
        .external_endpoint
        .resolve()?;

    wg::up(
        iface,
        &config.interface.private_key,
        config.interface.address,
        None,
        Some((
            &config.server.public_key,
            config.server.internal_endpoint.ip(),
            resolved_endpoint,
        )),
        network,
    )?;

    log::info!("Generating new keypair.");
    let keypair = wireguard_control::KeyPair::generate();

    log::info!(
        "Registering keypair with server (at {}).",
        &config.server.internal_endpoint
    );
    Api::new(&config.server).http_form::<_, ()>(
        "POST",
        "/user/redeem",
        RedeemContents {
            public_key: keypair.public.to_base64(),
        },
    )?;

    config.interface.private_key = keypair.private.to_base64();
    config.write_to_path(&target_conf, false, Some(0o600))?;
    log::info!(
        "New keypair registered. Copied config to {}.\n",
        target_conf.to_string_lossy()
    );

    log::info!("Changing keys and waiting 5s for server's WireGuard interface to transition.",);
    DeviceUpdate::new()
        .set_private_key(keypair.private)
        .apply(iface, network.backend)?;
    std::thread::sleep(REDEEM_TRANSITION_WAIT);

    let network = NetworkOpts {
        no_routing: false,
        backend: wireguard_control::Backend::Kernel,
        mtu: None,
    };

    let nat = NatOpts {
        no_nat_traversal: false,
        exclude_nat_candidates: Vec::new(),
        no_nat_candidates: false
    };

    let config_dir: PathBuf = CONFIG_DIR.into();
    let data_dir: PathBuf = DATA_DIR.into();

    fetch(
        &iface,
        &config_dir,
        &data_dir,
        &network,
        None,
        &nat
    )?;

    let interface = Interface::from_str(&iface.to_string())?;

    up(
        Some(interface),
        &config_dir,
        &data_dir,
        &network,
        Some(Duration::from_secs(60)),
        None,
        &nat,
    )?;

    Ok(())
}

fn fetch(
    interface: &InterfaceName,
    config_dir: &PathBuf,
    data_dir: &PathBuf,
    network: &NetworkOpts,
    hosts_path: Option<PathBuf>,
    nat: &NatOpts,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let config = InterfaceConfig::from_interface(&config_dir, interface)?;
    let interface_up = match Device::list(wireguard_control::Backend::Kernel) {
        Ok(interfaces) => interfaces.iter().any(|name| name == interface),
        _ => false,
    };

    if !interface_up {
        log::info!(
            "bringing up interface {}.",
            interface.as_str_lossy()
        );
        let resolved_endpoint = config
            .server
            .external_endpoint
            .resolve()?;
        wg::up(
            interface,
            &config.interface.private_key,
            config.interface.address,
            config.interface.listen_port,
            Some((
                &config.server.public_key,
                config.server.internal_endpoint.ip(),
                resolved_endpoint,
            )),
            *network,
        )?;
    }

    log::info!(
        "fetching state for {} from server...",
        interface.as_str_lossy()
    );
    let mut store = DataStore::open_or_create(&data_dir, interface)?;
    let api = Api::new(&config.server);
    let State { peers, cidrs } = api.http("GET", "/user/state")?;

    let device = Device::get(interface, network.backend)?;
    let modifications = device.diff(&peers);

    let updates = modifications
        .iter()
        .inspect(|diff| util::print_peer_diff(&store, diff))
        .cloned()
        .map(PeerConfigBuilder::from)
        .collect::<Vec<_>>();

    if !updates.is_empty() || !interface_up {
        DeviceUpdate::new()
            .add_peers(&updates)
            .apply(interface, network.backend)?;

        if let Some(path) = hosts_path {
            update_hosts_file(interface, path, &peers)?;
        }

        println!();
        log::info!("updated interface {}\n", interface.as_str_lossy());
    } else {
        log::info!("{}", "peers are already up to date");
    }
    let interface_updated_time = std::time::Instant::now();

    store.set_cidrs(cidrs);
    store.update_peers(&peers)?;
    store.write()?;

    let candidates: Vec<Endpoint> = get_local_addrs()?
        .filter(|ip| !nat.is_excluded(*ip))
        .map(|addr| SocketAddr::from((addr, device.listen_port.unwrap_or(51820))).into())
        .collect::<Vec<Endpoint>>();
    log::info!(
        "reporting {} interface address{} as NAT traversal candidates",
        candidates.len(),
        if candidates.len() == 1 { "" } else { "es" },
    );
    for candidate in &candidates {
        log::debug!("  candidate: {}", candidate);
    }
    match api.http_form::<_, ()>("PUT", "/user/candidates", &candidates) {
        Err(ureq::Error::Status(404, _)) => {
            log::warn!("your network is using an old version of innernet-server that doesn't support NAT traversal candidate reporting.")
        },
        Err(e) => return Err(e.into()),
        _ => {},
    }
    log::debug!("candidates successfully reported");

    if nat.no_nat_traversal {
        log::debug!("NAT traversal explicitly disabled, not attempting.");
    } else {
        let mut nat_traverse = NatTraverse::new(interface, network.backend, &modifications)?;

        // Give time for handshakes with recently changed endpoints to complete before attempting traversal.
        if !nat_traverse.is_finished() {
            std::thread::sleep(nat::STEP_INTERVAL - interface_updated_time.elapsed());
        }
        loop {
            if nat_traverse.is_finished() {
                break;
            }
            log::info!(
                "Attempting to establish connection with {} remaining unconnected peers...",
                nat_traverse.remaining()
            );
            nat_traverse.step()?;
        }
    }

    Ok(())
}

fn update_hosts_file(
    interface: &InterfaceName,
    hosts_path: PathBuf,
    peers: &[Peer],
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let mut hosts_builder = HostsBuilder::new(format!("innernet {interface}"));
    for peer in peers {
        hosts_builder.add_hostname(
            peer.contents.ip,
            format!("{}.{}.wg", peer.contents.name, interface),
        );
    }
    match hosts_builder.write_to(&hosts_path) {
        Ok(has_written) if has_written => {
            log::info!(
                "updated {} with the latest peers.",
                hosts_path.to_string_lossy()
            )
        },
        Ok(_) => {},
        Err(e) => log::warn!("failed to update hosts ({})", e),
    };

    Ok(())
}

fn up(
    interface: Option<Interface>,
    config_dir: &PathBuf,
    data_dir: &PathBuf,
    network: &NetworkOpts,
    loop_interval: Option<Duration>,
    hosts_path: Option<PathBuf>,
    nat: &NatOpts,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    loop {
        let interfaces = match &interface {
            Some(iface) => vec![iface.clone()],
            None => all_installed(&config_dir)?,
        };

        for iface in interfaces {
            fetch(&iface, config_dir, data_dir, network, hosts_path.clone(), nat)?;
        }

        match loop_interval {
            Some(interval) => std::thread::sleep(interval),
            None => break,
        }
    }

    Ok(())
}

pub fn init(
    conf: &ServerConfig,
    opts: InitializeOpts
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    log::info!("Setting up directories for configs...");
    shared::ensure_dirs_exist(&[conf.config_dir(), conf.database_dir()]).map_err(|e| {
        Box::new(
            e
        )
    })?;

    log::info!("Acquiring interface name...");
    let name: Interface = if let Some(name) = opts.network_name {
        name
    } else {
        Interface::from_str("formnet")?
    };

    log::info!("Acquiring root cidr...");
    let root_cidr: IpNet = if let Some(cidr) = opts.network_cidr {
        cidr
    } else {
        IpNet::new(
            IpAddr::V4(Ipv4Addr::new(10,0,0,0)),
            8
        )?
    };

    log::info!("Acquiring listen port...");
    let listen_port: u16 = if let Some(listen_port) = opts.listen_port {
        listen_port
    } else {
        51820
    };

    log::info!("listen port: {}", listen_port);

    log::info!("Acquiring endpoint from public ip...");
    let endpoint: Endpoint = if let Some(endpoint) = opts.external_endpoint {
        endpoint
    } else {
        let ip = publicip::get_any(Preference::Ipv4)
            .ok_or_else(|| {
                Box::new(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "couldn't get external IP"
                    )
                )
            })?;
        SocketAddr::new(ip, listen_port).into()
    };

    let our_ip = root_cidr
        .hosts()
        .find(|ip| root_cidr.is_assignable(ip))
        .unwrap();

    log::info!("Acquired formnet ip {our_ip}...");
    let config_path = conf.config_path(&name);
    let our_keypair = KeyPair::generate();

    log::info!("building config...");
    let config = ConfigFile {
        private_key: our_keypair.private.to_base64(),
        listen_port,
        address: our_ip,
        network_cidr_prefix: root_cidr.prefix_len(),
    };
    log::info!("writing config to config dir...");
    config.write_to_path(config_path)?;

    log::info!("Setting up Database Initial direcotry...");
    let db_init_data = DbInitData {
        network_name: name.to_string(),
        network_cidr: root_cidr,
        server_cidr: IpNet::new(our_ip, root_cidr.max_prefix_len())?,
        our_ip,
        public_key_base64: our_keypair.public.to_base64(),
        endpoint,
    };

    log::info!("Populating database initially...");
    let database_path = conf.database_path(&name);
    let conn = create_database(&database_path)?;
    populate_database(&conn, db_init_data)?;

    println!(
        "{} Created database at {}\n",
        "[*]",
        database_path.to_string_lossy()
    );

    log::info!("Setup up initial database... Adding CIDR");
    let cidr_opts = AddCidrOpts {
        name: Some(Hostname::from_str("peers-1")?),
        parent: Some("formnet".to_string()),
        cidr: Some(IpNet::new(
            IpAddr::V4(
                Ipv4Addr::new(
                    10, 1, 0, 0
                )
            ),
            16
        )?),
        yes: true,
    };

    add_cidr(&*name, conf, cidr_opts)?;

    log::info!("Added CIDR");
    Ok(())
}


#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use tokio::net::TcpListener;
    use reqwest::Client;

    #[tokio::test]
    async fn test_user_join() -> Result<(), Box<dyn std::error::Error>> {
        let (tx, rx) = tokio::sync::broadcast::channel(1);
        let api_shutdown = rx.resubscribe();
        
        let api_handle = tokio::spawn(async move {
            let api = create_router();
            let listener = TcpListener::bind("0.0.0.0:3001").await?;

            let _ = axum::serve(
                listener,
                api
            ).with_graceful_shutdown(
                api_shutdown_handler(api_shutdown)
            ).await;

            Ok::<(), Box<dyn std::error::Error + Send + Sync + 'static>>(())
        });

        tokio::time::sleep(Duration::from_secs(2)).await;

        let user_id = random_word::gen(random_word::Lang::En).to_string();
        let client = Client::new();
        let response = client.post("http://localhost:3001/join")
            .json(&JoinRequest::UserJoinRequest(
                UserJoinRequest {
                user_id
            })).send().await?;

        log::info!("{:?}", response);

        let status = response.status().clone();

        // Let's print out the error response body if we get a non-success status
        if !response.status().is_success() {
            let error_body = response.text().await?.clone();
            log::info!("Error response body: {}", error_body);
            // Now fail the test
            panic!("Request failed with status {} and error: {}", status, error_body);
        }

        assert!(response.status().is_success());

        let join_response = response.json::<JoinResponse>().await?;

        log::info!("{}", serde_json::to_string_pretty(&join_response)?);

        let _ = tx.send(());
        let _ = api_handle.await?;

        Ok(())
    }
}


