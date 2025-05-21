use crate::datastore::{DataStore, DB_HANDLE, AccountRequest};
use crate::db::write_datastore;
use crate::accounts::*;
use crate::auth::RecoveredAddress;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use axum::{extract::{State, Path, ConnectInfo}, Json, http::StatusCode, response::IntoResponse};
use serde_json::json;

pub async fn list_accounts(
    State(state): State<Arc<Mutex<DataStore>>>,
    recovered: Option<RecoveredAddress>,
    ConnectInfo(connection_info): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    let remote_addr = connection_info.to_string();
    let is_localhost = remote_addr.starts_with("127.0.0.1") || remote_addr.starts_with("::1");

    let datastore = state.lock().await;

    if is_localhost {
        // Localhost can see all accounts, regardless of `recovered`
        log::info!("list_accounts: Request from localhost. Listing all accounts.");
        let mut accounts = Vec::new();
        for read_ctx in datastore.account_state.map.iter() {
            let (_key, bft_register) = read_ctx.val;
            if let Some(account_wrapper) = bft_register.val() {
                accounts.push(account_wrapper.value());
            }
        }
        return (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "message": "Retrieved all accounts (localhost access)",
                "accounts": accounts,
                "total": accounts.len()
            }))
        );
    } 
    // If not localhost, a signature (RecoveredAddress) is required
    else if let Some(auth_data) = recovered {
        let authenticated_address = auth_data.as_hex();
        log::info!("list_accounts: Authenticated as {}. Checking admin status.", authenticated_address);
        
        if datastore.network_state.is_admin_address(&authenticated_address) {
            log::info!("list_accounts: Admin user {}. Listing all accounts.", authenticated_address);
            let mut accounts = Vec::new();
            for read_ctx in datastore.account_state.map.iter() {
                let (_key, bft_register) = read_ctx.val;
                if let Some(account_wrapper) = bft_register.val() {
                    accounts.push(account_wrapper.value());
                }
            }
            return (
                StatusCode::OK,
                Json(json!({
                    "success": true,
                    "message": "Retrieved all accounts (admin access)",
                    "accounts": accounts,
                    "total": accounts.len()
                }))
            );
        } else {
            log::info!("list_accounts: Regular user {}. Listing own account.", authenticated_address);
            if let Some(account) = datastore.account_state.get_account(&authenticated_address) {
                return (
                    StatusCode::OK,
                    Json(json!({
                        "success": true,
                        "message": "Retrieved your account",
                        "accounts": [account],
                        "total": 1
                    }))
                );
            } else {
                return (
                    StatusCode::NOT_FOUND,
                    Json(json!({
                        "success": false,
                        "error": "Your account was not found"
                    }))
                );
            }
        }
    } 
    // Not localhost and no recovered address means forbidden
    else {
        log::warn!("list_accounts: Unauthenticated non-localhost request. Denying access.");
        return (
            StatusCode::FORBIDDEN,
            Json(json!({
                "success": false,
                "error": "Authentication required to list accounts"
            }))
        );
    }
}

pub async fn get_account(
    State(state): State<Arc<Mutex<DataStore>>>,
    recovered: Option<RecoveredAddress>,
    ConnectInfo(connection_info): ConnectInfo<SocketAddr>,
    Path(address): Path<String>
) -> impl IntoResponse {
    let is_localhost = connection_info.to_string().starts_with("127.0.0.1") || connection_info.to_string().starts_with("::1");

    if recovered.is_none() && !is_localhost {
        log::warn!("Unauthorized: Non-localhost call to get_account for {} with no signature.", address);
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "success": false,
                "error": "Missing signature for non-localhost call"
            }))
        );
    }

    if let Some(auth_data) = recovered {
        let authenticated_address = auth_data.as_hex();
        log::info!("Account {} requesting account with address {}", authenticated_address, address);
        
        // Normalize both addresses for comparison: remove "0x" prefix if present and convert to lowercase
        let normalized_authenticated_address = authenticated_address.to_lowercase();
        let normalized_requested_address = address.strip_prefix("0x").unwrap_or(&address).to_lowercase();

        // Only allow users to access their own account, unless it's a localhost call
        if normalized_authenticated_address != normalized_requested_address && !is_localhost {
            log::warn!("Unauthorized: Authenticated address {} (normalized: {}) attempted to access account {} (normalized: {})", 
                authenticated_address, normalized_authenticated_address, address, normalized_requested_address);
            return (
                StatusCode::FORBIDDEN,
                Json(json!({
                    "success": false,
                    "error": "You can only access your own account",
                    "authenticated_as": authenticated_address,
                    "requested_address_in_path": address
                }))
            );
        }
    } else {
        // This case is: recovered is None AND is_localhost is true
        // OR recovered is None AND is_localhost is false (but this is handled by the first check already)
        // So, this effectively means: is_localhost is true and no signature was provided (bypassed by middleware)
        log::info!("Localhost call to get_account for address {}. No signature provided/checked.", address);
    }
    
    // Proceed with fetching the account data using the original (potentially prefixed) address from path
    if let Some(account) = state.lock().await.account_state.get_account(&address) {
        log::info!("Found account with address {address}");
        return (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "account": account
            }))
        );
    } 
    
    (
        StatusCode::NOT_FOUND,
        Json(json!({
            "success": false,
            "error": format!("Account with address {address} not found")
        }))
    )
}

