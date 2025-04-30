pub fn build_instance_id(node_id: String, build_id: String) -> Result<String, Box<dyn std::error::Error + Send + Sync + 'static>> {
    println!("Deriving instance id from node_id: {node_id} and build_id: {build_id}");
    let node_id_vec = &hex::decode(node_id)?[..20];
    let vm_name_bytes = &hex::decode(build_id.clone())?[..20];

    let instance_id = hex::encode(&vm_name_bytes.iter().zip(node_id_vec.iter()).map(|(&x, &y)| x ^ y).collect::<Vec<u8>>());

    Ok(instance_id)
}

pub(crate) fn is_gzip(data: &[u8]) -> bool {
    data.starts_with(&[0x1F, 0x8B]) // Gzip magic number
}

pub(crate) fn get_host_bridge_ip() -> Result<String, Box<dyn std::error::Error + Send + Sync +'static>> {
    let addrs = get_if_addrs::get_if_addrs()?;
    let pub_addr: Vec<_> = addrs.iter().filter_map(|iface| {
        if iface.name == "br0" {
            match &iface.addr {
                get_if_addrs::IfAddr::V4(ifv4) => {
                    Some(ifv4.ip.to_string())
                }
                _ => None
            }
        } else {
            None
        }
    }).collect::<Vec<_>>();

    let first = pub_addr.first()
        .ok_or(
            Box::new(
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Unable to find IP for br0, host is not set up to host instances"
                )
            )
        )?; 

    Ok(first.to_string())
}

