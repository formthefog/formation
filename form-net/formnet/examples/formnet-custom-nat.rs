use clap::{Parser, ValueEnum};
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    time,
};

#[derive(Parser, Debug)]
#[command(name = "toy_nat_tcp", version, about = "A minimal TCP handshake example for NAT traversal")]
struct Cli {
    /// Unique identifier for this node.
    #[arg(short, long)]
    id: String,

    /// Role of this node: server or client.
    #[arg(short, long, value_enum, default_value_t = Role::Client)]
    role: Role,

    /// Peer address (only required in client mode, e.g. "1.2.3.4:51820").
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

/// Handles an incoming connection on the server side.
async fn handle_connection(mut stream: TcpStream, id: String) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut buf = [0; 1024]; 

    let n = stream.read(&mut buf).await?;
    if buf.is_empty() {
        println!("Received an empty message");
        return Ok(());
    }
    let msg: Message = serde_json::from_slice(&buf[..n])?;
    match msg {
        Message::Ping { id: sender_id, nonce } => {
            println!("Received Ping from {}. Sending Pong...", sender_id);
            let pong = Message::Pong { id: id.clone(), nonce };
            let data = serde_json::to_vec(&pong)?;
            stream.write_all(&data).await?;
        }
        _ => println!("Received unexpected message: {:?}", msg),
    }
    Ok(())
}

/// Runs the server: listens on TCP port 51820 and spawns a task per connection.
async fn run_server(id: String) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr: SocketAddr = "0.0.0.0:51820".parse()?;
    let listener = TcpListener::bind(addr).await?;
    println!("TCP Server listening on {}", listener.local_addr()?);
    loop {
        let (stream, peer_addr) = listener.accept().await?;
        println!("Accepted connection from {}", peer_addr);
        let id_clone = id.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream, id_clone).await {
                eprintln!("Error handling connection from {}: {}", peer_addr, e);
            }
        });
    }
}

/// Runs the client: connects to the specified peer, sends a Ping, and waits for a Pong.
async fn run_client(id: String, peer: SocketAddr) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut stream = TcpStream::connect(peer).await?;
    println!("Connected to server at {}", peer);
    let nonce: u64 = thread_rng().gen();
    let ping = Message::Ping { id: id.clone(), nonce };
    let data = serde_json::to_vec(&ping)?;
    println!("Sending Ping: {:?}", ping);
    stream.write_all(&data).await?;
    stream.flush().await?;

    // Set a timeout for receiving the response.
    let mut buf = Vec::new();
    let timeout = Duration::from_secs(20);
    let _ = time::timeout(timeout, stream.read_to_end(&mut buf)).await??;
    let response: Message = serde_json::from_slice(&buf)?;
    match response {
        Message::Pong { id: server_id, nonce: recv_nonce } if recv_nonce == nonce => {
            println!("Received correct Pong from {} (server id: {})", peer, server_id);
        }
        _ => println!("Received unexpected response: {:?}", response),
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let cli = Cli::parse();
    println!("Starting TCP node with ID: {}", cli.id);
    println!("Role: {:?}", cli.role);

    match cli.role {
        Role::Server => {
            run_server(cli.id.clone()).await?;
        }
        Role::Client => {
            let peer_addr_str = cli.peer.ok_or("Client mode requires --peer <ADDRESS>:<PORT> argument")?;
            let peer: SocketAddr = peer_addr_str.parse()?;
            run_client(cli.id.clone(), peer).await?;
        }
    }
    Ok(())
}

