// src/service/vmm.rs
use std::{collections::HashMap, path::PathBuf};
use std::net::SocketAddr;
use formnet::{JoinRequest, JoinResponse, VmJoinRequest};
use http_body_util::{BodyExt, Full};
use hyper::StatusCode;
use hyper::{body::{Bytes, Incoming},  Method, Request, Response};
use hyper_util::client::legacy::Client;
use hyperlocal::{UnixConnector, UnixClientExt, Uri};
use serde::{Serialize, Deserialize, de::DeserializeOwned};
use shared::interface_config::InterfaceConfig;
use tokio::net::TcpListener;
use std::sync::Arc;
use libc::EFD_NONBLOCK;
use tokio::io::unix::AsyncFd;
use tokio::io::AsyncReadExt;
use conductor::publisher::PubStream;
use tokio::sync::mpsc;
use tokio::sync::broadcast;
use vmm_sys_util::signal::block_signal;
use std::sync::mpsc::Sender;
use vmm::{api::{ApiAction, ApiRequest, VmAddDevice, VmAddUserDevice, VmCoredumpData, VmCounters, VmInfo, VmReceiveMigrationData, VmRemoveDevice, VmResize, VmResizeZone, VmSendMigrationData, VmSnapshotConfig, VmmPingResponse}, config::RestoreConfig, vm_config::{DiskConfig, FsConfig, NetConfig, PmemConfig, VdpaConfig, VsockConfig}, PciDeviceInfo, VmmThreadHandle};
use vmm_sys_util::eventfd::EventFd;
use seccompiler::SeccompAction;
use tokio::task::JoinHandle;
use tokio::sync::Mutex;
use form_types::{FormnetMessage, FormnetTopic, GenericPublisher, PeerType, VmmEvent};
use crate::{api::VmmApi, util::ensure_directory};
use crate::util::add_tap_to_bridge;
use crate::ChError;
use crate::VmRuntime;
use crate::VmState;
use crate::{
    error::VmmError,
    config::create_vm_config,
    instance::{config::VmInstanceConfig, manager::{InstanceManager, VmInstance}},
    ServiceConfig,
};

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
        log::info!("Building URI...");
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

pub struct VmManager {
    // We need to stash threads & socket paths
    config: ServiceConfig,
    vm_monitors: HashMap<String, FormVmm>, 
    server: JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>>,
    tap_counter: u32,
    formnet_endpoint: String,
    api_response_sender: tokio::sync::mpsc::Sender<String>
    // Add subscriber to message broker
}

impl VmManager {
    pub fn new(
        event_sender: tokio::sync::mpsc::Sender<VmmEvent>,
        addr: SocketAddr,
        config: ServiceConfig,
        formnet_endpoint: String,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync + 'static>> {
        let (resp_tx, resp_rx) = tokio::sync::mpsc::channel(1024);
        let server = tokio::task::spawn(async move {
            let server = VmmApi::new(event_sender, resp_rx, addr);
            server.start().await?;
            Ok::<(), Box<dyn std::error::Error + Send + Sync + 'static>>(())
        });
        Ok(Self {
            config,
            vm_monitors: HashMap::new(),
            server, 
            tap_counter: 0,
            formnet_endpoint,
            api_response_sender: resp_tx 
        })
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
            (Some(format!("{path}/form-vm/{}.sock", config.name)), None)
        } else {
            let sock_path = format!("run/form-vmm/{}.sock", config.name);
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
            (Some(format!("/run/form-vm/{}.sock", config.name)), None) 
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
        log::info!("Createed new hypervisor");
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

        log::info!("Inserting Form VMM into vm_monitoris map");
        self.vm_monitors.insert(config.name.clone(), vmm);
        log::info!("Calling `boot` on FormVmm");
        self.boot(config.name.clone()).await?;

        if let Err(e) = add_tap_to_bridge("br0", &config.tap_device.clone()).await {
            log::error!("Error attempting to add tap device {} to bridge: {e}", &config.tap_device)
        };


        Ok(())
    }

    pub async fn boot(&mut self, name: String) -> ApiResult<()> {
        self.get_vmm(&name)?.api.boot().await
    }
    
    pub async fn ping(&self, name: String) -> ApiResult<VmmPingResponse> {
        self.get_vmm(&name)?.api.ping().await
    }

    pub async fn shutdown(&self, name: String) -> ApiResult<()> {
        self.get_vmm(&name)?.api.shutdown().await
    }

    pub async fn pause(&self, name: String) -> ApiResult<()> {
        self.get_vmm(&name)?.api.pause().await
    }

    pub async fn resume(&self, name: String) -> ApiResult<()> {
        self.get_vmm(&name)?.api.resume().await
    }

    pub async fn reboot(&self, name: String) -> ApiResult<()> {
        self.get_vmm(&name)?.api.reboot().await
    }

    pub async fn delete(&mut self, name: String) -> ApiResult<()> {
        let api = &self.get_vmm(&name)?.api;
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

    pub async fn power_button(&self, name: &String) -> ApiResult<()> {
        self.get_vmm(&name)?.api.power_button().await
    }

    pub async fn run(
        mut self,
        mut shutdown_rx: broadcast::Receiver<()>,
        mut api_rx: mpsc::Receiver<VmmEvent>
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        loop {
            tokio::select! {
                res = shutdown_rx.recv() => {
                    match res {
                        Ok(()) => log::warn!("Received shutdown signal, shutting VmManager down"),
                        Err(e) => log::error!("Received error from shutdown signal: {e}")
                    }
                    break;
                }
                Some(event) = api_rx.recv() => {
                    self.handle_vmm_event(event).await?;
                }
            }
        }

        Ok(())
    }

    async fn handle_vmm_event(&mut self, event: VmmEvent) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
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
                let invite = self.request_formnet_invite_for_vm_via_api(name).await?;
                log::info!("Received formnet invite... Building VmInstanceConfig...");

                let mut instance_config: VmInstanceConfig = (&event, &invite).try_into().map_err(|e: VmmError| {
                    VmmError::Config(e.to_string())
                })?;

                log::info!("Built VmInstanceConfig... Adding TAP device name");
                instance_config.tap_device = format!("vmnet{}", self.tap_counter);
                instance_config.ip_addr = format!("11.0.0.{}", self.tap_counter + 2);
                log::info!("Added TAP device name... Incrementing TAP counter...");
                self.tap_counter += 1;
                log::info!("Incremented TAP counter... Attempting to create VM");
                // TODO: return Future, and stash future in a `FuturesUnordered`
                // to be awaited asynchronously.
                self.create(&mut instance_config).await?;
                log::info!("Created VM");
            }
            VmmEvent::Stop { id, .. } => {
                //TODO: verify ownership/authorization, etc.
                self.pause(id).await?;
            }
            VmmEvent::Start {  id, .. } => {
                //TODO: verify ownership/authorization, etc.
                self.boot(id).await?;
            }
            VmmEvent::Delete { id, .. } => {
                self.delete(id).await?;
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

    async fn request_formnet_invite_for_vm_via_broker(
        &self,
        name: String,
        callback: SocketAddr
    ) -> Result<InterfaceConfig, VmmError> {
        // Request a innernet invitation from local innernet peer
        let mut publisher = GenericPublisher::new("127.0.0.1:5555").await.map_err(|e| {
            VmmError::NetworkError(format!("Unable to publish message to setup networking: {e}"))
        })?;

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
