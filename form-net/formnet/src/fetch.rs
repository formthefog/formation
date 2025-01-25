use std::{net::SocketAddr, path::PathBuf, str::FromStr};

use client::{data_store::DataStore, nat::{self, NatTraverse}, util::{self, Api}};
use hostsfile::HostsBuilder;
use shared::{get_local_addrs, interface_config::InterfaceConfig, wg::{self, DeviceExt}, Endpoint, IoErrorContext, NatOpts, NetworkOpts, Peer, State};
use wireguard_control::{Device, DeviceUpdate, InterfaceName, PeerConfigBuilder};

use crate::{CONFIG_DIR, DATA_DIR, NETWORK_NAME};

pub fn fetch(
    hosts_path: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let interface = InterfaceName::from_str(NETWORK_NAME)?;
    let config_dir = PathBuf::from(CONFIG_DIR);
    let data_dir = PathBuf::from(DATA_DIR);
    let network = NetworkOpts::default();
    let nat_opts = NatOpts::default();
    let config = InterfaceConfig::from_interface(&config_dir, &interface)?; 
    let interface_up = match Device::list(wireguard_control::Backend::Kernel) {
        Ok(interfaces) => interfaces.iter().any(|name| *name == interface),
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
            &interface,
            &config.interface.private_key,
            config.interface.address,
            config.interface.listen_port,
            Some((
                &config.server.public_key,
                config.server.internal_endpoint.ip(),
                resolved_endpoint,
            )),
            NetworkOpts::default(),
        )?;
    }

    log::info!(
        "fetching state for {} from server...",
        interface.as_str_lossy()
    );
    let mut store = DataStore::<String>::open_or_create(&data_dir, &interface)?;
    let api = Api::new(&config.server);
    let State { peers, cidrs } = api.http("GET", "/user/state")?;

    let device = Device::get(&interface, network.backend)?;
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
            .apply(&interface, network.backend)?;

        if let Some(path) = hosts_path {
            update_hosts_file(&interface, path, &peers)?;
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
    match api.http_form::<_, ()>("PUT", "/user/candidates", &candidates) {
        Err(ureq::Error::Status(404, _)) => {
            log::warn!("your network is using an old version of formnet-server that doesn't support NAT traversal candidate reporting.")
        },
        Err(e) => return Err(e.into()),
        _ => {},
    }
    log::debug!("candidates successfully reported");

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

