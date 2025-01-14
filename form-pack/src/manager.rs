use std::collections::HashMap;
use std::path::PathBuf;
use std::fs;
use std::thread::sleep;
use std::time::Duration;
use bollard::container::{Config, CreateContainerOptions, UploadToContainerOptions};
use bollard::exec::CreateExecOptions;
use bollard::Docker;
use reqwest::Client;

use crate::formfile::Formfile;

pub struct FormPackManager {
    // Monitor ID to monitor
    monitors: HashMap<String, FormPackMonitor>,
    // 8080
    min_port: u16,
    // 8180
    max_port: u16,
    // Server port to monitor ID
    active_ports: HashMap<u16, String>
}

impl FormPackManager {
    pub fn new() -> Self {
        Self {
            monitors: HashMap::new(),
            min_port: 8080,
            max_port: 8180,
            active_ports: HashMap::new()
        }
    }
}


pub struct FormPackMonitor {
    docker: Docker,
    container_id: Option<String>,
    build_server_id: Option<String>,
    build_server_uri: String,
    build_server_client: Client,
}

impl FormPackMonitor {
    pub fn new(
        build_server_uri: &str
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            docker: Docker::connect_with_local_defaults()?,
            container_id: None,
            build_server_id: None,
            build_server_uri: build_server_uri.to_string(),
            build_server_client: Client::new()
        })
    }

    pub async fn build_image(
        &mut self,
        formfile: Formfile,
        artifacts: PathBuf
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let container_id = self.start_build_container().await?;
        self.container_id = Some(container_id.clone());

        self.upload_artifacts(&container_id, artifacts).await?;
        self.start_build_server(&container_id).await?;
        self.execute_build(&formfile).await?;

        let image_path = self.extract_disk_image(&container_id).await?;
        self.cleanup().await?;
        Ok(image_path)
    }

    pub async fn start_build_container(&self) -> Result<String, Box<dyn std::error::Error>> {
        let options = Some(CreateContainerOptions {
            name: format!("form-builder-{}", uuid::Uuid::new_v4()),
            platform: None,
        });

        let config = Config {
            image: Some("form-builder:latest"),
            cmd: Some(vec!["/bin/bash"]),
            tty: Some(true),
            ..Default::default()
        };

        let container = self.docker.create_container(options, config).await?;
        self.docker.start_container::<String>(&container.id, None).await?;
        Ok(container.id)
    }

    pub async fn upload_artifacts(
        &self,
        container_id: &str,
        artifacts: PathBuf
    ) -> Result<(), Box<dyn std::error::Error>> {
        let options = UploadToContainerOptions {
            path: "/artifacts",
            ..Default::default()
        };

        let tar_contents = fs::read(artifacts)?;
        self.docker.upload_to_container(container_id, Some(options), tar_contents.into()).await?;

        Ok(())
    }

    pub async fn start_build_server(&mut self, container_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let exec_opts = CreateExecOptions {
            cmd: Some(vec!["/bin/bash/form-build-server"]),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            env: Some(vec!["RUST_LOG=info"]),
            tty: Some(true),
            privileged: Some(true),
            ..Default::default()
        };

        let exec = self.docker.create_exec(container_id, exec_opts).await?;
        self.build_server_id = Some(exec.id.clone());
        self.docker.start_exec(&exec.id, None).await?;

        sleep(Duration::from_secs(2));

        let max_retries = 5;
        let mut current_retry = 0;

        while current_retry < max_retries {
            match self.build_server_client
                .post(format!("{}/ping", self.build_server_uri))
                .send()
                .await {
                    Ok(resp) if resp.status().is_success() => {
                        return Ok(())
                    }
                    _ => {
                        current_retry += 1;
                        sleep(Duration::from_secs(1));
                    }
                }
        }

        Ok(())
    }

    pub async fn execute_build(
        &self,
        formfile: &Formfile,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let _resp = self.build_server_client
            .post(format!("{}/formfile", self.build_server_uri))
            .json(formfile)
            .send()
            .await?;

        Ok(())
    }

    pub async fn extract_disk_image(
        &self,
        _container_id: &str
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        todo!()
    }

    pub async fn cleanup(&mut self) -> Result<(), Box<dyn std::error::Error>> {
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
