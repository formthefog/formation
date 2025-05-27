# Planned Tasks Extract - Sub-sub-task 1.1.1.3

**Status:** ✅ COMPLETED  
**Date:** Formation Project Plan Phase 1.1.1.3  
**Purpose:** Extract all planned but not started tasks (marked with [ ]) from `implementation_plan.md` for roadmap creation.

---

## Phase 1: Testing Configurable Subnet (Manual Multi-Machine Setup)

### 📋 Task 1.3: Testing Configurable Subnet - **NOT STARTED**
**Goal:** Verify that the configurable `formnet_cidr` works correctly in a distributed scenario, allowing nodes to initialize and join a network with a custom CIDR.

**Dependencies:** ✅ All infrastructure completed (Task 1.1 and 1.2)  
**Total Sub-sub-tasks:** 13

- [ ] **Sub-task 1.3.1:** Prepare Test Environment & Build Artifacts
    - [ ] **Sub-sub-task 1.3.1.1:** Set up two machines (`Node-A`, `Node-B`) with OS and dependencies (Rust, Cargo, WireGuard tools)
    - [ ] **Sub-sub-task 1.3.1.2:** Ensure network connectivity between `Node-A` and `Node-B` (ports 51820, 3004)
    - [ ] **Sub-sub-task 1.3.1.3:** Build latest versions of `form-config-wizard`, `form-state`, `formnet`, `formnet-server`
    - [ ] **Sub-sub-task 1.3.1.4:** Distribute necessary binaries to both `Node-A` and `Node-B`

- [ ] **Sub-task 1.3.2:** Test Single Node Initialization with Custom CIDR (`Node-A`)
    - [ ] **Sub-sub-task 1.3.2.1:** Run `form-config-wizard` with custom CIDR (e.g., "192.168.70.0/24"), complete prompts, save config
    - [ ] **Sub-sub-task 1.3.2.2:** Start `form-state` datastore service, verify no errors
    - [ ] **Sub-sub-task 1.3.2.3:** Initialize and start `formnet` network service in bootstrap mode, verify IP assignment, config files, API responses

- [ ] **Sub-task 1.3.3:** Test Second Node Joining the Custom CIDR Network (`Node-B`)
    - [ ] **Sub-sub-task 1.3.3.1:** Run `form-config-wizard` with `Node-A` as bootstrap, save config
    - [ ] **Sub-sub-task 1.3.3.2:** Start `form-state` service and `formnet` network service
    - [ ] **Sub-sub-task 1.3.3.3:** Verify connectivity, IP assignment, WireGuard peer status, gossip synchronization

- [ ] **Sub-task 1.3.4:** Test with Different Valid CIDR Values
    - [ ] **Sub-sub-task 1.3.4.1:** Repeat Sub-tasks 1.3.2 and 1.3.3 using different valid CIDR (e.g., "10.100.0.0/16", "192.168.200.0/28")

- [ ] **Sub-task 1.3.5:** Test Behavior with Invalid CIDR in Config
    - [ ] **Sub-sub-task 1.3.5.1:** Use `form-config-wizard` to set invalid `formnet_cidr` (e.g., "not-a-cidr", "192.168.1.300/24")
    - [ ] **Sub-sub-task 1.3.5.2:** Attempt to initialize `formnet`, verify graceful failure with clear error message

---

## Phase 2: Testing Node Registration (Manual Multi-Machine Setup)

### 📋 Task 2.4: Testing Node Registration - **NOT STARTED**
**Goal:** Verify that `CrdtPeer` and `Node` objects are correctly registered in `form-state` and their state is properly propagated.

**Dependencies:** ✅ All infrastructure completed (Task 2.1, 2.2, 2.3) + Task 1.3 setup  
**Total Sub-sub-tasks:** 9

- [ ] **Sub-task 2.4.1:** Test Initial `CrdtPeer` and `Node` Registration (`Node-B`)
    - [ ] **Sub-sub-task 2.4.1.1:** `Node-B` joins `formnet` hosted by `Node-A`, verify `CrdtPeer` for `Node-B` exists and is correct
    - [ ] **Sub-sub-task 2.4.1.2:** `Node-B` registers its `Node` object with `form-state`, verify object exists with recent timestamps

- [ ] **Sub-task 2.4.2:** Test Node Heartbeat Updates (`Node-B`)
    - [ ] **Sub-sub-task 2.4.2.1:** Allow `Node-B` to run with active heartbeat mechanism
    - [ ] **Sub-sub-task 2.4.2.2:** Query `form-state` for `Node-B`'s `Node` object, verify `last_heartbeat` and `updated_at` are updated
    - [ ] **Sub-sub-task 2.4.2.3:** Stop heartbeat sender on `Node-B`, wait, verify `last_heartbeat` is not updated

