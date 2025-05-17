use std::sync::Arc;
use axum::{
    extract::{State, Path, Json, Extension},
    response::{IntoResponse, Response as AxumResponse,sse::Sse, sse::Event as SseEvent},
    http::StatusCode,
};
use serde_json::json;
use tokio::sync::Mutex;
use futures::stream::{self, StreamExt};
use std::convert::Infallible;
use reqwest::Client;
use std::time::Duration;
use chrono::Utc;
use tokio_stream::StreamExt as TokioStreamExt;

use crate::datastore::DataStore;
use crate::auth::RecoveredAddress; 
use super::agent_request::RunTaskRequest;
use crate::accounts::Account; // Assuming Account struct is here
use crate::agent::AIAgent; // Corrected path and struct name
use crate::instances::{Instance, InstanceStatus}; // Import InstanceStatus as well
use std::net::IpAddr;
use super::agent_response::{RunTaskResponse as AgentRunTaskResponse, UsageInfo as AgentUsageInfo, TaskStreamChunk, ApiError}; // Added TaskStreamChunk and ApiError
use crate::billing::{SubscriptionStatus, SubscriptionTier}; // For subscription logic
// Import error types if you have a specific one for billing/authz failures
// use crate::error::GatewayError; 

pub async fn run_agent_task_handler(
    State(datastore): State<Arc<Mutex<DataStore>>>,
    Extension(recovered_address): Extension<Arc<RecoveredAddress>>,
    Path(agent_id_from_path): Path<String>,
    Json(run_task_request): Json<RunTaskRequest>,
) -> Result<AxumResponse, ApiError> {
    let caller_address_hex = recovered_address.as_hex();
    let agent_id = agent_id_from_path.clone();
    let request_path = format!("/agents/{}/run_task", agent_id);

    log::info!(
        "RunTaskRequest for agent_id: {} from caller: {}, task: {:.50}...",
        agent_id, caller_address_hex, run_task_request.task
    );

    let account: Account;
    let agent_details: AIAgent; // Declare here

    // Scope for initial datastore lock to fetch account and agent details
    {
        let ds_guard = datastore.lock().await;

        // --- Part of Sub-task 4.2: Get Account --- 
        match ds_guard.account_state.get_account(&caller_address_hex) {
            Some(acc_data) => {
                account = acc_data; // No clone needed if only used in this scope for check
                log::info!("Account found for caller {}", caller_address_hex);
            },
            None => {
                log::warn!("Account not found for caller {}. Denying request.", caller_address_hex);
                return Err(ApiError::new("ACCOUNT_NOT_FOUND", "Account not found. Please ensure your account is provisioned.", StatusCode::FORBIDDEN.as_u16()).with_path(&request_path));
            }
        }

        // --- Part of Sub-task 4.3: Get Agent Details (needed for eligibility) ---
        match ds_guard.agent_state.get_agent(&agent_id) { 
            Some(ag_details) => {
                agent_details = ag_details.clone(); // Clone if needed outside this lock, or pass ref
                log::info!("Found agent details for agent_id: {}", agent_id);
            },
            None => {
                log::warn!("Agent with ID '{}' not found.", agent_id);
                return Err(ApiError::new("AGENT_NOT_FOUND", &format!("Agent '{}' not found.", agent_id), StatusCode::NOT_FOUND.as_u16()).with_path(&request_path));
            }
        }

        // --- Sub-task 4.2.2: Actual Eligibility Check --- 
        let mut eligible = true;
        let mut denial_reason = String::new();

        let minimum_operational_credits: u64 = 1; 
        if account.credits < minimum_operational_credits && account.subscription.as_ref().map_or(true, |sub_info| {
            match sub_info.status {
                SubscriptionStatus::Active | SubscriptionStatus::Trial | SubscriptionStatus::PastDue => false,
                _ => true, 
            }
        }) {
            eligible = false;
            denial_reason = "Account has insufficient credits and no active/trialing subscription.".to_string();
        }

        if eligible && agent_details.is_private {
            if !agent_details.owner_id.eq_ignore_ascii_case(&caller_address_hex) {
                eligible = false;
                denial_reason = format!("Agent '{}' is private and you are not the owner or authorized.", agent_id);
            }
        }
        
        if !eligible {
            log::warn!("Caller {} is not eligible for agent {}: {}", caller_address_hex, agent_id, denial_reason);
            return Err(ApiError::new("NOT_ELIGIBLE", &denial_reason, StatusCode::PAYMENT_REQUIRED.as_u16()).with_path(&request_path));
        }
        log::info!("Caller {} passed eligibility pre-check for agent {}", caller_address_hex, agent_id);
        // ds_guard lock is released when it goes out of scope here
    }

    // --- Sub-task 4.3 (continued): Instance Lookup --- 
    let instance_details: Instance;
    let agent_task_path: String;
    let agent_task_port: u16;
    let instance_formnet_ip: IpAddr;
    {
        let ds = datastore.lock().await; // Re-lock for instance state, or pass agent_details if it was cloned
        // agent_details is available from the scope above if it was cloned.
        // If not cloned, agent_details needs to be re-fetched or passed from the previous ds_guard scope somehow.
        // For simplicity, assume agent_details (cloned) is available here.

        agent_task_path = agent_details.metadata.get("task_endpoint_path").map(|s| s.to_string()).unwrap_or_else(|| "/default_task".to_string());
        agent_task_port = agent_details.metadata.get("task_endpoint_port").and_then(|s| s.parse::<u16>().ok()).unwrap_or(8000);

        let target_build_id = agent_details.metadata.get("build_id").map(|s| s.to_string()).unwrap_or_else(|| agent_details.agent_id.clone());
        let running_instance = ds.instance_state.map.iter()
            .filter_map(|ctx| { let (_id, reg) = ctx.val; reg.val().map(|v_reg| v_reg.value()) })
            .find(|instance: &Instance| instance.build_id == target_build_id && instance.status == InstanceStatus::Started && instance.formnet_ip.is_some());
        match running_instance {
            Some(instance) => {
                instance_details = instance.clone();
                instance_formnet_ip = instance_details.formnet_ip.unwrap();
                log::info!("Found running instance {} for agent {} at IP {}:{} with path {}", instance_details.instance_id, agent_id, instance_formnet_ip, agent_task_port, agent_task_path);
            },
            None => {
                log::warn!("No running instance found for agent_id: {} (target build_id: {}).", agent_id, target_build_id);
                return Err(ApiError::new("NO_AVAILABLE_INSTANCE", &format!("No available instance for agent '{}'. Please try again later.", agent_id), StatusCode::SERVICE_UNAVAILABLE.as_u16()).with_path(&request_path));
            }
        }
    }

    // --- Sub-task 4.4: Prepare and Proxy Request to Agent via Formnet (copied from previous state for completeness) --- 
    let agent_payload = serde_json::to_value(&run_task_request).unwrap_or_else(|_| json!({ "error": "Failed to serialize original request for agent" }));
    let client = Client::builder()
        .timeout(Duration::from_secs(run_task_request.timeout_seconds().into()))
        .build()
        .unwrap_or_else(|_| Client::new());
    let agent_target_url = format!("http://{}:{}{}", instance_formnet_ip, agent_task_port, agent_task_path);
    log::info!("Proxying request for agent {} to URL: {}", agent_id, agent_target_url);
    let agent_response_result = client.post(&agent_target_url).json(&agent_payload).send().await;

    // Clones for the billing task, to be moved into the spawned task if a successful response is sent.
    let datastore_for_billing = datastore.clone();
    let caller_hex_for_billing = caller_address_hex.clone();
    let agent_id_for_billing = agent_id.clone();
    let agent_details_for_billing = agent_details.clone(); // Assuming agent_details is in scope and cloned
    let usage_info_capture_arc: Arc<Mutex<Option<AgentUsageInfo>>> = Arc::new(Mutex::new(None));
    
    // The match block now directly determines the Result to be returned by the function.
    match agent_response_result {
        Ok(resp) => {
            if resp.status().is_success() {
                if run_task_request.streaming.unwrap_or(true) {
                    log::info!("Agent {} responded with success, preparing to stream response.", agent_id);
                    let usage_info_for_billing_stream_capture = usage_info_capture_arc.clone(); // Renamed for clarity
                    let agent_id_for_stream_log = agent_id.clone();
                    
                    let agent_byte_stream = resp.bytes_stream();
                    let client_sse_stream_unfold = futures::stream::unfold(agent_byte_stream, move |mut stream| {
                        let usage_capture_in_unfold = usage_info_for_billing_stream_capture.clone(); // Use specific clone
                        let agent_id_log_in_unfold = agent_id_for_stream_log.clone();
                        async move {
                            match tokio_stream::StreamExt::next(&mut stream).await {
                                Some(Ok(bytes)) => {
                                    let content_str = String::from_utf8_lossy(&bytes).to_string();
                                    if content_str.starts_with("FINAL_USAGE_INFO:") {
                                        let json_part = content_str.trim_start_matches("FINAL_USAGE_INFO:");
                                        match serde_json::from_str::<AgentUsageInfo>(json_part) {
                                            Ok(usage) => {
                                                log::info!("Captured UsageInfo from agent stream for {}: {:?}", agent_id_log_in_unfold, usage);
                                                let mut usage_guard = usage_capture_in_unfold.lock().await;
                                                *usage_guard = Some(usage);
                                                let event = SseEvent::default().event("final_usage_info_received").data(json_part.to_string());
                                                Some((Ok::<_, Infallible>(event), stream))
                                            }
                                            Err(e) => {
                                                log::error!("Failed to parse UsageInfo JSON from agent stream for {}: {}", agent_id_log_in_unfold, e);
                                                let event = SseEvent::default().event("stream_error").data(format!("Failed to parse final usage info: {}", e));
                                                Some((Ok::<_, Infallible>(event), stream))
                                            }
                                        }
                                    } else {
                                        let event = SseEvent::default().event("message").data(content_str);
                                        Some((Ok::<_, Infallible>(event), stream))
                                    }
                                }
                                Some(Err(e)) => {
                                    log::error!("Error in byte stream from agent {}: {}", agent_id_log_in_unfold, e);
                                    let event = SseEvent::default().event("stream_error").data(format!("Agent stream read error: {}", e));
                                    Some((Ok::<_, Infallible>(event), stream))
                                }
                                None => None,
                            }
                        }
                    });
                    let client_sse_stream_chained = futures::stream::StreamExt::chain(
                        client_sse_stream_unfold, 
                        stream::once(async { Ok::<_, Infallible>(SseEvent::default().event("stream_end").data("Agent stream ended")) })
                    );
                    let sse_response = Sse::new(client_sse_stream_chained).into_response();
                    
                    let usage_info_for_billing_task = usage_info_capture_arc.clone(); 
                    tokio::spawn(async move {
                        tokio::time::sleep(Duration::from_millis(500)).await; 
                        perform_billing(datastore_for_billing, caller_hex_for_billing, agent_id_for_billing, agent_details_for_billing, usage_info_for_billing_task).await;
                    });
                    Ok(sse_response) // Return Ok(AxumResponse)

                } else { // Non-Streaming path
                    match resp.json::<AgentRunTaskResponse>().await { 
                        Ok(mut agent_run_task_response) => {
                            log::info!("Agent {} (non-streaming) responded with: {:?}", agent_id, agent_run_task_response);
                            let usage_opt = agent_run_task_response.usage.clone();
                            agent_run_task_response.agent_id = agent_id.clone(); 
                            agent_run_task_response.task_id = run_task_request.task_id(); 
                            let ok_response = (StatusCode::OK, Json(agent_run_task_response)).into_response();
                            
                            let usage_info_for_billing_task = Arc::new(Mutex::new(usage_opt)); 
                            tokio::spawn(async move {
                                perform_billing(datastore_for_billing, caller_hex_for_billing, agent_id_for_billing, agent_details_for_billing, usage_info_for_billing_task).await;
                            });
                            Ok(ok_response) // Return Ok(AxumResponse)
                        }
                        Err(e) => {
                            log::error!("Failed to parse non-streaming JSON response from agent {}: {}", agent_id, e);
                            Err(ApiError::new("AGENT_RESPONSE_PARSE_ERROR", "Failed to parse response from agent.", StatusCode::BAD_GATEWAY.as_u16()).with_path(&request_path).with_details(json!(e.to_string())))
                        }
                    }
                }
            } else { 
                let err_status = resp.status();
                let err_body = resp.text().await.unwrap_or_else(|e| format!("Failed to read error body from agent: {}",e));
                log::error!("Agent {} at {} responded with error: {} - {}", agent_id, agent_target_url, err_status, err_body);
                Err(ApiError::new("AGENT_PROCESSING_ERROR", "Agent processing failed.", err_status.as_u16()).with_path(&request_path).with_details(json!(err_body)))
            }
        }
        Err(e) => { 
            log::error!("Failed to send request to agent {} at {}: {}", agent_id, agent_target_url, e);
            Err(ApiError::new("AGENT_COMMUNICATION_ERROR", "Failed to communicate with the agent.", StatusCode::BAD_GATEWAY.as_u16()).with_path(&request_path).with_details(json!(e.to_string())))
        }
    }
}