pub async fn create_account(
    State(state): State<Arc<Mutex<DataStore>>>,
    recovered: RecoveredAddress,
    Json(request): Json<AccountRequest>,
) -> impl IntoResponse {
    log::info!("Received account create request");
    
    // Get the authenticated user's address
    let authenticated_address = recovered.as_hex();
    
    // Extract the account to be created
    let account_to_create = match &request {
        AccountRequest::Create(account) => account.clone(),
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "success": false,
                    "error": "Invalid request type for account creation"
                }))
            );
        }
    };
    
    // Ensure users can only create accounts with their own address
    // Normalize both addresses by removing "0x" prefix if present, then converting to lowercase for comparison
    let normalized_authenticated_address = authenticated_address.to_lowercase();
    let normalized_requested_address = account_to_create.address.strip_prefix("0x").unwrap_or(&account_to_create.address).to_lowercase();

    if normalized_authenticated_address != normalized_requested_address {
        log::warn!("Unauthorized: Address {} (normalized: {}) attempted to create account for {} (normalized: {})", 
                  authenticated_address, normalized_authenticated_address, account_to_create.address, normalized_requested_address);
        return (
            StatusCode::FORBIDDEN,
            Json(json!({
                "success": false,
                "error": "You can only create an account with your own authenticated address",
                "authenticated_as": authenticated_address,
                "requested_account": account_to_create.address
            }))
        );
    }
    
    let mut datastore = state.lock().await;
    
    // Check if an account with this address already exists
    if datastore.account_state.get_account(&account_to_create.address).is_some() {
        return (
            StatusCode::CONFLICT,
            Json(json!({
                "success": false,
                "error": format!("Account with address {} already exists", account_to_create.address)
            }))
        );
    }
    
    // Create the account
    let op = datastore.account_state.update_account_local(account_to_create);
    
    // Apply the operation
    if let Err(e) = datastore.handle_account_op(op.clone()).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "success": false,
                "error": format!("Failed to create account: {}", e)
            }))
        );
    }
    
    // Get the created account
    match &op {
        crdts::map::Op::Up { key, .. } => {
            if let Some(account) = datastore.account_state.get_account(key) {
                // Write to persistent storage
                let _ = write_datastore(&DB_HANDLE, &datastore.clone());
                
                // Add to message queue
                if let Err(e) = DataStore::write_to_queue(AccountRequest::Op(op), 7, "global_crdt_ops".to_string()).await {
                    log::error!("Error writing to queue: {}", e);
                }
                
                return (
                    StatusCode::CREATED,
                    Json(json!({
                        "success": true,
                        "message": "Account created successfully",
                        "account": account
                    }))
                );
            } else {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "success": false,
                        "error": "Failed to retrieve created account"
                    }))
                );
            }
        },
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "success": false,
                    "error": "Invalid operation type for account creation"
                }))
            );
        }
    }
}

