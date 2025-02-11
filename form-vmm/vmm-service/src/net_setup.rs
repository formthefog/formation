use std::process::Command;
use std::net::Ipv4Addr;
use anyhow::{Result, Context};
use ipnetwork::Ipv4Network;

type CommandResult = Result<String>;

#[derive(Debug)]
pub enum NetworkSetupError {
    AlreadyExists,
    Critical(anyhow::Error),
}

impl From<anyhow::Error> for NetworkSetupError {
    fn from(e: anyhow::Error) -> Self {
        NetworkSetupError::Critical(e)
    }
}

// Core types
#[derive(Debug)]
pub struct NetworkConfig {
    bridge_name: String,
    ip_range: Ipv4Network,
    physical_iface: String,
    dhcp_range: (Ipv4Addr, Ipv4Addr),
    lease_time: String,
}

// Command execution wrapper
fn exec(cmd: &str, args: &[&str]) -> CommandResult {
    Command::new(cmd)
        .args(args)
        .output()
        .context(format!("Failed to execute: {} {:?}", cmd, args))
        .map(|output| String::from_utf8_lossy(&output.stdout).to_string())
}

// Get default interface
fn get_default_interface() -> Result<String> {
    let route_info = exec("ip", &["route", "show", "default"])?;
    route_info
        .split_whitespace()
        .nth(4)
        .map(String::from)
        .context("No default interface found")
}

// Get occupied IP ranges
fn get_occupied_ranges() -> Result<Vec<Ipv4Network>> {
    let ip_info = exec("ip", &["addr", "show"])?;
    Ok(ip_info
        .lines()
        .filter(|line| line.contains("inet "))
        .filter_map(|line| {
            line.split_whitespace()
                .find(|&word| word.contains("/"))
                .and_then(|ip| ip.parse::<Ipv4Network>().ok())
        })
        .collect())
}

// Find available IP range
fn find_available_range(occupied: &[Ipv4Network]) -> Result<Ipv4Network> {
    let possible_ranges = [
        "192.168.0.0/16",
        "172.16.0.0/12",
    ].iter()
        .filter_map(|&range| range.parse().ok())
        .collect::<Vec<Ipv4Network>>();

    for range in possible_ranges {
        let mut addr = range.network();
        let max_addr = range.broadcast();
        while addr <= max_addr {
            if let Ok(candidate) = Ipv4Network::new(addr, 24) {
                if !occupied.iter().any(|occ| occ.overlaps(candidate)) {
                    return Ok(candidate);
                }
            }
            let octets = addr.octets();
            addr = Ipv4Addr::new(octets[0], octets[1], octets[2] + 1, octets[3]);
        }
    }
    anyhow::bail!("No available IP ranges found")
}

fn check_bridge_ip(bridge: &str) -> Result<Option<Ipv4Network>, NetworkSetupError> {
    let output = exec("ip", &["addr", "show", "dev", bridge])
        .map_err(|_| NetworkSetupError::AlreadyExists)?;
    
    Ok(output
        .lines()
        .filter(|line| line.contains("inet "))
        .filter_map(|line| line.split_whitespace()
            .find(|&word| word.contains("/"))
            .and_then(|ip| ip.parse::<Ipv4Network>().ok()))
        .next())
}

// Setup bridge interface
fn setup_bridge(config: &NetworkConfig) -> Result<(), NetworkSetupError> {
    match exec("brctl", &["addbr", &config.bridge_name]) {
        Err(e) if e.to_string().contains("already exists") => {
            if check_bridge_ip(&config.bridge_name)?.is_none() {
                exec("ip", &["addr", "add", 
                    &format!("{}", config.ip_range.ip()), 
                    "dev", &config.bridge_name]
                )?;
            }
            exec("ip", &["link", "set", &config.bridge_name, "up"])
                .map_err(NetworkSetupError::Critical)?;
        }
        Err(e) => return Err(NetworkSetupError::Critical(e)),
        Ok(_) => {
            exec("ip", &["addr", "add", 
                &format!("{}", config.ip_range.ip()), 
                "dev", &config.bridge_name]
            )?;
            exec("ip", &["link", "set", &config.bridge_name, "up"])
                .map_err(NetworkSetupError::Critical)?;
        }

    }
    Ok(())
}

