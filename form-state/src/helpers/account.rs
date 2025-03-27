use crate::datastore::{DataStore, DB_HANDLE, AccountRequest};
use crate::db::write_datastore;
use crate::accounts::*;
use std::sync::Arc;
use tokio::sync::Mutex;
use axum::{extract::{State, Path}, Json};
use form_types::state::{Response, Success};

pub async fn list_accounts(
    State(state): State<Arc<Mutex<DataStore>>>,
) -> Json<Response<Account>> {
    log::info!("Requesting a list of all accounts...");
    let mut accounts = Vec::new();
    
    // Get all accounts from the map
    let datastore = state.lock().await;
    for ctx in datastore.account_state.map.iter() {
        let (_, reg) = ctx.val;
        if let Some(val) = reg.val() {
            accounts.push(val.value());
        }
    }
    
    log::info!("Retrieved a list of all accounts... Returning...");
    Json(Response::Success(Success::List(accounts)))
}

pub async fn get_account(
    State(state): State<Arc<Mutex<DataStore>>>,
    Path(address): Path<String>
) -> Json<Response<Account>> {
    log::info!("Request to get account with address {address}");
    if let Some(account) = state.lock().await.account_state.get_account(&address) {
        log::info!("Found account with address {address}");
        return Json(Response::Success(Success::Some(account)))
    } 
    Json(Response::Failure { reason: Some(format!("Account with address {address} not found")) })
}

pub async fn create_account(
    State(state): State<Arc<Mutex<DataStore>>>,
    Json(request): Json<AccountRequest>,
) -> Json<Response<Account>> {
    log::info!("Received account create request");
    
    let mut datastore = state.lock().await;
    
    match request {
        AccountRequest::Create(account) => {
            // Check if an account with this address already exists
            if datastore.account_state.get_account(&account.address).is_some() {
                return Json(Response::Failure { 
                    reason: Some(format!("Account with address {} already exists", account.address)) 
                });
            }
            
            // Create the account
            let op = datastore.account_state.update_account_local(account);
            
            // Apply the operation
            if let Err(e) = datastore.handle_account_op(op.clone()).await {
                return Json(Response::Failure { 
                    reason: Some(format!("Failed to create account: {}", e)) 
                });
            }
            
            // Get the created account
            match &op {
                crdts::map::Op::Up { key, .. } => {
                    if let Some(account) = datastore.account_state.get_account(key) {
                        // Write to persistent storage
                        let _ = write_datastore(&DB_HANDLE, &datastore.clone());
                        
                        // Add to message queue
                        if let Err(e) = DataStore::write_to_queue(AccountRequest::Op(op), 7).await {
                            log::error!("Error writing to queue: {}", e);
                        }
                        
                        return Json(Response::Success(Success::Some(account)));
                    } else {
                        return Json(Response::Failure { 
                            reason: Some("Failed to retrieve created account".to_string()) 
                        });
                    }
                },
                _ => {
                    return Json(Response::Failure { 
                        reason: Some("Invalid operation type for account creation".to_string()) 
                    });
                }
            }
        },
        _ => {
            return Json(Response::Failure { 
                reason: Some("Invalid request type for account creation".to_string()) 
            });
        }
    }
}

pub async fn update_account(
    State(state): State<Arc<Mutex<DataStore>>>,
    Json(request): Json<AccountRequest>,
) -> Json<Response<Account>> {
    log::info!("Received account update request");
    
    let mut datastore = state.lock().await;
    
    match request {
        AccountRequest::Update(account) => {
            // Check if the account exists
            if datastore.account_state.get_account(&account.address).is_none() {
                return Json(Response::Failure { 
                    reason: Some(format!("Account with address {} does not exist", account.address)) 
                });
            }
            
            // Update the account
            let op = datastore.account_state.update_account_local(account);
            
            // Apply the operation
            if let Err(e) = datastore.handle_account_op(op.clone()).await {
                return Json(Response::Failure { 
                    reason: Some(format!("Failed to update account: {}", e)) 
                });
            }
            
            // Get the updated account
            match &op {
                crdts::map::Op::Up { key, .. } => {
                    if let Some(account) = datastore.account_state.get_account(key) {
                        // Write to persistent storage
                        let _ = write_datastore(&DB_HANDLE, &datastore.clone());
                        
                        // Add to message queue
                        if let Err(e) = DataStore::write_to_queue(AccountRequest::Op(op), 7).await {
                            log::error!("Error writing to queue: {}", e);
                        }
                        
                        return Json(Response::Success(Success::Some(account)));
                    } else {
                        return Json(Response::Failure { 
                            reason: Some("Failed to retrieve updated account".to_string()) 
                        });
                    }
                },
                _ => {
                    return Json(Response::Failure { 
                        reason: Some("Invalid operation type for account update".to_string()) 
                    });
                }
            }
        },
        _ => {
            return Json(Response::Failure { 
                reason: Some("Invalid request type for account update".to_string()) 
            });
        }
    }
}

