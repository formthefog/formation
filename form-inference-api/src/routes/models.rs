use axum::{response::IntoResponse, routing::get, Json, Router};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize)]
struct Model {
    model_id: String,
    name: String,
    owner_id: String,
    version: String,
    description: String,
    documentation: Option<String>,
    license: String,
    model_type: String,
    framework: String,
    io_modes: Vec<String>,
    parameters: u64,
    quantization: Option<String>,
    context_length: Option<u32>,
    input_tokens_per_second: Option<u32>,
    output_tokens_per_second: Option<u32>,
    tags: Vec<String>,
    created_at: i64,
    updated_at: i64,
    formfile_template: String,
    weights_url: Option<String>,
    weights_checksum: Option<String>,
    weights_size_bytes: Option<u64>,
    resource_requirements: ResourceRequirements,
    capabilities: Vec<String>,
    average_rating: Option<u32>,
    deployment_count: u64,
    usage_count: u64,
    is_featured: bool,
    is_private: bool,
    metadata: serde_json::Value,
    repository_url: Option<String>,
    demo_url: Option<String>,
    paper_url: Option<String>,
    price_per_1m_tokens: Option<u64>,
    usage_tracking: UsageTracking,
    nodes: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ResourceRequirements {
    min_vcpus: u8,
    recommended_vcpus: u8,
    min_memory_mb: u64,
    recommended_memory_mb: u64,
    min_disk_gb: u64,
    recommended_disk_gb: u64,
    requires_gpu: bool,
    min_vram_gb: Option<u64>,
    recommended_vram_gb: Option<u64>,
    cuda_cores: Option<u32>,
    tensor_cores: Option<u32>,
    required_cpu_extensions: Vec<String>,
    required_cuda_version: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct UsageTracking {
    track_tokens: bool,
    track_requests: bool,
    enable_royalties: bool,
    royalty_percentage: u8,
    custom_metrics: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct FormStateModelsResponse {
    success: bool,
    models: Vec<FormStateModel>,
    total: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct FormStateModel {
    model_id: String,
    name: String,
    owner_id: String,
    version: String,
    description: String,
    documentation: Option<String>,
    license: String,
    model_type: String,
    framework: String,
    io_modes: Vec<String>,
    parameters: u64,
    quantization: Option<String>,
    context_length: Option<u32>,
    input_tokens_per_second: Option<u32>,
    output_tokens_per_second: Option<u32>,
    tags: Vec<String>,
    created_at: i64,
    updated_at: i64,
    formfile_template: String,
    weights_url: Option<String>,
    weights_checksum: Option<String>,
    weights_size_bytes: Option<u64>,
    resource_requirements: FormStateResourceRequirements,
    capabilities: Vec<String>,
    average_rating: Option<u32>,
    deployment_count: u64,
    usage_count: u64,
    is_featured: bool,
    is_private: bool,
    metadata: serde_json::Value,
    repository_url: Option<String>,
    demo_url: Option<String>,
    paper_url: Option<String>,
    price_per_1m_tokens: Option<u64>,
    usage_tracking: FormStateUsageTracking,
    nodes: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct FormStateResourceRequirements {
    min_vcpus: u8,
    recommended_vcpus: u8,
    min_memory_mb: u64,
    recommended_memory_mb: u64,
    min_disk_gb: u64,
    recommended_disk_gb: u64,
    requires_gpu: bool,
    min_vram_gb: Option<u64>,
    recommended_vram_gb: Option<u64>,
    cuda_cores: Option<u32>,
    tensor_cores: Option<u32>,
    required_cpu_extensions: Vec<String>,
    required_cuda_version: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct FormStateUsageTracking {
    track_tokens: bool,
    track_requests: bool,
    enable_royalties: bool,
    royalty_percentage: u8,
    custom_metrics: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ModelsResponse {
    success: bool,
    models: Vec<Model>,
    total: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct ErrorResponse {
    success: bool,
    error: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct SimplifiedModel {
    model_id: String,
    owner_id: String,
    name: String,
    io_modes: Vec<String>,
}

// Create a lazy static client
lazy_static::lazy_static! {
    static ref CLIENT: Client = Client::new();
}

pub fn router() -> Router<Arc<String>> {
    Router::new()
        .route("/models", get(list_models))
        .route("/models/:model_id", get(retrieve_model))
}

// Handler functions with proper form-state integration
async fn list_models() -> impl IntoResponse {
    // Fetch models from form-state API
    match CLIENT.get("http://localhost:3004/v1/models").send().await {
        Ok(response) => {
            if !response.status().is_success() {
                return (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "success": false,
                        "error": format!("Form-state API returned status: {}", response.status())
                    })),
                );
            }

            match response.json::<FormStateModelsResponse>().await {
                Ok(models_response) => {
                    if !models_response.success {
                        return (
                            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                            Json(serde_json::json!({
                                "success": false,
                                "error": "Form-state API returned unsuccessful response"
                            })),
                        );
                    }

                    // Convert form-state models to our format
                    let converted_models: Vec<Model> = models_response
                        .models
                        .into_iter()
                        .map(|form_state_model| Model {
                            model_id: form_state_model.model_id,
                            name: form_state_model.name,
                            owner_id: form_state_model.owner_id,
                            version: form_state_model.version,
                            description: form_state_model.description,
                            documentation: form_state_model.documentation,
                            license: form_state_model.license,
                            model_type: form_state_model.model_type,
                            framework: form_state_model.framework,
                            io_modes: form_state_model.io_modes,
                            parameters: form_state_model.parameters,
                            quantization: form_state_model.quantization,
                            context_length: form_state_model.context_length,
                            input_tokens_per_second: form_state_model.input_tokens_per_second,
                            output_tokens_per_second: form_state_model.output_tokens_per_second,
                            tags: form_state_model.tags,
                            created_at: form_state_model.created_at,
                            updated_at: form_state_model.updated_at,
                            formfile_template: form_state_model.formfile_template,
                            weights_url: form_state_model.weights_url,
                            weights_checksum: form_state_model.weights_checksum,
                            weights_size_bytes: form_state_model.weights_size_bytes,
                            resource_requirements: ResourceRequirements {
                                min_vcpus: form_state_model.resource_requirements.min_vcpus,
                                recommended_vcpus: form_state_model
                                    .resource_requirements
                                    .recommended_vcpus,
                                min_memory_mb: form_state_model.resource_requirements.min_memory_mb,
                                recommended_memory_mb: form_state_model
                                    .resource_requirements
                                    .recommended_memory_mb,
                                min_disk_gb: form_state_model.resource_requirements.min_disk_gb,
                                recommended_disk_gb: form_state_model
                                    .resource_requirements
                                    .recommended_disk_gb,
                                requires_gpu: form_state_model.resource_requirements.requires_gpu,
                                min_vram_gb: form_state_model.resource_requirements.min_vram_gb,
                                recommended_vram_gb: form_state_model
                                    .resource_requirements
                                    .recommended_vram_gb,
                                cuda_cores: form_state_model.resource_requirements.cuda_cores,
                                tensor_cores: form_state_model.resource_requirements.tensor_cores,
                                required_cpu_extensions: form_state_model
                                    .resource_requirements
                                    .required_cpu_extensions,
                                required_cuda_version: form_state_model
                                    .resource_requirements
                                    .required_cuda_version,
                            },
                            capabilities: form_state_model.capabilities,
                            average_rating: form_state_model.average_rating,
                            deployment_count: form_state_model.deployment_count,
                            usage_count: form_state_model.usage_count,
                            is_featured: form_state_model.is_featured,
                            is_private: form_state_model.is_private,
                            metadata: form_state_model.metadata,
                            repository_url: form_state_model.repository_url,
                            demo_url: form_state_model.demo_url,
                            paper_url: form_state_model.paper_url,
                            price_per_1m_tokens: form_state_model.price_per_1m_tokens,
                            usage_tracking: UsageTracking {
                                track_tokens: form_state_model.usage_tracking.track_tokens,
                                track_requests: form_state_model.usage_tracking.track_requests,
                                enable_royalties: form_state_model.usage_tracking.enable_royalties,
                                royalty_percentage: form_state_model
                                    .usage_tracking
                                    .royalty_percentage,
                                custom_metrics: form_state_model.usage_tracking.custom_metrics,
                            },
                            nodes: form_state_model.nodes,
                        })
                        .collect();

                    (
                        axum::http::StatusCode::OK,
                        Json(serde_json::json!({
                            "success": true,
                            "models": converted_models,
                            "total": converted_models.len()
                        })),
                    )
                }
                Err(e) => (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "success": false,
                        "error": format!("Failed to parse form-state response: {}", e)
                    })),
                ),
            }
        }
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "success": false,
                "error": format!("Failed to fetch models from form-state: {}", e)
            })),
        ),
    }
}

