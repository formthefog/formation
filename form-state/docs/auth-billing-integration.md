Axum API Integration Guide: Dynamic Auth JWT, Role-Based Access, and Stripe Billing

In this guide, we will walk through implementing authentication, authorization, and billing in a Rust Axum backend for an AI Agent and model marketplace. We cover:

Validating Dynamic Auth JWTs with JWKS

Enforcing project-level and role-based access control

Integrating Stripe for subscriptions and usage-based billing

1. Validating Dynamic Auth JWTs with JWKs
Dynamic Auth issues JSON Web Tokens (JWTs) signed using RS256. They expose a public JWKS endpoint that your API will use to verify tokens. The typical JWKS URL format is:

plaintext
Copy
https://app.dynamic.xyz/api/v0/sdk/<YOUR_DYNAMIC_ENV_ID>/.well-known/jwks
The JWT header contains a key ID (kid) to help select the correct key from the JWKS. We recommend using a library (such as jwt-authorizer or axum-jwks) which caches keys and handles JWK refresh automatically.

Example: Setting Up JWT Verification in Axum
rust
Copy
use axum::{Router, routing::get};
use jwt_authorizer::{JwtAuthorizer, JwtClaims, Authorizer, AuthError};
use serde::Deserialize;

// Define your custom JWT claims (include project and role claims)
#[derive(Debug, Deserialize)]
struct MyClaims {
    sub: String,     // User ID or wallet address
    exp: usize,      // Expiration timestamp
    project: String, // Custom claim: project ID
    role: String     // Custom claim: user role
}

#[tokio::main]
async fn main() {
    // JWKS URL for Dynamic Auth (replace <ENV_ID> with your environment ID)
    let jwks_url = std::env::var("DYNAMIC_JWKS_URL")
        .unwrap_or("https://app.dynamic.xyz/api/v0/sdk/<ENV_ID>/.well-known/jwks".into());

    // Build the JWT authorizer from the JWKS URL
    let auth_layer: Authorizer = JwtAuthorizer::from_jwks_url(jwks_url)
        .build().await.expect("Failed to load JWKS");
    
    // Define protected routes with the auth layer applied
    let protected_api = Router::new()
        .route("/protected/hello", get(protected_handler))
        .layer(auth_layer.into_layer());

    // Mount routes into the Axum app, including public endpoints
    let app = Router::new()
        .merge(protected_api)
        .route("/public/ping", get(public_handler));

    // Run the server...
}

// Example protected handler that extracts validated JWT claims
async fn protected_handler(
    JwtClaims(claims): JwtClaims<MyClaims>
) -> Result<String, AuthError> {
    Ok(format!("Hello, {}! Project={} Role={}", claims.sub, claims.project, claims.role))
}

// Public handler example
async fn public_handler() -> String {
    "pong".to_string()
}
Session Handling:
Since JWTs are stateless, each request includes the token that is validated on each call. Ensure you check the standard claims (like exp) to avoid processing expired tokens.

Note: If the Dynamic JWT does not include project or role claims by default, consider mapping the user ID from the token to additional details from your database.

2. Role and Project-Based Access Control (RBAC)
After authenticating, the next step is to enforce authorization based on the user's role and project. There are multiple approaches:

Using Extractors: Create custom extractors (implementing FromRequestParts) that automatically check for the required roles or project IDs.

Using Middleware: Write middleware to intercept requests and verify that the authenticated user has permissions based on claims.

Enforcing Project Scoping
If your endpoints include a project identifier (e.g., /projects/{project_id}/agents/...), verify that the JWT’s project claim matches the URL parameter:

rust
Copy
use axum::{extract::Path, http::StatusCode};
use axum::response::IntoResponse;

async fn get_agent(
    JwtClaims(claims): JwtClaims<MyClaims>,
    Path((project_id, agent_id)): Path<(String, String)>
) -> impl IntoResponse {
    // Check if the token's project matches the request's project_id
    if claims.project != project_id {
        return StatusCode::FORBIDDEN;
    }
    // Proceed to fetch and return the agent data
    // ...
}
Enforcing Role-Based Access
You can enforce roles by checking within your handler or through custom extractors. For instance, a custom extractor for admin-only routes:

rust
Copy
use axum::{async_trait, extract::FromRequestParts, http::{request::Parts, StatusCode}};

struct AdminClaims(MyClaims);

#[async_trait]
impl<S> FromRequestParts<S> for AdminClaims 
where 
    S: Send + Sync 
{
    type Rejection = StatusCode;
    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // Extract existing JwtClaims (requires the auth layer to be applied)
        let claims = JwtClaims::<MyClaims>::from_request_parts(parts, state)
            .await.map_err(|_| StatusCode::UNAUTHORIZED)?;
        // Verify admin role
        if claims.0.role != "admin" {
            return Err(StatusCode::FORBIDDEN);
        }
        Ok(AdminClaims(claims.0))
    }
}

// Handler that requires an admin user
async fn admin_only_endpoint(AdminClaims(admin_claims): AdminClaims) -> impl IntoResponse {
    format!("Hello Admin user {} on project {}", admin_claims.sub, admin_claims.project)
}
Using extractors keeps your handlers clean while ensuring that only users with the proper role can reach sensitive endpoints.

3. Integrating Stripe Billing
Your billing model has two components:

Subscriptions: Fixed monthly charges with possible quotas.

Usage-based Billing: Metered billing (e.g., per agent call or per million tokens used).

