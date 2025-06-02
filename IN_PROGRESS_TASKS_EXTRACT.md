# In-Progress Tasks Extract - Sub-sub-task 1.1.1.2

**Status:** ✅ COMPLETED  
**Date:** Formation Project Plan Phase 1.1.1.2  
**Purpose:** Extract all in-progress/partially completed tasks from `implementation_plan.md` for roadmap creation.

---

## Phase 3: Task 3 (Configurable Admin Account & Auth)

### 🚧 Task 3.2: Update `form-net` to Use Configurable Admin Account - **PARTIALLY COMPLETED**
**Progress:** 1/4 sub-tasks completed (25%)

**✅ Completed:**
- [x] **Sub-task 3.2.1:** Modify `form-net/formnet/src/init.rs`
    - [x] **Sub-sub-task 3.2.1.1:** Use `initial_admin_public_key` in `init` function

**🔲 Remaining:**
- [ ] **Sub-task 3.2.2:** Modify `form-net/formnet/src/main.rs` (the `formnet` binary)
    - [ ] **Sub-sub-task 3.2.2.1:** When `OperatorConfig` is loaded, retrieve `op_config.initial_admin_public_key`
    - [ ] **Sub-sub-task 3.2.2.2:** Handle `None`/empty values with default behavior
    - [ ] **Sub-sub-task 3.2.2.3:** Pass resolved `initial_admin_public_key` to `formnet::init::init()`
- [ ] **Sub-task 3.2.3:** Review and update other `form-net` components
    - [ ] **Sub-sub-task 3.2.3.1:** Check `form-net/formnet/src/lib.rs` and remove/deprecate `NETWORK_CIDR` constant
    - [ ] **Sub-sub-task 3.2.3.2:** Examine `formnet::join::request_to_join` and related logic
    - [ ] **Sub-sub-task 3.2.3.3:** Inspect `form-net/server/src/lib.rs` (or `main.rs`)
    - [ ] **Sub-sub-task 3.2.3.4:** Verify `formnet-server::ConfigFile` is correctly populated
- [ ] **Sub-task 3.2.4:** Update `form-state` interaction
    - [ ] **Sub-sub-task 3.2.4.1:** Ensure `populate_crdt_datastore` reflects configurable `root_cidr`

---

## Phase 6: Implement Node-Side Task Reception & Execution

### 🚧 Task 6.1: Enhance `form-vmm-service` to Receive and Execute Dispatched `LaunchInstance` Tasks - **PARTIALLY COMPLETED**
**Progress:** 1/2 sub-tasks completed (50%)

**✅ Completed:**
- [x] **Sub-task 6.1.2:** When `VmmEvent::ProcessLaunchTask(launch_info)` is received
    - [x] **Sub-sub-task 6.1.2.1:** Update task status in `form-state` to `InProgress` via API call
    - [x] **Sub-sub-task 6.1.2.2:** Extract `formfile_content`, `instance_name` from `launch_info`
    - [x] **Sub-sub-task 6.1.2.3:** Construct `VmInstanceConfig` robustly (parsing `formfile_content`)
    - [x] **Sub-sub-task 6.1.2.4:** Call `VmManager::create()`
    - [x] **Sub-sub-task 6.1.2.5:** Update final task status in `form-state`

**🔲 Remaining:**
- [ ] **Sub-task 6.1.1:** Define mechanism for `form-vmm-service` to receive dispatched `LaunchTaskInfo`
    - [ ] **Sub-sub-task 6.1.1.1 (devnet):** Implement internal, authenticated HTTP API endpoint (`POST /internal/dispatch_launch_task`) that accepts `LaunchTaskInfo`
    - [ ] **Sub-sub-task 6.1.1.2 (Production):** Ensure `form-state` can enqueue `VmmEvent::ProcessLaunchTask` via `form-p2p` delivery

---

## Phase 7: API Versioning for `form-state`

### 🚧 Task 7.4: Testing API Versioning (Manual & Automated) - **NOT STARTED**
**Progress:** 0/3 sub-tasks completed (0%)

**🔲 Remaining:**
- [ ] **Sub-task 7.4.1:** Test all `form-state` API endpoints using new `/v1/...` paths to ensure they function as before
- [ ] **Sub-task 7.4.2:** Verify calls to old paths (without `/v1/`) correctly result in 404 Not Found errors
- [ ] **Sub-task 7.4.3:** Conduct integration tests to ensure inter-service communication works correctly with versioned paths

---

## Phase 8: API Versioning for Other Services

### 🚧 Task 8.1: Version `form-pack` API Endpoints - **PARTIALLY COMPLETED**
**Progress:** 2/4 sub-tasks completed (50%)

**✅ Completed:**
- [x] **Sub-task 8.1.1 (Plan):** Identify `form-pack` API routes in `form-pack/src/helpers/api/mod.rs`
- [x] **Sub-task 8.1.2 (Implement):** Modify `build_routes` to nest all routes under `/v1`

**🔲 Remaining:**
- [ ] **Sub-task 8.1.3 (Planning):** Identify and plan updates for any clients of `form-pack` API (e.g., CLIs, other services)
- [ ] **Sub-task 8.1.4 (Test):** Test new `/v1/...` `form-pack` endpoints and ensure old paths are 404

### 🚧 Task 8.2: Version `form-vmm-service` API Endpoints - **PARTIALLY COMPLETED**
**Progress:** 2/3 sub-tasks completed (67%)

**✅ Completed:**
- [x] **Sub-task 8.2.1 (Plan):** Identify `form-vmm-service` API routes (`public_routes` and `protected_routes`)
- [x] **Sub-sub-task 8.2.2 (Implement):** Nest all routes under `/v1` similar to `form-state` and `form-pack`