- [ ] **Sub-task 2.4.3:** Test Node Metrics/Capability Updates (Conceptual)
    - [ ] **Sub-sub-task 2.4.3.1:** Trigger a metrics/capabilities update from `Node-B`
    - [ ] **Sub-sub-task 2.4.3.2:** Query `form-state` for `Node-B`'s `Node` object, verify `metrics`/`capabilities` field changes and new `updated_at`

- [ ] **Sub-task 2.4.4:** Test Graceful Node Leave (`Node-B`)
    - [ ] **Sub-sub-task 2.4.4.1:** `Node-B` executes "leave network" command, trigger API calls for `Node`/`CrdtPeer` removal
    - [ ] **Sub-sub-task 2.4.4.2:** Verify on `Node-A`: `Node-B`'s objects removed/updated, no longer active WireGuard peer

---

## Phase 3: Testing Configurable Admin Account (Manual Multi-Machine Setup)

### 📋 Task 3.3: Testing Configurable Admin Account - **NOT STARTED**
**Goal:** Verify that configurable admin account (`initial_admin_public_key` and `Account.is_global_admin`) correctly grants authorization for admin-only API endpoints in `form-state`.

**Dependencies:** ✅ Task 3.1 completed, 🚧 Task 3.2 needs completion  
**Total Sub-sub-tasks:** 16

- [ ] **Sub-task 3.3.1:** Configure and Initialize with a Specific Admin Key
    - [ ] **Sub-sub-task 3.3.1.1:** On `Node-A`, use `form-config-wizard` to set specific `initial_admin_public_key` (e.g., `AdminKey-A`)
    - [ ] **Sub-sub-task 3.3.1.2:** Start `form-state` on `Node-A`
    - [ ] **Sub-sub-task 3.3.1.3:** Start `formnet` on `Node-A`, verify admin account creation via `/bootstrap/ensure_admin_account`

- [ ] **Sub-task 3.3.2:** Test Admin-Only `form-state` API Endpoint with Admin Key
    - [ ] **Sub-sub-task 3.3.2.1:** Choose admin-only endpoint (e.g., `/node/create`)
    - [ ] **Sub-sub-task 3.3.2.2:** Construct valid request for this endpoint
    - [ ] **Sub-sub-task 3.3.2.3:** Sign request using private key for `AdminKey-A`
    - [ ] **Sub-sub-task 3.3.2.4:** Send signed request to `form-state` API, verify success (HTTP 200/201)

- [ ] **Sub-task 3.3.3:** Test Admin-Only `form-state` API Endpoint with Non-Admin Key
    - [ ] **Sub-sub-task 3.3.3.1:** Use same endpoint and payload from 3.3.2
    - [ ] **Sub-sub-task 3.3.3.2:** Sign request using non-admin key (e.g., `OperatorKey-A` or random key)
    - [ ] **Sub-sub-task 3.3.3.3:** Send signed request, verify failure (HTTP 401 Unauthorized)

- [ ] **Sub-task 3.3.4:** Test Non-Admin `form-state` API Endpoint
    - [ ] **Sub-sub-task 3.3.4.1:** Choose public endpoint (e.g., `/health`)
    - [ ] **Sub-sub-task 3.3.4.2:** Send request (unsigned if public), verify success

- [ ] **Sub-task 3.3.5 (Optional):** Test with Default Admin (First Node Operator)
    - [ ] **Sub-sub-task 3.3.5.1:** On `Node-A` (clean setup), use `form-config-wizard`, leave `initial_admin_public_key` empty
    - [ ] **Sub-sub-task 3.3.5.2:** Start `form-state` and `formnet`, verify `OperatorKey-A` becomes admin
    - [ ] **Sub-sub-task 3.3.5.3:** Repeat 3.3.2 and 3.3.3 using `OperatorKey-A` as admin

### 📋 Task 3.5: Testing Admin Account (Manual Multi-Machine Setup) - **NOT STARTED**
**Goal:** Duplicate of Task 3.3 with same structure and verification steps.

**Dependencies:** Same as Task 3.3  
**Total Sub-sub-tasks:** 16 (identical structure to Task 3.3)

- [ ] **Sub-task 3.5.1:** Configure and Initialize with a Specific Admin Key (3 sub-sub-tasks)
- [ ] **Sub-task 3.5.2:** Test Admin-Only `form-state` API Endpoint with Admin Key (4 sub-sub-tasks)
- [ ] **Sub-task 3.5.3:** Test Admin-Only `form-state` API Endpoint with Non-Admin Key (3 sub-sub-tasks)
- [ ] **Sub-task 3.5.4:** Test Non-Admin `form-state` API Endpoint (2 sub-sub-tasks)
- [ ] **Sub-task 3.5.5 (Optional):** Test with Default Admin (3 sub-sub-tasks)

---

## Phase 4: Testing Proof of Claim & Dispatch Mechanism

