use std::sync::{Arc, Mutex};
use std::net::SocketAddr;
use futures::future::FuturesUnordered;
use tokio::sync::mpsc;
use std::pin::Pin;
use std::future::Future;
use std::error::Error;
use std::collections::HashMap;
use std::sync::broadcast;
use hex;
use vmm_api::VmmApi;
use vmm_subscriber::VmmSubscriber;

#[derive(Debug)]
pub struct VmManager {
    vm_monitors: HashMap<String, Arc<Mutex<VmMonitor>>>,
    server: tokio::task::JoinHandle<Result<(), Box<dyn Error + Send + Sync + 'static>>>,
    tap_counter: u64,
    formnet_endpoint: String,
    signing_key: String,
    api_response_sender: mpsc::Sender<String>,
    subscriber: Option<VmmSubscriber>,
    publisher_addr: Option<String>,
    #[cfg(not(feature = "devnet"))]
    queue_reader: tokio::task::JoinHandle<()>,
    create_futures: Arc<Mutex<FuturesUnordered<Pin<Box<dyn Future<Output = Result<VmmEvent, Box<dyn Error + Send + Sync>>> + Send + 'static>>>>>,
    internal_event_sender: mpsc::Sender<VmmEvent>,
}

impl VmManager {
    pub async fn new(
        event_sender: tokio::sync::mpsc::Sender<VmmEvent>,
        addr: SocketAddr,
        formnet_endpoint: String,
        signing_key: String,
        subscriber_uri: Option<&str>,
        publisher_addr: Option<String>,
        shutdown_rx: broadcast::Receiver<()>
    ) -> Result<Self, Box<dyn Error + Send + Sync + 'static>> {
        let pk = SigningKey::from_slice(
            &hex::decode(&signing_key)?
        )?;

        let _node_id = hex::encode(Address::from_private_key(&pk));
        let (resp_tx, resp_rx) = tokio::sync::mpsc::channel(1024);
        
        let internal_event_sender_clone = event_sender.clone();

        let api_channel = Arc::new(Mutex::new(VmmApiChannel::new(
            event_sender,
            resp_rx,
        )));
        let api_channel_server = api_channel.clone();
        let server = tokio::task::spawn(async move {
            let server = VmmApi::new(api_channel_server.clone(), addr);
            server.start_api_server().await?;
            Ok::<(), Box<dyn Error + Send + Sync + 'static>>(())
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
            internal_event_sender: internal_event_sender_clone,
        })
    }

    pub async fn run(
    ) {
    }
} 