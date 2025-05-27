use crate::datastore::{DataStore, DB_HANDLE, AccountRequest};
use crate::db::write_datastore;
use crate::accounts::*;
use crate::auth::{RecoveredAddress, AuthenticationContext};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use axum::{extract::{State, Path, ConnectInfo}, Json, http::StatusCode, response::IntoResponse};
use serde_json::json;

/// List accounts with admin bypass logic
/// 
/// Sub-sub-task 3.5.2.1: Add admin bypass logic to account management handlers
/// Sub-sub-task 3.5.2.2: Implement admin context extraction for user/owner identification
pub async fn list_accounts(
    State(state): State<Arc<Mutex<DataStore>>>,
    auth_context: AuthenticationContext,
) -> impl IntoResponse {
    let datastore = state.lock().await;

    // Admin bypass: Admins can see all accounts
    if auth_context.is_admin() {
        log::info!("list_accounts: Admin access by {}. Listing all accounts.", auth_context.description());
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
                "message": format!("Retrieved all accounts (admin access by {})", auth_context.description()),
                "accounts": accounts,
                "total": accounts.len(),
                "access_level": "admin"
            }))
        );
    }

    // Normal user access: can only see their own account
    match auth_context {
        AuthenticationContext::User { address, .. } => {
            let authenticated_address = address.as_hex();
            log::info!("list_accounts: Regular user {}. Listing own account.", authenticated_address);
            
            if let Some(account) = datastore.account_state.get_account(&authenticated_address) {
                return (
                    StatusCode::OK,
                    Json(json!({
                        "success": true,
                        "message": "Retrieved your account",
                        "accounts": [account],
                        "total": 1,
                        "access_level": "user"
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
        _ => {
            // This shouldn't happen since middleware should handle authentication
            log::error!("list_accounts: Unexpected authentication context: {:?}", auth_context);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "success": false,
                    "error": "Internal authentication error"
                }))
            );
        }
    }
}

/// Get account with admin bypass logic
/// 
/// Sub-sub-task 3.5.2.1: Add admin bypass logic to account management handlers
/// Sub-sub-task 3.5.2.2: Implement admin context extraction for user/owner identification
pub async fn get_account(
    State(state): State<Arc<Mutex<DataStore>>>,
    auth_context: AuthenticationContext,
    Path(address): Path<String>
) -> impl IntoResponse {
    log::info!("get_account: {} requesting account {}", auth_context.description(), address);

    // Admin bypass: Admins can access any account
    if auth_context.is_admin() {
        log::info!("get_account: Admin access by {} for account {}", auth_context.description(), address);
        
        if let Some(account) = state.lock().await.account_state.get_account(&address) {
            log::info!("Found account with address {} (admin access)", address);
            return (
                StatusCode::OK,
                Json(json!({
                    "success": true,
                    "account": account,
                    "access_level": "admin",
                    "accessed_by": auth_context.description()
                }))
            );
        } else {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({
                    "success": false,
                    "error": format!("Account with address {} not found", address),
                    "access_level": "admin"
                }))
            );
        }
    }

    // Normal user access: can only access their own account
    match auth_context {
        AuthenticationContext::User { address: user_address, .. } => {
            let authenticated_address = user_address.as_hex();
            
            // Normalize both addresses for comparison: remove "0x" prefix if present and convert to lowercase
            let normalized_authenticated_address = authenticated_address.to_lowercase();
            let normalized_requested_address = address.strip_prefix("0x").unwrap_or(&address).to_lowercase();

            // Only allow users to access their own account
            if normalized_authenticated_address != normalized_requested_address {
                log::warn!("Unauthorized: Authenticated address {} (normalized: {}) attempted to access account {} (normalized: {})", 
                    authenticated_address, normalized_authenticated_address, address, normalized_requested_address);
                return (
                    StatusCode::FORBIDDEN,
                    Json(json!({
                        "success": false,
                        "error": "You can only access your own account",
                        "authenticated_as": authenticated_address,
                        "requested_address_in_path": address,
                        "access_level": "user"
                    }))
                );
            }

            // User is accessing their own account
            if let Some(account) = state.lock().await.account_state.get_account(&address) {
                log::info!("Found account with address {} (user access)", address);
                return (
                    StatusCode::OK,
                    Json(json!({
                        "success": true,
                        "account": account,
                        "access_level": "user"
                    }))
                );
            } else {
                return (
                    StatusCode::NOT_FOUND,
                    Json(json!({
                        "success": false,
                        "error": format!("Account with address {} not found", address)
                    }))
                );
            }
        }
        _ => {
            // This shouldn't happen since middleware should handle authentication
            log::error!("get_account: Unexpected authentication context: {:?}", auth_context);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "success": false,
                    "error": "Internal authentication error"
                }))
            );
        }
    }
}

