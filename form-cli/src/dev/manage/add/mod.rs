use clap::Subcommand;

pub mod device;
pub mod disk;
pub mod fs;

pub use device::*;
pub use disk::*;
pub use fs::*;

#[derive(Clone, Debug, Subcommand)]
pub enum AddCommand {
    Device(AddDeviceCommand),
    Disk(AddDiskCommand),
    Fs(AddFilesystemCommand)
}