// Helper function for billing to be called from spawned task
async fn perform_billing(
    datastore: Arc<Mutex<DataStore>>,
    caller_address_hex: String,
    agent_id: String,
    agent_details: AIAgent,
    usage_info_arc: Arc<Mutex<Option<AgentUsageInfo>>>
) {
    let captured_usage_option = { 
        let mut usage_guard = usage_info_arc.lock().await;
        usage_guard.take()
    };

    if captured_usage_option.is_none() {
        log::info!("Post-response task: No usage info captured/provided for agent {}, skipping billing.", agent_id);
        return;
    }

    let usage = captured_usage_option.unwrap(); 

    log::info!("Post-response task: Validating UsageInfo for caller: {}, agent: {}, usage: {:?}", 
        caller_address_hex, agent_id, usage);

    // --- Sub-task 5.1.3: Validate received UsageInfo ---
    let mut is_valid_usage = true;
    let mut validation_errors: Vec<String> = Vec::new();

    // Tokens are u32, inherently non-negative. Check for reasonable upper bounds if necessary (omitted for now).
    // duration_ms and billable_duration_ms are u64, inherently non-negative.

    if let Some(cost) = usage.provider_cost {
        if cost < 0.0 {
            validation_errors.push(format!("Provider cost ({}) is negative.", cost));
            is_valid_usage = false;
        }
    }

    // Check for consistency: prompt_tokens + completion_tokens should ideally equal total_tokens.
    // This is a soft check; we'll log a warning but proceed with total_tokens for billing.
    if usage.prompt_tokens.saturating_add(usage.completion_tokens) != usage.total_tokens {
        log::warn!("Post-response task: Token count inconsistency for agent {}. 
                   Prompt: {}, Completion: {}, Reported Total: {}. Using reported total_tokens for billing.",
                   agent_id, usage.prompt_tokens, usage.completion_tokens, usage.total_tokens);
    }
    
    if !is_valid_usage {
        log::error!("Post-response task: Invalid UsageInfo received for agent {}. Errors: {:?}. Skipping billing.", agent_id, validation_errors);
        return;
    }
    // --- End of Validation ---

    if usage.total_tokens == 0 && usage.duration_ms == 0 {
        log::warn!("Post-response task: Usage info with zero tokens and zero duration for agent {}. Skipping billing deduction.", agent_id);
    } else {
        let mut ds_lock = datastore.lock().await;
        if let Some(mut user_account) = ds_lock.account_state.get_account(&caller_address_hex) {
            
            // --- Sub-task 5.2.2: Calculate cost/deduction --- 
            let mut calculated_gross_cost_credits: u64 = 0;

            // 1. Per-call cost (if defined for the agent)
            if let Some(cost_str) = agent_details.metadata.get("cost_per_call") {
                if let Ok(per_call_cost) = cost_str.parse::<u64>() {
                    calculated_gross_cost_credits += per_call_cost;
                    log::info!("Cost component: Per-call cost = {} credits", per_call_cost);
                }
            }

            // 2. Token-based cost
            //    Defaults are applied if specific metadata keys are not found.
            let cost_per_1k_input_tokens: f64 = agent_details.metadata
                .get("cost_per_1k_input_tokens")
                .and_then(|s| s.parse().ok())
                .unwrap_or(1.0); // Default: 1.0 credit per 1k input tokens
            
            let cost_per_1k_output_tokens: f64 = agent_details.metadata
                .get("cost_per_1k_output_tokens")
                .and_then(|s| s.parse().ok())
                .unwrap_or(1.5); // Default: 1.5 credits per 1k output tokens

            let input_token_cost = ((usage.prompt_tokens as f64 / 1000.0) * cost_per_1k_input_tokens).ceil() as u64;
            let output_token_cost = ((usage.completion_tokens as f64 / 1000.0) * cost_per_1k_output_tokens).ceil() as u64;
            calculated_gross_cost_credits += input_token_cost;
            calculated_gross_cost_credits += output_token_cost;
            log::info!("Cost component: Input token cost = {} credits ({} tokens @ {}/1k)", input_token_cost, usage.prompt_tokens, cost_per_1k_input_tokens);
            log::info!("Cost component: Output token cost = {} credits ({} tokens @ {}/1k)", output_token_cost, usage.completion_tokens, cost_per_1k_output_tokens);

            // 3. Duration-based cost (if defined)
            if let Some(cost_str) = agent_details.metadata.get("cost_per_minute") {
                if let Ok(per_minute_cost) = cost_str.parse::<u64>() {
                    if usage.billable_duration_ms > 0 {
                        let minutes_used = (usage.billable_duration_ms as f64 / 60000.0).ceil() as u64;
                        let duration_cost = minutes_used * per_minute_cost;
                        calculated_gross_cost_credits += duration_cost;
                        log::info!("Cost component: Duration cost = {} credits ({} minutes @ {}/min)", duration_cost, minutes_used, per_minute_cost);
                    }
                }
            }
            
            // Ensure a minimum cost if any significant usage occurred and no flat fee covered it
            if (usage.total_tokens > 0 || usage.billable_duration_ms > 1000) && 
               calculated_gross_cost_credits == 0 && 
               agent_details.metadata.get("cost_per_call").is_none() {
                calculated_gross_cost_credits = 1; // Minimum 1 credit for any actual work if not covered by per-call
                log::info!("Applied minimum 1 credit for usage without per-call fee.");
            }
            log::info!("Post-response task: Calculated GROSS cost: {} credits for agent {}", calculated_gross_cost_credits, agent_id);

            // --- Placeholder for Applying Subscription Benefits --- 
            let mut net_cost_credits = calculated_gross_cost_credits;
            if let Some(subscription) = &user_account.subscription {
                if subscription.status == SubscriptionStatus::Active || subscription.status == SubscriptionStatus::Trial || subscription.status == SubscriptionStatus::PastDue {
                    // Example: Apply a flat X% discount for Pro+ tiers on this agent category if defined.
                    // This logic would need to be much more sophisticated, checking agent_details.category vs subscription.tier.benefits etc.
                    // Also consider free inference credits provided by subscription.quota().inference_credits.
                    // For now, simple illustrative discount:
                    let agent_model_tier_category = agent_details.metadata.get("model_tier_category").map_or("basic", |s| s.as_str());
                    if (subscription.tier == SubscriptionTier::ProPlus || subscription.tier == SubscriptionTier::Power || subscription.tier == SubscriptionTier::PowerPlus) && (agent_model_tier_category != "basic") {
                        let discount_percentage = 0.10; // 10% discount for higher tiers on non-basic models
                        let discount_amount = (net_cost_credits as f64 * discount_percentage).floor() as u64;
                        net_cost_credits = net_cost_credits.saturating_sub(discount_amount);
                        if discount_amount > 0 {
                             log::info!("Applied subscription discount of {} credits. Net cost before pay-as-you-go: {}", discount_amount, net_cost_credits);
                        }
                    }
                    // TODO: More detailed logic for using `subscription.inference_credits_per_period` from `quota()` 
                    // against `user_account.usage_tracker()` to see if some/all of `net_cost_credits` can be covered.
                }
            }
            log::info!("Post-response task: Net cost after potential subscription benefits: {} credits for agent {}", net_cost_credits, agent_id);
            // --- End of Subscription Benefits Placeholder --- 

            if user_account.deduct_credits(net_cost_credits) { 
                // user_account.updated_at = Utc::now().timestamp(); // deduct_credits should handle timestamp
                log::info!("Post-response task: Attempting to persist deduction of {} credits from account {}. New balance: {}.", 
                           net_cost_credits, caller_address_hex, user_account.credits);
                let account_op = ds_lock.account_state.update_account_local(user_account.clone());
                if let Err(e) = ds_lock.handle_account_op(account_op).await {
                    log::error!("CRITICAL Post-response task: Failed to persist account update for {}: {}. Billing inconsistent.", caller_address_hex, e);
                } else {
                    log::info!("Post-response task: Successfully billed {} for {} credits.", 
                               caller_address_hex, net_cost_credits);
                }
            } else {
                log::warn!("Post-response task: Insufficient credits for {} for agent {} cost. Required: {}, Available: {}. Billing inconsistent.", 
                           caller_address_hex, agent_id, net_cost_credits, user_account.credits);
            }
        } else {
            log::error!("CRITICAL Post-response task: Account {} not found for billing for agent {}. Billing failed.", caller_address_hex, agent_id);
        }
    }
} 