async fn retrieve_model(
    axum::extract::Path(model_id): axum::extract::Path<String>,
) -> impl IntoResponse {
    // Fetch specific model from form-state API
    match CLIENT
        .get(format!("http://localhost:3004/v1/models/{model_id}"))
        .send()
        .await
    {
        Ok(response) => {
            if response.status() == reqwest::StatusCode::NOT_FOUND {
                return (
                    axum::http::StatusCode::NOT_FOUND,
                    Json(serde_json::json!({
                        "success": false,
                        "error": format!("Model {} not found", model_id)
                    })),
                );
            }

            if !response.status().is_success() {
                return (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "success": false,
                        "error": format!("Form-state API returned status: {}", response.status())
                    })),
                );
            }

            match response.json::<serde_json::Value>().await {
                Ok(json_response) => {
                    // Extract the model from the response
                    if let Some(model_data) = json_response.get("model") {
                        match serde_json::from_value::<FormStateModel>(model_data.clone()) {
                            Ok(form_state_model) => {
                                let converted_model = Model {
                                    model_id: form_state_model.model_id,
                                    name: form_state_model.name,
                                    owner_id: form_state_model.owner_id,
                                    version: form_state_model.version,
                                    description: form_state_model.description,
                                    documentation: form_state_model.documentation,
                                    license: form_state_model.license,
                                    model_type: form_state_model.model_type,
                                    framework: form_state_model.framework,
                                    io_modes: form_state_model.io_modes,
                                    parameters: form_state_model.parameters,
                                    quantization: form_state_model.quantization,
                                    context_length: form_state_model.context_length,
                                    input_tokens_per_second: form_state_model
                                        .input_tokens_per_second,
                                    output_tokens_per_second: form_state_model
                                        .output_tokens_per_second,
                                    tags: form_state_model.tags,
                                    created_at: form_state_model.created_at,
                                    updated_at: form_state_model.updated_at,
                                    formfile_template: form_state_model.formfile_template,
                                    weights_url: form_state_model.weights_url,
                                    weights_checksum: form_state_model.weights_checksum,
                                    weights_size_bytes: form_state_model.weights_size_bytes,
                                    resource_requirements: ResourceRequirements {
                                        min_vcpus: form_state_model.resource_requirements.min_vcpus,
                                        recommended_vcpus: form_state_model
                                            .resource_requirements
                                            .recommended_vcpus,
                                        min_memory_mb: form_state_model
                                            .resource_requirements
                                            .min_memory_mb,
                                        recommended_memory_mb: form_state_model
                                            .resource_requirements
                                            .recommended_memory_mb,
                                        min_disk_gb: form_state_model
                                            .resource_requirements
                                            .min_disk_gb,
                                        recommended_disk_gb: form_state_model
                                            .resource_requirements
                                            .recommended_disk_gb,
                                        requires_gpu: form_state_model
                                            .resource_requirements
                                            .requires_gpu,
                                        min_vram_gb: form_state_model
                                            .resource_requirements
                                            .min_vram_gb,
                                        recommended_vram_gb: form_state_model
                                            .resource_requirements
                                            .recommended_vram_gb,
                                        cuda_cores: form_state_model
                                            .resource_requirements
                                            .cuda_cores,
                                        tensor_cores: form_state_model
                                            .resource_requirements
                                            .tensor_cores,
                                        required_cpu_extensions: form_state_model
                                            .resource_requirements
                                            .required_cpu_extensions,
                                        required_cuda_version: form_state_model
                                            .resource_requirements
                                            .required_cuda_version,
                                    },
                                    capabilities: form_state_model.capabilities,
                                    average_rating: form_state_model.average_rating,
                                    deployment_count: form_state_model.deployment_count,
                                    usage_count: form_state_model.usage_count,
                                    is_featured: form_state_model.is_featured,
                                    is_private: form_state_model.is_private,
                                    metadata: form_state_model.metadata,
                                    repository_url: form_state_model.repository_url,
                                    demo_url: form_state_model.demo_url,
                                    paper_url: form_state_model.paper_url,
                                    price_per_1m_tokens: form_state_model.price_per_1m_tokens,
                                    usage_tracking: UsageTracking {
                                        track_tokens: form_state_model.usage_tracking.track_tokens,
                                        track_requests: form_state_model
                                            .usage_tracking
                                            .track_requests,
                                        enable_royalties: form_state_model
                                            .usage_tracking
                                            .enable_royalties,
                                        royalty_percentage: form_state_model
                                            .usage_tracking
                                            .royalty_percentage,
                                        custom_metrics: form_state_model
                                            .usage_tracking
                                            .custom_metrics,
                                    },
                                    nodes: form_state_model.nodes,
                                };

                                (
                                    axum::http::StatusCode::OK,
                                    Json(serde_json::json!(converted_model)),
                                )
                            }
                            Err(e) => (
                                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                                Json(serde_json::json!({
                                    "success": false,
                                    "error": format!("Failed to parse model data: {}", e)
                                })),
                            ),
                        }
                    } else {
                        (
                            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                            Json(serde_json::json!({
                                "success": false,
                                "error": "Invalid response format from form-state"
                            })),
                        )
                    }
                }
                Err(e) => (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "success": false,
                        "error": format!("Failed to parse form-state response: {}", e)
                    })),
                ),
            }
        }
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "success": false,
                "error": format!("Failed to fetch model from form-state: {}", e)
            })),
        ),
    }
}
