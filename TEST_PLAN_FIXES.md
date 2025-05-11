# Integration Test Fixes & Enhanced Logging Plan

This document outlines the tasks for creating an integration test to verify the delegated authentication mechanism between `form-state` and `form-pack-manager`, including a full image build process.

## Phase 1: Refine `form-state` Helper Authorizaton Logic

**Goal:** Ensure `form-state` API handlers consistently support localhost privileged access when an end-user signature isn't (or shouldn't be) present for internal service-to-service calls.

-   [x] **Task 1.1: Modify `form-state/src/helpers/account.rs` - `list_accounts`**
    -   [x] Sub-task 1.1.1: Change signature from `recovered: RecoveredAddress` to `recovered: Option<RecoveredAddress>`.
    -   [x] Sub-task 1.1.2: Adjust authorization logic:
        - If `is_localhost` (from `ConnectInfo`), allow listing all accounts. (Corrected: `recovered` being `None` for localhost is implicitly handled by overall logic now).
        - If not `is_localhost`, `recovered` must be `Some`.
        - If `recovered` is `Some`, check if the authenticated address is an admin. If admin, list all.
        - Otherwise (authenticated non-admin, non-localhost), list only the user's own account.
        - If not `is_localhost` and `recovered` is `None`, return 403 Forbidden.
        - Corrected iteration over `crdts::Map` using `read_ctx.val` and then `bft_register.val()`.
-   [x] **Task 1.2: Modify `form-state/src/helpers/agent.rs` - `get_agent`**
    -   [x] Sub-task 1.2.1: Change signature from `recovered: RecoveredAddress` to `recovered: Option<RecoveredAddress>`.
    -   [x] Sub-task 1.2.2: Adjust authorization logic:
        - If `is_localhost` and `recovered` is `None`, allow access to any agent (even private).
        - If `recovered` is `Some`, apply existing logic (owner or admin for private agents).
        - If not `is_localhost` and `recovered` is `None`, deny access to private agents (public still okay).
-   [x] **Task 1.3: Modify `form-state/src/helpers/agent.rs` - `list_agents`**
    -   [x] Sub-task 1.3.1: Change signature from `recovered: RecoveredAddress` to `recovered: Option<RecoveredAddress>`.
    -   [x] Sub-task 1.3.2: Adjust authorization logic:
        - If `is_localhost` and `recovered` is `None`, list all agents (including private).
        - If `recovered` is `Some`, apply existing logic (filter private unless owner/admin).
        - If not `is_localhost` and `recovered` is `None`, list only public agents.
-   [x] **Task 1.4: Modify `form-state/src/helpers/instances.rs` - `update_instance`**
    -   [x] Sub-task 1.4.1: Change signature from `recovered: RecoveredAddress` to `recovered: Option<RecoveredAddress>`.
    -   [x] Sub-task 1.4.2: Adjust authorization logic:
        - If `is_localhost`, allow update (trusting `payload.instance_owner`).
        - Else (not localhost), `recovered` must be `Some`. Check if authenticated user owns the instance or is admin.
        - Prevent `instance_owner` field change unless caller is localhost or admin.
-   [x] **Task 1.5: Modify `form-state/src/helpers/instances.rs` - `get_instance`**
    -   [x] Sub-task 1.5.1: Change signature from `recovered: RecoveredAddress` to `recovered: Option<RecoveredAddress>`.
    -   [x] Sub-task 1.5.2: Adjust authorization logic:
        - If `is_localhost` and `recovered` is `None`, allow access to any instance.
        - If `recovered` is `Some`, apply existing logic (owner/authorized/admin).
        - If not `is_localhost` and `recovered` is `None`, return 403/404 as appropriate.

## Phase 2: Enhance Logging in `form-pack` for Calls to `form-state`

**Goal:** Get clear visibility into the requests `form-pack` makes to `form-state` during the build status update process, and the responses it receives. This will primarily involve `form-pack/src/helpers/api/write.rs`.

