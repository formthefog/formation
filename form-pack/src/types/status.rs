use form_state::{instances::Instance, agent::AIAgent, model::AIModel};
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PackBuildStatus {
    Started(String),
    Failed {
        build_id: String,
        reason: String, 
    },
    Completed {
        instance: Instance,
        agent: Option<AIAgent>,
        model: Option<AIModel>
    }
}
