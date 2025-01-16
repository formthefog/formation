use std::collections::HashMap;
use std::io::Write;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::fs::{self, OpenOptions};
use std::thread::sleep;
use std::time::Duration;
use std::sync::Arc;
use futures_util::stream::FuturesUnordered;
use tokio::sync::broadcast::Receiver;
use tokio::sync::oneshot::Sender;
use tokio::sync::Mutex;
use axum::{Router, routing::post, Json, extract::State};
use bollard::container::{Config, CreateContainerOptions, UploadToContainerOptions};
use bollard::models::{HostConfig, DeviceMapping};
use bollard::exec::CreateExecOptions;
use bollard::Docker;
use reqwest::Client;
use serde_json::Value;
use serde::{Serialize, Deserialize};
use tempfile::tempdir;
use crate::pack::FormPack;
use crate::formfile::Formfile;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PackResponse {
    Success,
    Failure
}

pub struct FormPackManager {
    // Monitor ID to monitor
    monitors: HashMap<String, FormPackMonitor>,
    // 8080
    min_port: u16,
    // 8180
    max_port: u16,
    // Server port to monitor ID
    active_ports: HashMap<u16, String>,
    addr: SocketAddr
}

impl FormPackManager {
    pub fn new(addr: SocketAddr) -> Self {
        Self {
            monitors: HashMap::new(),
            min_port: 8080,
            max_port: 8180,
            active_ports: HashMap::new(),
            addr
        }
    }

    pub async fn run(self, mut shutdown: Receiver<()>) -> Result<(), Box<dyn std::error::Error>> {
        let addr = self.addr.to_string();
        tokio::select! {
            res = serve(&addr, Arc::new(Mutex::new(self))) => {
                return res
            }
            _ = shutdown.recv() => {
                eprintln!("Received shutdown signal");
                return Ok(())
            }
        }
    }
}

async fn build_routes(state: Arc<Mutex<FormPackManager>>) -> Router {
    Router::new()
        .route("/ping", post(handle_ping))
        .route("/build", post(handle_pack))
        .with_state(state)
}

async fn serve(addr: &str, state: Arc<Mutex<FormPackManager>>) -> Result<(), Box<dyn std::error::Error>> {
    let routes = build_routes(state).await;

    let listener = tokio::net::TcpListener::bind(
        addr
    ).await?;


    if let Err(e) = axum::serve(listener, routes).await {
        eprintln!("Error in FormPackManager API Server: {e}");
    }

    Ok(())
}

async fn handle_ping() -> Json<Value> {
    Json(serde_json::json!({"ping": "pong"}))
}

async fn handle_pack(
    State(manager): State<Arc<Mutex<FormPackManager>>>,
    Json(pack): Json<FormPack>
) -> Json<PackResponse> {
    let FormPack { formfile, artifacts } = pack; 

    let packdir = if let Ok(td) = tempdir() {
        td
    } else {
        return Json(PackResponse::Failure);
    };

    let mut file = if let Ok(f) = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(packdir.path().join("artifacts.tar.gz")) {
            f
    } else {
        return Json(PackResponse::Failure);
    };

    if let Err(_) = file.write_all(&artifacts) {
        return Json(PackResponse::Failure);
    }

    let manager = manager.lock().await;

    let next_port = manager.active_ports.keys().max().unwrap_or(&8080) + 1; 
    if next_port > manager.max_port {
        return Json(PackResponse::Failure);
    }

    let mut monitor = if let Ok(monitor) = FormPackMonitor::new(
        &format!("http://127.0.0.1:{next_port}")
    ).await {
        monitor
    } else {
        return Json(PackResponse::Failure);
    };

    match monitor.build_image(
        formfile,
        packdir.path().join("artifacts.tar.gz"),
        next_port
    ).await {
        Ok(_res) => {
            return Json(PackResponse::Success);
        }
        Err(_) => {
            return Json(PackResponse::Failure)
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
    pub async fn new(
        build_server_uri: &str
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut monitor = Self {
            docker: Docker::connect_with_local_defaults()?,
            container_id: None,
            build_server_id: None,
            build_server_uri: build_server_uri.to_string(),
            build_server_client: Client::new()
        };

        let container_id = monitor.start_build_container().await?;
        monitor.container_id = Some(container_id.clone());

        Ok(monitor)
    }

    pub fn container_id(&self) -> &Option<String> {
        &self.container_id
    }

    pub async fn build_image(
        &mut self,
        formfile: Formfile,
        artifacts: PathBuf,
        port: u16,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let container_id = self.container_id.take().ok_or(
            Box::new(
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Container ID should be some by the time build_image is called"
                )
            )
        )?;

        self.upload_artifacts(&container_id, artifacts).await?;
        self.start_build_server(&container_id, port).await?;
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

        let host_config = HostConfig {
            devices: Some(vec![DeviceMapping {
                path_on_host: Some("/dev/kvm".to_string()),
                path_in_container: Some("/dev/kvm".to_string()),
                ..Default::default()
            }]),
            ..Default::default()
        };

        let config = Config {
            image: Some("form-builder:latest"),
            cmd: Some(vec!["/bin/bash"]),
            tty: Some(true),
            host_config: Some(host_config),
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

    pub async fn start_build_server(&mut self, container_id: &str, port: u16) -> Result<(), Box<dyn std::error::Error>> {
        let port = port.to_string();
        let exec_opts = CreateExecOptions {
            cmd: Some(vec!["form-build-server", "--port", &port]),
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
