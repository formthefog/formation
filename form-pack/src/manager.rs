#![allow(unused_assignments)]
use std::collections::BTreeMap;
use std::io::{Cursor, Read, Write};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::fs::{self, File, OpenOptions};
use std::sync::Arc;
use std::thread::sleep;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use alloy_primitives::Address;
use axum::extract::{Path, State};
use flate2::read::GzDecoder;
use form_p2p::queue::{QueueRequest, QueueResponse, QUEUE_PORT};
use form_state::datastore::InstanceRequest;
use futures::{StreamExt, TryStreamExt};
use k256::ecdsa::{RecoveryId, Signature, VerifyingKey};
use tiny_keccak::{Hasher, Sha3};
use tokio::sync::broadcast::Receiver;
use axum::{Router, routing::{post, get}, Json, extract::Multipart};
use bollard::container::{Config, CreateContainerOptions, DownloadFromContainerOptions, UploadToContainerOptions};
use bollard::models::{HostConfig, DeviceMapping, PortBinding};
use bollard::exec::CreateExecOptions;
use bollard::Docker;
use reqwest::Client;
use serde_json::Value;
use serde::{Serialize, Deserialize};
use tempfile::tempdir;
use tokio::sync::Mutex;
use crdts::bft_reg::RecoverableSignature;
use form_state::instances::{Instance, InstanceAnnotations, InstanceCluster, InstanceEncryption, InstanceMetadata, InstanceMonitoring, InstanceResources, InstanceSecurity, InstanceStatus};
use crate::image_builder::IMAGE_PATH;
use crate::formfile::Formfile;

