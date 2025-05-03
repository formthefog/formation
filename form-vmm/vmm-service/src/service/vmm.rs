use std::collections::BTreeMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::{collections::HashMap, path::PathBuf};
use std::net::{IpAddr, SocketAddr};
use alloy_primitives::Address;
use form_pack::formfile::Formfile;
use form_state::datastore::InstanceRequest;
use form_state::instances::{ClusterMember, Instance, InstanceAnnotations, InstanceCluster, InstanceEncryption, InstanceMetadata, InstanceMonitoring, InstanceResources, InstanceSecurity, InstanceStatus};
use formnet::{JoinRequest, JoinResponse, VmJoinRequest};
use formnet_server::db::CrdtMap;
use formnet_server::DatabasePeer;
use futures::stream::{FuturesUnordered, StreamExt};
use http_body_util::{BodyExt, Full};
use hyper::StatusCode;
use hyper::{body::{Bytes, Incoming},  Method, Request, Response};
use hyper_util::client::legacy::Client;
use hyperlocal::{UnixConnector, UnixClientExt, Uri};
use k256::ecdsa::SigningKey;
use publicip::Preference;
use serde::{Serialize, Deserialize, de::DeserializeOwned};
use shared::interface_config::InterfaceConfig;
use tokio::net::TcpListener;
use libc::EFD_NONBLOCK;
use tokio::io::AsyncReadExt;
use tokio::sync::{mpsc, Mutex};
use tokio::sync::broadcast;
use tokio::time::interval;
use vmm_sys_util::signal::block_signal;
use vmm::{api::{VmAddDevice, VmAddUserDevice, VmCoredumpData, VmCounters, VmInfo, VmReceiveMigrationData, VmRemoveDevice, VmResize, VmResizeZone, VmSendMigrationData, VmSnapshotConfig, VmmPingResponse}, config::RestoreConfig, vm_config::{DiskConfig, FsConfig, NetConfig, PmemConfig, VdpaConfig, VsockConfig}, PciDeviceInfo, VmmThreadHandle};
use vmm_sys_util::eventfd::EventFd;
use seccompiler::SeccompAction;
use tokio::task::JoinHandle;
use form_types::{FormnetMessage, FormnetTopic, GenericPublisher, PeerType, VmmEvent, VmmSubscriber};
use form_broker::{subscriber::SubStream, publisher::PubStream};
use futures::future::join_all;
use crate::api::VmmApiChannel;
use crate::{api::VmmApi, util::ensure_directory};
use crate::util::add_tap_to_bridge;
use crate::{
    error::VmmError,
    config::create_vm_config,
    instance::config::VmInstanceConfig,
};
use form_pack::helpers::utils::build_instance_id;
use std::io::{Cursor, Write};
use std::convert::TryFrom;
use std::error::Error;
use crate::ChError;
use crate::queue::write::write_to_queue;
use crate::IMAGE_DIR;

type VmmResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync + 'static>>;
type ApiResult<T> = Result<ApiResponse<T>, Box<dyn std::error::Error + Send + Sync + 'static>>; 

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ApiResponse<T> {
    SuccessNoContent {
        code: String, 
    },
    Success {
        code: String,
        content: Option<T>
    },
    Error {
        code: String,
        reason: String,
    }
}

pub struct FormVmm {
    socket_path: String,
    thread: Option<VmmThreadHandle>,
    api: FormVmApi,
}

impl FormVmm {
    fn new(
        socket_path: &str,
        thread: VmmThreadHandle
    ) -> Self {
        Self { socket_path: socket_path.to_string(), thread: Some(thread), api: FormVmApi::new(socket_path) }
    }

    pub fn socket_path(&self) -> &str {
        &self.socket_path
    }
    
    pub async fn join(&mut self) -> VmmResult<()> {
        let handle = self.thread.take();
        if let Some(h) = handle {
            let _ = h.thread_handle
                .join()
                .map_err(|_| Box::new(VmmError::SystemError(format!("Error trying to join vmm thread in FormVmm::join"))))?
                .map_err(|_| Box::new(VmmError::SystemError(format!("Error trying to join vmm thread in FormVmm::join"))))?;
            self.thread = None;
        }

        Ok(())
    }
}

pub struct FormVmApi {
    client: Client<UnixConnector, Full<Bytes>>,
    socket_path: String,
}

impl FormVmApi {
    pub const URI_BASE: &'static str = "localhost/api/v1";
    pub fn new(socket_path: &str) -> Self {
        let client = Client::unix();
        Self {
            client, socket_path: socket_path.to_string()
        }
    }

    pub async fn ping(&self) -> ApiResult<VmmPingResponse> {
        self.get::<VmmPingResponse>("vmm.ping").await
    }

    pub async fn shutdown(&self) -> ApiResult<()> {
        self.empty_body_request("vmm.shutdown").await
    }