/// Create account with admin bypass logic
/// 
/// Sub-sub-task 3.5.2.1: Add admin bypass logic to account management handlers
/// Sub-sub-task 3.5.2.2: Implement admin context extraction for user/owner identification
pub async fn create_account(
    State(state): State<Arc<Mutex<DataStore>>>,
    auth_context: AuthenticationContext,
    Json(request): Json<AccountRequest>,
) -> impl IntoResponse {
    log::info!("Received account create request from {}", auth_context.description());
    
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
    
    // Admin bypass: Admins can create accounts for any address
    if auth_context.is_admin() {
        log::info!("create_account: Admin {} creating account for address {}", 
                   auth_context.description(), account_to_create.address);
        
        let mut datastore = state.lock().await;
        
        // Check if an account with this address already exists
        if datastore.account_state.get_account(&account_to_create.address).is_some() {
            return (
                StatusCode::CONFLICT,
                Json(json!({
                    "success": false,
                    "error": format!("Account with address {} already exists", account_to_create.address),
                    "access_level": "admin"
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
                    "error": format!("Failed to create account: {}", e),
                    "access_level": "admin"
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
                            "message": format!("Account created successfully by admin {}", auth_context.description()),
                            "account": account,
                            "access_level": "admin",
                            "created_by": auth_context.description()
                        }))
                    );
                } else {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({
                            "success": false,
                            "error": "Failed to retrieve created account",
                            "access_level": "admin"
                        }))
                    );
                }
            },
            _ => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({
                        "success": false,
                        "error": "Invalid operation type for account creation",
                        "access_level": "admin"
                    }))
                );
            }
        }
    }

    // Normal user access: can only create accounts with their own address
    match auth_context {
        AuthenticationContext::User { address, .. } => {
            let authenticated_address = address.as_hex();
            
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
                        "requested_account": account_to_create.address,
                        "access_level": "user"
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
                        "error": format!("Account with address {} already exists", account_to_create.address),
                        "access_level": "user"
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
                        "error": format!("Failed to create account: {}", e),
                        "access_level": "user"
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
                                "account": account,
                                "access_level": "user"
                            }))
                        );
                    } else {
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(json!({
                                "success": false,
                                "error": "Failed to retrieve created account",
                                "access_level": "user"
                            }))
                        );
                    }
                },
                _ => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(json!({
                            "success": false,
                            "error": "Invalid operation type for account creation",
                            "access_level": "user"
                        }))
                    );
                }
            }
        }
        _ => {
            // This shouldn't happen since middleware should handle authentication
            log::error!("create_account: Unexpected authentication context: {:?}", auth_context);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "success": false,
                    "error": "Internal authentication error"
                }))
            );
        }
    }
}