pub const VM_IMAGE_PATH: &str = "/var/lib/formation/vm-images/";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FormVmmService(SocketAddr);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PackResponse {
    Success,
    Failure,
    Status(PackBuildStatus)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PackBuildRequest {
    pub sig: RecoverableSignature,
    pub hash: [u8; 32],
    pub request: PackRequest,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PackRequest {
    pub name: String,
    pub formfile: Formfile,
    pub artifacts: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PackBuildResponse {
    status: PackBuildStatus,
    request: PackBuildRequest,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PackBuildStatus {
    Started(String),
    Failed {
        build_id: String,
        reason: String, 
    },
    Completed(Instance),
}


pub struct FormPackManager {
    addr: SocketAddr,
    node_id: String,
}

impl FormPackManager {
    pub fn new(addr: SocketAddr, node_id: String,) -> Self {
        Self {
            addr,
            node_id
        }
    }

    pub async fn run(self, mut shutdown: Receiver<()>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let addr = self.addr.to_string();
        let pack_manager = Arc::new(Mutex::new(self));
        let inner_addr = addr.clone();
        let inner_pack_manager = pack_manager.clone();
        let handle = tokio::spawn(async move {
            if let Err(e) = serve(inner_addr, inner_pack_manager).await {
                eprintln!("Error serving pack manager api server: {e}");
            }
        });

        let mut n = 0;
        loop {
            tokio::select! {
                Ok(messages) = Self::read_from_queue(Some(n), None) => {
                    for message in &messages {
                        let mut manager = pack_manager.lock().await;
                        if let Err(e) = manager.handle_message(message.to_vec()).await {
                            eprintln!("Error handling message: {e}");
                        };
                    }
                    n += messages.len();
                },
                _ = tokio::time::sleep(Duration::from_millis(100)) => {}
                _ = shutdown.recv() => {
                    eprintln!("Received shutdown signal");
                    handle.abort();
                    break
                }
            }
        }

        Ok(())
    }

    pub async fn handle_message(&mut self, message: Vec<u8>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let subtopic = message[0];
        let request = &message[1..];
        match subtopic {
            0 =>  {
                let msg: PackBuildRequest = serde_json::from_slice(request)?; 
                if let Err(e) = self.handle_pack_request(msg.clone()).await {
                    Self::write_pack_status_failed(&msg, e.to_string()).await?;
                    return Err(e)
                }
            }
            1 => {
                let _msg: PackBuildResponse = serde_json::from_slice(request)?;
            }
            _ => unreachable!()
        }

        Ok(())
    }

    pub async fn write_to_queue(
        message: impl Serialize + Clone,
        sub_topic: u8,
        topic: &str
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut hasher = Sha3::v256();
        let mut topic_hash = [0u8; 32];
        hasher.update(topic.as_bytes());
        hasher.finalize(&mut topic_hash);
        let mut message_code = vec![sub_topic];
        message_code.extend(serde_json::to_vec(&message)?);
        let request = QueueRequest::Write { 
            content: message_code, 
            topic: hex::encode(topic_hash) 
        };

        match Client::new()
            .post(format!("http://127.0.0.1:{}/queue/write_local", QUEUE_PORT))
            .json(&request)
            .send().await?
            .json::<QueueResponse>().await? {
                QueueResponse::OpSuccess => return Ok(()),
                QueueResponse::Failure { reason } => return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("{reason:?}")))),
                _ => return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Invalid response variant for write_local endpoint")))
        }
    }

    pub async fn read_from_queue(
        last: Option<usize>,
        n: Option<usize>,
    ) -> Result<Vec<Vec<u8>>, Box<dyn std::error::Error + Send + Sync>> {
        let mut endpoint = format!("http://127.0.0.1:{}/queue/pack", QUEUE_PORT);
        if let Some(idx) = last {
            endpoint.push_str(&format!("/{idx}"));
            if let Some(n) = n {
                endpoint.push_str(&format!("/{n}/get_n_after"));
            } else {
                endpoint.push_str("/get_after");
            }
        } else {
            if let Some(n) = n {
                endpoint.push_str(&format!("/{n}/get_n"))
            } else {
                endpoint.push_str("/get")
            }
        }

        match Client::new()
            .get(endpoint.clone())
            .send().await?
            .json::<QueueResponse>().await? {
                QueueResponse::List(list) => Ok(list),
                QueueResponse::Failure { reason } => Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("{reason:?}")))),
                _ => Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Invalid response variant for {endpoint}")))) 
        }
    }

    pub async fn write_pack_status_started(message: &PackBuildRequest) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let signer_address = {
            let pk = VerifyingKey::recover_from_msg(
                &message.hash,
                &Signature::from_slice(&hex::decode(message.sig.sig.clone())?)?,
                RecoveryId::from_byte(message.sig.rec).ok_or(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "invalid recovery id")))?
            )?;
            Address::from_public_key(&pk)
        };
        let mut hasher = Sha3::v256();
        let mut hash = [0u8; 32];
        hasher.update(signer_address.as_ref());
        hasher.update(message.request.formfile.name.as_bytes());
        hasher.finalize(&mut hash);
        let status_message = PackBuildResponse {
            status: PackBuildStatus::Started(hex::encode(hash)),
            request: message.clone()
        };

        Self::write_to_queue(status_message, 1, "pack").await?;

        Ok(())
    }

    pub async fn write_pack_status_completed(message: &PackBuildRequest, node_id: String) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let signer_address = {
            let pk = VerifyingKey::recover_from_msg(
                &message.hash,
                &Signature::from_slice(&hex::decode(message.sig.sig.clone())?)?,
                RecoveryId::from_byte(message.sig.rec).ok_or(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "invalid recovery id")))?
            )?;
            Address::from_public_key(&pk)
        };
        println!("signer address: {signer_address:x}");
        let mut hasher = Sha3::v256();
        let mut build_id = [0u8; 32];
        hasher.update(signer_address.as_ref());
        hasher.update(message.request.formfile.name.as_bytes());
        hasher.finalize(&mut build_id);
        let instance_id = build_instance_id(node_id.clone(), hex::encode(build_id))?;

        let instance = Instance {
            instance_id,
            node_id,
            build_id: hex::encode(build_id),
            instance_owner: hex::encode(signer_address),
            dns_record: None,
            formnet_ip: None,
            created_at: 0,
            updated_at: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64,
            last_snapshot: 0,
            status: InstanceStatus::Built,
            host_region: String::new(),
            cluster: InstanceCluster {
                members: BTreeMap::new()
            },
            formfile: serde_json::to_string(&message.request.formfile)?,
            metadata: InstanceMetadata {
                tags: vec![],
                description: String::new(),
                annotations: InstanceAnnotations {
                    deployed_by: String::new(),
                    build_commit: None,
                    network_id: 0,
                },
                security: InstanceSecurity {
                    encryption: InstanceEncryption {
                        is_encrypted: false,
                        scheme: None
                    },
                    tee: false,
                    hsm: false,
                },
                monitoring: InstanceMonitoring {
                    logging_enabled: false,
                    metrics_endpoint: String::new(),
                }
            },
            snapshots: None,
            resources: InstanceResources {
                vcpus: message.request.formfile.get_vcpus(),
                memory_mb: message.request.formfile.get_memory() as u32,
                bandwidth_mbps: 1000,
                gpu: None,
            }
        };

        let status_message = PackBuildResponse {
            status: PackBuildStatus::Completed(instance.clone()),
            request: message.clone()
        };

        Self::write_to_queue(status_message, 1, "pack").await?;

        let request = InstanceRequest::Create(instance);

        Self::write_to_queue(request, 4, "state").await?;

        Ok(())
    }

    pub async fn write_pack_status_failed(message: &PackBuildRequest, reason: String) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let signer_address = {
            let pk = VerifyingKey::recover_from_msg(
                &message.hash,
                &Signature::from_slice(&hex::decode(message.sig.sig.clone())?)?,
                RecoveryId::from_byte(message.sig.rec).ok_or(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "invalid recovery id")))?
            )?;
            Address::from_public_key(&pk)
        };
        let mut hasher = Sha3::v256();
        let mut hash = [0u8; 32];
        hasher.update(signer_address.as_ref());
        hasher.update(message.request.formfile.name.as_bytes());
        hasher.finalize(&mut hash);

        let status_message = PackBuildResponse {
            status: PackBuildStatus::Failed { build_id: hex::encode(hash), reason },
            request: message.clone()
        };

        Self::write_to_queue(status_message, 1, "pack").await?;
        Ok(())
    }

    pub async fn handle_pack_request(&mut self, message: PackBuildRequest) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Self::write_pack_status_started(&message).await?;
        let packdir = tempdir()?;

        println!("Created temporary directory to put artifacts into...");

        let artifacts_path = packdir.path().join("artifacts.tar.gz");
        let metadata_path = packdir.path().join("formfile.json");

        std::fs::write(&metadata_path, serde_json::to_string(&message.request.formfile)?)?;
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(artifacts_path.clone())?;

        file.write_all(&message.request.artifacts)?;

        println!("Reading Formfile json metadata into Formfile struct...");
        let formfile: Formfile = std::fs::read_to_string(&metadata_path)
            .and_then(|s| serde_json::from_str(&s)
                .map_err(|_| {
                    std::io::Error::from(
                        std::io::ErrorKind::InvalidData
                    )
                })
            )?; 

        println!("Building FormPackMonitor for {} build...", formfile.name);
        let mut monitor = FormPackMonitor::new().await?; 
        println!("Attmpting to build image for {}...", formfile.name);
        monitor.build_image(
            self.node_id.clone(),
            message.request.name.clone(),
            formfile,
            artifacts_path,
        ).await?; 

        Self::write_pack_status_completed(&message, self.node_id.clone()).await?;

        Ok(())
    }
}

