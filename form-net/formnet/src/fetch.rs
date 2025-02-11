use std::{net::SocketAddr, path::PathBuf, str::FromStr};

use client::{data_store::DataStore, nat::{self, NatTraverse}, util};
use formnet_server::ConfigFile;
use hostsfile::HostsBuilder;
use reqwest::Client;
use shared::{get_local_addrs, wg::{self, DeviceExt}, Endpoint, IoErrorContext, NatOpts, NetworkOpts, Peer};
use wireguard_control::{Device, DeviceUpdate, InterfaceName, PeerConfigBuilder};

use crate::{api::{BootstrapInfo, Response}, CONFIG_DIR, DATA_DIR, NETWORK_NAME};

pub async fn fetch(
    hosts_path: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let interface = InterfaceName::from_str(NETWORK_NAME)?;
    let config_dir = PathBuf::from(CONFIG_DIR);
    let data_dir = PathBuf::from(DATA_DIR);
    let network = NetworkOpts::default();
    let nat_opts = NatOpts::default();
    let config = ConfigFile::from_file(config_dir.join(NETWORK_NAME).with_extension("conf"))?; 
    #[cfg(target_os = "linux")]
    let interface_up = match Device::list(wireguard_control::Backend::Kernel) {
        Ok(interfaces) => interfaces.iter().any(|name| *name == interface),
        _ => false,
    };

    log::info!("Interface up?: {interface_up}");

    #[cfg(not(target_os = "linux"))]
    let interface_up = match Device::list(wireguard_control::Backend::Userspace) {
        Ok(interfaces) => interfaces.iter().any(|name| *name == interface),
        _ => false,
    };

    let (pubkey, internal, external) = {
        if let Some(bootstrap) = config.bootstrap {
            let bytes = hex::decode(bootstrap)?;
            let info: BootstrapInfo = serde_json::from_slice(&bytes)?;
            if let (Some(external), Some(internal)) = (info.external_endpoint, info.internal_endpoint) {
                (info.pubkey, internal, external)
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
    };


    if !interface_up {
        log::info!(
            "bringing up interface {}.",
            interface.as_str_lossy()
        );
        wg::up(
            &interface,
            &config.private_key,
            config.address.into(),
            config.listen_port,
            Some((
                &pubkey,
                internal,
                external,
            )),
            NetworkOpts::default(),
        )?;
    }

    log::info!(
        "fetching state for {} from server...",
        interface.as_str_lossy()
    );

    let resp = Client::new().get(format!("http://{internal}:51820/fetch")).send().await;
    match resp {
        Ok(r) => match r.json::<Response>().await {
            Ok(Response::Fetch(peers)) => {
                let device = Device::get(&interface, network.backend)?;
                let modifications = device.diff(&peers);
                let mut store = DataStore::open_or_create(&data_dir, &interface)?;
                let updates = modifications
                    .iter()
                    .inspect(|diff| util::print_peer_diff(&store, diff))
                    .cloned()
                    .map(PeerConfigBuilder::from)
                    .collect::<Vec<_>>();

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
                let interface_updated_time = std::time::Instant::now();
                store.update_peers(&peers)?;
                store.write()?;
                let candidates: Vec<Endpoint> = get_local_addrs()?
                    .filter(|ip| !nat_opts.is_excluded(*ip))
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
                match Client::new().post(format!("http://{internal}/{}/candidates", config.address))
                    .json(&candidates)
                    .send().await {
                        Ok(_) => log::info!("Successfully sent candidates"),
                        Err(e) => log::error!("Unable to send candidates: {e}")
                }
                if nat_opts.no_nat_traversal {
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
            }
            Err(e) => {
                log::error!("Error trying to fetch peers: {e}");
            }
            _ => {
                log::error!("Received an invalid response from `fetch`"); 
            }
        }
        Err(e) => {
            log::error!("Error fetching users: {e}");
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

