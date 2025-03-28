use crate::datastore::{AccountRequest, AgentRequest, DataStore, DB_HANDLE};
use crate::db::write_datastore;
use crate::agent::*;
use std::sync::Arc;
use tokio::sync::Mutex;
use axum::{extract::{State, Path}, Json};
use form_types::state::{Response, Success};

pub async fn create_agent(
    State(state): State<Arc<Mutex<DataStore>>>,
    Json(request): Json<AgentRequest>
) -> Json<Response<AIAgent>> {
    log::info!("Received agent create request");
    
    let mut datastore = state.lock().await;
    
    match request {
        AgentRequest::Create(agent) => {
            // Check if an agentt with this id already exists
            if datastore.agent_state.get_agent(&agent.agent_id).is_some() {
                return Json(Response::Failure { 
                    reason: Some(format!("Agent with id {} already exists", agent.agent_id)) 
                });
            }
            
            // Create the agent 
            let op = datastore.agent_state.update_agent_local(agent);
            
            // Apply the operation
            if let Err(e) = datastore.handle_agent_op(op.clone()).await {
                return Json(Response::Failure { 
                    reason: Some(format!("Failed to create agent: {}", e)) 
                });
            }
            
            // Get the created agent 
            match &op {
                crdts::map::Op::Up { key, .. } => {
                    if let Some(agent) = datastore.agent_state.get_agent(key) {
                        // Write to persistent storage
                        let _ = write_datastore(&DB_HANDLE, &datastore.clone());
                        
                        // Add to message queue
                        if let Err(e) = DataStore::write_to_queue(AgentRequest::Op(op), 8).await {
                            log::error!("Error writing to queue: {}", e);
                        }
                        
                        return Json(Response::Success(Success::Some(agent)));
                    } else {
                        return Json(Response::Failure { 
                            reason: Some("Failed to retrieve created agent".to_string()) 
                        });
                    }
                },
                _ => {
                    return Json(Response::Failure { 
                        reason: Some("Invalid operation type for agent creation".to_string()) 
                    });
                }
            }
        },
        _ => {
            return Json(Response::Failure { 
                reason: Some("Invalid request type for agent creation".to_string()) 
            });
        }
    }
}

pub async fn update_agent(
    State(datatore): State<Arc<Mutex<DataStore>>>
) {}

pub async fn delete_agent(
    State(datatore): State<Arc<Mutex<DataStore>>>
) {}

pub async fn get_agent(
    State(datatore): State<Arc<Mutex<DataStore>>>
) {}

pub async fn list_agent(
    State(datatore): State<Arc<Mutex<DataStore>>>
) {}