async fn build_routes(manager: Arc<Mutex<FormPackManager>>) -> Router {
    Router::new()
        .route("/ping", post(handle_ping))
        .route("/build", post(handle_pack))
        .route("/:build_id/get_status", get(get_status))
        .with_state(manager)
}

async fn get_status(
    Path(build_id): Path<String>,
) -> Json<PackResponse> {
    let messages: Vec<PackBuildStatus> = if let Ok(messages) = FormPackManager::read_from_queue(None, None).await {
        let msgs = messages.iter().filter_map(|bytes| {
            let subtopic = bytes[0];
            let msg = &bytes[1..];
            match &subtopic {
                1 => {
                    let msg: PackBuildStatus = match serde_json::from_slice(msg) {
                        Ok(msg) => msg,
                        Err(_) => return None,
                    };

                    match msg {
                        PackBuildStatus::Started(ref id) => if *id == build_id {
                            Some(msg)
                        } else {
                            None
                        },
                        PackBuildStatus::Failed { ref build_id, .. } => if build_id == build_id {
                            Some(msg)
                        } else {
                            None
                        },
                        PackBuildStatus::Completed(ref instance) => if instance.build_id == build_id {
                            Some(msg)
                        } else {
                            None
                        }
                    }
                }
                _ => None,
            }
        }).collect();
        msgs
    } else {
        return Json(PackResponse::Failure)
    };

    if !messages.is_empty() {
        return Json(PackResponse::Status(messages.last().unwrap().clone()))
    } else {
        return Json(PackResponse::Failure)
    }
}

async fn serve(addr: String, manager: Arc<Mutex<FormPackManager>>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("Building routes...");
    let routes = build_routes(manager).await;

    println!("binding listener to addr: {addr}");
    let listener = tokio::net::TcpListener::bind(
        &addr
    ).await?;


    println!("serving server on: {addr}");
    if let Err(e) = axum::serve(listener, routes).await {
        eprintln!("Error in FormPackManager API Server: {e}");
    }

    Ok(())
}

async fn handle_ping() -> Json<Value> {
    println!("Received ping request, responding");
    Json(serde_json::json!({"ping": "pong"}))
}

