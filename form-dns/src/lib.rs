use std::net::Ipv4Addr;

pub mod store;
pub mod proxy;
pub mod authority;
pub mod api;
pub mod geolocation;
pub mod geo_resolver;
pub mod geo_util;
pub mod health;
pub mod health_tracker;

pub fn resolvectl_domain() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::process::Command::new("resolvectl")
        .arg("domain")
        .arg("br0")
        .arg(r#"'~.'"#)
        .output()?;

    let out = if output.status.success() {
        String::from_utf8_lossy(&output.stdout)
    } else {
        String::from_utf8_lossy(&output.stderr)
    };

    println!("Process finished with output: {out}");

    Ok(())
}

pub fn resolvectl_dns(ips: Vec<Ipv4Addr>) -> Result<(), Box<dyn std::error::Error>> {
    let mut command = std::process::Command::new("resolvectl");
    command.arg("dns").arg("-p").arg("5453").arg("formnet");

    for ip in ips { command.arg(ip.to_string()); }
    let output = command.output()?;
    let out = if output.status.success() {
        String::from_utf8_lossy(&output.stdout)
    } else {
        String::from_utf8_lossy(&output.stderr)
    };

    println!("Process finished with output: {out}");

    Ok(())
}

pub fn resolvectl_revert() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::process::Command::new("resolvectl")
        .arg("revert")
        .arg("formnet")
        .output()?;

    let out = if output.status.success() {
        String::from_utf8_lossy(&output.stdout)
    } else {
        String::from_utf8_lossy(&output.stderr)
    };

    println!("Process finished with output: {out}");

    Ok(())
}

pub fn resolvectl_flush_cache() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::process::Command::new("resolvectl")
        .arg("--flush-caches")
        .output()?;

    let out = if output.status.success() {
        String::from_utf8_lossy(&output.stdout)
    } else {
        String::from_utf8_lossy(&output.stderr)
    };

    println!("Process finished with output: {out}");

    Ok(())
}