pub async fn update_account(
    State(state): State<Arc<Mutex<DataStore>>>,
    recovered: Option<RecoveredAddress>,
    ConnectInfo(connection_info): ConnectInfo<SocketAddr>,
    Json(request): Json<AccountRequest>,
) -> impl IntoResponse {
    log::info!("Received account update request");
    
    
    let remote_addr = connection_info.to_string();
    let is_localhost = remote_addr.starts_with("127.0.0.1") || remote_addr.starts_with("::1");

    // Extract the account to be updated
    let account_address = match &request {
        AccountRequest::Update(account) => account.address.clone(),
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "success": false,
                    "error": "Invalid request type for account update"
                }))
            );
        }
    };

    if recovered.is_none() {
        if !is_localhost {
            log::warn!(
                "Unauthorized: Remote attempted to update account {} with no authenticated address", 
                account_address
            );
            return (
                StatusCode::FORBIDDEN,
                Json(json!({
                    "success": false,
                    "error": "You can only update your own account",
                    "authenticated_as": "none",
                    "requested_update_for": account_address
                }))
            );
        }
    } else {
        let auth_addr = recovered.unwrap().as_hex();
        if auth_addr.to_lowercase() != account_address.to_lowercase() {
            log::warn!("Unauthorized: Address {} attempted to update account {}", 
                     auth_addr, account_address);
            return (
                StatusCode::FORBIDDEN,
                Json(json!({
                    "success": false,
                    "error": "You can only update your own account",
                    "authenticated_as": auth_addr,
                    "requested_update_for": account_address
                }))
            );
        }
    };

    let mut datastore = state.lock().await;
    
    match request {
        AccountRequest::Update(account) => {
            // Check if the account exists
            if datastore.account_state.get_account(&account.address).is_none() {
                return (
                    StatusCode::NOT_FOUND,
                    Json(json!({
                        "success": false,
                        "error": format!("Account with address {} does not exist", account.address)
                    }))
                );
            }
            
            // Update the account
            let op = datastore.account_state.update_account_local(account);
            
            // Apply the operation
            if let Err(e) = datastore.handle_account_op(op.clone()).await {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "success": false,
                        "error": format!("Failed to update account: {}", e)
                    }))
                );
            }
            
            // Get the updated account
            match &op {
                crdts::map::Op::Up { key, .. } => {
                    if let Some(account) = datastore.account_state.get_account(key) {
                        // Write to persistent storage
                        let _ = write_datastore(&DB_HANDLE, &datastore.clone());
                        
                        // Add to message queue
                        if let Err(e) = DataStore::write_to_queue(AccountRequest::Op(op), 7, "global_crdt_ops".to_string()).await {
                            log::error!("Error writing to queue: {}", e);
                        }
                        
                        return (
                            StatusCode::OK,
                            Json(json!({
                                "success": true,
                                "message": "Account updated successfully",
                                "account": account
                            }))
                        );
                    } else {
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(json!({
                                "success": false,
                                "error": "Failed to retrieve updated account"
                            }))
                        );
                    }
                },
                _ => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(json!({
                            "success": false,
                            "error": "Invalid operation type for account update"
                        }))
                    );
                }
            }
        },
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "success": false,
                    "error": "Invalid request type for account update"
                }))
            );
        }
    }
}

pub async fn delete_account(
    State(state): State<Arc<Mutex<DataStore>>>,
    recovered: RecoveredAddress,
    Json(request): Json<AccountRequest>,
) -> impl IntoResponse {
    log::info!("Received account delete request");
    
    // Get the authenticated user's address
    let authenticated_address = recovered.as_hex();
    
    // Extract the account to be deleted
    let account_address = match &request {
        AccountRequest::Delete(address) => address.clone(),
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "success": false,
                    "error": "Invalid request type for account deletion"
                }))
            );
        }
    };
    
    // Only allow users to delete their own account
    if authenticated_address.to_lowercase() != account_address.to_lowercase() {
        log::warn!("Unauthorized: Address {} attempted to delete account {}", 
                 authenticated_address, account_address);
        return (
            StatusCode::FORBIDDEN,
            Json(json!({
                "success": false,
                "error": "You can only delete your own account",
                "authenticated_as": authenticated_address,
                "requested_deletion_for": account_address
            }))
        );
    }
    
    let mut datastore = state.lock().await;
    
    match request {
        AccountRequest::Delete(address) => {
            // Check if the account exists
            let account = match datastore.account_state.get_account(&address) {
                Some(account) => account,
                None => {
                    return (
                        StatusCode::NOT_FOUND,
                        Json(json!({
                            "success": false,
                            "error": format!("Account with address {} does not exist", address)
                        }))
                    );
                }
            };
            
            // Create a copy for the response
            let account_copy = account.clone();
            
            // Attempt to delete the account
            if let Err(e) = datastore.handle_account_delete(address).await {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "success": false,
                        "error": format!("Failed to delete account: {}", e)
                    }))
                );
            }
            
            // Write to persistent storage
            let _ = write_datastore(&DB_HANDLE, &datastore.clone());
            
            return (
                StatusCode::OK,
                Json(json!({
                    "success": true,
                    "message": "Account deleted successfully",
                    "account": account_copy
                }))
            );
        },
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "success": false,
                    "error": "Invalid request type for account deletion"
                }))
            );
        }
    }
}