// Configure NAT
fn setup_nat(config: &NetworkConfig, persist: bool) -> Result<(), NetworkSetupError> {
    match exec("sysctl", &["-w", "net.ipv4.ip_forward=1"]) {
        Err(e) => return Err(NetworkSetupError::Critical(e)),
        Ok(_) => {}
    }

    match exec("iptables", &["-C", "nat", "POSTROUTING",
        "-s", &config.ip_range.to_string(),
        "-o", &config.physical_iface,
        "-j", "MASQUERADE"]) {
        Ok(_) => return Ok(()),
        Err(_) => {
            exec("iptables", &["-t", "nat", "-A", "POSTROUTING",
            "-s", &config.ip_range.to_string(),
            "-o", &config.physical_iface,
            "-j", "MASQUERADE"])?;
        }
    }

    if persist {
        exec("netfilter-persistent", &["save"])?;
    }

    Ok(())
}

// Configure DNSMASQ
fn setup_dnsmasq(config: &NetworkConfig) -> Result<(), NetworkSetupError> {
    let conf_content = format!(
        "interface={}\n\
         port=0\n\
         dhcp-range={},{},{}\n\
         dhcp-option=6,8.8.8.8,8.8.4.4,1.1.1.1\n",
        config.bridge_name,
        config.dhcp_range.0,
        config.dhcp_range.1,
        config.lease_time
    );
    
    std::fs::write("/etc/dnsmasq.d/br0.conf", conf_content)
        .map_err(|e| NetworkSetupError::Critical(anyhow::anyhow!(e)))?;
    exec("systemctl", &["restart", "dnsmasq"])
        .map_err(NetworkSetupError::Critical)?;
    Ok(())
}

// Main configuration function
pub async fn configure_bridge_network(validate: bool, persist: bool) -> Result<(), NetworkSetupError> {
    log::info!("Configuring bridge network br0...");
    let physical_iface = get_default_interface()?;
    log::info!("Discovered main physical interface: {physical_iface}");
    let occupied_ranges = get_occupied_ranges()?;
    log::info!("Discovered Occupied IP Ranges: {occupied_ranges:?}");
    let ip_range = find_available_range(&occupied_ranges)?;
    log::info!("Found Available IP Range: {ip_range:?}");
    
    let config = NetworkConfig {
        bridge_name: "br0".to_string(),
        ip_range,
        physical_iface,
        dhcp_range: (
            ip_range.nth(10).context("Failed to get DHCP start")?,
            ip_range.nth(200).context("Failed to get DHCP end")?
        ),
        lease_time: "24h".to_string(),
    };

    log::info!("Setting up bridge");
    setup_bridge(&config)?;
    log::info!("Setting up NAT for bridge");
    setup_nat(&config, persist)?;
    log::info!("Setting up DHCP Leasing and Nameservers for bridge");
    setup_dnsmasq(&config)?;

    if validate {
        log::info!("Validating setup...");
        validate_setup(&config).await?;
        log::info!("Setup validated...");
    }

    log::info!("br0 is setup and ready to use with formation");

    Ok(())
}

// Optional validation function
pub async fn validate_setup(config: &NetworkConfig) -> Result<()> {
    let test_ns = "testns";
    
    exec("ip", &["netns", "add", test_ns])?;
    exec("ip", &["link", "add", "veth-host", "type", "veth", "peer", "name", "veth-ns"])?;
    exec("ip", &["link", "set", "veth-host", "master", &config.bridge_name])?;
    exec("ip", &["link", "set", "veth-host", "up"])?;
    exec("ip", &["link", "set", "veth-ns", "netns", test_ns])?;
    
    let test_ip = config.ip_range.nth(5).context("Failed to get test IP")?;
    exec("ip", &["netns", "exec", test_ns, "ip", "addr", "add", 
        &test_ip.to_string(), "dev", "veth-ns"])?;
    exec("ip", &["netns", "exec", test_ns, "ip", "link", "set", "veth-ns", "up"])?;
    exec("ip", &["netns", "exec", test_ns, "ip", "link", "set", "lo", "up"])?;
    
    // Test connectivity
    exec("ip", &["netns", "exec", test_ns, "ping", "-c", "3", "8.8.8.8"])?;
    
    // Cleanup
    exec("ip", &["netns", "del", test_ns])?;
    
    Ok(())
}