Stripe Setup
Use Stripe’s official Rust SDK (or an async alternative such as async-stripe) to integrate billing into your API.

Creating Customers & Subscriptions:
Each project (or user) should be associated with a Stripe Customer and a Subscription. When a new project is created, your backend should:

Create a Stripe Customer.

Create a Subscription for the selected plan.

Store the returned customer_id and subscription_id in your datastore.

Usage-based Billing with Stripe:
For metered billing, create a metered price in Stripe and attach it to the subscription. Then, report usage by creating usage records:

rust
Copy
use stripe::{UsageRecord, CreateUsageRecord, UsageRecordAction};

async fn report_usage(stripe_client: &stripe::Client, subscription_item_id: &str, quantity: u64) -> stripe::StripeResult<UsageRecord> {
    let params = CreateUsageRecord {
        quantity,
        timestamp: Some(crate::util::current_timestamp()),  // Current timestamp
        action: Some(UsageRecordAction::Increment),           // Increment usage
        ..Default::default()
    };
    // Posts to /v1/subscription_items/{id}/usage_records
    UsageRecord::create(stripe_client, subscription_item_id, params).await
}
Call this function after completing an AI agent operation, using quantity equal to the number of tokens consumed or simply 1 per call.

Handling Subscriptions:
Use Stripe webhooks (e.g., invoice.payment_failed, customer.subscription.deleted) to keep your system’s billing status up to date. Update your internal records accordingly when payment events occur.

Usage Tracking and Enforcement
Integrate usage tracking to enforce quotas and monitor usage:

Internal Usage Counters:
Maintain counters (perhaps using your CRDT key-value store) for each project’s usage. Increment the counter each time an agent call is made.

Quota Checks:
Before processing a request, check if the project’s usage exceeds its plan’s quota. If so, block the request or return a clear error such as HTTP 402 Payment Required.

Reporting to Stripe:
For metered subscriptions, use the Stripe API to report the usage (either immediately per request or batched periodically).

Example: Enforcing Quotas and Reporting Usage
rust
Copy
async fn call_agent(
    JwtClaims(claims): JwtClaims<MyClaims>,
    State(app_state): State<AppState>,       // Contains DB and Stripe client
    Json(request): Json<AgentRequest>
) -> Result<Json<AgentResponse>, ApiError> {
    // 1. Authorization: Ensure the user has permission to call this agent
    let project_id = &claims.project;
    let user_role = &claims.role;
    if user_role != "admin" && user_role != "developer" {
        return Err(ApiError::Forbidden("Role not allowed to call agent"));
    }

    // 2. Check plan quota before processing the request
    let plan = app_state.db.get_project_plan(project_id)?;
    let usage = app_state.db.get_project_usage(project_id)?;
    if let Some(quota) = plan.quota {
        if usage.tokens_this_period >= quota {
            return Err(ApiError::PaymentRequired("Quota exceeded, please upgrade plan"));
        }
    }

    // 3. Perform the agent call (this will consume tokens)
    let result = ai::run_agent(request).await?;
    let tokens_used = result.tokens_used;
    
    // 4. Update internal usage counters
    app_state.db.increment_usage(project_id, tokens_used)?;
    
    // 5. Report usage to Stripe for metered billing
    if plan.metered {
        if let Some(subscription_item) = plan.stripe_subscription_item.as_deref() {
            let stripe = app_state.stripe_client.clone();
            tokio::spawn(async move {
                if let Err(e) = report_usage(&stripe, subscription_item, tokens_used).await {
                    tracing::error!("Failed to report usage to Stripe: {:?}", e);
                }
            });
        }
    }
    
    // 6. Return the AI agent's response
    Ok(Json(result))
}
In this example, steps include:

Checking that the caller’s role is permitted.

Verifying that the project has not exceeded its quota.

Updating internal usage stats.

Offloading the Stripe usage report to a background task.

4. Best Practices for Secure & Efficient Integration
Centralize Authentication
Use middleware/extractors to validate JWTs before any business logic runs.

Prefer libraries that handle JWKS caching and token validation.

Utilize Axum’s Shared State
Define an AppState struct to share resources (Stripe client, DB connection, JWKS cache):

rust
Copy
#[derive(Clone)]
struct AppState {
    stripe_client: stripe::Client,
    jwks: axum_jwks::Jwks,    // if using axum_jwks
    db: DatabaseConnection,   // your data store
}
Layer Ordering
Authentication Layer: Validate JWTs.

Authorization/RBAC: Check for correct role and project.

Request Handling: Process the request.

Post-Processing: Record usage, perform billing tasks.

Security Measures
Validate standard claims (exp, iss, aud).

Prevent algorithm downgrade attacks.

Use reasonable cache TTLs to manage JWKS rotation.

Secure Stripe keys and validate webhook signatures.

Efficiency & Concurrency
Reuse the Stripe client.

Consider batching usage reports if real-time reporting isn’t critical.

Leverage CRDTs to manage distributed usage counters if needed.

Test each component separately and in integration.

Conclusion
By following the above architecture, you ensure that:

Authentication is performed using Dynamic Auth JWTs verified against a JWKS endpoint.

Authorization is enforced by checking roles and project context—either with custom extractors or middleware.

Billing is integrated through Stripe to handle both subscriptions and usage-based charges, with usage tracking implemented in your core logic.

This approach provides a secure, scalable, and maintainable integration for your AI Agent and model marketplace built on Axum.
