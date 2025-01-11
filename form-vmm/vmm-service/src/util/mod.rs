#![allow(unused)]
use std::{any::{Any, TypeId}, io::Write, path::{Path, PathBuf}, process::Command};
use crate::Distro;
use serde::Deserialize;
use futures::stream::TryStreamExt;
use rtnetlink::{new_connection, Handle, Error};
use netlink_packet_route::link::nlas::InfoKind;

pub const PREP_MOUNT_POINT: &str = "/mnt/cloudimg";
pub const DEFAULT_NETPLAN_FILENAME: &str = "01-netplan-custom-config.yaml";
pub const DEFAULT_NETPLAN: &str = "/var/lib/formation/netplan/01-custom-netplan.yaml";
pub const DEFAULT_FORMNET_INSTALL: &str = "etc/systemd/system/formnet-install.service";
pub const DEFAULT_FORMNET_UP: &str = "etc/systemd/system/formnet-up.service";
pub const FORMNET_BINARY: &str = "/var/lib/formation/formnet/formnet";
pub const BASE_DIRECTORY: &str  = "/var/lib/formation/vm-images";

pub const UBUNTU: &str = "https://cloud-images.ubuntu.com/jammy/20241217/jammy-server-cloudimg-amd64.img";
pub const FEDORA: &str = "https://download.fedoraproject.org/pub/fedora/linux/releases/41/Cloud/x86_64/images/Fedora-Cloud-Base-AmazonEC2-41-1.4.x86_64.raw.xz";
pub const DEBIAN: &str = "https://cdimage.debian.org/images/cloud/bullseye/20241202-1949/debian-11-generic-amd64-20241202-1949.raw";
pub const CENTOS: &str = "https://cloud.centos.org/centos/8/x86_64/images/CentOS-8-GenericCloud-8.4.2105-20210603.0.x86_64.qcow2";
pub const ARCH: &str = "https://geo.mirror.pkgbuild.com/images/latest/Arch-Linux-x86_64-cloudimg.qcow2";
pub const ALPINE: &str = "https://dl-cdn.alpinelinux.org/alpine/v3.21/releases/cloud/generic_alpine-3.21.0-x86_64-bios-tiny-r0.qcow2";

type UtilError = Box<dyn std::error::Error + Send + Sync + 'static>;

#[derive(Debug, Deserialize)]
struct LsblkOutput {
    blockdevices: Vec<BlockDevice>
}

#[derive(Debug, Deserialize)]
struct BlockDevice {
    name: String,
    #[serde(default)]
    children: Vec<BlockDevice>,
    #[serde(default)]
    fstype: Option<String>,
    #[serde(default)]
    mountpoint: Option<String>,
    #[serde(default)]
    size: Option<String>
}

pub fn ensure_directory<P: AsRef<Path>>(path: P) -> Result<(), UtilError> {
    log::info!("ensuring directory {} exists", path.as_ref().display());
    if !path.as_ref().exists() {
        std::fs::create_dir_all(&path)?;
    }
    Ok(())
}

fn download_image(url: &str, dest: &str) -> Result<(), UtilError> {
    log::info!("Attempting to download {url} and place in {dest}");
    let status = Command::new("wget")
        .arg("-q")
        .arg("-O")
        .arg(dest)
        .arg(url)
        .status()?;

    if !status.success() {
        return Err(Box::new(std::io::Error::last_os_error()));
    }

    log::info!("Download of {url} completed successfully");

    Ok(())
}

fn decompress_xz(src: &str, dest: &str) -> Result<(), UtilError> {
    let status = Command::new("xz")
        .arg("--decompress")
        .arg("--keep")
        .arg("--stdout")
        .arg(src)
        .output()?;

    if !status.status.success() {
        return Err(
            Box::new(
                std::io::Error::last_os_error()
            )
        )
    }

    std::fs::write(dest, status.stdout)?;

    Ok(())
}

fn convert_qcow2_to_raw(qcow2_path: &str, raw_path: &str) -> Result<(), UtilError> {
    log::info!("Attempting to convert {qcow2_path} from qcow to {raw_path} raw disk image");

    let status = Command::new("qemu-img")
        .args(&["convert", "-p", "-f", "qcow2", "-O", "raw", qcow2_path, raw_path])
        .status()?;

    if !status.success() {
        return Err(Box::new(std::io::Error::last_os_error()));
    }

    Ok(())
}