**🔲 Remaining:**
- [ ] **Sub-sub-task 8.2.3 (Planning):** Identify clients and plan for their updates (e.g., `form-state` for `BootComplete`, CLI/UI tools)

### 🚧 Task 8.3: Version `form-dns` API Endpoints - **PARTIALLY COMPLETED**
**Progress:** 1/5 sub-tasks completed (20%)

**✅ Completed:**
- [x] **Sub-task 8.3.1 (Plan):** Identify `form-dns` API routes (e.g., `/record/...`, `/bootstrap/...`)

**🔲 Remaining:**
- [ ] **Sub-sub-task 8.3.2 (Plan):** Plan to nest all routes under `/v1` in `form-dns/src/api.rs`
- [ ] **Sub-sub-task 8.3.3 (Planning):** Identify clients (`form-state`, `form-config-wizard`) and plan updates to use `/v1` paths
- [ ] **Sub-task 8.3.4 (Implement):** Implement changes in `form-dns/src/api.rs` and client call sites
- [ ] **Sub-task 8.3.5 (Test):** Test new `/v1/...` `form-dns` endpoints and client interactions

### 🚧 Task 8.4: Version `form-net` API Endpoints - **NOT STARTED**
**Progress:** 0/3 sub-tasks completed (0%)

**🔲 Remaining:**
- [ ] **Sub-task 8.4.1 (Plan):** Identify any `formnet-server` API routes exposed for peer joining or management
- [ ] **Sub-sub-task 8.4.2 (Plan):** Plan to nest these under `/v1` if applicable
- [ ] **Sub-sub-task 8.4.3 (Planning):** Identify clients (e.g., `formnet` CLI, other nodes) and plan for their updates

---

## Tasks That Are Not Started But Have Testing Dependencies

### 🔲 Phase 1: Testing Configurable Subnet (Manual Multi-Machine Setup)

**All Task 1.3 sub-tasks are not started but depend on completed infrastructure:**

- [ ] **Sub-task 1.3.1:** Prepare Test Environment & Build Artifacts (4 sub-sub-tasks)
- [ ] **Sub-task 1.3.2:** Test Single Node Initialization with Custom CIDR (3 sub-sub-tasks) 
- [ ] **Sub-task 1.3.3:** Test Second Node Joining the Custom CIDR Network (3 sub-sub-tasks)
- [ ] **Sub-task 1.3.4:** Test with Different Valid CIDR Values (1 sub-sub-task)
- [ ] **Sub-task 1.3.5:** Test Behavior with Invalid CIDR in Config (2 sub-sub-tasks)

### 🔲 Phase 2: Testing Node Registration (Manual Multi-Machine Setup)

**All Task 2.4 sub-tasks are not started but depend on completed infrastructure:**

- [ ] **Sub-task 2.4.1:** Test Initial `CrdtPeer` and `Node` Registration (2 sub-sub-tasks)
- [ ] **Sub-task 2.4.2:** Test Node Heartbeat Updates (3 sub-sub-tasks)
- [ ] **Sub-task 2.4.3:** Test Node Metrics/Capability Updates (2 sub-sub-tasks)
- [ ] **Sub-task 2.4.4:** Test Graceful Node Leave (2 sub-sub-tasks)

### 🔲 Phase 3: Testing Configurable Admin Account (Manual Multi-Machine Setup)

**All Task 3.3 and 3.5 sub-tasks are not started but depend on completed infrastructure:**

- [ ] **Task 3.3:** Testing Configurable Admin Account (5 sub-tasks, 16 sub-sub-tasks)
- [ ] **Task 3.5:** Testing Admin Account (5 sub-tasks, 16 sub-sub-tasks)

### 🔲 Phase 4: Testing Proof of Claim & Dispatch Mechanism

**All Task 4.4 sub-tasks are not started but depend on completed infrastructure:**

- [ ] **Sub-task 4.4.1:** Setup `form-state` with multiple mock `Node` entries
- [ ] **Sub-task 4.4.2:** Create a "BuildImage" task via `form-state` API
- [ ] **Sub-task 4.4.3:** Test with `target_redundancy > 1`

### 🔲 Phase 6: Additional Node-Side Task Reception & Execution

**Task 6.2 and 6.3 are not started:**

- [ ] **Task 6.2:** Enhance `form-pack` to Receive and Execute Dispatched `BuildImage` Tasks (2 sub-tasks, 6 sub-sub-tasks)
- [ ] **Task 6.3:** Testing Node-Side Task Reception & Execution (2 sub-tasks, 2 sub-sub-tasks)

---

## Summary Statistics

**Total In-Progress Tasks:** 8 major tasks across 4 phases

**Phase Breakdown:**
- **Phase 3 (Admin Account & Auth):** 1 partially completed task (25% done)
- **Phase 6 (Node-Side Task Execution):** 1 partially completed task (50% done)
- **Phase 7 (form-state API Versioning):** 1 not-started testing task (0% done)
- **Phase 8 (Other Services API Versioning):** 4 partially completed tasks (20-67% done), 1 not-started task

**Ready-for-Testing Tasks:** 6 major testing task groups that depend on completed infrastructure but haven't been executed yet

**Priority Classification:**
- **🔥 High Priority:** Task 3.2 (Admin Account integration), Task 7.4 (API versioning testing)
- **🔥 High Priority:** Task 6.1.1 (VMM service task reception mechanism)
- **🔥 Medium Priority:** Task 8.1-8.4 (Complete API versioning for all services)
- **📋 Low Priority:** All manual testing tasks (can be done for validation)

**Total Remaining Sub-sub-tasks in In-Progress Tasks:** ~35 individual sub-sub-tasks

---

**Next Action:** Proceed to Sub-sub-task 1.1.1.3 - Extract all planned but not started tasks 