pub async fn transfer_instance_ownership(
    State(state): State<Arc<Mutex<DataStore>>>,
    recovered: RecoveredAddress,
    Json(request): Json<AccountRequest>,
) -> impl IntoResponse {
    log::info!("Received instance ownership transfer request");
    
    // Get the authenticated user's address
    let authenticated_address = recovered.as_hex();
    
    // Extract the transfer details
    let (from_address, to_address, instance_id) = match &request {
        AccountRequest::TransferOwnership { from_address, to_address, instance_id } => {
            (from_address.clone(), to_address.clone(), instance_id.clone())
        },
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "success": false,
                    "error": "Invalid request type for ownership transfer"
                }))
            );
        }
    };
    
    // Only allow the owner of the instance to transfer ownership
    if authenticated_address.to_lowercase() != from_address.to_lowercase() {
        log::warn!("Unauthorized: Address {} attempted to transfer ownership from account {}", 
                 authenticated_address, from_address);
        return (
            StatusCode::FORBIDDEN,
            Json(json!({
                "success": false,
                "error": "You can only transfer ownership of instances that you own",
                "authenticated_as": authenticated_address,
                "requested_transfer_from": from_address
            }))
        );
    }
    
    let mut datastore = state.lock().await;
    
    // Check if source account exists
    if datastore.account_state.get_account(&from_address).is_none() {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({
                "success": false,
                "error": format!("Source account with address {} does not exist", from_address)
            }))
        );
    }
    
    // Check if destination account exists
    if datastore.account_state.get_account(&to_address).is_none() {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({
                "success": false,
                "error": format!("Destination account with address {} does not exist", to_address)
            }))
        );
    }
    
    // Check if the instance exists
    if let Some(_instance) = datastore.instance_state.get_instance(instance_id.clone()) {
        // Check if the source account owns the instance
        let owners = datastore.account_state.get_owners_of_instance(&instance_id);
        let is_owned_by_source = owners.iter().any(|account| account.address == from_address);
        
        if !is_owned_by_source {
            return (
                StatusCode::FORBIDDEN,
                Json(json!({
                    "success": false,
                    "error": format!("Source account does not own the instance {}", instance_id)
                }))
            );
        }
        
        // Verify that the source account has Owner authorization level
        if !datastore.account_state.verify_authorization(&from_address, &instance_id, &AuthorizationLevel::Owner) {
            return (
                StatusCode::FORBIDDEN,
                Json(json!({
                    "success": false,
                    "error": format!("Source account does not have Owner authorization for instance {}", instance_id)
                }))
            );
        }
        
        // Perform the ownership transfer
        if let Err(e) = datastore.handle_transfer_ownership(from_address.clone(), to_address.clone(), instance_id.clone()).await {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "success": false,
                    "error": format!("Failed to transfer ownership: {}", e)
                }))
            );
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
            if let Err(e) = DataStore::write_to_queue(transfer_request, 7, "global_crdt_ops".to_string()).await {
                log::error!("Error writing transfer operation to queue: {}", e);
            }
            
            return (
                StatusCode::OK,
                Json(json!({
                    "success": true,
                    "message": "Ownership transferred successfully",
                    "instance_id": instance_id,
                    "from_address": from_address,
                    "to_address": to_address,
                    "updated_account": account
                }))
            );
        } else {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "success": false,
                    "error": "Failed to retrieve updated account after transfer"
                }))
            );
        }
    } else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({
                "success": false,
                "error": format!("Instance with ID {} does not exist", instance_id)
            }))
        );
    }
}

