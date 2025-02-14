use get_if_addrs::IfAddr;


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addrs = get_if_addrs::get_if_addrs()?;
    let pub_addr: Vec<_> = addrs.iter().filter_map(|iface| {
        if iface.name == "br0" {
            match &iface.addr {
                IfAddr::V4(ifv4) => {
                    Some(ifv4.ip.to_string())
                }
                _ => None
            }
        } else {
            None
        }
    }).collect::<Vec<_>>();

    if let Some(first) = pub_addr.first() {
        println!("{first}");
    }

    Ok(())
}
