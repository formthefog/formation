#[cfg(target_os = "linux")]
pub async fn revert_formnet_resolver() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::process::Command::new("resolvectl")
        .arg("revert")
        .arg("formnet")
        .output()?;

    if output.status.success() {
        let info = String::from_utf8_lossy(&output.stdout);
        log::info!("Successfully reverted existing formnet resolver: {info}");
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        log::error!("Error attempting to revert existing formnet {err}"); 
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, err)));
    }

    Ok(())
}

#[cfg(target_os = "linux")]
pub async fn set_formnet_resolver(formnet_ip: &str, tld: &str) -> Result<(), Box<dyn std::error::Error>> {
    let output = std::process::Command::new("resolvectl")
        .arg("domain")
        .arg("formnet")
        .arg(tld)
        .output()?;

    if output.status.success() {
        let info = String::from_utf8_lossy(&output.stdout);
        log::info!("Successfully set formnet resolver: {info}");
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        log::error!("Error attempting to revert existing formnet {err}"); 
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, err)));
    }

    let output = std::process::Command::new("resolvectl")
        .arg("dns")
        .arg("formnet")
        .arg(formnet_ip)
        .output()?;

    if output.status.success() {
        let info = String::from_utf8_lossy(&output.stdout);
        log::info!("Successfully set formnet resolver: {info}");
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        log::error!("Error attempting to revert existing formnet {err}"); 
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, err)));
    }

    Ok(())
}
