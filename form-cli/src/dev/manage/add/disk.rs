use clap::Args;
use anyhow::Result;

#[derive(Clone, Debug, Args)]
pub struct AddDiskCommand {
    /// The ID of the instance to modify
    #[clap(long, short)]
    pub id: Option<String>,
    
    /// The name of the instance to modify, an alternative to ID
    #[clap(long, short)]
    pub name: Option<String>,
    
    /// Private key file for authentication
    #[clap(long)]
    pub private_key: Option<String>,
    
    /// Keyfile containing the private key
    #[clap(long)]
    pub keyfile: Option<String>,
    
    /// Mnemonic for key derivation
    #[clap(long)]
    pub mnemonic: Option<String>,
    
    /// Path to the disk image file to add
    #[clap(long)]
    pub path: Option<String>,
    
    /// Set disk as read-only
    #[clap(long)]
    pub readonly: bool,
    
    /// Use direct I/O for better performance
    #[clap(long)]
    pub direct: bool,
    
    /// Enable IOMMU for this disk
    #[clap(long)]
    pub iommu: bool,
    
    /// Optional disk identifier
    #[clap(long)]
    pub disk_id: Option<String>,
    
    /// Send request via queue instead of direct API call
    #[clap(long)]
    pub queue: bool,
}

impl AddDiskCommand {
    /// Handle the add disk command using direct API communication
    pub async fn handle(
        &self,
        _provider: &dyn std::fmt::Debug,
        _vmm_port: u16,
    ) -> Result<()> {
        // This feature is not yet implemented
        println!("The add disk command is not yet implemented");
        println!("This command will allow adding a disk to a running VM in the future");
        
        // Future implementation will:
        // 1. Validate the VM exists and is running
        // 2. Prepare the disk configuration
        // 3. Send a request to the VMM API to add the disk
        // 4. Handle the response and display results
        
        Ok(())
    }

    /// Handle the add disk command using queue-based communication
    pub async fn handle_queue(
        &self,
        _provider: &dyn std::fmt::Debug,
        _keystore: Option<String>,
    ) -> Result<()> {
        // This feature is not yet implemented
        println!("The add disk via queue command is not yet implemented");
        println!("This command will allow adding a disk to a running VM via the message queue in the future");
        
        // Future implementation will:
        // 1. Validate the VM exists
        // 2. Create and sign a queue message
        // 3. Send the message to the queue
        // 4. Display confirmation to the user
        
        Ok(())
    }
}
