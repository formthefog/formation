use std::fs::{self, File};
use std::io::{Cursor, Read};
use std::time::Duration;
use std::path::PathBuf;
use tokio::time::sleep;
use flate2::read::GzDecoder;
use reqwest::Client;
use futures::StreamExt;
use bollard::{Docker, exec::CreateExecOptions, container::{DownloadFromContainerOptions, UploadToContainerOptions, CreateContainerOptions, Config}, models::{DeviceMapping, HostConfig, PortBinding}};
use crate::helpers::utils::{is_gzip, build_instance_id, get_host_bridge_ip};
use crate::image_builder::IMAGE_PATH;
use crate::formfile::Formfile;

pub struct FormPackMonitor {
    docker: Docker,
    container_id: Option<String>,
    container_name: Option<String>,
    build_server_id: Option<String>,
    build_server_uri: String,
    build_server_client: Client,
}

impl FormPackMonitor {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        println!("Building default monitor...");
        let mut monitor = Self {
            docker: Docker::connect_with_local_defaults()?,
            container_id: None,
            container_name: None,
            build_server_id: None,
            build_server_uri: String::new(),
            build_server_client: Client::new(),
        };

        println!("Attempting to start build container...");
        let (container_id, container_name, container_ip) = monitor.start_build_container().await?;
        monitor.container_id = Some(container_id.clone());
        monitor.container_name = Some(container_name.clone());
        monitor.build_server_uri = format!("http://{container_ip}:{}", 8080);

