// form-fuzzing/src/mutators/p2p.rs
//! Mutators for P2P message queue fuzzing based on the actual form-p2p crate

use crate::generators::p2p::{
    QueueRequest, QueueResponse, FormMQGenerator,
    generate_node_id, generate_topic, generate_failure_reason
};
use crate::mutators::Mutator;
use rand::{Rng, thread_rng, seq::SliceRandom};
use crate::harness::p2p::{Topic, NodeId};

/// QueueRequest mutator
pub struct QueueRequestMutator;

impl QueueRequestMutator {
    pub fn new() -> Self {
        Self {}
    }
}

impl Mutator<QueueRequest> for QueueRequestMutator {
    fn mutate(&self, queue_request: &mut QueueRequest) {
        let mut rng = thread_rng();
        
        match queue_request {
            QueueRequest::Operation { topic, content, node_id } => {
                // Decide what to mutate
                let mutation_options = ["topic", "content", "node_id", "type"];
                let mutation = mutation_options.choose(&mut rng).unwrap();
                
                match *mutation {
                    "topic" => {
                        // Mutate topic
                        match rng.gen_range(0..4) {
                            0 => {
                                // Replace with a new topic
                                *topic = Topic::from(generate_topic());
                            },
                            1 => {
                                // Add a subtopic
                                let t = topic.0.clone();
                                *topic = Topic::from(format!("{}/subtopic-{}", t, rng.gen::<u16>()));
                            },
                            2 => {
                                // Prepend a parent topic
                                let t = topic.0.clone();
                                *topic = Topic::from(format!("parent-{}/{}", rng.gen::<u16>(), t));
                            },
                            _ => {
                                // Make an invalid topic with special characters
                                let t = topic.0.clone();
                                *topic = Topic::from(format!("{}#$%^&*", t));
                            }
                        }
                    },
                    "content" => {
                        // Mutate content
                        match rng.gen_range(0..3) {
                            0 => {
                                // Empty the content
                                content.clear();
                            },
                            1 => {
                                // Add random bytes
                                let additional_bytes = (0..rng.gen_range(1..50))
                                    .map(|_| rng.gen::<u8>())
                                    .collect::<Vec<u8>>();
                                content.extend(additional_bytes);
                            },
                            _ => {
                                // Replace with new content
                                *content = (0..rng.gen_range(1..100))
                                    .map(|_| rng.gen::<u8>())
                                    .collect();
                            }
                        }
                    },
                    "node_id" => {
                        // Mutate node_id
                        match rng.gen_range(0..3) {
                            0 => {
                                // Use a random node ID
                                *node_id = NodeId::from(generate_node_id());
                            },
                            1 => {
                                // Make an empty node ID
                                *node_id = NodeId::from(String::new());
                            },
                            _ => {
                                // Make an invalid node ID with special characters
                                let id = node_id.0.clone();
                                *node_id = NodeId::from(format!("{}#$%^&*", id));
                            }
                        }
                    },
                    "type" => {
                        // Change to a Write request
                        let t = topic.0.clone();
                        let c = content.clone();
                        *queue_request = QueueRequest::Write {
                            topic: Topic::from(t),
                            content: c,
                        };
                    },
                    _ => {}
                }
            },
            QueueRequest::Write { topic, content } => {
                // Decide what to mutate
                let mutation_options = ["topic", "content", "type"];
                let mutation = mutation_options.choose(&mut rng).unwrap();
                
                match *mutation {
                    "topic" => {
                        // Mutate topic
                        match rng.gen_range(0..4) {
                            0 => {
                                // Replace with a new topic
                                *topic = Topic::from(generate_topic());
                            },
                            1 => {
                                // Add a subtopic
                                let t = topic.0.clone();
                                *topic = Topic::from(format!("{}/subtopic-{}", t, rng.gen::<u16>()));
                            },
                            2 => {
                                // Prepend a parent topic
                                let t = topic.0.clone();
                                *topic = Topic::from(format!("parent-{}/{}", rng.gen::<u16>(), t));
                            },
                            _ => {
                                // Make an invalid topic with special characters
                                let t = topic.0.clone();
                                *topic = Topic::from(format!("{}#$%^&*", t));
                            }
                        }
                    },
                    "content" => {
                        // Mutate content
                        match rng.gen_range(0..3) {
                            0 => {
                                // Empty the content
                                content.clear();
                            },
                            1 => {
                                // Add random bytes
                                let additional_bytes = (0..rng.gen_range(1..50))
                                    .map(|_| rng.gen::<u8>())
                                    .collect::<Vec<u8>>();
                                content.extend(additional_bytes);
                            },
                            _ => {
                                // Replace with new content
                                *content = (0..rng.gen_range(1..100))
                                    .map(|_| rng.gen::<u8>())
                                    .collect();
                            }
                        }
                    },
                    "type" => {
                        // Change to an Operation request
                        let t = topic.0.clone();
                        let c = content.clone();
                        *queue_request = QueueRequest::Operation {
                            topic: Topic::from(t),
                            content: c,
                            node_id: NodeId::from(generate_node_id()),
                        };
                    },
                    _ => {}
                }
            }
        }
    }
}

/// QueueResponse mutator
pub struct QueueResponseMutator;

impl QueueResponseMutator {
    pub fn new() -> Self {
        Self {}
    }
}

impl Mutator<QueueResponse> for QueueResponseMutator {
    fn mutate(&self, queue_response: &mut QueueResponse) {
        let mut rng = thread_rng();
        
        // Either change the type or mutate the content
        let change_type = rng.gen_bool(0.3);
        
        if change_type {
            // Change response type
            let new_type = rng.gen_range(0..4);
            
            *queue_response = match new_type {
                0 => QueueResponse::Ok,
                1 => {
                    let content = (0..rng.gen_range(1..100))
                        .map(|_| rng.gen::<u8>())
                        .collect();
                    QueueResponse::Content(content)
                },
                2 => QueueResponse::Message(format!("Mutated-{}", rng.gen::<u16>())),
                _ => QueueResponse::Failure(generate_failure_reason())
            };
        } else {
            // Mutate the existing content
            match queue_response {
                QueueResponse::Ok => {
                    // Nothing to mutate for Ok
                },
                QueueResponse::Content(content) => {
                    match rng.gen_range(0..3) {
                        0 => {
                            // Empty the content
                            content.clear();
                        },
                        1 => {
                            // Add random bytes
                            let additional_bytes = (0..rng.gen_range(1..50))
                                .map(|_| rng.gen::<u8>())
                                .collect::<Vec<u8>>();
                            content.extend(additional_bytes);
                        },
                        _ => {
                            // Replace with new content
                            *content = (0..rng.gen_range(1..100))
                                .map(|_| rng.gen::<u8>())
                                .collect();
                        }
                    }
                },
                QueueResponse::Message(msg) => {
                    match rng.gen_range(0..3) {
                        0 => {
                            // Empty the message
                            msg.clear();
                        },
                        1 => {
                            // Add random string
                            msg.push_str(&format!("-mutated-{}", rng.gen::<u16>()));
                        },
                        _ => {
                            // Replace with new message
                            *msg = format!("New-message-{}", rng.gen::<u16>());
                        }
                    }
                },
                QueueResponse::Failure(reason) => {
                    match rng.gen_range(0..3) {
                        0 => {
                            // Empty the reason
                            reason.clear();
                        },
                        1 => {
                            // Add random string
                            reason.push_str(&format!(" (mutated error code: {})", rng.gen::<u16>()));
                        },
                        _ => {
                            // Replace with new reason
                            *reason = generate_failure_reason();
                        }
                    }
                }
            }
        }
    }
} 