async fn handle_pack(
    State(manager): State<Arc<Mutex<FormPackManager>>>,
    mut multipart: Multipart
) -> Json<PackResponse> {
    println!("Received a multipart Form, attempting to extract data...");
    let packdir = if let Ok(td) = tempdir() {
        td
    } else {
        return Json(PackResponse::Failure);
    };
    println!("Created temporary directory to put artifacts into...");
    let artifacts_path = packdir.path().join("artifacts.tar.gz");
    let metadata_path = packdir.path().join("formfile.json");

    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let name = field.name().unwrap_or_default();

        if name == "metadata" {
            let data = match field.text().await {
                Ok(text) => text,
                Err(_) => return Json(PackResponse::Failure)
            };

            println!("Extracted metadata field...");
            if let Err(_) = std::fs::write(&metadata_path, data) {
                return Json(PackResponse::Failure);
            }
            println!("Wrote metadata to file...");
        } else if name == "artifacts" {
            let mut file = if let Ok(f) = OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .open(packdir.path().join("artifacts.tar.gz")) {
                    f
            } else {
                return Json(PackResponse::Failure);
            };

            println!("Created file for artifacts...");
            let mut field_stream = field.into_stream();
            println!("Converted artifacts field into stream...");
            while let Some(chunk) = field_stream.next().await {
                println!("Attempting to write stream chunks into file...");
                match chunk {
                    Ok(data) => {
                        if let Err(_) = file.write_all(&data) {
                            return Json(PackResponse::Failure)
                        }
                    }
                    Err(_) => return Json(PackResponse::Failure),
                }
            }
            println!("Wrote artifacts to file...");
        }
    }

    println!("Reading metadata into Formfile struct...");
    let formfile: Formfile = match std::fs::read_to_string(&metadata_path)
        .and_then(|s| serde_json::from_str(&s)
            .map_err(|_| {
                std::io::Error::from(
                    std::io::ErrorKind::InvalidData
                )
            })
        ) {
            Ok(ff) => ff,
            Err(_) => return Json(PackResponse::Failure)
    };

    println!("Building FormPackMonitor for {} build...", formfile.name);
    let mut monitor = match FormPackMonitor::new().await {
        Ok(monitor) => monitor,
        Err(e) => {
            println!("Error building monitor: {e}");
            return Json(PackResponse::Failure);
        }
    }; 

    let guard = manager.lock().await;
    let node_id = guard.node_id.clone();
    drop(guard);
    println!("Attmpting to build image for {}...", formfile.name);
    match monitor.build_image(
        node_id.clone(),
        formfile.name.clone(),
        formfile,
        artifacts_path,
    ).await {
        Ok(_res) => Json(PackResponse::Success),
        Err(e) => {
            println!("Error building image: {e}");
            Json(PackResponse::Failure)
        }
    }
}


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

        println!("Uploading artifacts to {container_id}");
        self.upload_artifacts(&container_id, artifacts).await?;
        println!("Starting build server for {}", formfile.name);
        self.start_build_server(&container_id).await?;
        println!("Requesting image build for {}", formfile.name);
        self.execute_build(node_id.clone(), vm_name.clone(), &formfile).await?;
        self.extract_disk_image(&container_id, vm_name.clone()).await?;
        println!("Image build completed for {} successfully, cleaning up {container_id}...", formfile.name);
        self.cleanup().await?;

        Ok(())
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

        sleep(Duration::from_secs(2));

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
                        return Ok(())
                    }
                    _ => {
                        current_retry += 1;
                        sleep(Duration::from_secs(1));
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

fn is_gzip(data: &[u8]) -> bool {
    data.starts_with(&[0x1F, 0x8B]) // Gzip magic number
}

fn get_host_bridge_ip() -> Result<String, Box<dyn std::error::Error + Send + Sync +'static>> {
    let addrs = get_if_addrs::get_if_addrs()?;
    let pub_addr: Vec<_> = addrs.iter().filter_map(|iface| {
        if iface.name == "br0" {
            match &iface.addr {
                get_if_addrs::IfAddr::V4(ifv4) => {
                    Some(ifv4.ip.to_string())
                }
                _ => None
            }
        } else {
            None
        }
    }).collect::<Vec<_>>();

    let first = pub_addr.first()
        .ok_or(
            Box::new(
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Unable to find IP for br0, host is not set up to host instances"
                )
            )
        )?; 

    Ok(first.to_string())
}

pub fn build_instance_id(node_id: String, build_id: String) -> Result<String, Box<dyn std::error::Error + Send + Sync + 'static>> {
    println!("Deriving instance id from node_id: {node_id} and build_id: {build_id}");
    let node_id_vec = &hex::decode(node_id)?[..20];
    let vm_name_bytes = &hex::decode(build_id.clone())?[..20];

    let instance_id = hex::encode(&vm_name_bytes.iter().zip(node_id_vec.iter()).map(|(&x, &y)| x ^ y).collect::<Vec<u8>>());

    Ok(instance_id)
}