        Ok(monitor)
    }

    pub fn container_id(&self) -> &Option<String> {
        &self.container_id
    }

    pub async fn build_image(
        &mut self,
        node_id: String,
        vm_name: String,
        formfile: Formfile,
        artifacts: PathBuf,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let container_id = self.container_id.take().ok_or(
            Box::new(
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Container ID should be some by the time build_image is called"
                )
            )
        )?;
        println!("Build server for {} is {container_id}", formfile.name);

        // Use a Result to track success/failure for cleanup
        let build_result = async {
            println!("Uploading artifacts to {container_id}");
            self.upload_artifacts(&container_id, artifacts).await?;
            println!("Starting build server for {}", formfile.name);
            self.start_build_server(&container_id).await?;
            println!("Requesting image build for {}", formfile.name);
            self.execute_build(node_id.clone(), vm_name.clone(), &formfile).await?;
            self.extract_disk_image(&container_id, vm_name.clone()).await?;
            println!("Image build completed for {} successfully", formfile.name);
            Ok(())
        }.await;

        // Always cleanup the container regardless of build success or failure
        println!("Cleaning up container {container_id}...");
        if let Err(cleanup_err) = self.cleanup().await {
            println!("Error during container cleanup: {cleanup_err}");
            // If the build was successful but cleanup failed, still return the cleanup error
            if build_result.is_ok() {
                return Err(cleanup_err);
            }
            // Otherwise, return the original build error and log the cleanup error
        }

        // Return the original build result
        build_result
    }

    pub async fn start_build_container(&self) -> Result<(String, String, String), Box<dyn std::error::Error + Send + Sync>> {
        let container_name = format!("form-pack-builder-{}", uuid::Uuid::new_v4());
        let options = Some(CreateContainerOptions {
            name: container_name.clone(), 
            platform: None,
        });

        let ports = Some([(
            "8080/tcp".to_string(),
            Some(vec![PortBinding {
                host_ip: Some("0.0.0.0".to_string()),
                host_port: Some("8080".to_string())
            }]),
        )].into_iter().collect());

        let host_config = HostConfig {
            port_bindings: ports,
            devices: Some(vec![DeviceMapping {
                path_on_host: Some("/dev/kvm".to_string()),
                path_in_container: Some("/dev/kvm".to_string()),
                cgroup_permissions: Some("rwm".to_string())
            }]),
            ..Default::default()
        };

        let host_ip_var = format!("HOST_BRIDGE_IP={}", get_host_bridge_ip()?);
        println!("Build HostConfig: {host_config:?}");
        let config = Config {
            image: Some("form-build-server:latest"),
            cmd: None, 
            tty: Some(true),
            host_config: Some(host_config),
            env: Some(vec![&host_ip_var]),
            ..Default::default()
        };

        println!("Build Config: {config:?}");

        println!("Calling create container...");
        let container = self.docker.create_container(options, config).await?;
        println!("Calling start container...");
        self.docker.start_container::<String>(&container.id, None).await?;
        let container_ip = self.docker.inspect_container(&container_name, None)
                .await?
                .network_settings.ok_or(
                    Box::new(
                        std::io::Error::new(
                            std::io::ErrorKind::Other,
                            "Unable to acquire container network settings"
                        )
                    )
                )?.networks.ok_or(
                    Box::new(
                        std::io::Error::new(
                            std::io::ErrorKind::Other,
                            "Unable to acquire container networks"
                        )
                    )
                )?.iter().find(|(k, _)| {
                    *k == "bridge"
                }).ok_or(
                    Box::new(
                        std::io::Error::new(
                            std::io::ErrorKind::Other,
                            "Unable to find bridge network"
                        )
                    )
                )?.1.ip_address.clone().ok_or(
                    Box::new(
                        std::io::Error::new(
                            std::io::ErrorKind::Other,
                            "Unable to find IP Address"
                        )
                    )

                )?;
        Ok((container.id, container_name, container_ip))
    }

    pub async fn upload_artifacts(
        &self,
        container_id: &str,
        artifacts: PathBuf
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let options = UploadToContainerOptions {
            path: "/artifacts",
            ..Default::default()
        };

        let tar_contents = fs::read(artifacts)?;
        self.docker.upload_to_container(
            container_id,
            Some(options),
            tar_contents.into()
        ).await?;

        Ok(())
    }

    pub async fn start_build_server(&mut self, container_id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let exec_opts = CreateExecOptions {
            cmd: Some(vec!["sh", "-c", "form-build-server -p 8080 > /var/log/form-build-server.log 2>&1"]),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            env: Some(vec!["RUST_LOG=info"]),
            tty: Some(true),
            privileged: Some(true),
            ..Default::default()
        };

        println!("Creating exec {exec_opts:?} to run on {container_id}");
        let exec = self.docker.create_exec(container_id, exec_opts).await?;
        self.build_server_id = Some(exec.id.clone());
        println!("starting exec on {container_id}");
        self.docker.start_exec(&exec.id, None).await?;

        sleep(Duration::from_secs(2)).await;

        let max_retries = 5;
        let mut current_retry = 0;
        let mut ping_resp = None;

        while current_retry < max_retries {
            match self.build_server_client
                .post(format!("{}/ping", self.build_server_uri))
                .send()
                .await {
                    Ok(resp) if resp.status().is_success() => {
                        ping_resp = Some(resp);
                    }
                    _ => {
                        current_retry += 1;
                        sleep(Duration::from_secs(1)).await;
                    }
                }
        }

        match ping_resp {
            Some(r) => {
                println!("Received response from ping: {r:?}");
                return Ok(())
            },
            None => return Err(
                Box::new(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Build server never started, no response from ping request"
                    )
                )
            )
        }

    }

    pub async fn execute_build(
        &self,
        node_id: String,
        vm_name: String,
        formfile: &Formfile,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        println!("Sending Formfile {formfile:?} for {} to build_server: {}", formfile.name, self.build_server_uri);
        let instance_id = build_instance_id(node_id, vm_name.clone())?; 

        let resp = self.build_server_client
            .post(format!("{}/{}/{}/formfile", self.build_server_uri, vm_name, instance_id))
            .json(formfile)
            .send()
            .await?;

        println!("Received response: {resp:?}");

        Ok(())
    }

    pub async fn extract_disk_image(
        &self,
        container_name: &str,
        vm_name: String,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let options = Some(
            DownloadFromContainerOptions {
                path: IMAGE_PATH
            }
        );
        let mut buf = Vec::new();
        let mut stream = self.docker.download_from_container(container_name, options);
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            buf.extend_from_slice(&chunk);
        }

        let data: Box<dyn Read> = {
            if is_gzip(&buf) {
                Box::new(GzDecoder::new(Cursor::new(buf)))
            } else {
                Box::new(Cursor::new(buf))
            }
        };

        let mut archive = tar::Archive::new(data);
        let mut entries = archive.entries()?;
        let mut num_entries = 0;

        while let Some(entry) = entries.next() {
            let mut entry = entry?;
            num_entries += 1;

            if num_entries > 1 {
                return Err(
                    Box::new(
                        std::io::Error::new(
                            std::io::ErrorKind::Other,
                            "Archive should only have 1 entry for the disk image"
                        )
                    )
                )
            }

            let output_path = format!("/var/lib/formation/vm-images/{vm_name}.raw");
            let mut output_file = File::create(output_path)?;
            std::io::copy(&mut entry, &mut output_file)?;
        }

        if num_entries == 0 {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Archive is empty"
            )))
        }
        
        return Ok(())
    }

    pub async fn cleanup(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(container_id) = self.container_id.take() {
            self.docker.stop_container(&container_id, None).await?;
            self.docker.remove_container(
                &container_id,
                None,
            ).await?;
        }

        Ok(())
    }
}
