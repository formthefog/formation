use clap::Args;

/// Create a new instance
#[derive(Debug, Clone, Args)]
pub struct CreateCommmand {
    /// The Linux Distro you would like your instance to use 
    #[clap(long, short, default_value="ubuntu")]
    pub distro: String,
    /// The version of the distro of your choice. If it is not a valid version 
    /// it will be rejected
    #[clap(long, short, default_value="22.04")]
    pub version: String,
    /// The amount of memory in megabytes you'd like to have allocated to your
    /// instance.
    #[clap(long, short='b', default_value_t=512)]
    pub memory_mb: u64,
    /// The number of virtual CPUs you would like to have allocated to your
    /// instance
    #[clap(long, short='c', default_value_t=1)]
    pub vcpu_count: u8,
    /// A human readable name you'd like your instance to have, if left
    /// blank, a random name will be assigned
    #[clap(long, short)]
    pub name: Option<String>,
    /// The path to a user-data.yaml file, must be compatible with cloud-init
    /// (see https://cloudinit.readthedocs.io/en/latest/reference/examples.html for examples)
    /// You can use the cloud-init-wizard command to build a valid custom cloud-init file.
    //TODO: Add feature to provide common config files like Dockerfile type formats and 
    // auto convert them to valid cloud-init user data
    #[clap(long, short)]
    pub user_data: Option<String>,
    /// The path to a meta-data.yaml file, must be compatible with cloud-init
    /// (see https://cloudinit.readthedocs.io/en/latest/reference/examples.html) 
    /// You can use the cloud-init-wizard command to build a valid custom cloud-init file.
    //TODO: Add feature to provide common config files like Dockerfile type formats and 
    // auto convert them to valid cloud-init user data
    #[clap(long, short='t')]
    pub meta_data: Option<String>,
    /// A hexadecimal or base64 representation of a valid private key for 
    /// signing the request. Given this is the create command, this will
    /// be how the network derives ownership of the instance. Authorization
    /// to other public key/wallet addresses can be granted by the owner
    /// after creation, however, this key will be the initial owner until
    /// revoked or changed by a request made with the same signing key
    #[clap(long, short)]
    pub private_key: Option<String>,
    /// An altenrative to private key or mnemonic. If you have a keyfile
    /// stored locally, you can use the keyfile to read in your private key
    //TODO: Add support for HSM and other Enclave based key storage
    #[clap(long, short)]
    pub keyfile: Option<String>,
    /// An alternative to private key or keyfile. If you have a 12 or 24 word 
    /// BIP39 compliant mnemonic phrase, you can use it to derive the signing
    /// key for this request
    //TODO: Add support for HSM and other Enclave based key storage
    #[clap(long, short)]
    pub mnemonic: Option<String>,
}