    pub async fn create(&self, config: &VmInstanceConfig) -> ApiResult<()> {
        let json_body = serde_json::to_string(
            &create_vm_config(config)
        ).map_err(|e| {
            Box::new(VmmError::OperationFailed(
                format!("vm.create faield to convert body of request to json: {e}")
            ))
        })?;
        Ok(self.body_request("vm.create", json_body).await.map_err(|e| {
            Box::new(VmmError::OperationFailed(
                format!("vm.create failed to send request succesfully: {e}")
            ))
        })?)

    }

    pub async fn boot(&self) -> ApiResult<()> {
        self.empty_body_request("vm.boot").await
    }

    pub async fn delete(&self) -> ApiResult<()> {
        self.empty_body_request("vm.delete").await
    }

    pub async fn reboot(&self) -> ApiResult<()> {
        self.empty_body_request("vm.reboot").await
    }

    pub async fn power_button(&self) -> ApiResult<()> {
        self.empty_body_request("vm.power-button").await
    }

    pub async fn pause(&self) -> ApiResult<()> {
        self.empty_body_request("vm.pause").await
    }

    pub async fn resume(&self) -> ApiResult<()> {
        self.empty_body_request("vm.resume").await
    }

    pub async fn snapshot(&self, config: &VmSnapshotConfig) -> ApiResult<()> {
        let body = serde_json::to_string(config)?;
        self.body_request("vm.snapshot", body).await
    }

    pub async fn coredump(&self, data: &VmCoredumpData) -> ApiResult<()> {
        let body = serde_json::to_string(data)?;
        self.body_request("vm.coredump", body).await
    }

    pub async fn restore(&self, config: &RestoreConfig) -> ApiResult<()> {
        let body = serde_json::to_string(config)?;
        self.body_request("vm.restore", body).await
    }

    pub async fn resize(&self, data: &VmResize) -> ApiResult<()> {
        let body = serde_json::to_string(data)?;
        self.body_request("vm.resize", body).await
    }

    pub async fn resize_zone(&self, data: &VmResizeZone) -> ApiResult<()> {
        let body = serde_json::to_string(data)?;
        self.body_request("vm.resize-zone", body).await
    }

    pub async fn info(&self) -> ApiResult<VmInfo> {
        self.get::<VmInfo>("vm.info").await
    }

    pub async fn add_device(&self, data: &VmAddDevice) -> ApiResult<PciDeviceInfo> {
        let body = serde_json::to_string(data)?;
        self.body_request("vm.add-device", body).await
    }

    pub async fn add_disk(&self, config: &DiskConfig) -> ApiResult<PciDeviceInfo> {
        let body = serde_json::to_string(config)?;
        self.body_request("vm.add-disk", body).await
    }

    pub async fn add_fs(&self, config: &FsConfig) -> ApiResult<PciDeviceInfo> {
        let body = serde_json::to_string(config)?;
        self.body_request("vm.add-fs", body).await
    }

    pub async fn add_pmem(&self, config: &PmemConfig) -> ApiResult<PciDeviceInfo> {
        let body = serde_json::to_string(config)?;
        self.body_request("vm.add-pmem", body).await
    }

    pub async fn add_net(&self, config: &NetConfig) -> ApiResult<PciDeviceInfo> {
        let body = serde_json::to_string(config)?;
        self.body_request("vm.add-net", body).await
    }

    pub async fn add_user_device(&self, data: &VmAddUserDevice) -> ApiResult<PciDeviceInfo> {
        let body = serde_json::to_string(data)?;
        self.body_request("vm.add-user-device", body).await
    }

    pub async fn add_vdpa(&self, config: &VdpaConfig) -> ApiResult<PciDeviceInfo> {
        let body = serde_json::to_string(config)?;
        self.body_request("vm.add-vdpa", body).await
    }

    pub async fn add_vsock(&self, config: &VsockConfig) -> ApiResult<PciDeviceInfo> {
        let body = serde_json::to_string(config)?;
        self.body_request("vm.add-vsock", body).await
    }

    pub async fn remove_device(&self, data: &VmRemoveDevice) -> ApiResult<()> {
        let body = serde_json::to_string(data)?;
        self.body_request("vm.remove-device", body).await
    }

    pub async fn counters(&self) -> ApiResult<VmCounters> {
        self.get::<VmCounters>("vm.counters").await
    }

    pub async fn nmi(&self) -> ApiResult<()> {
        self.empty_body_request("vm.nmi").await
    }

    pub async fn receive_migration(&self, data: VmReceiveMigrationData) -> ApiResult<()> {
        let body = serde_json::to_string(&data)?;
        self.body_request("vm.receive-migration", body).await
    }

    pub async fn send_migration(&self, data: VmSendMigrationData) -> ApiResult<()> {
        let body = serde_json::to_string(&data)?;
        self.body_request("vm.send-migration", body).await
    }

    async fn build_uri(&self, endpoint: &str) -> hyper::http::Uri {
        log::info!("Building URI for {}/{}...", self.socket_path, endpoint);
        Uri::new(
            self.socket_path.clone(), 
            &format!("{}/{}", Self::URI_BASE, endpoint)
        ).into()
    }

    async fn get<T: DeserializeOwned>(&self, endpoint: &str) -> ApiResult<T> {
        let mut response = self.client.get(self.build_uri(endpoint).await).await?;
        self.recv::<T>(&mut response).await
    }