pub async fn delete_account(
    State(state): State<Arc<Mutex<DataStore>>>,
    Json(request): Json<AccountRequest>,
) -> Json<Response<Account>> {
    log::info!("Received account delete request");
    
    let mut datastore = state.lock().await;
    
    match request {
        AccountRequest::Delete(address) => {
            // Check if the account exists
            let account = match datastore.account_state.get_account(&address) {
                Some(account) => account,
                None => {
                    return Json(Response::Failure { 
                        reason: Some(format!("Account with address {} does not exist", address)) 
                    });
                }
            };
            
            // Create a copy for the response
            let account_copy = account.clone();
            
            // Attempt to delete the account
            if let Err(e) = datastore.handle_account_delete(address).await {
                return Json(Response::Failure { 
                    reason: Some(format!("Failed to delete account: {}", e)) 
                });
            }
            
            // Write to persistent storage
            let _ = write_datastore(&DB_HANDLE, &datastore.clone());
            
            return Json(Response::Success(Success::Some(account_copy)));
        },
        _ => {
            return Json(Response::Failure { 
                reason: Some("Invalid request type for account deletion".to_string()) 
            });
        }
    }
}

pub async fn transfer_instance_ownership(
    State(state): State<Arc<Mutex<DataStore>>>,
    Json(request): Json<AccountRequest>,
) -> Json<Response<Account>> {
    log::info!("Received instance ownership transfer request");
    
    let mut datastore = state.lock().await;
    
    match request {
        AccountRequest::TransferOwnership { from_address, to_address, instance_id } => {
            // Check if source account exists
            if datastore.account_state.get_account(&from_address).is_none() {
                return Json(Response::Failure { 
                    reason: Some(format!("Source account with address {} does not exist", from_address)) 
                });
            }
            
            // Check if destination account exists
            if datastore.account_state.get_account(&to_address).is_none() {
                return Json(Response::Failure { 
                    reason: Some(format!("Destination account with address {} does not exist", to_address)) 
                });
            }
            
            // Check if the instance exists
            if let Some(_instance) = datastore.instance_state.get_instance(instance_id.clone()) {
                // Check if the source account owns the instance
                let owners = datastore.account_state.get_owners_of_instance(&instance_id);
                let is_owned_by_source = owners.iter().any(|account| account.address == from_address);
                
                if !is_owned_by_source {
                    return Json(Response::Failure { 
                        reason: Some(format!("Source account does not own the instance {}", instance_id)) 
                    });
                }
                
                // Verify that the source account has Owner authorization level
                if !datastore.account_state.verify_authorization(&from_address, &instance_id, &AuthorizationLevel::Owner) {
                    return Json(Response::Failure { 
                        reason: Some(format!("Source account does not have Owner authorization for instance {}", instance_id)) 
                    });
                }
                
                // Perform the ownership transfer
                if let Err(e) = datastore.handle_transfer_ownership(from_address.clone(), to_address.clone(), instance_id.clone()).await {
                    return Json(Response::Failure { 
                        reason: Some(format!("Failed to transfer ownership: {}", e)) 
                    });
                }
                
                // Get the updated account
                if let Some(account) = datastore.account_state.get_account(&to_address) {
                    // Write to persistent storage
                    let _ = write_datastore(&DB_HANDLE, &datastore.clone());
                    
                    // Add transfer operation to message queue
                    let transfer_request = AccountRequest::TransferOwnership {
                        from_address: from_address.clone(),
                        to_address: to_address.clone(),
                        instance_id: instance_id.clone(),
                    };
                    if let Err(e) = DataStore::write_to_queue(transfer_request, 7).await {
                        log::error!("Error writing transfer operation to queue: {}", e);
                    }
                    
                    return Json(Response::Success(Success::Some(account)));
                } else {
                    return Json(Response::Failure { 
                        reason: Some("Failed to retrieve updated account after transfer".to_string()) 
                    });
                }
            } else {
                return Json(Response::Failure { 
                    reason: Some(format!("Instance with ID {} does not exist", instance_id)) 
                });
            }
        },
        _ => {
            return Json(Response::Failure { 
                reason: Some("Invalid request type for ownership transfer".to_string()) 
            });
        }
    }
}

