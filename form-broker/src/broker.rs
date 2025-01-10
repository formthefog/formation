#![allow(unreachable_code)]
use tokio::io::AsyncReadExt;
use tokio::net::{TcpStream, TcpListener};
use tokio::sync::mpsc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::util::{handle_new_subscription, handle_message_parsing};

pub struct Broker {
    frontend: Arc<TcpListener>,
    backend: Arc<TcpListener>,
    topics: Arc<Mutex<HashMap<String, Vec<Arc<Mutex<TcpStream>>>>>>
}

impl Broker {
    pub async fn new(frontend_uri: &str, backend_uri: &str) -> std::io::Result<Self> {
        let frontend = Arc::new(TcpListener::bind(frontend_uri).await?);
        let backend = Arc::new(TcpListener::bind(backend_uri).await?);
        log::info!("Broker receiving messages on {frontend_uri}...");
        log::info!("Broker receiving subscriptions on {backend_uri}...");
        Ok(Self {
            frontend,
            backend,
            topics: Arc::new(Mutex::new(HashMap::new()))
        })
    }

    pub async fn start(&self) -> std::io::Result<()> {

        let (tx, mut rx) = mpsc::channel(4096);

        let frontend = self.frontend.clone();
        let backend = self.backend.clone();
        let subscriptions = self.topics.clone();
        let frontend_task = tokio::spawn(async move {
            loop {
                match frontend.accept().await {
                    Ok((mut stream, addr)) => {
                        log::info!("Successfully accepted stream from {addr:?}");
                        let tx = tx.clone();
                        tokio::spawn(async move {
                            loop {
                                let mut buffer = [0; 4096];
                                match stream.read(&mut buffer).await {
                                    Ok(n) => {
                                        if n == 0 {
                                            log::warn!("Buffer is empty, breaking loop");
                                            break;
                                        }
                                        match tx.send(buffer[..n].to_vec()).await {
                                            Err(e) => {
                                                log::error!("Channel closed: {e}");
                                                break;
                                            }
                                            _ => {
                                                log::info!("Successfully transferred {n} bytes to backend...");
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        log::info!("Error reading stream to buffer: {e}");
                                        break;
                                    }
                                }
                            }
                            Ok::<(), std::io::Error>(())
                        });
                    }
                    Err(e) => {
                        log::error!("Error accepting stream: {e}");
                        break;
                    }
                }
            }
            Ok::<(), std::io::Error>(())
        });

        let backend_task = tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(mut msg) = rx.recv() => {
                        log::info!("Received new message, attempting to parse");
                        match handle_message_parsing(&mut msg, subscriptions.clone()).await {
                            Err(e) => {
                                log::error!("Error handling message parsing: {e}");
                            }
                            _ => {
                                log::info!("Successfully parsed message...");
                            }
                        }
                    }

                    Ok((stream, _)) = backend.accept() => {
                        match handle_new_subscription(subscriptions.clone(), stream).await {
                            Err(e) => log::error!("Error adding new subscriber: {e}"),
                            _ => {
                                log::info!("Successfully added new subscriber...");
                            }
                        };
                    }
                }
            }

            Ok::<(), std::io::Error>(())
        });

        let _ = tokio::join!(frontend_task, backend_task);

        Ok(())
    }
}