    async fn empty_body_request<T: DeserializeOwned>(&self, endpoint: &str) -> ApiResult<T> {
        log::info!("Endpoint: {endpoint}");
        let request = Request::builder()
            .method(Method::PUT)
            .uri(self.build_uri(endpoint).await)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .body(Full::new(Bytes::from("")))?;

        let mut response = self.client.request(request).await.map_err(|e| {
            Box::new(
                VmmError::OperationFailed(
                    format!("calling {endpoint} failed on call to self.client.request(reuqest) in `body_request` function: {e}")
                )
            )
        })?;

        log::info!("{response:?}");

        let status = response.status();

        log::info!("{status}");
        
        if status == StatusCode::NO_CONTENT {
            return Ok(ApiResponse::SuccessNoContent { code: status.to_string() })
        }

        if response.status().is_success() {
            return Ok(self.recv::<T>(&mut response).await.map_err(|e| {
                Box::new(VmmError::OperationFailed(
                        format!("calling {endpoint} failed: {e}, received response {}", response.status())
                ))
            })?)
        }
        return Err(
                Box::new(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Received non-success status code from api calling {endpoint}")
                    )
                )
            )
    }

    async fn body_request<T: DeserializeOwned>(&self, endpoint: &str, body: String) -> ApiResult<T> {
        log::info!("Endpoint: {endpoint}");
        let request = Request::builder()
            .method(Method::PUT)
            .uri(self.build_uri(endpoint).await)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .body(Full::new(Bytes::from(body)))?;

        let mut response = self.client.request(request).await.map_err(|e| {
            Box::new(
                VmmError::OperationFailed(
                    format!("calling {endpoint} failed on call to self.client.request(reuqest) in `body_request` function: {e}")
                )
            )
        })?;

        log::info!("{response:?}");

        let status = response.status();

        log::info!("{status}");
        
        if status == StatusCode::NO_CONTENT {
            return Ok(ApiResponse::SuccessNoContent { code: status.to_string() })
        }

        if response.status().is_success() {
            return Ok(self.recv::<T>(&mut response).await.map_err(|e| {
                Box::new(VmmError::OperationFailed(
                        format!("calling {endpoint} failed: {e}, received response {}", response.status())
                ))
            })?)
        }
        return Err(
                Box::new(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Received non-success status code from api calling {endpoint}")
                    )
                )
            )
    }

    async fn recv<T: DeserializeOwned>(&self, resp: &mut Response<Incoming>) -> ApiResult<T> {
        let mut segments: Vec<u8> = Vec::new();
        while let Some(frame_result) = resp.frame().await {
            let frame = frame_result?;

            if let Some(segment) = frame.data_ref() {
                segments.extend(segment.to_vec());
            }
        }

        Ok(ApiResponse::Success {
            code: resp.status().to_string(),
            content: Some(
                serde_json::from_slice::<T>(&segments).map_err(|e| {
                    Box::new(
                        VmmError::OperationFailed(
                            format!("unable to acquire response successuflly in recv() call: {e}")
                        )
                    )
                })?
            )
        })
    }
}

#[allow(dead_code, unused)]
pub struct VmManager {
    vm_monitors: HashMap<String, FormVmm>, 
    server: JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>>,
    #[cfg(not(feature = "devnet"))]
    queue_reader: JoinHandle<()>,
    tap_counter: u32,
    formnet_endpoint: String,
    api_response_sender: tokio::sync::mpsc::Sender<String>,
    subscriber: Option<VmmSubscriber>,
    signing_key: String,
    publisher_addr: Option<String>,
    create_futures: Arc<Mutex<FuturesUnordered<Pin<Box<dyn Future<Output = Result<VmmEvent, Box<dyn std::error::Error + Send + Sync>>> + Send + 'static>>>>>
}

impl VmManager {
    pub async fn new(
        event_sender: tokio::sync::mpsc::Sender<VmmEvent>,
        addr: SocketAddr,
        formnet_endpoint: String,
        signing_key: String,
        subscriber_uri: Option<&str>,
        publisher_addr: Option<String>,
        shutdown_rx: broadcast::Receiver<()>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync + 'static>> {
        let pk = SigningKey::from_slice(
            &hex::decode(&signing_key)?
        )?;

        let _node_id = hex::encode(Address::from_private_key(&pk));
        let (resp_tx, resp_rx) = tokio::sync::mpsc::channel(1024);
        let api_channel = Arc::new(Mutex::new(VmmApiChannel::new(
            event_sender,
            resp_rx,
        )));
        let api_channel_server = api_channel.clone();
        
        let server = tokio::task::spawn(async move {
            let server = VmmApi::new(api_channel_server.clone(), addr);
            if let Some(config) = config {
                server.start_api_server(&config).await?;
            } else {
                let default_config = Config {
                    base_dir: PathBuf::from("/var/lib/formation"),
                    network: Default::default(),
                    limits: Default::default(),
                    default_vm_params: Default::default(),
                    pack_manager: "127.0.0.1:3003".to_string(),
                    api: ApiConfig {
                        port: 3002,
                        signature_public_key: None,
                        auth_token: None,
                    },
                };
                server.start_api_server(&default_config).await?;
            }
            Ok::<(), Box<dyn std::error::Error + Send + Sync + 'static>>(())
        });

        let subscriber = if let Some(uri) = subscriber_uri {
            let subscriber = if let Ok(subscriber) = VmmSubscriber::new(uri).await {
                Some(subscriber)
            } else {
                None
            };
            subscriber
        } else {
            None
        };

        #[cfg(not(feature = "devnet"))]
        let queue_handle = tokio::task::spawn(async move {
            if let Err(e) = VmmApi::start_queue_reader(api_channel.clone(), shutdown_rx).await {
                eprintln!("Error in queue_reader: {e}");
            }
        });

        Ok(Self {
            vm_monitors: HashMap::new(),
            server, 
            tap_counter: 0,
            formnet_endpoint,
            signing_key,
            api_response_sender: resp_tx,
            subscriber,
            publisher_addr,
            #[cfg(not(feature = "devnet"))]
            queue_reader: queue_handle,
            create_futures: Arc::new(Mutex::new(FuturesUnordered::new())),
        })
    }

