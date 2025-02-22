use serde::{Serialize, Deserialize};

pub mod capabilities;
pub mod capacity;
pub mod metrics;
pub mod heartbeat;
pub mod util;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NodeMetricsRequest {
    SetInitialMetrics {
        node_id: String,
        node_capabilities: crate::capabilities::NodeCapabilities,
        node_capacity: crate::capacity::NodeCapacity,
    },
    UpdateMetrics {
        node_id: String,
        node_capacity: crate::capacity::NodeCapacity,
        node_metrics: crate::metrics::NodeMetrics,
    },
    Heartbeat {
        node_id: String,
        timestamp: i64,
    }
}