pub async fn fetch_and_prepare_images() -> Result<(), UtilError> {
    log::info!("Attempting to write base netplan");
    write_default_netplan()?;
    write_default_formnet_install_service()?;
    write_default_formnet_up_service()?;
    let base = PathBuf::from(BASE_DIRECTORY);
    let urls = [
        (UBUNTU, base.join("ubuntu/22.04/base.img")),
        /*
        (FEDORA, base.join("fedora/41/base.raw.xz")),
        (DEBIAN, base.join("debian/11/base.raw")), 
        (CENTOS, base.join("centos/8/base.img")),
        (ARCH, base.join("arch/latest/base.img")), 
        (ALPINE, base.join("alpine/3.21/base.img"))
        */
    ];

    let mut handles = Vec::new();

    for (url, dest) in urls {
        handles.push(tokio::spawn(async move {
            let dest_string = dest.display().to_string();
            let dest_dir = dest.parent().ok_or(
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Unable to find parent for destination..."
                )
            )?;
            ensure_directory(dest_dir)?;
            download_image(url, &dest_string)?;
            if dest_string.ends_with(".img") {
                log::info!("Found qcow2 image, converting to raw");
                convert_qcow2_to_raw(&dest_string, &dest.parent().ok_or(
                    Box::new(
                        std::io::Error::new(
                            std::io::ErrorKind::Other, 
                            "Conversion from qcow2 to raw failed: destination has no parent"
                        )
                    ) as Box<dyn std::error::Error + Send + Sync + 'static>
                )?.join("base.raw").display().to_string())?;
                log::info!("Successfully converted qcow2 image to raw");
            } else if dest_string.ends_with(".xz") {
                decompress_xz(&dest_string, &dest.parent().ok_or(
                    Box::new(
                        std::io::Error::new(
                            std::io::ErrorKind::Other, 
                            "Decompression of xz failed: destination has no parent"
                        )
                    ) as Box<dyn std::error::Error + Send + Sync + 'static>
                )?.join("base.raw").display().to_string())?;
            } else if dest_string.ends_with(".raw") {} else {
                return Err(
                    Box::new(
                        std::io::Error::new(
                            std::io::ErrorKind::Other,
                            format!("disk format is not valid: {}", dest.display())
                        )
                    ) as Box<dyn std::error::Error + Send + Sync + 'static>
                )
            }

            Ok::<(), Box<dyn std::error::Error + Send + Sync + 'static>>(())
        }));
    }

    for handle in handles {
        let _ = handle.await?;
    }

    log::info!("Base images acquired and placed in /var/lib/formation/vm-images");

    let base_imgs = [
        base.join("ubuntu/22.04/base.raw"),
        /*
        base.join("fedora/41/base.raw"),
        base.join("debian/11/base.raw"),
        base.join("centos/8/base.raw"),
        base.join("arch/latest/base.raw"),
        base.join("alpine/3.21/base.raw"),
        */
    ];

    for img in base_imgs {
        let netplan_to = PathBuf::from(PREP_MOUNT_POINT).join("etc/netplan").join(DEFAULT_NETPLAN_FILENAME);
        let formnet_install_to = PathBuf::from(PREP_MOUNT_POINT).join(DEFAULT_FORMNET_INSTALL);
        let formnet_up_to = PathBuf::from(PREP_MOUNT_POINT).join(DEFAULT_FORMNET_UP);

        mount_base_image(&img.display().to_string())?;
        copy_default_netplan(
            &PathBuf::from(
                netplan_to
            )
        )?;
        copy_default_formnet_up_service(
            &PathBuf::from(
                formnet_up_to
            )
        )?;
        copy_default_formnet_invite_service(
            &PathBuf::from(
                formnet_install_to
            )
        )?;
        copy_formnet_client(
            &PathBuf::from(
                PREP_MOUNT_POINT
            ).join("usr/local/bin/")
            .join("formnet")
            .display().to_string()
        )?;
        unmount_base_image()?;
    }

    Ok(())
}

