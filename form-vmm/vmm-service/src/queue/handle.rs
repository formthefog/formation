use crate::api::VmmApiChannel;
use crate::error::VmmError;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::queue::helpers::{
    create::handle_create_vm_message,
    boot::handle_boot_vm_message,
    delete::handle_delete_vm_message,
    stop::handle_stop_vm_message,
    reboot::handle_reboot_vm_message,
    start::handle_start_vm_message
};

pub async fn handle_message(message: Vec<u8>, channel: Arc<Mutex<VmmApiChannel>>) -> Result<(), VmmError> {
    let subtopic = message[0];
    log::info!("Received subtopic: {subtopic}");
    let msg = &message[1..];
    match subtopic {
        0 => handle_create_vm_message(msg, channel.clone()).await?,
        1 => handle_boot_vm_message(msg, channel.clone()).await?, 
        2 => handle_delete_vm_message(msg, channel.clone()).await?,
        3 => handle_stop_vm_message(msg, channel.clone()).await?,
        4 => handle_reboot_vm_message(msg, channel.clone()).await?,
        5 => handle_start_vm_message(msg, channel.clone()).await?,
        _ => unreachable!()
    }
    Ok(())
}