# Agent Gateway Implementation Plan for form-state

This document outlines the plan to implement an API gateway within the `form-state` service for interacting with deployed agents. This gateway will handle authentication, authorization, billing, and proxying requests to agents running on the Formnet.

## I. Update Request/Response Structures (`form-state/src/helpers/`)

- [x] **Task 1: Review and Finalize `agent_request.rs` (`RunTaskRequest`).**
    - [x] **Sub-task 1.1:** Confirm `RunTaskRequest` includes all fields for `form-state` (routing, billing pre-checks, transformations) and for agent execution (prompt, model config, etc.).
    - [x] **Sub-task 1.2:** Confirm agent payload structure expected by typical agent applications.
    - [x] **Sub-task 1.3:** Verify client-to-form-state auth uses ECDSA headers (`X-Message` hash, `X-Signature`, `X-Recovery-Id`).
    - [x] **Sub-task 1.4:** Remove client-side auth token fields (e.g., `jwt`, `formation_api_key`) from `RunTaskRequest` if superseded by header auth. (Agent's own `provider_api_key` for external LLMs remains).

- [x] **Task 2: Review and Finalize `agent_response.rs` (`RunTaskResponse`, `TaskStreamChunk`).**
    - [x] **Sub-task 2.1:** Ensure `RunTaskResponse` (non-streaming) structure is adequate for full agent output, errors, and final `UsageInfo`.
    - [x] **Sub-task 2.2:** Ensure `TaskStreamChunk` (streaming) handles intermediate content, errors, and final `UsageInfo` (in the last chunk).
    - [x] **Sub-task 2.3:** Confirm `UsageInfo` captures all necessary billing metrics (tokens, duration, costs) and refined by removing `formation_cost`.

## II. Implement `form-state` API Endpoint (`/agents/:agent_id/run_task`)

- [x] **Task 3: Define the new API Route in `form-state/src/api.rs`.**
    - [x] **Sub-task 3.1:** Add `POST /agents/:agent_id/run_task` to the router (e.g., in `api_routes`).
    - [x] **Sub-task 3.2:** Define if `:agent_id` is `buildId` or a separate registered agent ID. Document this. (Using path param `:agent_id`, interpretation by handler).
    - [x] **Sub-task 3.3:** Ensure `ecdsa_auth_middleware` is applied to this route.

- [x] **Task 4: Implement the `run_task` Handler Function (e.g., in `form-state/src/helpers/agent_gateway.rs`).** (Core structure implemented; internal logic for eligibility rules, cost calculation, and robust streaming UsageInfo protocol are TBD placeholders).
    - [x] **Sub-task 4.1: Handler Signature and Initial Setup.** (File created, handler signature defined, wired up).
    - [x] **Sub-task 4.2: Authorization & Billing Pre-Check.** (Account fetching, policy for non-existent account, and placeholder eligibility check structure corrected after linter fixes. Actual eligibility rules TBD).
        - [x] **4.2.1:** Fetch caller's account from `AccountState` via `recovered_address.as_hex()`. Handle missing account.
        - [ ] **4.2.2:** Implement basic eligibility check (credits/subscription status via `AccountState` and new `BillingState`/rules) (Structure for check corrected, placeholder logic for rules remains. Actual rules TBD).
        - [x] **4.2.3:** Return JSON error (HTTP 402/403) if pre-check fails.
    - [x] **Sub-task 4.3: Agent and Instance Lookup.** (Logic for querying `AgentState` and `InstanceState` with placeholder metadata access implemented, import and linter fixes done).
    - [x] **Sub-task 4.4: Prepare and Proxy Request to Agent via Formnet.** (Request construction, `reqwest::Client` usage, and call to agent implemented).
    - [x] **Sub-task 4.5: Handle Agent Response (Streaming and Non-Streaming).**
        - [x] **4.5.1 (Non-Streaming Path):** (Basic implementation done, parses agent response into `AgentRunTaskResponse`, captures `usage_info_for_billing`).
        - [x] **4.5.2 (Streaming Path):** (Basic SSE piping structure corrected after linter fixes. `UsageInfo` capture via `Arc<Mutex<...>>` and placeholder protocol `FINAL_USAGE_INFO:` added. Actual agent streaming protocol for robust `UsageInfo` TBD).
    - [x] **Sub-task 4.6: Comprehensive Error Handling in Handler.** (Handler refactored to return `Result<AxumResponse, ApiError>` and use `ApiError` for error responses, including recent fix for return structure).

## III. Implement Post-Task Processing (within `run_task` handler or related functions)

- [ ] **Task 5: Usage Extraction and Billing Update.** (Overall task still in progress due to 4.2.2 and 5.2.2 actual rules/logic, and streaming UsageInfo protocol).
    - [x] **Sub-task 5.1: Extract/Receive `UsageInfo` from Agent.** (Marking parent as [x] as sub-components cover the extraction part structurally)
        - [x] **5.1.1 (Non-Streaming):** `usage_info_for_billing` Arc is populated from agent response.
        - [x] **5.1.2 (Streaming):** `usage_info_for_billing` Arc is populated from agent stream via placeholder `FINAL_USAGE_INFO:` protocol. Actual agent streaming protocol for robust `UsageInfo` TBD.
        - [x] **5.1.3:** Validate received `UsageInfo`. Define policy for missing/invalid data (Basic validation for provider_cost, token consistency warning, and handling for zero usage implemented in `perform_billing`. Policy for other invalid data can be enhanced).
    - [ ] **Sub-task 5.2: Perform Billing Update in `AccountState`.** (Billing performed in spawned task `perform_billing`. Actual cost calculation and subscription benefit logic is placeholder/illustrative, hence this sub-task is not fully complete, nor is Task 5).
        - [x] **5.2.1:** Acquire lock on user's account in `AccountState` (Done in `perform_billing`).
        - [ ] **5.2.2:** Calculate cost/deduction based on `UsageInfo` and billing rules (More detailed placeholder calculation added, using agent metadata for pricing and illustrative subscription discount. Actual rules & full subscription benefit logic TBD).
        - [x] **5.2.3:** Atomically update user's credit balance/usage counters (Structure for this is in place in `perform_billing` using `deduct_credits`).
        - [x] **5.2.4:** Log billing transaction for audit (user ID, agent ID, usage, cost, timestamp) (Basic logging added in `perform_billing`).

## IV. Supporting Components and Considerations

- [ ] **Task 6: Agent and Instance Metadata for Gateway Routing.**
    - [ ] **Sub-task 6.1:** Define storage for agent's internal API endpoint (path, port, method) in `AgentState` or `InstanceState` (Currently uses placeholder `agent_details.metadata.get("task_endpoint_path")`).
    - [ ] **Sub-task 6.2:** Define storage/access for agent billing properties (cost per token/call, tiers) (Currently uses placeholder `agent_details.metadata.get("cost_per_call")` etc.).
- [ ] **Task 7: `form-state` Configuration.**
    - [ ] **Sub-task 7.1:** Configuration for `form-state`'s Formnet identity if needed for outbound calls.
    - [ ] **Sub-task 7.2:** Configurable default timeouts for proxied agent calls.
    - [ ] **Sub-task 7.3:** System for configuring/managing billing rates and rules.
- [ ] **Task 8: Robust Testing Strategy.**
    - [ ] **Sub-task 8.1:** Unit tests for new helpers (billing calculation, request transformation).
    - [ ] **Sub-task 8.2:** Integration tests for `/agents/:agent_id/run_task` endpoint, mocking `DataStore` and `reqwest::Client` calls to agent.
    - [ ] **Sub-task 8.3 (Optional):** Plan for end-to-end tests with `form-state` calling a simple, deployed agent.
- [ ] **Task 9: Documentation Update.**
    - [ ] **Sub-task 9.1:** Draft OpenAPI/Swagger specs for `/agents/:agent_id/run_task`.
    - [ ] **Sub-task 9.2:** Update `README.md` and dev guides for the new agent interaction endpoint.
    - [ ] **Sub-task 9.3:** Document requirements for agent apps (task endpoint exposure, `UsageInfo` reporting). 