/// Update account with admin bypass logic
/// 
/// Sub-sub-task 3.5.2.1: Add admin bypass logic to account management handlers
/// Sub-sub-task 3.5.2.2: Implement admin context extraction for user/owner identification
pub async fn update_account(
    State(state): State<Arc<Mutex<DataStore>>>,
    auth_context: AuthenticationContext,
    Json(request): Json<AccountRequest>,
) -> impl IntoResponse {
    log::info!("Received account update request from {}", auth_context.description());

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

    // Admin bypass: Admins can update any account
    if auth_context.is_admin() {
        log::info!("update_account: Admin {} updating account {}", 
                   auth_context.description(), account_address);
        
        let mut datastore = state.lock().await;
        
        match request {
            AccountRequest::Update(account) => {
                // Check if the account exists
                if datastore.account_state.get_account(&account.address).is_none() {
                    return (
                        StatusCode::NOT_FOUND,
                        Json(json!({
                            "success": false,
                            "error": format!("Account with address {} does not exist", account.address),
                            "access_level": "admin"
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
                            "error": format!("Failed to update account: {}", e),
                            "access_level": "admin"
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
                                    "message": format!("Account updated successfully by admin {}", auth_context.description()),
                                    "account": account,
                                    "access_level": "admin",
                                    "updated_by": auth_context.description()
                                }))
                            );
                        } else {
                            return (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(json!({
                                    "success": false,
                                    "error": "Failed to retrieve updated account",
                                    "access_level": "admin"
                                }))
                            );
                        }
                    },
                    _ => {
                        return (
                            StatusCode::BAD_REQUEST,
                            Json(json!({
                                "success": false,
                                "error": "Invalid operation type for account update",
                                "access_level": "admin"
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
                        "error": "Invalid request type for account update",
                        "access_level": "admin"
                    }))
                );
            }
        }
    }

    // Normal user access: can only update their own account
    match auth_context {
        AuthenticationContext::User { address, .. } => {
            let authenticated_address = address.as_hex();
            
            if authenticated_address.to_lowercase() != account_address.to_lowercase() {
                log::warn!("Unauthorized: Address {} attempted to update account {}", 
                         authenticated_address, account_address);
                return (
                    StatusCode::FORBIDDEN,
                    Json(json!({
                        "success": false,
                        "error": "You can only update your own account",
                        "authenticated_as": authenticated_address,
                        "requested_update_for": account_address,
                        "access_level": "user"
                    }))
                );
            }

            let mut datastore = state.lock().await;
            
            match request {
                AccountRequest::Update(account) => {
                    // Check if the account exists
                    if datastore.account_state.get_account(&account.address).is_none() {
                        return (
                            StatusCode::NOT_FOUND,
                            Json(json!({
                                "success": false,
                                "error": format!("Account with address {} does not exist", account.address),
                                "access_level": "user"
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
                                "error": format!("Failed to update account: {}", e),
                                "access_level": "user"
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
                                        "account": account,
                                        "access_level": "user"
                                    }))
                                );
                            } else {
                                return (
                                    StatusCode::INTERNAL_SERVER_ERROR,
                                    Json(json!({
                                        "success": false,
                                        "error": "Failed to retrieve updated account",
                                        "access_level": "user"
                                    }))
                                );
                            }
                        },
                        _ => {
                            return (
                                StatusCode::BAD_REQUEST,
                                Json(json!({
                                    "success": false,
                                    "error": "Invalid operation type for account update",
                                    "access_level": "user"
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
                            "error": "Invalid request type for account update",
                            "access_level": "user"
                        }))
                    );
                }
            }
        }
        _ => {
            // This shouldn't happen since middleware should handle authentication
            log::error!("update_account: Unexpected authentication context: {:?}", auth_context);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "success": false,
                    "error": "Internal authentication error"
                }))
            );
        }
    }
}

/// Delete account with admin bypass logic
/// 
/// Sub-sub-task 3.5.2.1: Add admin bypass logic to account management handlers
/// Sub-sub-task 3.5.2.2: Implement admin context extraction for user/owner identification
pub async fn delete_account(
    State(state): State<Arc<Mutex<DataStore>>>,
    auth_context: AuthenticationContext,
    Json(request): Json<AccountRequest>,
) -> impl IntoResponse {
    log::info!("Received account delete request from {}", auth_context.description());
    
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
    
    // Admin bypass: Admins can delete any account
    if auth_context.is_admin() {
        log::info!("delete_account: Admin {} deleting account {}", 
                   auth_context.description(), account_address);
        
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
                                "error": format!("Account with address {} does not exist", address),
                                "access_level": "admin"
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
                            "error": format!("Failed to delete account: {}", e),
                            "access_level": "admin"
                        }))
                    );
                }
                
                // Write to persistent storage
                let _ = write_datastore(&DB_HANDLE, &datastore.clone());
                
                return (
                    StatusCode::OK,
                    Json(json!({
                        "success": true,
                        "message": format!("Account deleted successfully by admin {}", auth_context.description()),
                        "account": account_copy,
                        "access_level": "admin",
                        "deleted_by": auth_context.description()
                    }))
                );
            },
            _ => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({
                        "success": false,
                        "error": "Invalid request type for account deletion",
                        "access_level": "admin"
                    }))
                );
            }
        }
    }

    // Normal user access: can only delete their own account
    match auth_context {
        AuthenticationContext::User { address, .. } => {
            let authenticated_address = address.as_hex();
            
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
                        "requested_deletion_for": account_address,
                        "access_level": "user"
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
                                    "error": format!("Account with address {} does not exist", address),
                                    "access_level": "user"
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
                                "error": format!("Failed to delete account: {}", e),
                                "access_level": "user"
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
                            "account": account_copy,
                            "access_level": "user"
                        }))
                    );
                },
                _ => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(json!({
                            "success": false,
                            "error": "Invalid request type for account deletion",
                            "access_level": "user"
                        }))
                    );
                }
            }
        }
        _ => {
            // This shouldn't happen since middleware should handle authentication
            log::error!("delete_account: Unexpected authentication context: {:?}", auth_context);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "success": false,
                    "error": "Internal authentication error"
                }))
            );
        }
    }
}

