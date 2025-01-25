use std::{path::PathBuf, str::FromStr};

use client::util::Api;
use shared::{interface_config::InterfaceConfig, wg, NetworkOpts, RedeemContents, REDEEM_TRANSITION_WAIT};
use wireguard_control::{DeviceUpdate, InterfaceName};
use crate::{fetch, CONFIG_DIR};

pub fn redeem(mut invitation: InterfaceConfig) -> Result<(), Box<dyn std::error::Error>> {
    let iface = InterfaceName::from_str(&invitation.interface.network_name)?;
    let private_key = invitation.interface.private_key;
    let address = invitation.interface.address;
    let server_pubkey = invitation.server.public_key.clone();
    let server_internal_ip = invitation.server.internal_endpoint.ip();
    let resolved_endpoint = invitation 
        .server
        .external_endpoint
        .resolve()?;

    wg::up(
        &iface,
        &private_key,
        address,
        None,
        Some((
            &server_pubkey,
            server_internal_ip,
            resolved_endpoint,
        )),
        NetworkOpts::default()
    )?;

    log::info!("Generating new keypair.");
    let keypair = wireguard_control::KeyPair::generate();

    log::info!(
        "Registering keypair with server (at {}).",
        &invitation.server.internal_endpoint
    );
    Api::new(&invitation.server).http_form::<_, ()>(
        "POST",
        "/user/redeem",
        RedeemContents {
            public_key: keypair.public.to_base64(),
        },
    )?;

    invitation.interface.private_key = keypair.private.to_base64();

    std::fs::create_dir_all(CONFIG_DIR)?;
    let target_conf = PathBuf::from(CONFIG_DIR).join(iface.to_string()).with_extension("conf");

    invitation.write_to_path(&target_conf, false, Some(0o600))?;
    log::info!(
        "New keypair registered. Copied config to {}.\n",
        target_conf.to_string_lossy()
    );

    log::info!("Changing keys and waiting 5s for server's WireGuard interface to transition.",);
    DeviceUpdate::new()
        .set_private_key(keypair.private)
        .apply(&iface, NetworkOpts::default().backend)?;
    std::thread::sleep(REDEEM_TRANSITION_WAIT);

    fetch(None)?;

    Ok(())
}
