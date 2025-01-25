pub mod init;
pub mod add_peer;
pub mod serve;
pub mod join;
pub mod up;
pub mod fetch;
pub mod redeem;

pub use init::*;
pub use add_peer::*;
pub use serve::*;
pub use join::*;
pub use up::*;
pub use fetch::*;
pub use redeem::*;

pub const CONFIG_DIR: &'static str = "/etc/formnet";
pub const DATA_DIR: &'static str = "/var/lib/formnet";
pub const SERVER_CONFIG_DIR: &'static str = "/etc/formnet";
pub const SERVER_DATA_DIR: &'static str = "/var/lib/formnet";
pub const NETWORK_NAME: &str = "formnet";
pub const NETWORK_CIDR: &str = "10.0.0.0/8"; 

/*
pub async fn respond_with_peer_invitation<'a>(
    peer: &Peer<String>,
    server: ServerInfo,
    root_cidr: &CidrTree<'a, String>,
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

async fn api_respond_with_peer_invitation(
    peer: &Peer<String>,
    server: ServerInfo,
    root_cidr: &CidrTree<'a, String>,
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

    if let Some(parent) = target_conf.parent().take() {
        std::fs::create_dir_all(parent)?;
    }

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

    fetch::<T>(
        &iface,
        &config_dir,
        &data_dir,
        &network,
        None,
        &nat
    )?;

    Ok(())
}

fn fetch<T: Display + Clone + PartialEq + DeserializeOwned + Serialize>(
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
    let mut store = DataStore::<T>::open_or_create(&data_dir, interface)?;
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
            log::warn!("your network is using an old version of formnet-server that doesn't support NAT traversal candidate reporting.")
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

fn update_hosts_file<T: Display + Clone + PartialEq>(
    interface: &InterfaceName,
    hosts_path: PathBuf,
    peers: &[Peer<T>],
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let mut hosts_builder = HostsBuilder::new(format!("formnet {interface}"));
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

pub fn up<T: Display + Clone + PartialEq + Serialize + DeserializeOwned>(
    interface: Option<Interface>,
    config_dir: &PathBuf,
    data_dir: &PathBuf,
    network: &NetworkOpts,
    loop_interval: Option<Duration>,
    hosts_path: Option<PathBuf>,
    nat: &NatOpts,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    loop {
        log::info!("acquiring interfaces");
        let interfaces = match &interface {
            Some(iface) => vec![iface.clone()],
            None => all_installed(&config_dir)?,
        };
        log::info!("acquired interfaces: {interfaces:?}");

        for iface in interfaces {
            log::info!("calling fetch for interface: {iface}");
            fetch::<T>(&iface, config_dir, data_dir, network, hosts_path.clone(), nat)?;
            log::info!("called fetch for interface: {iface}");
        }

        match loop_interval {
            Some(interval) => std::thread::sleep(interval),
            None => break,
        }
    }

    Ok(())
}

/// Only used if a bootstrap node is NOT provided
pub async fn init(
    conf: &ServerConfig,
    opts: InitializeOpts,
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
        bootstrap
    };
    log::info!("writing config to config dir...");
    config.write_to_path(config_path)?;

    log::info!("Setting up Database Initial directory...");
    let db_init_data = DbInitData {
        network_name: name.to_string(),
        network_cidr: root_cidr,
        server_cidr: IpNet::new(our_ip, root_cidr.max_prefix_len())?,
        our_ip,
        public_key_base64: our_keypair.public.to_base64(),
        endpoint,
    };

    ensure_crdt_datastore(bootstrap).await?;
    bootstrap_crdt_datastore(db_init_data).await?;

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

    <CrdtMap as FormnetNode>::add_cidr(&CrdtMap, interface, conf, opts).await?;

    log::info!("Added CIDR");
    Ok(())
}

async fn ensure_crdt_datastore(
    bootstrap: String,
) -> Result<(), Box<dyn std::error::Error>> {
    todo!()
}

async fn bootstrap_crdt_datastore(
    bootstrap_node: PeerContents<String>,
    network_cidr: CidrContents<String>,
*/
