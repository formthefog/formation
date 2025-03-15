// form-fuzzing/src/generators/p2p.rs
//! Generators for P2P message queue fuzzing based on the actual form-p2p crate

use crate::generators::Generator;
use rand::{Rng, thread_rng, seq::SliceRandom};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;
use crate::harness::p2p::{Message, BFTQueue, Topic, NodeId, NodeIdentity, P2PConfig, Subscription, QoS};

/// Generate a QueueOp for a BFTQueue containing Vec<u8>
pub fn generate_queue_op() -> String {
    let node_id = generate_node_id();
    let mut queue = BFTQueue::<Vec<u8>>::new();
    
    let mut rng = thread_rng();
    let content = (0..rng.gen_range(1..100)).map(|_| rng.gen::<u8>()).collect::<Vec<_>>();
    let message = Message::new(content);
    
    // Add the message to create an Operation
    queue.add(message, NodeId::from(node_id.clone()))
}

/// Generate a NodeIdentity object
pub fn generate_node_identity() -> NodeIdentity {
    let mut rng = thread_rng();
    let id = Uuid::new_v4().to_string();
    let public_key = format!("pk-{}", Uuid::new_v4());
    
    // Generate 1-3 addresses
    let addr_count = rng.gen_range(1..4);
    let addresses = (0..addr_count)
        .map(|_| format!("{}:{}", 
            (0..4).map(|_| rng.gen_range(1..255).to_string()).collect::<Vec<_>>().join("."),
            rng.gen_range(1000..65535)
        ))
        .collect();
    
    NodeIdentity {
        id,
        public_key,
        addresses,
    }
}

/// Generate a P2PConfig object
pub fn generate_p2p_config() -> P2PConfig {
    let mut rng = thread_rng();
    
    P2PConfig {
        max_connections: rng.gen_range(5..50),
        timeout_ms: rng.gen_range(100..5000),
        keep_alive_interval_ms: rng.gen_range(1000..30000),
    }
}

/// Generate a Subscription object
pub fn generate_subscription() -> Subscription {
    let mut rng = thread_rng();
    
    // Generate QoS
    let qos = match rng.gen_range(0..3) {
        0 => QoS::AtMostOnce,
        1 => QoS::AtLeastOnce,
        _ => QoS::ExactlyOnce,
    };
    
    Subscription {
        id: Uuid::new_v4().to_string(),
        topic: generate_topic(),
        qos,
    }
}

/// Generate a Message object with a string payload
pub fn generate_message() -> Message<String> {
    let topics = [
        "system.notification",
        "user.event",
        "network.status",
        "cluster.update",
        "node.metrics",
    ];
    
    let content = topics.choose(&mut thread_rng())
        .unwrap_or(&"default.message")
        .to_string();
    
    Message::new(content)
}

/// Generate QueueRequest with random content
pub struct QueueRequestGenerator;

impl QueueRequestGenerator {
    pub fn new() -> Self {
        Self {}
    }
}

impl Generator<QueueRequest> for QueueRequestGenerator {
    fn generate(&self) -> QueueRequest {
        let mut rng = thread_rng();
        
        // Generate either an operation request or a write request
        if rng.gen_bool(0.5) {
            // Generate operation request
            let topic = Topic::from(format!("topic-{}", rng.gen::<u16>()));
            let content = (0..rng.gen_range(10..50))
                .map(|_| rng.gen::<u8>())
                .collect::<Vec<u8>>();
            let node_id = NodeId::from(format!("node-{}", rng.gen::<u16>()));
            
            QueueRequest::Operation {
                topic,
                content,
                node_id,
            }
        } else {
            // Generate write request
            let topic = Topic::from(format!("topic-{}", rng.gen::<u16>()));
            let content = (0..rng.gen_range(10..50))
                .map(|_| rng.gen::<u8>())
                .collect::<Vec<u8>>();
            
            QueueRequest::Write {
                topic,
                content,
            }
        }
    }
}

/// Generate QueueResponse with random content
pub struct QueueResponseGenerator;

impl QueueResponseGenerator {
    pub fn new() -> Self {
        Self {}
    }
}

impl Generator<QueueResponse> for QueueResponseGenerator {
    fn generate(&self) -> QueueResponse {
        let mut rng = thread_rng();
        
        match rng.gen_range(0..4) {
            0 => {
                // Generate Ok response
                QueueResponse::Ok
            },
            1 => {
                // Generate Content response
                let content = (0..rng.gen_range(10..50))
                    .map(|_| rng.gen::<u8>())
                    .collect::<Vec<u8>>();
                QueueResponse::Content(content)
            },
            2 => {
                // Generate Message response
                let message = format!("Message-{}", rng.gen::<u16>());
                QueueResponse::Message(message)
            },
            _ => {
                // Generate Failure response
                let reason = format!("Failure-reason-{}", rng.gen::<u16>());
                QueueResponse::Failure(reason)
            }
        }
    }
}

/// Generate a random node ID
pub fn generate_node_id() -> String {
    Uuid::new_v4().to_string()
}

/// Generate a random topic
pub fn generate_topic() -> String {
    let topics = [
        "system.events",
        "user.notifications",
        "network.changes",
        "instance.updates",
        "node.metrics",
        "dns.updates",
        "vm.operations",
        "storage.events",
        "auth.events",
        "cluster.operations"
    ];
    
    topics.choose(&mut thread_rng())
        .unwrap_or(&"default.topic")
        .to_string()
}

/// Generate a random failure reason
pub fn generate_failure_reason() -> String {
    let reasons = [
        "Connection timeout",
        "Not authorized",
        "Topic not found",
        "Queue full",
        "Invalid request format",
        "Operation failed",
        "Node not available",
        "Rate limited",
        "Topic locked",
        "Resource exhausted"
    ];
    
    reasons.choose(&mut thread_rng())
        .unwrap_or(&"Unknown error")
        .to_string()
}

/// FormMQ generator
pub struct FormMQGenerator;

impl FormMQGenerator {
    pub fn new() -> Self {
        Self {}
    }
    
    /// Generate FormMQ parameters
    pub fn generate_params(&self) -> (String, String, String) {
        let node_id = generate_node_id();
        let pk = format!("0x{}", hex::encode((0..32).map(|_| thread_rng().gen::<u8>()).collect::<Vec<_>>()));
        let state_uri = format!("http://{}:{}/state", 
            (0..4).map(|_| thread_rng().gen_range(1..255).to_string()).collect::<Vec<_>>().join("."),
            thread_rng().gen_range(8000..9000)
        );
        
        (node_id, pk, state_uri)
    }
}

/// QueueRequest enum based on actual codebase
#[derive(Clone, Debug)]
pub enum QueueRequest {
    Operation {
        topic: Topic,
        content: Vec<u8>,
        node_id: NodeId,
    },
    Write {
        topic: Topic,
        content: Vec<u8>,
    }
}

/// QueueResponse enum based on actual codebase
#[derive(Clone, Debug)]
pub enum QueueResponse {
    Ok,
    Content(Vec<u8>),
    Message(String),
    Failure(String)
} 