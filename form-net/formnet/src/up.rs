use std::{path::PathBuf, time::Duration};
use client::util::all_installed;
use crate::{fetch, CONFIG_DIR};


pub fn up(
    loop_interval: Option<Duration>,
    hosts_path: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        log::info!("acquiring interfaces");
        let interfaces = all_installed(&PathBuf::from(CONFIG_DIR))?;
        log::info!("acquired interfaces: {interfaces:?}");

        for iface in interfaces {
            log::info!("calling fetch for interface: {iface}");
            fetch(hosts_path.clone())?;
            log::info!("called fetch for interface: {iface}");
        }

        match loop_interval {
            Some(interval) => std::thread::sleep(interval),
            None => break,
        }
    }

    Ok(())
}

