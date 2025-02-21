use std::{net::{IpAddr, SocketAddr}, path::PathBuf, str::FromStr, time::Instant};
use form_types::state::{Response as StateResponse, Success};
use client::{data_store::DataStore, nat::{self, NatTraverse}, util};
use formnet_server::ConfigFile;
use futures::{stream::FuturesUnordered, StreamExt};
use hostsfile::HostsBuilder;
use reqwest::{Client, Response as ServerResponse};
use shared::{get_local_addrs, wg::{self, DeviceExt}, Endpoint, IoErrorContext, NatOpts, NetworkOpts, Peer, PeerDiff};
use wireguard_control::{Backend, Device, DeviceUpdate, InterfaceName, PeerConfigBuilder};

use crate::{api::{BootstrapInfo, Response}, CONFIG_DIR, DATA_DIR, NETWORK_NAME};

pub async fn fetch(
    hosts_path: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let interface = InterfaceName::from_str(NETWORK_NAME)?;
    let config_dir = PathBuf::from(CONFIG_DIR);
    let data_dir = PathBuf::from(DATA_DIR);
    let network = NetworkOpts::default();
    let config = ConfigFile::from_file(config_dir.join(NETWORK_NAME).with_extension("conf"))?; 
    let interface_up = interface_up(interface.clone()).await;
    let (pubkey, internal, external) = get_bootstrap_info_from_config(&config).await?;
    let store = DataStore::<String>::open_or_create(&data_dir, &interface)?;

    let admins = store.peers().iter().filter_map(|p| {
        if p.is_admin {
            Some(p.clone())
        } else {
            None
        }
    }).collect::<Vec<Peer<String>>>();

    let host_port = external.port();

    if !interface_up {
        log::info!(
            "bringing up interface {}.",
            interface.as_str_lossy()
        );
        wg::up(
            &interface,
            &config.private_key,
            config.address.into(),
            None,
            Some((
                &pubkey,
                internal,
                external.clone(),
            )),
            NetworkOpts::default(),
        )?;
    }

    log::info!(
        "fetching state for {} from server...",
        interface.as_str_lossy()
    );

    let bootstrap_resp = Client::new().get(format!("http://{external}/fetch")).send();
    match bootstrap_resp.await {
        Ok(resp) => {
            if let Err(e) = handle_server_response(resp, &interface, network, data_dir.clone(), interface_up, external.to_string(), config.address.to_string(), host_port, hosts_path.clone()).await {
                log::error!(
                    "Error handling server response from fetch call: {e}"
                )
            }
        }
        Err(e) => {
            log::error!("Error fetching from bootstrap: {e}");
            for admin in admins {
                if let Some(ref external) = &admin.endpoint {
                    if let Ok(endpoint) = external.resolve() {
                        if let Ok(resp) = Client::new().get(format!("http://{endpoint}/fetch")).send().await {
                            match handle_server_response(
                                resp, 
                                &interface, 
                                network, 
                                data_dir.clone(), 
                                interface_up, 
                                endpoint.to_string(),
                                config.address.to_string(), 
                                endpoint.port(), 
                                hosts_path.clone()).await 
                            {
                                Ok(_) => break,
                                Err(e) => log::error!("Error handling server response from fetch call to {external}: {e}"),
                            }
                        }
                    }
                }
            }
        },
    }

    Ok(())
}

async fn interface_up(interface: InterfaceName) -> bool {
    #[cfg(target_os = "linux")]
    {
        let up = match Device::list(wireguard_control::Backend::Kernel) {
            Ok(interfaces) => interfaces.iter().any(|name| *name == interface),
            _ => false,
        };
        log::info!("Interface up?: {up}");
        up
    }
    #[cfg(not(target_os = "linux"))]
    {
        let up = match Device::list(wireguard_control::Backend::Userspace) {
            Ok(interfaces) => interfaces.iter().any(|name| *name == interface),
            _ => false,
        };
        log::info!("Interface up?: {up}");
        up
    }
}

async fn get_bootstrap_info_from_config(config: &ConfigFile) -> Result<(String, IpAddr, SocketAddr), Box<dyn std::error::Error>> {
    if let Some(bootstrap) = &config.bootstrap {
        let bytes = hex::decode(bootstrap)?;
        let info: BootstrapInfo = serde_json::from_slice(&bytes)?;
        if let (Some(external), Some(internal)) = (info.external_endpoint, info.internal_endpoint) {
            return Ok((info.pubkey, internal, external))
        } else {
            return Err(Box::new(
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Bootstrap peer must have both an external and internal endpoint"
                ))
            )
        }
    } else {
        return Err(Box::new(
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "Cannot fetch without a bootstrap peer"
            )
        ))
    }
}

