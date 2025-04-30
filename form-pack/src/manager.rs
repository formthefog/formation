#![allow(unused_assignments)]
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration; 
use tokio::sync::broadcast::Receiver;
use serde::{Serialize, Deserialize};
use tokio::sync::Mutex;
use crate::types::response::PackBuildResponse;
use crate::types::request::PackBuildRequest;
use crate::helpers::api::serve;
use crate::helpers::queue::write::write_pack_status_failed;
use crate::helpers::queue::build::handle_pack_request;
use crate::helpers::queue::read::read_from_queue;

pub const VM_IMAGE_PATH: &str = "/var/lib/formation/vm-images/";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FormVmmService(SocketAddr);

pub struct FormPackManager {
    addr: SocketAddr,
    pub(crate) node_id: String,
}

impl FormPackManager {
    pub fn new(addr: SocketAddr, node_id: String,) -> Self {
        Self {
            addr,
            node_id
        }
    }

    pub async fn run(self, mut shutdown: Receiver<()>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let addr = self.addr.to_string();
        let pack_manager = Arc::new(Mutex::new(self));
        let inner_addr = addr.clone();
        let inner_pack_manager = pack_manager.clone();
        let handle = tokio::spawn(async move {
            if let Err(e) = serve(inner_addr, inner_pack_manager).await {
                eprintln!("Error serving pack manager api server: {e}");
            }
        });

        let mut n = 0;
        loop {
            tokio::select! {
                Ok(messages) = read_from_queue(Some(n), None) => {
                    for message in &messages {
                        let mut manager = pack_manager.lock().await;
                        if let Err(e) = manager.handle_message(message.to_vec()).await {
                            eprintln!("Error handling message: {e}");
                        };
                    }
                    n += messages.len();
                },
                _ = tokio::time::sleep(Duration::from_millis(100)) => {}
                _ = shutdown.recv() => {
                    eprintln!("Received shutdown signal");
                    handle.abort();
                    break
                }
            }
        }

        Ok(())
    }

    pub async fn handle_message(&mut self, message: Vec<u8>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let subtopic = message[0];
        let request = &message[1..];
        match subtopic {
            0 =>  {
                let msg: PackBuildRequest = serde_json::from_slice(request)?; 
                if let Err(e) = handle_pack_request(self, msg.clone()).await {
                    write_pack_status_failed(&msg, e.to_string()).await?;
                    return Err(e)
                }
            }
            1 => {
                let _msg: PackBuildResponse = serde_json::from_slice(request)?;
            }
            _ => unreachable!()
        }

        Ok(())
    }
}
