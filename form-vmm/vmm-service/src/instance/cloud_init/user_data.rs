use serde::{Deserialize, Serialize};
use crate::Distro;

use super::write_files::WriteFile;

#[derive(Debug, Serialize, Deserialize)]
pub struct UserData {
    pub hostname: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub users: Option<Vec<User>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chpasswd: Option<ChPasswd>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssh_pwauth: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_root: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package_update: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package_upgrade: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub packages: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub write_files: Option<Vec<WriteFile>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runcmd: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bootcmd: Option<Vec<String>>,
}

impl UserData {
    pub fn default_from_distro(distro: Distro) -> Self {
        match distro {
            Distro::Ubuntu => Self {
                hostname: "ubuntu-vm".to_string(),
                users: Some(vec![User {
                    name: "ubuntu".to_string(),
                    passwd: Some("$6$rounds=4096$Pm.pKkm3DJr/Wzkx$cHIPaq/JiKNA3da3Toif53Er3jCh.fdTp87zVHezIBN9SNqH0vxCoMCpihM0DY4BUmTQOnWJ1plT2wj0BSmg40".to_string()),
                    lock_passwd: false,
                    sudo: Some("ALL=(ALL) NOPASSWD:ALL".to_string()),
                    groups: Some("sudo".to_string()),
                    shell: Some("/bin/bash".to_string()),
                    ssh_authorized_keys: None,
                }]),
                chpasswd: Some(ChPasswd {
                    expire: false,
                    list: vec!["ubuntu:ubuntu".to_string()],
                }),
                ssh_pwauth: Some(true),
                disable_root: Some(true),
                package_update: Some(true),
                package_upgrade: Some(true),
                packages: None,
                write_files: None,
                runcmd: None,
                bootcmd: None,
            },
            // For now, use Ubuntu defaults for other distros
            // We can customize these later for each distro
            _ => Self::default()
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct User { 
    pub name: String,
    pub lock_passwd: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub passwd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sudo: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub groups: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shell: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssh_authorized_keys: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChPasswd {
    pub expire: bool,
    pub list: Vec<String>,
}
