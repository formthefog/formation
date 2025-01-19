use clap::Subcommand;

pub mod device;
pub mod disk;
pub mod fs;

pub use device::*;
pub use disk::*;
pub use fs::*;

#[derive(Clone, Debug, Subcommand)]
pub enum RemoveCommand {
    Device(RemoveDeviceCommand),
    Disk(RemoveDiskCommand),
    Fs(RemoveFilesystemCommand),
}