pub fn copy_disk_image(
    distro: Distro,
    version: &str,
    instance_id: &str,
    node_id: &str
) -> Result<(), UtilError> {
    let base_path = PathBuf::from(BASE_DIRECTORY).join(distro.to_string()).join(version).join("base.raw");
    let dest_path = PathBuf::from(BASE_DIRECTORY).join(node_id).join(format!("{}.raw", instance_id));

    ensure_directory(
        dest_path.parent().ok_or(
            Box::new(
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Destination path has no parent"
                )
            )
        )?
    )?;

    std::fs::copy(
        base_path,
        dest_path
    )?;

    Ok(())
}

fn copy_default_formnet_invite_service(to: impl AsRef<Path>) -> Result<(), UtilError> {
    log::info!("Attempting to copy default formnet install service to {}", to.as_ref().display());
    let parent = to.as_ref().parent().ok_or(
        Box::new(
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "Unable to find parent of netplan directory"
            )
        )
    )?;

    std::fs::create_dir_all(&parent)?;
    std::fs::copy(
        DEFAULT_FORMNET_INSTALL,
        &to
    )?;

    log::info!("Successfully copied default formnet install service to {}", to.as_ref().display());
    Ok(())
}

fn copy_default_formnet_up_service(to: impl AsRef<Path>) -> Result<(), UtilError> {
    log::info!("Attempting to copy default formnet up service to {}", to.as_ref().display());
    let parent = to.as_ref().parent().ok_or(
        Box::new(
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "Unable to find parent of netplan directory"
            )
        )
    )?;

    std::fs::create_dir_all(&parent)?;
    std::fs::copy(
        DEFAULT_FORMNET_UP,
        &to
    )?;

    log::info!("Successfully copied default formnet up service to {}", to.as_ref().display());
    Ok(())
}

fn copy_default_netplan(to: impl AsRef<Path>) -> Result<(), UtilError> {
    log::info!("Attempting to copy default netplan to {}", to.as_ref().display());
    let parent = to.as_ref().parent().ok_or(
        Box::new(
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "Unable to find parent of netplan directory"
            )
        )
    )?;

    std::fs::create_dir_all(&parent)?;
    std::fs::copy(
        DEFAULT_NETPLAN,
        &to
    )?;

    log::info!("Successfully copied default netplan to {}", to.as_ref().display());

    Ok(())
}

fn write_default_formnet_install_service() -> Result<(), UtilError> {
    let formnet_install_string = r#"[Unit]
Description=Formnet Install
After=network-online.target
Wants=network-online.target

# Only run if we haven't installed yet (optional safeguard)
ConditionPathExists=!/etc/formnet/state.toml

[Service]
Type=oneshot
ExecStart=/usr/local/bin/formnet install --default-name -d /etc/formnet/invite.toml
ExecStart=/bin/touch /etc/formnet/state.toml
RemainAfterExit=yes
StandardOutput=append:/var/log/formnet.log
StandardError=append:/var/log/formnet.log

[Install]
WantedBy=multi-user.target
"#;

    let formnet_install_service_path = PathBuf::from(DEFAULT_FORMNET_INSTALL);
    let formnet_install_path = formnet_install_service_path.parent()
        .ok_or(
            Box::new(
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "formnet install default path has no parent"
                )
            )
        )?;

    ensure_directory(formnet_install_path)?;

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(DEFAULT_FORMNET_INSTALL)?;

    file.write_all(formnet_install_string.as_bytes())?;

    log::info!("Successfully wrote default formnet install to {}", DEFAULT_FORMNET_INSTALL);
    Ok(())
}

fn write_default_formnet_up_service() -> Result<(), UtilError> {
    log::info!("Attempting to write default formnet up service to {}", DEFAULT_FORMNET_UP);
    let formnet_up_string = r#"[Unit]
Description=Formnet Up
After=formnet-install.service
Wants=formnet-install.service
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart=/usr/local/bin/formnet up -d --interval 60
Restart=always
RestartSec=5
StandardOutput=append:/var/log/formnet.log
StandardError=append:/var/log/formnet.log


[Install]
WantedBy=multi-user.target
"#;
    let formnet_up_service_path = PathBuf::from(DEFAULT_FORMNET_UP);
    let formnet_up_path = formnet_up_service_path.parent()
        .ok_or(
            Box::new(
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "formnet up default path has no parent"
                )
            )
        )?;

    ensure_directory(formnet_up_path)?;

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(DEFAULT_FORMNET_UP)?;

    file.write_all(formnet_up_string.as_bytes())?;

    log::info!("Successfully wrote default formnet install to {}", DEFAULT_FORMNET_UP);
    Ok(())
}