### 📋 Task 4.4: Testing Proof of Claim & Dispatch Mechanism - **NOT STARTED**
**Goal:** Test the Proof of Claim task assignment and dispatch system within `form-state`.

**Dependencies:** ✅ Task 4.1, 4.2, 4.3 completed  
**Total Sub-sub-tasks:** 3

- [ ] **Sub-task 4.4.1:** Setup `form-state` with multiple mock `Node` entries with varying capabilities and service endpoints
- [ ] **Sub-task 4.4.2:** Create a "BuildImage" task via `form-state` API
    - *Verification:* `Task.responsible_nodes` populated by PoC, Status `PoCAssigned`
    - *Verification:* `form-state` attempts to dispatch (logs API calls for devnet or queue writes for prod)
- [ ] **Sub-task 4.4.3:** Test with `target_redundancy > 1`

---

## Phase 6: Additional Node-Side Task Reception & Execution

### 📋 Task 6.2: Enhance `form-pack` to Receive and Execute Dispatched `BuildImage` Tasks - **NOT STARTED**
**Goal:** Enable `form-pack` service/agent to receive dispatched build tasks and execute them.

**Dependencies:** ✅ Task 4.3 dispatch logic completed  
**Total Sub-sub-tasks:** 6

- [ ] **Sub-task 6.2.1:** Define mechanism for `form-pack` service to receive dispatched build tasks
    - [ ] **Sub-sub-task 6.2.1.1 (devnet):** Implement internal, authenticated API endpoint (`POST /internal/dispatch_build_task`)
    - [ ] **Sub-sub-task 6.2.1.2 (Production):** Ensure `form-state` can publish to `FormPackManager` queue for correct node with `task_id`

- [ ] **Sub-task 6.2.2:** When build task is received (e.g., `PackBuildRequest` with `task_id`)
    - [ ] **Sub-sub-task 6.2.2.1:** Update corresponding task status to `InProgress` in `form-state` using `task_id`
    - [ ] **Sub-sub-task 6.2.2.2:** Execute image build via existing `handle_pack_request` or core build logic
    - [ ] **Sub-sub-task 6.2.2.3:** Update final task status in `form-state` with `result_info` (Formfile content/ID, artifact path/ID)

### 📋 Task 6.3: Testing Node-Side Task Reception & Execution - **NOT STARTED**
**Goal:** Manual multi-machine testing of task reception and execution for both `form-pack` and `form-vmm-service`.

**Dependencies:** 🚧 Task 6.1 and 6.2 completion  
**Total Sub-sub-tasks:** 2

- [ ] **Sub-task 6.3.1:** Setup `form-pack` and `form-vmm-service` with a mock `Node` and task
- [ ] **Sub-task 6.3.2:** Verify `form-pack` and `form-vmm-service` can receive and execute a dispatched task
    - *Verification:* Services should process the task and update status in `form-state`

---

## Summary Statistics

**Total Planned Tasks:** 8 major tasks across 6 phases

**Phase Breakdown:**
- **Phase 1 (Configurable Subnet Testing):** 1 task, 13 sub-sub-tasks
- **Phase 2 (Node Registration Testing):** 1 task, 9 sub-sub-tasks  
- **Phase 3 (Admin Account Testing):** 2 tasks, 32 sub-sub-tasks
- **Phase 4 (Proof of Claim Testing):** 1 task, 3 sub-sub-tasks
- **Phase 6 (Node-Side Task Execution):** 2 tasks, 8 sub-sub-tasks

**Total Sub-sub-tasks:** 65 individual sub-sub-tasks

**Category Breakdown:**
- **🧪 Testing Tasks:** 6 major testing task groups (57 sub-sub-tasks)
- **⚙️ Implementation Tasks:** 2 major implementation task groups (8 sub-sub-tasks)

**Dependency Status:**
- **✅ Ready to Execute:** Tasks 1.3, 2.4, 4.4 (infrastructure completed)
- **🚧 Blocked:** Tasks 3.3, 3.5 (need Task 3.2 completion)
- **🚧 Blocked:** Tasks 6.2, 6.3 (need Task 6.1 completion)

**Priority Classification:**
- **🔥 High Priority:** Task 6.2 (form-pack build task reception)
- **🔥 Medium Priority:** All testing tasks 1.3, 2.4, 4.4 (validation of completed systems)
- **📋 Low Priority:** Admin testing tasks 3.3, 3.5 (blocked until Task 3.2 complete)
- **📋 Low Priority:** Task 6.3 (integration testing, depends on 6.1 + 6.2)

**Execution Readiness:**
- **29 sub-sub-tasks** are immediately executable (Tasks 1.3, 2.4, 4.4)
- **36 sub-sub-tasks** are blocked by in-progress dependencies

---

**Next Action:** Proceed to Sub-sub-task 1.1.1.4 - Extract all testing tasks and their completion status 