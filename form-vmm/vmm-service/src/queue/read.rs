pub async fn read_from_queue(
    last: Option<usize>,
    n: Option<usize>,
) -> Result<Vec<Vec<u8>>, Box<dyn std::error::Error + Send + Sync>> {
    let mut endpoint = format!("http://127.0.0.1:{}/queue/vmm", QUEUE_PORT);
    if let Some(idx) = last {
        endpoint.push_str(&format!("/{idx}"));
        if let Some(n) = n {
            endpoint.push_str(&format!("/{n}/get_n_after"));
        } else {
            endpoint.push_str("/get_after");
        }
    } else {
        if let Some(n) = n {
            endpoint.push_str(&format!("/{n}/get_n"))
        } else {
            endpoint.push_str("/get")
        }
    }

    match Client::new()
        .get(endpoint.clone())
        .send().await?
        .json::<QueueResponse>().await? {
            QueueResponse::List(list) => Ok(list),
            QueueResponse::Failure { reason } => Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("{reason:?}")))),
            _ => Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Invalid response variant for {endpoint}")))) 
    }
}