fn write_default_netplan() -> Result<(), UtilError> {
    log::info!("Attempting to write default netplan to {}", DEFAULT_NETPLAN);
    let netplan_string = r#"network:
  version: 2
  renderer: networkd

  ethernets:
    rename-this-nic:
      match:
        name: "en*"
      set-name: eth0
      dhcp4: true
    "#;

    let netplan_path = PathBuf::from(DEFAULT_NETPLAN);
    let netplan_path = netplan_path.parent().ok_or(
        Box::new(
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "Netplan default path has no parent"
            )
        )
    )?;

    ensure_directory(netplan_path)?;

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(DEFAULT_NETPLAN)?;

    file.write_all(netplan_string.as_bytes())?;

    log::info!("Successfully wrote default netplan to {}", DEFAULT_NETPLAN);
    Ok(())
}

fn copy_formnet_client(to: &str) -> Result<(), UtilError> {
    log::info!("Attempting to copy formnet binary from {FORMNET_BINARY} to {to}");

    let to = PathBuf::from(to);
    let parent = to.parent().ok_or(
        Box::new(
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "Unable to find parent for formnet to directory"
            )
        )
    )?;
    ensure_directory(parent)?;

    std::fs::copy(
        FORMNET_BINARY,
        to.clone()
    )?;

    log::info!("Succesfully copied formnet binary from {FORMNET_BINARY} to {}", to.display());
    Ok(())
}

pub fn ensure_bridge_exists() -> Result<(), UtilError> {
    if !brctl::BridgeController::check_bridge_exists("br0")? {
        brctl::BridgeController::create_bridge("br0")?;
    }

    Ok(())
}


pub async fn add_tap_to_bridge(bridge_name: &str, tap: &str) -> Result<(), UtilError> {
        let (connection, handle, _) = new_connection()?;
    tokio::spawn(connection);

    // Get bridge index
    let mut bridge_links = handle.link().get().match_name(bridge_name.to_string()).execute();
    let bridge_index = if let Some(link) = bridge_links.try_next().await? {
        link.header.index
    } else {
        return Err(
            Box::new(
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Bridge {} not found", bridge_name)
                )
            )
        )
    };
    // Get interface index and add it to the bridge
    let mut interface_links = handle.link().get().match_name(tap.to_string()).execute();
    if let Some(link) = interface_links.try_next().await? {
        handle.link().set(link.header.index)
            .master(bridge_index)
            .execute()
            .await?;
    }

    Ok(())
}

fn mount_base_image(image_path: &str) -> Result<(), UtilError> {
    log::info!("Mounting {image_path} to {PREP_MOUNT_POINT}");
    let status = Command::new("guestmount")
        .args(["-a", image_path, "-i", "--rw", PREP_MOUNT_POINT])
        .status()?;

    if !status.success() {
        return Err(Box::new(std::io::Error::last_os_error()));
    }

    log::info!("Successfully mounted {image_path} to {PREP_MOUNT_POINT}");
    Ok(())
}

fn unmount_base_image() -> Result<(), UtilError> {
    log::info!("Unmounting base disk image from {PREP_MOUNT_POINT}");
    let status = Command::new("guestunmount")
        .arg(PREP_MOUNT_POINT)
        .status()?;

    if !status.success() {
        return Err(Box::new(std::io::Error::last_os_error()));
    }

    Ok(())
}

fn get_image_loop_device(image_path: &str) -> Result<String, UtilError> {
    log::info!("Getting loop device from {image_path}");
    let output = Command::new("guestmount")
        .args(["--partscan", "--find", "--show", image_path])
        .output()?;
    if !output.status.success() {
        return Err(Box::new(std::io::Error::last_os_error()))
    }
    let loop_device = String::from_utf8_lossy(&output.stdout).trim().to_string();
    log::info!("Found {} is located at loop device {}", image_path, loop_device);
    Ok(loop_device)
}