async fn handle_server_response(
    resp: ServerResponse,
    interface: &InterfaceName,
    network: NetworkOpts,
    data_dir: PathBuf,
    interface_up: bool,
    external: String,
    my_ip: String,
    host_port: u16,
    hosts_path: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    match resp.json::<Response>().await {
        Ok(Response::Fetch(peers)) => {
            if let Err(e) = handle_peer_updates(
                peers,
                &interface,
                network,
                data_dir,
                interface_up,
                hosts_path,
                external,
                my_ip,
                host_port,
            ).await {
                log::error!("Error handling peer updates: {e}");
            }
        }
        Err(e) => {
            log::error!("Error trying to fetch peers: {e}");

        }
        _ => {
            log::error!("Received an invalid response from `fetch`"); 
        }
    }

    Ok(())
}

async fn handle_peer_updates(
    peers: Vec<Peer<String>>,
    interface: &InterfaceName,
    network: NetworkOpts,
    data_dir: PathBuf,
    interface_up: bool,
    hosts_path: Option<PathBuf>,
    external: String,
    my_ip: String,
    _host_port: u16
) -> Result<(), Box<dyn std::error::Error>> {
    let device = Device::get(&interface, network.backend)?;
    log::info!("Current peer info:");
    for peer in &device.peers {
        log::info!("\t{:?}\n", peer);
    }
    let modifications = device.diff(&peers);
    let mut store = DataStore::open_or_create(&data_dir, &interface)?;
    let updates = modifications
        .iter()
        .inspect(|diff| util::print_peer_diff(&store, diff))
        .cloned()
        .map(PeerConfigBuilder::from)
        .collect::<Vec<_>>();

    log::info!("Updating peers: {updates:?}");

    if !updates.is_empty() || !interface_up {
        DeviceUpdate::new()
            .add_peers(&updates)
            .apply(&interface, network.backend)?;

        if let Some(path) = hosts_path {
            update_hosts_file(&interface, path, &peers)?;
        }

        log::info!("updated interface {}\n", interface.as_str_lossy());
    } else {
        log::info!("{}", "peers are already up to date");
    }
    log::info!("Updated interface, updating datastore");
    let interface_updated_time = std::time::Instant::now();
    store.update_peers(&peers)?;
    log::info!("Updated peers, writing to datastore");
    store.write()?;
    log::info!("Getting candidates...");
    let candidates: Vec<Endpoint> = get_local_addrs()?
        .filter(|ip| !NatOpts::default().is_excluded(*ip))
        .map(|addr| SocketAddr::from((addr, device.listen_port.unwrap_or(51820))).into())
        .collect::<Vec<Endpoint>>().iter().filter_map(|ep| match ep.resolve() {
            Ok(addr) => Some(addr.into()),
            Err(_) => None
        }).collect::<Vec<Endpoint>>();
    log::info!(
        "reporting {} interface address{} as NAT traversal candidates",
        candidates.len(),
        if candidates.len() == 1 { "" } else { "es" },
    );
    for candidate in &candidates {
        log::debug!("  candidate: {}", candidate);
    }

    if !candidates.is_empty() {
        match Client::new().post(format!("http://{external}/{}/candidates", my_ip))
            .json(&candidates)
            .send().await {
                Ok(_) => log::info!("Successfully sent candidates"),
                Err(e) => log::error!("Unable to send candidates: {e}")
        }
    }
    if NatOpts::default().no_nat_traversal {
        log::debug!("NAT traversal explicitly disabled, not attempting.");
    } else {
        let mut nat_traverse = NatTraverse::new(&interface, network.backend, &modifications)?;

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

    log::info!("New device info:");
    for peer in &device.peers {
        log::info!("\t{:?}\n", peer);
    }

    Ok(())
}

#[allow(unused)]
async fn try_nat_traversal_server(
    device: Device, 
    my_ip: String, 
    interface: InterfaceName, 
    network: NetworkOpts, 
    modifications: &[PeerDiff<'static, String>],
    interface_updated_time: Instant,
) -> Result<(), Box<dyn std::error::Error>> {
    let candidates: Vec<Endpoint> = get_local_addrs()?
        .filter(|ip| !NatOpts::default().is_excluded(*ip))
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
    if !candidates.is_empty() {
        let all_admin = Client::new().get(format!("http://127.0.0.1:3004/user/list_admin")).send().await?.json::<StateResponse<Peer<String>>>().await?;
        if let StateResponse::Success(Success::List(admin)) = all_admin {
            let valid_admin: Vec<_> = admin.iter().filter_map(|p| {
                match &p.endpoint {
                    Some(endpoint) => {
                        match endpoint.resolve() {
                            Ok(_) => Some(p.clone()),
                            Err(e) => {
                                log::error!("Unable to resolve endpoint for {}: {e}", &p.id);
                                None
                            }
                        }
                    }
                    None => None
                }
            }).collect();
            let mut futures: FuturesUnordered<_> = valid_admin.iter().map(|p| {
                let addr = p.endpoint.clone().unwrap().resolve().unwrap();
                let ip = addr.ip().to_string();
                let port = addr.port();
                Client::new().post(format!("http://{ip}:{port}/{}/candidates", my_ip))
                    .json(&candidates)
                    .send()     
            }).collect();

            while let Some(complete) = futures.next().await {
                if let Err(e) = complete {
                    log::error!("Error sending candidates to one of admin: {e}"); 
                }
            }
        }
    }

    if NatOpts::default().no_nat_traversal {
        log::debug!("NAT traversal explicitly disabled, not attempting.");
        return Ok(())
    } else {
        let mut nat_traverse = NatTraverse::new(&interface, network.backend, &modifications)?;

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
    peers: &[Peer<String>],
) -> Result<(), Box<dyn std::error::Error>> {
    let mut hosts_builder = HostsBuilder::new(format!("innernet {interface}"));
    for peer in peers {
        hosts_builder.add_hostname(
            peer.contents.ip,
            format!("{}.{}.wg", peer.contents.name, interface),
        );
    }
    match hosts_builder.write_to(&hosts_path).with_path(&hosts_path) {
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

pub async fn fetch_server(
    peers: Vec<Peer<String>>
) -> Result<(), Box<dyn std::error::Error>> {
    let interface = InterfaceName::from_str("formnet")?;
    let config = ConfigFile::from_file(PathBuf::from(CONFIG_DIR).join(NETWORK_NAME).with_extension("conf"))?; 
    let device = Device::get(&interface, NetworkOpts::default().backend)?;
    let modifications = device.diff(&peers);
    let updates = modifications
        .iter()
        .cloned()
        .map(PeerConfigBuilder::from)
        .collect::<Vec<_>>();

    let interface_up = interface_up(interface.clone()).await;
    let interface_updated_time = std::time::Instant::now();
    if !updates.is_empty() || !interface_up {
        DeviceUpdate::new()
            .add_peers(&updates)
            .apply(&interface, NetworkOpts::default().backend)?;

        log::info!("updated interface {}\n", interface.as_str_lossy());
    } else {
        log::info!("{}", "peers are already up to date");
    }

    let candidates: Vec<Endpoint> = get_local_addrs()?
        .filter(|ip| !NatOpts::default().is_excluded(*ip))
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
    let all_admin = Client::new().get(format!("http://127.0.0.1:3004/user/list_admin")).send().await?.json::<StateResponse<Peer<String>>>().await?;
    if let StateResponse::Success(Success::List(admin)) = all_admin {
        let valid_admin: Vec<_> = admin.iter().filter_map(|p| {
            match &p.endpoint {
                Some(endpoint) => {
                    match endpoint.resolve() {
                        Ok(_) => Some(p.clone()),
                        Err(e) => {
                            log::error!("Unable to resolve endpoint for {}: {e}", &p.id);
                            None
                        }
                    }
                }
                None => None
            }
        }).collect();

        let mut futures: FuturesUnordered<_> = valid_admin.iter().map(|p| {
            let addr = p.endpoint.clone().unwrap().resolve().unwrap();
            let ip = addr.ip().to_string();
            let port = addr.port();
            Client::new().post(format!("http://{ip}:{port}/{}/candidates", config.address.to_string()))
                .json(&candidates)
                .send()     
        }).collect();

        while let Some(complete) = futures.next().await {
            if let Err(e) = complete {
                log::error!("Error sending candidates to one of admin: {e}"); 
            }
        }
    }

    if NatOpts::default().no_nat_traversal {
        log::debug!("NAT traversal explicitly disabled, not attempting.");
        return Ok(())
    } else {
        let mut nat_traverse = NatTraverse::new(&interface, NetworkOpts::default().backend, &modifications)?;
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

pub async fn report_initial_candidates(bootstraps: Vec<String>, my_ip: String) -> Result<(), Box<dyn std::error::Error>> {
    let device = Device::get(&InterfaceName::from_str("formnet")?, Backend::default())?;
    log::info!("Getting candidates...");
    let candidates: Vec<Endpoint> = get_local_addrs()?
        .filter(|ip| !NatOpts::default().is_excluded(*ip))
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

    for bootstrap in bootstraps {
        log::info!("reporting candidates to {bootstrap}/{my_ip}/candidates");
        if let Err(e) = Client::new().post(format!("http://{bootstrap}/{}/candidates", my_ip))
            .json(&candidates)
            .send().await {
                log::error!("Error sending NAT candidates: {e}");
        } else {
            log::info!("Successfully sent candidates");
            break;
        }
    }

    Ok(())
}

pub async fn report_candidates(admins: Vec<String>, my_ip: String) -> Result<(), Box<dyn std::error::Error>> { 
    let device = Device::get(&InterfaceName::from_str("formnet")?, Backend::default())?;
    log::info!("Getting candidates...");
    let candidates: Vec<Endpoint> = get_local_addrs()?
        .filter(|ip| !NatOpts::default().is_excluded(*ip))
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
    for admin in admins {
        if let Ok(_) = Client::new().post(format!("http://{admin}/{}/candidates", my_ip))
            .json(&candidates)
            .send().await {
                log::info!("Successfully sent candidates");
                break;
        }
    }

    Ok(())
}