-   [x] **Task 2.1: Define `FORM_STATE_URL` in `form-pack/src/helpers/api/write.rs`**
    -   [x] Sub-task 2.1.1: Add `const FORM_STATE_URL: &str = "http://127.0.0.1:3004";` (or retrieve from env/config if that's the pattern in `form-pack`).
-   [ ] **Task 2.2: Add Logging to `write_pack_status_started` (and similar for `_completed`, `_failed`)**
    *Within each function in `form-pack/src/helpers/api/write.rs` that calls `form-state` (e.g., for `/instance/create`, `/agent/create`, `/account/update`, `/instance/update`, `/agents/update`):*
    -   [x] **Sub-task 2.2.1 (for `write_pack_status_started`):** Before sending a request to `form-state`:
        - Log (`info!`) the full URL being called.
        - Log (`info!`) the full JSON payload being sent (use `{:?}` or `serde_json::to_string_pretty`).
    -   [x] **Sub-task 2.2.2 (for `write_pack_status_started`):** After receiving a response from `form-state`:
        - Log (`info!`) the HTTP status code.
        - Read the response body as text.
        - Log (`info!`) the raw response body text.
        - If the status is not success, log this as an `error!`.
        - If attempting to parse the text as JSON and it fails, log an `error!` including the raw text.
    -   [x] **Sub-task 2.2.3 (for `write_pack_status_started`):** Specifically for `/account/update` calls:
        - Ensure the payload being sent is correctly wrapped, e.g., `json!({ "Update": account_object })` to match `AccountRequest::Update(Account)`.
    -   [x] **Sub-task 2.2.4 (for `write_pack_status_started`):** Ensure all relevant `log` macros (`info!`, `warn!`, `error!`) and `SystemTime`, `UNIX_EPOCH` are imported if used for timestamps.
    -   [ ] **Sub-task 2.2.5:** Apply similar logging (2.2.1-2.2.4) to `write_pack_status_completed`.
    -   [ ] **Sub-task 2.2.6:** Apply similar logging (2.2.1-2.2.4) to `write_pack_status_failed`.

## Phase 3: Rebuild, Test, and Iterate

-   [ ] **Task 3.1: Code Implementation**
    -   [ ] Sub-task 3.1.1: Implement all changes from Phase 1 and Phase 2.
-   [ ] **Task 3.2: Build & Deploy**
    -   [ ] Sub-task 3.2.1: Rebuild `form-state` crate and its Docker image.
    -   [ ] Sub-task 3.2.2: Rebuild `form-pack` crate and its Docker image (for `form-pack-manager`).
    -   [ ] Sub-task 3.2.3: Restart Docker Compose services.
-   [ ] **Task 3.3: Run Integration Test**
    -   [ ] Sub-task 3.3.1: Execute `cargo test --test delegated_auth_and_build_flow -- --nocapture`.
-   [ ] **Task 3.4: Analyze Results**
    -   [ ] Sub-task 3.4.1: Examine test output.
    -   [ ] Sub-task 3.4.2: Examine `form-pack-manager` logs for detailed traces of calls to `form-state`.
    -   [ ] Sub-task 3.4.3: Examine `form-state` logs to see how it received and processed calls from `form-pack-manager`.
    -   [ ] Sub-task 3.4.4: Identify the point of failure if the test does not pass.
-   [ ] **Task 3.5: Iterate**
    -   [ ] Sub-task 3.5.1: Based on analysis, make further targeted code corrections or logging additions.
    -   [ ] Sub-task 3.5.2: Repeat from Task 3.2.

---
This plan should give us a clear path. We'll address the `form-state` handler signatures first to ensure they are compatible with `localhost` calls that might not carry an end-user's signature, then we'll add the deep logging into `form-pack` to trace exactly what it's doing when it tries to communicate back to `form-state`. 