fn mount_partition(loop_device: &str, partition_idx: u8) -> Result<(), UtilError> {
    log::info!("Ensuring {} exists...", PREP_MOUNT_POINT);
    std::fs::create_dir_all(PREP_MOUNT_POINT)?;

    let partition = format!("/dev/{}", get_fs_partition(loop_device)?);
    log::info!("Using partition {}", partition);

    let status = Command::new("mount")
        .args([&partition, PREP_MOUNT_POINT])
        .status()?;

    if !status.success() {
        return Err(Box::new(std::io::Error::last_os_error()));
    }

    log::info!("Successfully mounted partition");
    Ok(())
}

fn unmount_partition() -> Result<(), UtilError> {
    let status = Command::new("umount")
        .args([PREP_MOUNT_POINT])
        .status()?;

    if !status.success() {
        return Err(Box::new(std::io::Error::last_os_error()));
    }

    log::info!("Successfully unmounted partition");
    Ok(())
}

fn departition_loop_device(loop_device: &str) -> Result<(), UtilError> {
    let status = std::process::Command::new("losetup")
        .args(["-d", loop_device])
        .stderr(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .status()?;

    if !status.success() {
        return Err(Box::new(std::io::Error::last_os_error()));
    }

    log::info!("Successfully departitioned loop device {loop_device}");
    Ok(())
}

pub fn copy_distro_base(distro: Distro, version: &str, name: &str) -> Result<String, UtilError> {
    let instance_disk_directory = PathBuf::from(BASE_DIRECTORY).join(name);
    std::fs::create_dir_all(
        instance_disk_directory.clone()
    )?;

    std::fs::copy(
        distro.rootfs_disk_path(version),
        instance_disk_directory.join("disk.raw")
    )?;

    return Ok(instance_disk_directory.join("disk.raw").display().to_string())
}

pub fn get_fs_partition(loop_device: &str) -> Result<String, UtilError> {
    let output = std::process::Command::new("lsblk")
        .args(["--json", loop_device])
        .output()?;

    let lsblk_output: LsblkOutput = serde_json::from_slice(&output.stdout)?;

    let root_device = &lsblk_output.blockdevices[0];

    let mut fs: &str = &format!("{}p1", loop_device);
    let mut largest: Option<u128> = None;

    for child in &root_device.children {
        let partition_name = &child.name;
        let size = child.size.as_deref().unwrap_or("unknown");
        log::info!("Partition: {partition_name}, Size: {size}");
        let size_in_bytes = {
            if let Ok(n) = try_convert_size_to_bytes(size) {
                Some(n)
            } else { 
                None
            }
        };

        if let Some(s) = size_in_bytes {
            if let Some(n) = largest {
                if s > n {
                    largest = Some(s);
                    fs = partition_name;
                }
            } else {
                largest = Some(s);
                fs = partition_name;
            }
        }
    }

    return Ok(fs.to_string())
}

pub fn try_convert_size_to_bytes(size: &str) -> Result<u128, UtilError> {
    let mut chars: Vec<char>  = size.chars().collect();
    let suffix = chars.pop().ok_or(
        Box::new(
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "size not available"
            )
        )
    )?;

    let num: f64 = {
        let size: String = chars.iter().collect();
        let num: f64 = size.parse()?;
        num
    };

    let num_bytes = match String::from(suffix).to_lowercase().as_str() {
        "t" => {
            let nb = num * 1_000_000_000_000.0;
            nb as u128
        }
        "g" => {
            let nb = num * 1_000_000_000.0;
            nb as u128
        }
        "m" => {
            let nb = num * 1_000_000.0;
            nb as u128
        }
        "k" => {
            let nb = num * 1_000.0;
            nb as u128
        }
        _ => {
            return Err(
                Box::new(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "unable to convert size"
                    )
                )
            )
        }
    };

    Ok(num_bytes)
}

pub fn is_unit_type<T: ?Sized + Any>() -> bool {
    TypeId::of::<T>() == TypeId::of::<()>()
}
