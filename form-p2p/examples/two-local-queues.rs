use std::sync::Arc;
use tiny_keccak::{Hasher, Sha3};
use tokio::sync::RwLock;
use alloy::{primitives::Address, signers::k256::ecdsa::SigningKey};
use form_p2p::{api::serve, queue::FormMQ};


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pk1 = SigningKey::random(&mut rand::thread_rng());
    let node1_id = hex::encode(Address::from_private_key(&pk1));
    let pk1_hex = hex::encode(&pk1.to_bytes());
    let queue1 = Arc::new(RwLock::new(FormMQ::new(node1_id, pk1_hex, "localhost:3005".to_string())));

    let pk2 = SigningKey::random(&mut rand::thread_rng());
    let node2_id = hex::encode(Address::from_private_key(&pk2));
    let pk2_hex = hex::encode(&pk2.to_bytes());
    let queue2 = Arc::new(RwLock::new(FormMQ::new(node2_id, pk2_hex, "localhost:3005".to_string())));

    let api_queue_1 = queue1.clone();
    let queue_1_api_server = tokio::spawn(async move {
        if let Err(e) = serve(api_queue_1, 3006).await {
            eprintln!("Error serving Queue1 API Server: {e}");
        }
    });

    let api_queue_2 = queue2.clone();
    let queue_2_api_server = tokio::spawn(async move {
        if let Err(e) = serve(api_queue_2, 3007).await {
            eprintln!("Error serving Queue1 API Server: {e}");
        }
    });

    let mut queue_1_writer = queue1.write().await;
    let topic = "test-topic";
    let mut hasher = Sha3::v256();
    let mut topic_hash = [0u8; 32];
    hasher.update(&topic.as_bytes());
    hasher.finalize(&mut topic_hash);

    println!("Writing to Queue1 for topic {topic}: hash {topic_hash:?}");

    let contents = b"This is a Test Queue1".to_vec();
    let op = queue_1_writer.write_local(topic_hash, contents)?;
    queue_1_writer.apply(op.clone());
    if queue_1_writer.op_success(op.clone()) {
        println!("OP applied succesfully sending to Queue2");
        queue_1_writer.send_op(op, "127.0.0.1".parse()?, 3007).await?;
    }

    drop(queue_1_writer);

    let mut queue_2_writer = queue2.write().await;
    let topic = "test-topic";
    let mut hasher = Sha3::v256();
    let mut topic_hash = [0u8; 32];
    hasher.update(&topic.as_bytes());
    hasher.finalize(&mut topic_hash);

    println!("Writing to Queue2 for topic {topic}: hash {topic_hash:?}");

    let contents = b"This is a Test Queue2".to_vec();
    let op = queue_2_writer.write_local(topic_hash, contents)?;
    queue_2_writer.apply(op.clone());
    if queue_2_writer.op_success(op.clone()) {
        println!("OP applied succesfully sending to Queue1");
        queue_2_writer.send_op(op, "127.0.0.1".parse()?, 3006).await?;
    }
    drop(queue_2_writer);

    let queue_1_reader = queue1.read().await;
    let messages = queue_1_reader.read(topic_hash).expect("Queue 1 read returned none");

    assert!(messages.len() > 0);

    for msg in messages {
        let content = String::from_utf8_lossy(&msg.content).to_string();
        println!("Message from Queue1: {content}");
    }


    let queue_2_reader = queue2.read().await;
    let messages = queue_2_reader.read(topic_hash).expect("Queue 2 read returned none");

    assert!(messages.len() > 0);

    for msg in messages {
        let content = String::from_utf8_lossy(&msg.content).to_string();
        println!("Message from Queue2: {content}");
    }

    queue_1_api_server.abort();
    queue_2_api_server.abort();
    Ok(())
}