/// Transfer instance ownership with admin bypass logic
/// 
/// Sub-sub-task 3.5.2.1: Add admin bypass logic to account management handlers
/// Sub-sub-task 3.5.2.2: Implement admin context extraction for user/owner identification
pub async fn transfer_instance_ownership(
    State(state): State<Arc<Mutex<DataStore>>>,
    auth_context: AuthenticationContext,
    Json(request): Json<AccountRequest>,
) -> impl IntoResponse {
    log::info!("Received instance ownership transfer request from {}", auth_context.description());
    
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
    
    // Admin bypass: Admins can transfer ownership between any accounts
    if auth_context.is_admin() {
        log::info!("transfer_instance_ownership: Admin {} transferring instance {} from {} to {}", 
                   auth_context.description(), instance_id, from_address, to_address);
        
        let mut datastore = state.lock().await;
        
        // Check if source account exists
        if datastore.account_state.get_account(&from_address).is_none() {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({
                    "success": false,
                    "error": format!("Source account with address {} does not exist", from_address),
                    "access_level": "admin"
                }))
            );
        }
        
        // Check if destination account exists
        if datastore.account_state.get_account(&to_address).is_none() {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({
                    "success": false,
                    "error": format!("Destination account with address {} does not exist", to_address),
                    "access_level": "admin"
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
                        "error": format!("Source account does not own the instance {}", instance_id),
                        "access_level": "admin"
                    }))
                );
            }
            
            // Admin bypass: Skip authorization level check for admins
            // Perform the ownership transfer
            if let Err(e) = datastore.handle_transfer_ownership(from_address.clone(), to_address.clone(), instance_id.clone()).await {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "success": false,
                        "error": format!("Failed to transfer ownership: {}", e),
                        "access_level": "admin"
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
                        "message": format!("Ownership transferred successfully by admin {}", auth_context.description()),
                        "instance_id": instance_id,
                        "from_address": from_address,
                        "to_address": to_address,
                        "updated_account": account,
                        "access_level": "admin",
                        "transferred_by": auth_context.description()
                    }))
                );
            } else {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "success": false,
                        "error": "Failed to retrieve updated account after transfer",
                        "access_level": "admin"
                    }))
                );
            }
        } else {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({
                    "success": false,
                    "error": format!("Instance with ID {} does not exist", instance_id),
                    "access_level": "admin"
                }))
            );
        }
    }

    // Normal user access: can only transfer ownership of instances they own
    match auth_context {
        AuthenticationContext::User { address, .. } => {
            let authenticated_address = address.as_hex();
            
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
                        "requested_transfer_from": from_address,
                        "access_level": "user"
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
                        "error": format!("Source account with address {} does not exist", from_address),
                        "access_level": "user"
                    }))
                );
            }
            
            // Check if destination account exists
            if datastore.account_state.get_account(&to_address).is_none() {
                return (
                    StatusCode::NOT_FOUND,
                    Json(json!({
                        "success": false,
                        "error": format!("Destination account with address {} does not exist", to_address),
                        "access_level": "user"
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
                            "error": format!("Source account does not own the instance {}", instance_id),
                            "access_level": "user"
                        }))
                    );
                }
                
                // Verify that the source account has Owner authorization level
                if !datastore.account_state.verify_authorization(&from_address, &instance_id, &AuthorizationLevel::Owner) {
                    return (
                        StatusCode::FORBIDDEN,
                        Json(json!({
                            "success": false,
                            "error": format!("Source account does not have Owner authorization for instance {}", instance_id),
                            "access_level": "user"
                        }))
                    );
                }
                
                // Perform the ownership transfer
                if let Err(e) = datastore.handle_transfer_ownership(from_address.clone(), to_address.clone(), instance_id.clone()).await {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({
                            "success": false,
                            "error": format!("Failed to transfer ownership: {}", e),
                            "access_level": "user"
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
                            "updated_account": account,
                            "access_level": "user"
                        }))
                    );
                } else {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({
                            "success": false,
                            "error": "Failed to retrieve updated account after transfer",
                            "access_level": "user"
                        }))
                    );
                }
            } else {
                return (
                    StatusCode::NOT_FOUND,
                    Json(json!({
                        "success": false,
                        "error": format!("Instance with ID {} does not exist", instance_id),
                        "access_level": "user"
                    }))
                );
            }
        }
        _ => {
            // This shouldn't happen since middleware should handle authentication
            log::error!("transfer_instance_ownership: Unexpected authentication context: {:?}", auth_context);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "success": false,
                    "error": "Internal authentication error"
                }))
            );
        }
    }
}