    pub async fn derive_address(&self) -> Result<String, Box<dyn std::error::Error + Send + Sync + 'static>> {
        let pk = SigningKey::from_slice(
            &hex::decode(&self.signing_key)?
        )?;

        Ok(hex::encode(Address::from_private_key(&pk)))
    }

    pub async fn create(
        &mut self,
        config: &VmInstanceConfig
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        log::info!("Received create request to create vm instance {}...", config.name);
        let (api_socket_path, api_socket_fd) = if let Ok(path) = std::env::var("XDG_RUNTIME_DIR") {
            let sock_path = format!("{path}/form-vmm/{}.sock", config.name);
            ensure_directory(
                PathBuf::from(&sock_path).parent().ok_or(
                    Box::new(
                        std::io::Error::new(
                            std::io::ErrorKind::NotFound,
                            format!("Parent directory for {sock_path} not found")
                        )
                    )
                )?
            )?;
            (Some(format!("{path}/form-vmm/{}.sock", config.name)), None)
        } else {
            let sock_path = format!("/run/form-vmm/{}.sock", config.name);
            ensure_directory(
                PathBuf::from(&sock_path).parent().ok_or(
                    Box::new(
                        std::io::Error::new(
                            std::io::ErrorKind::NotFound,
                            format!("Parent directory for {sock_path} not found")
                        )
                    )
                )?
            )?;
            (Some(format!("/run/form-vmm/{}.sock", config.name)), None) 
        };
        log::info!("Established API Socket for vm instance {}: {:?}...", config.name, api_socket_path);

        // Create channels and EventFDs
        let (api_request_sender, api_request_receiver) = std::sync::mpsc::channel();
        log::info!("Created Api Request channel");

        let api_evt = EventFd::new(EFD_NONBLOCK).map_err(ChError::CreateApiEventFd).map_err(|e| {
            Box::new(VmmError::Config(format!("Unable to acquire EventFd: {e}")))
        })?;

        log::info!("Created api event EventFd");
        // Signal handling
        unsafe {
            libc::signal(libc::SIGCHLD, libc::SIG_IGN);
        }

        log::info!("Set up signal handling");
        for sig in &vmm::vm::Vm::HANDLED_SIGNALS {
            let _ = block_signal(*sig).map_err(|e| eprintln!("Error blocking signals: {e}"));
        }

        for sig in &vmm::Vmm::HANDLED_SIGNALS {
            let _ = block_signal(*sig).map_err(|e| eprintln!("Error blocking signals: {e}"));
        }

        log::info!("Handled signals");
        // Initialize hypervisor
        let hypervisor = hypervisor::new().map_err(|e| Box::new(VmmError::SystemError(
            format!("Unable to create hypervisor: {e}")
        )))?;
        log::info!("Created new hypervisor");
        let exit_evt = EventFd::new(EFD_NONBLOCK).map_err(|e| {
            Box::new(VmmError::Config(
                format!("Unable to create EventFd: {e}")
            ))
        })?;

        log::info!("Created new exit event EventFd");
        // Start the VMM thread
        log::info!("Attempting to start vmm thread");
        let vmm_thread_handle = vmm::start_vmm_thread(
            vmm::VmmVersionInfo::new(env!("BUILD_VERSION"), env!("CARGO_PKG_VERSION")),
            &api_socket_path,
            api_socket_fd,
            api_evt.try_clone()?,
            api_request_sender.clone(),
            api_request_receiver,
            exit_evt.try_clone()?,
            &SeccompAction::Trap,
            hypervisor,
            false,
        )
        .map_err(|e| {
            Box::new(
                VmmError::SystemError(
                    format!("Unable to start vmm thread:{e}")
                )
            )
        })?;
        log::info!("Started VMM Thread");

        // At this point api_socket_path is always Some
        // we can safely unwrap
        log::info!("Creating new FormVmm");
        let vmm = FormVmm::new(
            &api_socket_path.unwrap(),
            vmm_thread_handle
        );

        log::info!("Created new FormVmm");
        log::info!("Calling `create` on FormVmm");
        vmm.api.create(config).await.map_err(|e| {
            Box::new(
                VmmError::OperationFailed(
                    format!("vmm.api.create(config) failed: {e}") 
                )
            )
        })?;

        let formfile: Formfile = serde_json::from_str(&config.formfile)?;

        let node_id = self.derive_address().await?;
        let build_id = config.name.clone();
        log::info!("Deriving instance id from node_id: {node_id} and build_id: {build_id}");
        let instance_id = build_instance_id(node_id, build_id)?; 
        let mut instance = Instance {
            instance_id, 
            node_id: self.derive_address().await?,
            build_id: config.name.clone(),
            dns_record: None,
            formnet_ip: None,
            instance_owner: config.owner.clone(),
            created_at: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64,
            updated_at: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64,
            status: InstanceStatus::Created,
            last_snapshot: 0,
            host_region: String::new(),
            formfile: config.formfile.clone(),
            cluster: InstanceCluster {
                members: BTreeMap::new(),
                scaling_policy: None,
                template_instance_id: None,
                session_affinity_enabled: false,
                scaling_manager: None,
            },
            snapshots: None,
            metadata: InstanceMetadata {
                annotations: InstanceAnnotations {
                    deployed_by: config.owner.clone(),
                    build_commit: None,
                    network_id: 0,
                },
                description: String::new(),
                monitoring: InstanceMonitoring {
                    logging_enabled: false,
                    metrics_endpoint: String::new()
                },
                security: InstanceSecurity {
                    encryption: InstanceEncryption {
                        is_encrypted: false,
                        scheme: None,
                    },
                    hsm: false,
                    tee: false
                },
                tags: vec![]
            },
            resources: InstanceResources {
                vcpus: formfile.get_vcpus(),
                memory_mb: formfile.get_memory() as u32,
                bandwidth_mbps: 1024,
                gpu: None
            },
        };

        #[cfg(not(feature = "devnet"))]
        write_to_queue(InstanceRequest::Update(instance.clone()), 4, "state").await?;

        #[cfg(feature = "devnet")]
        reqwest::Client::new().post("http://127.0.0.1:3004/instance/update")
            .json(&InstanceRequest::Update(instance.clone()))
            .send()
            .await?
            .json()
            .await?;

        log::info!("Inserting Form VMM into vm_monitoris map");
        self.vm_monitors.insert(config.name.clone(), vmm);
        log::info!("Calling `boot` on FormVmm");
        self.boot(&config.name).await?;

        instance.status = InstanceStatus::Started;
        instance.updated_at = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;

        if let Err(e) = add_tap_to_bridge("br0", &config.tap_device.clone()).await {
            log::error!("Error attempting to add tap device {} to bridge: {e}", &config.tap_device)
        };

        #[cfg(feature = "devnet")]
        reqwest::Client::new().post("http://127.0.0.1:3004/instance/update")
            .json(&InstanceRequest::Update(instance.clone()))
            .send()
            .await?
            .json()
            .await?;

        #[cfg(not(feature = "devnet"))]
        write_to_queue(InstanceRequest::Update(instance.clone()), 4, "state").await?;

        Ok(())
    }

    pub async fn boot(&mut self, name: &String) -> ApiResult<()> {
        self.get_vmm(name)?.api.boot().await
    }
    
    pub async fn ping(&self, name: &String) -> ApiResult<VmmPingResponse> {
        self.get_vmm(name)?.api.ping().await
    }

    pub async fn shutdown(&self, name: &String) -> ApiResult<()> {
        self.get_vmm(name)?.api.shutdown().await
    }

    pub async fn pause(&self, name: &String) -> ApiResult<()> {
        self.get_vmm(name)?.api.pause().await
    }

    pub async fn resume(&self, name: &String) -> ApiResult<()> {
        self.get_vmm(name)?.api.resume().await
    }

    pub async fn reboot(&self, name: &String) -> ApiResult<()> {
        self.get_vmm(name)?.api.reboot().await
    }

    pub async fn delete(&mut self, name: &String) -> ApiResult<()> {
        let api = &self.get_vmm(name)?.api;
        let resp = api.delete().await?;
        match &resp {
            ApiResponse::SuccessNoContent { .. } => {
                std::fs::remove_file(&api.socket_path)?;
                self.remove_vmm(&name)?;
                return Ok(resp.clone())
            }
            ApiResponse::Error { .. } => {
                return Ok(resp.clone())
            }
            ApiResponse::Success { code, content } => {
                return Err(
                    Box::new(
                        VmmError::OperationFailed(
                            format!("Received invalid response from `vm.delete` endpoint: {code:?} {content:?}")
                        )
                    )
                )
            }
        }
    }

    pub async fn info(&self, name: &String) -> ApiResult<VmInfo> {
        self.get_vmm(name)?.api.info().await
    }

    pub async fn power_button(&self, name: &String) -> ApiResult<()> {
        self.get_vmm(name)?.api.power_button().await
    }

    pub async fn run(
        mut self,
        mut shutdown_rx: broadcast::Receiver<()>,
        mut api_rx: mpsc::Receiver<VmmEvent>
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        if let Some(mut subscriber) = self.subscriber.take() {
            let futures_clone = self.create_futures.clone();
            let mut interval = interval(Duration::from_secs(20));
            loop {
                tokio::select! {
                    res = shutdown_rx.recv() => {
                        match res {
                            Ok(()) => {
                                log::warn!("Received shutdown signal, shutting VmManager down");
                                self.server.abort();
                                let _ = self.server.await;
                            }
                            Err(e) => log::error!("Received error from shutdown signal: {e}")
                        }
                        break;
                    }
                    Some(event) = api_rx.recv() => {
                        if let Err(e) = self.handle_vmm_event(&event).await {
                            log::error!("Error while handling event: {event:?}: {e}"); 
                        }
                    }
                    Ok(events) = subscriber.receive() => {
                        for event in events {
                            if let Err(e) = self.handle_vmm_event(&event).await {
                                log::error!("Error while handling event: {event:?}: {e}");
                            }
                        }
                    }
                    _ = interval.tick() => {
                        let mut guard = futures_clone.lock().await;
                        while let Some(Ok(event)) = guard.next().await {
                            if let Err(e) = self.handle_vmm_event(&event).await {
                                log::error!("Error while handling event: {event:?}: {e}");
                            }
                        }
                        drop(guard);
                    }
                }
            }
        } else {
            let futures_clone = self.create_futures.clone();
            let mut interval = interval(Duration::from_secs(20));
            loop {
                tokio::select! {
                    res = shutdown_rx.recv() => {
                        match res {
                            Ok(()) => {
                                log::warn!("Received shutdown signal, shutting VmManager down");
                                self.server.abort();
                                let _ = self.server.await;
                            }
                            Err(e) => log::error!("Received error from shutdown signal: {e}")
                        }
                        break;
                    }
                    Some(event) = api_rx.recv() => {
                        if let Err(e) = self.handle_vmm_event(&event).await {
                            log::error!("Error while handling event: {event:?}: {e}"); 
                        }
                    }
                    _ = interval.tick() => {
                        let mut guard = futures_clone.lock().await;
                        while let Some(Ok(event)) = guard.next().await {
                            if let Err(e) = self.handle_vmm_event(&event).await {
                                log::error!("Error while handling event: {event:?}: {e}");
                            }
                        }
                        drop(guard);
                    }
                }
            }
        }

        Ok(())
    }

    async fn handle_vmm_event(&mut self, event: &VmmEvent) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        match event {
            VmmEvent::Ping { name } => {
                let resp = self.ping(name).await?;
                self.api_response_sender.send(
                    serde_json::to_string(&resp)?
                ).await?;
            }
            VmmEvent::Create { 
                ref name, 
                ..
            } => {
                log::info!("Instance name: {name}");
                if PathBuf::from(IMAGE_DIR).join(name).with_extension("raw").exists() {
                    let mut instance_config: VmInstanceConfig = event.try_into()
                        .map_err(|e: VmmError| {
                            VmmError::Config(e.to_string())
                        })?;

                    log::info!("Built VmInstanceConfig... Adding TAP device name");
                    instance_config.tap_device = format!("vmnet{}", self.tap_counter);
                    log::info!("Added TAP device name... Incrementing TAP counter...");
                    self.tap_counter += 1;
                    log::info!("Incremented TAP counter... Attempting to create VM");
                    // TODO: return Future, and stash future in a `FuturesUnordered`
                    // to be awaited asynchronously.
                    self.create(&mut instance_config).await?;
                    log::info!("Created VM");
                } else {
                    let await_event = event.clone();
                    let await_res = Box::pin(async {
                        let future = async {
                            let mut interval = interval(Duration::from_secs(20));
                            loop {
                                interval.tick().await;
                                if let VmmEvent::Create { ref name, .. } = &await_event {
                                    if PathBuf::from(IMAGE_DIR).join(name).with_extension("raw").exists() {
                                        break;
                                    }
                                } 
                            }
                            await_event
                        };
                        let complete = tokio::time::timeout(
                            Duration::from_secs(1200),
                            future
                        ).await?;
                        Ok(complete)
                    });
                    let guard = self.create_futures.lock().await;
                    guard.push(await_res);
                    drop(guard);
                    log::error!("Unable to find Formpack");
                    log::error!(r#"
Formpack for {name} doesn't exist:

    Have you succesfully built your Formpack yet? 

    Run 

        ```form pack build .``` 

    from inside your project root directory 
    and ensure the build is successful before
    calling `form pack ship`
"#);
                }
            }
            VmmEvent::BootComplete { id, formnet_ip, build_id, .. } => {
                //TODO: Write this information into State so that 
                //users/developers can "get" the IP address
                log::info!("Received boot complete event, getting self");
                let me = DatabasePeer::<String, CrdtMap>::get(self.derive_address().await?).await?.inner.ip;
                log::info!("Getting instance: {id}");
        
                let mut instance = Instance::get(id).await.ok_or(
                    Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Instance doesn't exist")))?;
                log::info!("Adding cluster member to instance...");

                let cluster_member = ClusterMember {
                    instance_id: instance.instance_id.clone(),
                    node_id: self.derive_address().await?,
                    node_public_ip: publicip::get_any(Preference::Ipv4).ok_or(
                        Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Unable to get node public ip"))
                    )?,
                    node_formnet_ip: me,
                    instance_formnet_ip: formnet_ip.parse()?,
                    status: "Started".to_string(),
                    last_heartbeat: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64,
                    heartbeats_skipped: 0,
                };


                log::info!("Built AddClusterMember InstanceRequest");
                let request = InstanceRequest::AddClusterMember { build_id: build_id.to_string(), cluster_member }; 
                log::info!("Writing AddClusterMember InstanceRequest to queue...");
                #[cfg(not(feature = "devnet"))]
                write_to_queue(request.clone(), 4, "state").await?;
                
                #[cfg(feature = "devnet")]
                reqwest::Client::new().post("http://127.0.0.1:3004/instance/update")
                    .json(&request)
                    .send()
                    .await?
                    .json()
                    .await?;

                log::info!("Adding formnet_ip to instance");
                instance.formnet_ip = Some(formnet_ip.parse()?);
                instance.status = InstanceStatus::Started;
                let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64; 
                instance.updated_at = timestamp;
                
                // Automatic DNS Provisioning
                log::info!("Starting automatic DNS provisioning for instance: {id}");
                
                // Create a vanity domain based on the build ID
                let domain_name = format!("{}.fog", build_id);
                log::info!("Generated vanity domain: {domain_name}");
                
                // Create the DNS record pointing to the instance
                let parsed_formnet_ip = formnet_ip.parse::<IpAddr>()?;
                let socket_addr = SocketAddr::new(parsed_formnet_ip, 22); // Default port for SSH
                
                // Construct request to the DNS API
                let dns_provider = self.publisher_addr.clone().unwrap_or_else(|| "127.0.0.1".to_string());
                let dns_endpoint = format!("http://{dns_provider}:3004/dns/{domain_name}/{build_id}/request_vanity");
                
                log::info!("Sending request to DNS API at: {dns_endpoint}");
                
                // Make the API call
                match reqwest::Client::new()
                    .post(&dns_endpoint)
                    .send()
                    .await {
                        Ok(response) => {
                            match response.status() {
                                reqwest::StatusCode::OK => {
                                    log::info!("Successfully provisioned vanity domain: {domain_name} for instance: {id}");
                                    
                                    // The DNS record will be stored automatically by the DNS service
                                    // We just inform the user that the domain has been provisioned in the logs
                                    log::info!("Instance {id} is now accessible at {domain_name}");
                                },
                                _ => {
                                    let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                                    log::error!("Failed to provision vanity domain: {domain_name}. Error: {error_text}");
                                }
                            }
                        },
                        Err(e) => {
                            log::error!("Failed to send request to DNS API for domain: {domain_name}. Error: {e}");
                        }
                    }

                log::info!("Updating instance...");
                let request = InstanceRequest::Update(instance);

                log::info!("Writing Update request with formnet IP to queue...");
                #[cfg(not(feature = "devnet"))]
                write_to_queue(request.clone(), 4, "state").await?; 

                #[cfg(feature = "devnet")]
                reqwest::Client::new().post("http://127.0.0.1:3004/instance/update")
                    .json(&request)
                    .send()
                    .await?
                    .json()
                    .await?;

                log::info!("Boot Complete for {id}: formnet id: {formnet_ip}");
            }
            VmmEvent::Stop { id, .. } => {
                //TODO: verify ownership/authorization, etc.
                self.pause(id).await?;
                let instance_id = build_instance_id(self.derive_address().await?, id.to_string())?;
                let mut instance = Instance::get(&instance_id).await.ok_or(
                    Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Instance doesn't exist"))
                )?;
                instance.status = InstanceStatus::Stopped;
                let node_id = self.derive_address().await?;
                instance.cluster.members = instance.cluster.members.iter_mut().map(|(k, v)| {
                    if v.node_id == node_id {
                        v.status = "Stopped".to_string();
                    }
                    (k.clone(), v.clone())
                }).collect();
                let request = InstanceRequest::Update(instance);
                #[cfg(not(feature = "devnet"))]
                write_to_queue(request.clone(), 4, "state").await?; 

                #[cfg(feature = "devnet")]
                reqwest::Client::new().post("http://127.0.0.1:3004/instance/update")
                    .json(&request)
                    .send()
                    .await?
                    .json()
                    .await?;

            }
            VmmEvent::Start {  id, .. } => {
                //TODO: verify ownership/authorization, etc.
                self.boot(id).await?;
                let instance_id = build_instance_id(self.derive_address().await?, id.to_string())?;
                let mut instance = Instance::get(&instance_id).await.ok_or(
                    Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Instance doesn't exist"))
                )?;
                instance.status = InstanceStatus::Started;
                let node_id = self.derive_address().await?;
                instance.cluster.members = instance.cluster.members.iter_mut().map(|(k, v)| {
                    if v.node_id == node_id {
                        v.status = "Started".to_string();
                    }
                    (k.clone(), v.clone())
                }).collect();
                let request = InstanceRequest::Update(instance);
                #[cfg(not(feature = "devnet"))]
                write_to_queue(request.clone(), 4, "state").await?; 

                #[cfg(feature = "devnet")]
                reqwest::Client::new().post("http://127.0.0.1:3004/instance/update")
                    .json(&request)
                    .send()
                    .await?
                    .json()
                    .await?;

            }
            VmmEvent::Delete { id, .. } => {
                self.delete(id).await?;
            }
            VmmEvent::Get { id, .. } => {
                let resp = serde_json::to_string(&self.info(id).await?)?;
                self.api_response_sender.send(
                    resp
                ).await?;
            }
            VmmEvent::GetList { .. } => {
                let resp_futures = join_all(self.vm_monitors.iter().map(|(_id, vmm)| async {
                    vmm.api.info().await
                }).collect::<Vec<_>>()).await;
                let resp = resp_futures.iter().filter_map(|info| {
                    match info {
                        Ok(ApiResponse::Success { code: _, content }) => {
                            match content {
                                Some(content) => Some(content),
                                None => None
                            }
                        }
                        _ => None
                    }
                }).collect::<Vec<_>>();

                let resp = serde_json::to_string(&resp)?;
                self.api_response_sender.send(
                    resp
                ).await?;
            }
            _ => {}
            
        }
        Ok(())
    }

    async fn request_formnet_invite_for_vm_via_api(&self, name: &str) -> Result<InterfaceConfig, VmmError> {
        log::info!("Requesting formnet invite for vm {name}");
        log::info!("Building VmJoinRequest");
        let join_request = VmJoinRequest { vm_id: name.to_string() };
        log::info!("Wrapping VmJoinRequest in a JoinRequest");
        let join_request = JoinRequest::InstanceJoinRequest(join_request);
        log::info!("Getting a new client");
        let client = reqwest::Client::new();
        log::info!("Posting request to endpoint using client, awaiting response...");
        let resp = client.post(&format!("http://{}/join", self.formnet_endpoint.clone()))
            .json(&join_request)
            .send().await.map_err(|e| {
                VmmError::NetworkError(e.to_string())
            })?.json::<JoinResponse>().await.map_err(|e| {
                VmmError::NetworkError(e.to_string())
            })?;

        log::info!("Response text: {resp:?}");

        match resp {
            JoinResponse::Success { invitation } => return Ok(invitation),
            JoinResponse::Error(reason) => return Err(VmmError::NetworkError(reason.clone()))
        }
    }

    #[allow(unused)]
    async fn request_formnet_invite_for_vm_via_broker(
        &self,
        name: String,
        callback: SocketAddr
    ) -> Result<InterfaceConfig, VmmError> {
        // Request a innernet invitation from local innernet peer
        let mut publisher = if let Some(addr) = &self.publisher_addr {
            GenericPublisher::new(addr).await.map_err(|e| {
                VmmError::NetworkError(format!("Unable to publish message to setup networking: {e}"))
            })?
        } else {
            return self.request_formnet_invite_for_vm_via_api(&name).await;
        };

        let listener = TcpListener::bind(callback.clone()).await.map_err(|e| {
            VmmError::NetworkError(
                format!("Unable to bind listener to callback socket to receive formnet invite: {e}")
            )
        })?;

        publisher.publish(
            Box::new(FormnetTopic),
            Box::new(FormnetMessage::AddPeer { 
                peer_id: name.clone(),
                peer_type: PeerType::Instance,
                callback
            })
        ).await.map_err(|e| {
            VmmError::NetworkError(
                format!("Error sending message to broker to request formnet invite: {e}")
            )
        })?;

        tokio::select! {
            Ok((mut stream, _)) = listener.accept() => {
                let mut buf: Vec<u8> = vec![];
                if let Ok(n) = stream.read_to_end(&mut buf).await {
                    let invite: shared::interface_config::InterfaceConfig = serde_json::from_slice(&buf[..n]).map_err(|e| {
                        VmmError::NetworkError(
                            format!("Error converting response into InterfaceConfig: {e}")
                        )
                    })?;
                    return Ok(invite);
                }

                return Err(VmmError::NetworkError(format!("Unable to read response on TcpStream: Error awaiting response to formnet invite request")));
            }
            _ = tokio::time::sleep(std::time::Duration::from_secs(10)) => {
                log::error!("Timed out awaiting invitation response from formnet");
                return Err(VmmError::NetworkError(format!("Timed out awaiting invite from formnet for VM {}", name)));
            }
        }
    }

    fn get_vmm(&self, name: &str) -> VmmResult<&FormVmm> {
        Ok(self.vm_monitors.get(name).ok_or(
            VmmError::VmNotFound(
                format!("Unable to find Vm Monitor for {name}")
            )
        )?)
    }

    fn remove_vmm(&mut self, name: &str) -> VmmResult<()> {
        self.vm_monitors.remove(name);
        Ok(())
    }
}
