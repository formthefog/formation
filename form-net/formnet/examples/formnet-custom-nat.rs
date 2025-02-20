use clap::{Parser, ValueEnum};
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, time::Duration};
use std::sync::Arc;
use tokio::{net::UdpSocket, time};

#[derive(Parser, Debug)]
#[command(name = "toy_nat", version, about = "A minimal NAT traversal handshake example")]
struct Cli {
    /// Unique identifier for this node
    #[arg(short, long)]
    id: String,

    /// Role of this node: server or client
    #[arg(short, long, value_enum, default_value_t = Role::Client)]
    role: Role,

    /// Peer address (only required in client mode, e.g. "1.2.3.4:51820")
    #[arg(short, long)]
    peer: Option<String>,
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    Server,
    Client,
}

#[derive(Serialize, Deserialize, Debug)]
enum Message {
    Ping { id: String, nonce: u64 },
    Pong { id: String, nonce: u64 },
}

/// In client mode, sends a Ping to the given peer address and waits for a matching Pong.
async fn send_handshake(
    socket: Arc<UdpSocket>,
    peer: SocketAddr,
    id: String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let nonce: u64 = thread_rng().gen();
    let ping = Message::Ping {
        id: id.to_string(),
        nonce,
    };
    let data = serde_json::to_vec(&ping)?;
    println!("Sending Ping to {peer}: {:?}", ping);
    socket.send_to(&data, peer).await?;

    let mut buf = vec![0u8; 1024];
    let timeout = Duration::from_secs(5);
    let (len, src) = time::timeout(timeout, socket.recv_from(&mut buf)).await??;
    let received: Message = serde_json::from_slice(&buf[..len])?;
    match received {
        Message::Pong {
            id: ref peer_id,
            nonce: received_nonce,
        } if received_nonce == nonce => {
            println!("Received correct Pong from {src} (peer id: {peer_id})");
        }
        _ => println!("Received unexpected message: {:?}", received),
    }
    Ok(())
}

/// Continuously listens for Ping messages and replies with a corresponding Pong.
async fn respond_to_ping(
    socket: Arc<UdpSocket>,
    id: String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let mut buf = vec![0u8; 1024];
    loop {
        tokio::select! {
            res = socket.recv_from(&mut buf) => {
                match res {
                    Ok((len, src)) => {
                        let received: Message = serde_json::from_slice(&buf[..len])?;
                        if let Message::Ping { id: ref sender_id, nonce } = received {
                            println!("Received Ping from {src} (sender id: {sender_id}). Sending Pong...");
                            let pong = Message::Pong {
                                id: id.to_string(),
                                nonce,
                            };
                            let data = serde_json::to_vec(&pong)?;
                            socket.send_to(&data, src).await?;
                        } else {
                            println!("Ignoring non-Ping message: {:?}", received);
                        }
                    }
                    Err(e) => eprintln!("{e}"),
                }
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let cli = Cli::parse();
    println!("Starting node with ID: {}", cli.id);
    println!("Running as {:?}\n", cli.role);

    match cli.role {
        Role::Server => {
            // Server binds to a fixed port (51820)
            let bind_addr: SocketAddr = "0.0.0.0:51820".parse()?;
            let socket = Arc::new(UdpSocket::bind(bind_addr).await?);
            println!("Server listening on {}.", socket.local_addr()?);
            // In server mode, simply respond to incoming Ping messages.
            respond_to_ping(socket, cli.id.clone()).await?;
        }
        Role::Client => {
            // Client binds to an ephemeral port.
            let socket = Arc::new(UdpSocket::bind("0.0.0.0:0").await?);
            println!("Client bound to {}.", socket.local_addr()?);
            // Client mode requires a peer address.
            let peer_addr_str = cli
                .peer
                .ok_or("Client mode requires --peer <ADDRESS>:<PORT> argument")?;
            let peer_addr: SocketAddr = peer_addr_str.parse()?;

            // Spawn both sending and responding tasks.
            let send_task = tokio::spawn(send_handshake(socket.clone(), peer_addr, cli.id.clone()));
            let respond_task = tokio::spawn(respond_to_ping(socket, cli.id.clone()));

            // Await the handshake result.
            send_task.await??;
            // After a short delay, cancel the responder (in a real application, you’d likely keep it running).
            time::sleep(Duration::from_secs(2)).await;
            respond_task.abort();
        }
    }

    Ok(())
}
