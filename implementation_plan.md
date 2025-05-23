# Implementation Plan

## Phase 1: Foundational Work & Task 1 (Configurable `formnet` Subnet)

**Goal:** Make the `formnet` subnet configurable at launch, moving away from the hardcoded `10.0.0.0/8`.

### Task 1.1: Update Configuration Structure
- [x] **Sub-task 1.1.1:** Modify `form-config/src/lib.rs`.
    - [x] **Sub-sub-task 1.1.1.1:** Add a new field to the `OperatorConfig` struct, e.g., `formnet_cidr: Option<String>`.
        - *Detail:* This field will store the desired CIDR for the `formnet` network (e.g., "10.42.0.0/16"). Mark it as `Option<String>` to allow for a default if not provided. Add `#[clap(long)]` attribute for CLI parsing.
    - [x] **Sub-sub-task 1.1.1.2:** Update the `prompts` module in `form-config/src/lib.rs`.
        - *Detail:* Add a new prompt function (e.g., `prompts::formnet_cidr`) to ask the user for the `formnet` CIDR during the `run_config_wizard`. Provide a sensible default (e.g., "10.42.0.0/16" or keep "10.0.0.0/8" as a default if preferred, but make it explicit). Integrate this prompt into the `run_config_wizard` function.
    - [x] **Sub-sub-task 1.1.1.3:** Ensure the new `formnet_cidr` field is correctly (de)serialized with the rest of `OperatorConfig`.
- [x] **Sub-task 1.1.2:** Verify `form-config-wizard` functionality.
    - [x] **Sub-sub-task 1.1.2.1:** Run the `form-config-wizard` and confirm it prompts for the new `formnet_cidr`.
    - [x] **Sub-sub-task 1.1.2.2:** Check that the generated `./secrets/.operator-config.json` (or custom path) includes the `formnet_cidr` field with the provided or default value.

### Task 1.2: Update `form-net` to Use Configurable Subnet
- [x] **Sub-task 1.2.1:** Modify `form-net/formnet/src/init.rs`.
    - [x] **Sub-sub-task 1.2.1.1:** Change the `init` function signature to accept the `formnet_cidr: String` (or `IpNet`) as a parameter.
    - [x] **Sub-sub-task 1.2.1.2:** Inside `init`, parse `formnet_cidr_str` into an `ipnet::IpNet` object. Handle potential parsing errors.
    - [x] **Sub-sub-task 1.2.1.3:** Replace the hardcoded `let root_cidr: IpNet = IpNet::new(IpAddr::V4(Ipv4Addr::new(10,0,0,0)), 8)?;` with the parsed `IpNet` from the function parameter.
    - [x] **Sub-sub-task 1.2.1.4:** Ensure all subsequent logic in `init` that uses `root_cidr` correctly uses the new configurable CIDR.
- [x] **Sub-task 1.2.2:** Modify `form-net/formnet/src/main.rs` (the `formnet` binary).
    - [x] **Sub-sub-task 1.2.2.1:** When `OperatorConfig` (`op_config`) is loaded, retrieve the `op_config.formnet_cidr`.
    - [x] **Sub-sub-task 1.2.2.2:** If `op_config.formnet_cidr` is `None` or empty, decide on a default behavior.
    - [x] **Sub-sub-task 1.2.2.3:** When calling `formnet::init::init(...)` in the "no bootstraps specified" code path, pass the resolved `formnet_cidr` string.
- [x] **Sub-task 1.2.3:** Review and update other `form-net` components.
    - [x] **Sub-sub-task 1.2.3.1:** Check `form-net/formnet/src/lib.rs` and remove or deprecate the `NETWORK_CIDR` constant.
    - [x] **Sub-sub-task 1.2.3.2:** Examine `formnet::join::request_to_join` and related logic. Ensure that when a node joins, it correctly learns and uses the network's actual CIDR.
    - [x] **Sub-sub-task 1.2.3.3:** Inspect `form-net/server/src/lib.rs` (or `main.rs`).
    - [x] **Sub-sub-task 1.2.3.4:** Verify that the `formnet-server::ConfigFile` is correctly populated.
- [x] **Sub-task 1.2.4:** Update `form-state` interaction.
    - [x] **Sub-sub-task 1.2.4.1:** Ensure that when `form-net/formnet/init.rs` calls `populate_crdt_datastore`, the `DbInitData.network_cidr` field correctly reflects the new configurable `root_cidr`.

### Task 1.3: Testing Configurable Subnet (Manual Multi-Machine Setup)

**Goal:** Verify that the configurable `formnet_cidr` works correctly in a distributed scenario, allowing nodes to initialize and join a network with a custom CIDR.

**Preamble:** These tests require at least two machines (VMs or physical) that can communicate over a network (e.g., public internet or a shared LAN). Let's call them `Node-A` (first/bootstrap node) and `Node-B` (joining node).

- [ ] **Sub-task 1.3.1: Prepare Test Environment & Build Artifacts**
    - [ ] **Sub-sub-task 1.3.1.1:** Set up two machines (`Node-A`, `Node-B`) with necessary OS and dependencies (Rust, Cargo, WireGuard tools).
    - [ ] **Sub-sub-task 1.3.1.2:** Ensure network connectivity between `Node-A` and `Node-B` (e.g., `Node-B` can reach `Node-A` on its public IP and the necessary ports like 51820 for WireGuard and the `formnet` API).
    - [ ] **Sub-sub-task 1.3.1.3:** Build the latest versions of `form-config` (for `form-config-wizard`), `form-state` (datastore service), `form-net/formnet` (for `formnet` CLI), and `form-net/server` (for `innernet-server` or `formnet-server` if it's a separate binary used by the bootstrap node).
    - [ ] **Sub-sub-task 1.3.1.4:** Distribute the necessary binaries to both `Node-A` and `Node-B`.

- [ ] **Sub-task 1.3.2: Test Single Node Initialization with Custom CIDR (`Node-A`)**
    - [ ] **Sub-sub-task 1.3.2.1:** On `Node-A`, run `form-config-wizard`.
        - *Detail:* Provide a custom CIDR (e.g., "192.168.70.0/24") for `formnet_cidr`. Complete other prompts as needed (e.g., generate new keys). Save the `operator-config.json`.
    - [ ] **Sub-sub-task 1.3.2.2:** On `Node-A`, start the `form-state` datastore service, configured to use the generated `operator-config.json` (if it needs identity/bootstrap info from it) or its own default config.
        - *Verification:* Ensure `form-state` starts without errors.
    - [ ] **Sub-sub-task 1.3.2.3:** On `Node-A`, initialize and start the `formnet` network service in bootstrap mode, using the generated `operator-config.json`.
        - *Command Example (conceptual):* `path/to/formnet operator join --config-path path/to/operator-config.json` (This command assumes no bootstrap nodes are listed in the config initially, forcing it into `init` mode).
        - *Verification (on Node-A):*
            - A WireGuard interface (e.g., `formnet0`) is created.
            - The interface has an IP from the custom CIDR (e.g., 192.168.70.1/24).
            - The local `formnet` config file (e.g., `/etc/formnet/formnet.conf`) shows the correct IP and prefix length.
            - Query `form-state` (running on `Node-A`, e.g., `curl http://localhost:3004/network/cidrs`) to confirm the root `CrdtCidr` for the network reflects "192.168.70.0/24".
            - The `formnet` service (and its API on port 51820) is listening.

- [ ] **Sub-task 1.3.3: Test Second Node Joining the Custom CIDR Network (`Node-B`)**
    - [ ] **Sub-sub-task 1.3.3.1:** On `Node-B`, run `form-config-wizard`.
        - *Detail:* For `formnet_cidr`, it can be left default or match `Node-A`'s; it won't be used for joining an existing network directly but good for consistency if `Node-B` were to initialize its own. Generate new keys for `Node-B`. Crucially, configure `Node-A`'s public IP and WireGuard port (e.g., `<Node-A-Public-IP>:51820`) as a `bootstrap_nodes` entry. Save `operator-config-node-b.json`.
    - [ ] **Sub-sub-task 1.3.3.2:** On `Node-B`, start its `form-state` datastore service (it will sync from `Node-A`).
    - [ ] **Sub-sub-task 1.3.3.3:** On `Node-B`, start the `formnet` network service, using `operator-config-node-b.json` (which specifies `Node-A` as a bootstrap).
        - *Command Example (conceptual):* `path/to/formnet operator join --config-path path/to/operator-config-node-b.json`.
        - *Verification (on Node-B):*
            - A WireGuard interface is created.
            - The interface receives an IP from `Node-A`'s custom CIDR (e.g., 192.168.70.2/24).
            - The local `formnet` config file shows the correct IP and prefix length.
            - `Node-B` can ping `Node-A`'s `formnet` IP (e.g., `ping 192.168.70.1`).
        - *Verification (on Node-A):*
            - `wg show formnet0` (or equivalent) should list `Node-B` as a peer.
            - Query `form-state` on `Node-A` to see `Node-B`'s `CrdtPeer` entry.

- [ ] **Sub-task 1.3.4: Test with Different Valid CIDR Values**
    - [ ] **Sub-sub-task 1.3.4.1:** Repeat Sub-tasks 1.3.2 and 1.3.3 using a different valid CIDR (e.g., "10.100.0.0/16", or a smaller one like "192.168.200.0/28"). Verify all steps.

- [ ] **Sub-task 1.3.5: Test Behavior with Invalid CIDR in Config**
    - [ ] **Sub-sub-task 1.3.5.1:** On `Node-A` (clean state), use `form-config-wizard` to set an invalid `formnet_cidr` (e.g., "not-a-cidr", "192.168.1.300/24").
    - [ ] **Sub-sub-task 1.3.5.2:** Attempt to initialize `formnet` on `Node-A`.
        - *Verification:* The `formnet init` process (or the `formnet` binary startup) should fail gracefully with a clear error message indicating the CIDR is invalid. It should not attempt to use a malformed CIDR.

**User Story: Walkthrough for Manual Two-Machine Test (e.g., CIDR "172.20.0.0/16")**

1.  **Setup:**
    *   Alice (QA Engineer) provisions two Linux VMs: `vm1` (Node-A) and `vm2` (Node-B).
    *   She ensures `vm1` has a known public IP (e.g., `A.A.A.A`) and `vm2` has `B.B.B.B`.
    *   Firewall on `vm1` allows inbound UDP on port 51820 (for WireGuard) and TCP on 51820 (for `formnet` API) and 3004 (for `form-state` API, if accessed directly by `vm2`).
    *   Alice compiles the latest `form-config-wizard`, `form-state`, and `formnet` binaries and copies them to both VMs.

2.  **Node-A (Bootstrap Node) Configuration & Startup:**
    *   On `vm1`, Alice runs `./form-config-wizard wizard`.
        *   Network ID: (default)
        *   Keys: Generate new.
        *   Bootstrap Nodes: (leave empty)
        *   Bootstrap Domain: (leave empty)
        *   Is Bootstrap Node: Yes
        *   Region: (e.g., us-east)
        *   Ports: (defaults)
        *   **Formnet CIDR: `172.20.0.0/16`**
        *   Contract: (leave empty)
        *   Saves to `./operator-config-vm1.json`.
    *   On `vm1`, Alice starts `form-state`: `./form-state --config-path ./operator-config-vm1.json` (adjust if `form-state` doesn't use this config directly for its own identity).
    *   On `vm1`, Alice starts `formnet`: `./formnet operator join --config-path ./operator-config-vm1.json`.
    *   Alice verifies on `vm1`:
        *   `ip addr show formnet0` (or similar) shows `inet 172.20.0.1/16 ...`.
        *   `/etc/formnet/formnet.conf` shows `address = 172.20.0.1` and `network_cidr_prefix = 16`.
        *   `curl http://localhost:3004/network/cidrs` includes an entry for `"cidr": "172.20.0.0/16"`.

3.  **Node-B (Joining Node) Configuration & Startup:**
    *   On `vm2`, Alice runs `./form-config-wizard wizard`.
        *   Network ID: (default, same as vm1)
        *   Keys: Generate new.
        *   **Bootstrap Nodes: `A.A.A.A:51820`** (public IP and WireGuard/API port of `vm1`)
        *   Bootstrap Domain: (leave empty)
        *   Is Bootstrap Node: No
        *   Region: (e.g., us-west)
        *   Ports: (defaults)
        *   Formnet CIDR: (can leave default, it will be ignored for join)
        *   Contract: (leave empty)
        *   Saves to `./operator-config-vm2.json`.
    *   On `vm2`, Alice starts `form-state`: `./form-state --config-path ./operator-config-vm2.json --to-dial A.A.A.A:3004` (assuming `form-state` needs to dial for CRDT sync, and API port on vm1 is 3004 and accessible).
    *   On `vm2`, Alice starts `formnet`: `./formnet operator join --config-path ./operator-config-vm2.json`.
    *   Alice verifies on `vm2`:
        *   `ip addr show formnet0` shows an IP like `inet 172.20.x.y/16` (e.g., `172.20.0.2/16`).
        *   `/etc/formnet/formnet.conf` shows its assigned IP and `network_cidr_prefix = 16`.
        *   `ping 172.20.0.1` (vm1's formnet IP) is successful.
    *   Alice verifies on `vm1`:
        *   `wg show formnet0` lists `vm2`'s public key as a peer with an endpoint and its allowed IP (`172.20.x.y/32`).
        *   `curl http://localhost:3004/network/peers` (or similar) shows `vm2`'s peer entry.

4.  **Conclusion:** If all verifications pass, the configurable CIDR feature is working as intended for this scenario.

---

## Phase 2: Task 2 (Node Registration in Datastore)

**Goal:** Ensure new nodes are properly registered in the `form-state` datastore, making their existence and properties known to the network.

### Task 2.1: Solidify `CrdtPeer` Registration
- [x] **Sub-task 2.1.1:** Review `form-net/formnet/src/init.rs` (`populate_crdt_datastore`) and `formnet::join::request_to_join`.
    - [x] **Sub-sub-task 2.1.1.1:** Confirm that when a node initializes or successfully joins, a `PeerContents` struct is created.
    - [x] **Sub-sub-task 2.1.1.2:** Confirm this `PeerContents` is used to create/update a `CrdtPeer` entry in `form-state`.
    - [x] **Sub-sub-task 2.1.1.3:** Ensure all relevant fields in `CrdtPeer` are populated.
- [x] **Sub-task 2.1.2:** Verify gossip of `CrdtPeer` updates.
    - [x] **Sub-sub-task 2.1.2.1:** Confirm that the `PeerOp` is passed to `DataStore::write_to_queue`.
    - [x] **Sub-sub-task 2.1.2.2:** Confirm this queue write triggers `form-p2p` to gossip the operation.

### Task 2.2: Solidify `Node` Registration
- [x] **Sub-task 2.2.1:** Identify where a new node registers its `Node` object in `form-state`'s `NodeState.map`.
- [x] **Sub-task 2.2.2:** Implement or verify the logic for a node to create its `Node` struct.
    - [x] **Sub-sub-task 2.2.2.1:** Populate all `Node` struct fields correctly.
- [x] **Sub-task 2.2.3:** Ensure the node calls the appropriate `form-state` API for `Node` registration.
- [x] **Sub-task 2.2.4:** Verify gossip of `Node` updates.
    - [x] **Sub-sub-task 2.2.4.1:** Confirm the `NodeOp` is passed to `DataStore::write_to_queue`.
    - [x] **Sub-sub-task 2.2.4.2:** Assess if `NodeOp`s need broader gossip.

### Task 2.3: Node Heartbeat and Liveness
- [x] **Sub-task 2.3.1:** Implement or verify a mechanism for nodes to periodically send heartbeats.
- [x] **Sub-task 2.3.2:** Consider a mechanism for stale node detection/removal.

### Task 2.4: Testing Node Registration (Manual Multi-Machine Setup)

**Goal:** Verify that `CrdtPeer` and `Node` objects are correctly registered in `form-state` and their state (including heartbeats and leave events) is properly propagated. This builds upon the setup from Task 1.3.

**Preamble:** Assumes `Node-A` is up and running the `formnet` (with a custom CIDR) and `form-state`. `Node-B` is ready to join. We also assume a client mechanism exists on nodes to:
    a. Register/update their `Node` object in `form-state` after joining `formnet`.
    b. Send periodic heartbeats to `form-state`.

- [ ] **Sub-task 2.4.1: Test Initial `CrdtPeer` and `Node` Registration (`Node-B`)**
    - [ ] **Sub-sub-task 2.4.1.1:** `Node-B` joins the `formnet` hosted by `Node-A`.
        - *Verification (on Node-A and Node-B after sync):* `CrdtPeer` for `Node-B` exists and is correct.
    - [ ] **Sub-sub-task 2.4.1.2:** `Node-B` registers its `Node` object with `form-state`.
        - *Verification (on Node-A and Node-B after sync):* `Node` object for `Node-B` exists and is correct; timestamps are recent.

- [ ] **Sub-task 2.4.2: Test Node Heartbeat Updates (`Node-B`)**
    - [ ] **Sub-sub-task 2.4.2.1:** Allow `Node-B` to run with active heartbeat mechanism.
    - [ ] **Sub-sub-task 2.4.2.2:** Query `form-state` for `Node-B`'s `Node` object.
        - *Verification:* `last_heartbeat` and `updated_at` should be updated.
    - [ ] **Sub-sub-task 2.4.2.3:** Stop heartbeat sender on `Node-B`; wait.
        - *Verification:* `last_heartbeat` should not have updated recently.

- [ ] **Sub-task 2.4.3: Test Node Metrics/Capability Updates (Conceptual)**
    - [ ] **Sub-sub-task 2.4.3.1:** Trigger a metrics/capabilities update from `Node-B`.
    - [ ] **Sub-sub-task 2.4.3.2:** Query `form-state` for `Node-B`'s `Node` object.
        - *Verification:* `metrics` or `capabilities` field should reflect changes; `updated_at` new.

- [ ] **Sub-task 2.4.4: Test Graceful Node Leave (`Node-B`)**
    - [ ] **Sub-sub-task 2.4.4.1:** `Node-B` executes a "leave network" command.
        - *Detail:* This should trigger API calls to `form-state` for `Node` and `CrdtPeer` removal/update.
    - [ ] **Sub-sub-task 2.4.4.2:** Verify on `Node-A` (after sync):
        - `Node-B`'s `Node` and `CrdtPeer` objects are removed/updated in `form-state`.
        - `Node-B` is no longer an active WireGuard peer.

---

## Phase 3: Task 3 (Configurable Admin Account & Auth)

**Goal:** Set up a global `admin` account configurable at launch, and ensure its key can authorize admin actions.

### Task 3.1: Define Admin Identity in Configuration
- [x] **Sub-task 3.1.1:** Modify `form-config/src/lib.rs` (`OperatorConfig`).
    - [x] **Sub-sub-task 3.1.1.1:** Add a field like `initial_admin_public_key: Option<String>`.
    - [x] **Sub-sub-task 3.1.1.2:** Update `prompts` module and `run_config_wizard` to ask for this key.
    - [x] **Sub-sub-task 3.1.1.3:** Ensure the new `initial_admin_public_key` field is correctly (de)serialized with the rest of `OperatorConfig`.
- [x] **Sub-task 3.1.2:** Update first node initialization logic.
    - [x] **Sub-sub-task 3.1.2.1:** Read `initial_admin_public_key` from `OperatorConfig`.
    - [x] **Sub-sub-task 3.1.2.2:** Set `CrdtPeer.is_admin = true` appropriately.
    - [x] **Sub-sub-task 3.1.2.3:** Set `Node.node_owner` and add to `Node.operator_keys`.

### Task 3.2: Update `form-net` to Use Configurable Admin Account
- [x] **Sub-task 3.2.1:** Modify `form-net/formnet/src/init.rs` to use the new `initial_admin_public_key`.
    - [x] **Sub-sub-task 3.2.1.1:** Ensure the new `initial_admin_public_key` is used in the `init` function.
- [ ] **Sub-task 3.2.2:** Modify `form-net/formnet/src/main.rs` (the `formnet` binary) to use the new `initial_admin_public_key`.
    - [ ] **Sub-sub-task 3.2.2.1:** When `OperatorConfig` (`op_config`) is loaded, retrieve the `op_config.initial_admin_public_key`.
    - [ ] **Sub-sub-task 3.2.2.2:** If `op_config.initial_admin_public_key` is `None` or empty, decide on a default behavior.
    - [ ] **Sub-sub-task 3.2.2.3:** When calling `formnet::init::init(...)` in the "no bootstraps specified" code path, pass the resolved `initial_admin_public_key` string.
- [ ] **Sub-task 3.2.3:** Review and update other `form-net` components.
    - [ ] **Sub-sub-task 3.2.3.1:** Check `form-net/formnet/src/lib.rs` and remove or deprecate the `NETWORK_CIDR` constant.
    - [ ] **Sub-sub-task 3.2.3.2:** Examine `formnet::join::request_to_join` and related logic. Ensure that when a node joins, it correctly learns and uses the network's actual CIDR.
    - [ ] **Sub-sub-task 3.2.3.3:** Inspect `form-net/server/src/lib.rs` (or `main.rs`).
    - [ ] **Sub-sub-task 3.2.3.4:** Verify that the `formnet-server::ConfigFile` is correctly populated.
- [ ] **Sub-task 3.2.4:** Update `form-state` interaction.
    - [ ] **Sub-sub-task 3.2.4.1:** Ensure that when `form-net/formnet/init.rs` calls `populate_crdt_datastore`, the `DbInitData.network_cidr` field correctly reflects the new configurable `root_cidr`.

### Task 3.3: Testing Configurable Admin Account (Manual Multi-Machine Setup)

**Goal:** Verify that the configurable admin account (`initial_admin_public_key` and `Account.is_global_admin`) correctly grants authorization for admin-only API endpoints in `form-state`.

**Preamble:** Builds on the two-machine setup (`Node-A`, `Node-B`) from Task 1.3 and 2.4.
*   `Node-A` is the initial bootstrap node.
*   `form-state` is running on `Node-A`.

- [ ] **Sub-task 3.3.1: Configure and Initialize with a Specific Admin Key**
    - [ ] **Sub-sub-task 3.3.1.1:** On `Node-A`, use `form-config-wizard` to set a specific `initial_admin_public_key` (e.g., `AdminKey-A`). `Node-A`'s operator key is `OperatorKey-A`.
    - [ ] **Sub-sub-task 3.3.1.2:** Start `form-state` on `Node-A`.
    - [ ] **Sub-sub-task 3.3.1.3:** Start `formnet` on `Node-A`.
        - *Verification:* `formnet` calls `/bootstrap/ensure_admin_account` with `AdminKey-A`. `form-state` API for `GET /account/{AdminKey-A}/is_global_admin` returns `true`. `GET /account/{OperatorKey-A}/is_global_admin` (if different) returns `false`. `CrdtPeer` for `Node-A` has `is_admin: true`.

- [ ] **Sub-task 3.3.2: Test Admin-Only `form-state` API Endpoint with Admin Key**
    - [ ] **Sub-sub-task 3.3.2.1:** Choose an admin-only endpoint from `form-state` (e.g., `/node/create`).
    - [ ] **Sub-sub-task 3.3.2.2:** Construct a valid request for this endpoint.
    - [ ] **Sub-sub-task 3.3.2.3:** Sign request using private key for `AdminKey-A`.
    - [ ] **Sub-sub-task 3.3.2.4:** Send signed request to `form-state` API.
        - *Verification:* Request succeeds (HTTP 200/201).

- [ ] **Sub-task 3.3.3: Test Admin-Only `form-state` API Endpoint with Non-Admin Key**
    - [ ] **Sub-sub-task 3.3.3.1:** Use same endpoint and payload from 3.3.2.
    - [ ] **Sub-sub-task 3.3.3.2:** Sign request using a non-admin key (e.g., `OperatorKey-A` if not global admin, or a new random key).
    - [ ] **Sub-sub-task 3.3.3.3:** Send signed request to `form-state` API.
        - *Verification:* Request fails (HTTP 401 Unauthorized).

- [ ] **Sub-task 3.3.4: Test Non-Admin `form-state` API Endpoint**
    - [ ] **Sub-sub-task 3.3.4.1:** Choose a public endpoint (e.g., `/health`).
    - [ ] **Sub-sub-task 3.3.4.2:** Send request (unsigned if public).
        - *Verification:* Request succeeds.

- [ ] **Sub-task 3.3.5 (Optional): Test with Default Admin (First Node Operator)**
    - [ ] **Sub-sub-task 3.3.5.1:** On `Node-A` (clean setup), use `form-config-wizard`, leave `initial_admin_public_key` empty. `Node-A`'s `OperatorKey-A` becomes admin.
    - [ ] **Sub-sub-task 3.3.5.2:** Start `form-state` and `formnet`.
        - *Verification:* `ensure_admin_account` called with `OperatorKey-A`. `GET /account/{OperatorKey-A}/is_global_admin` shows `true`.
    - [ ] **Sub-sub-task 3.3.5.3:** Repeat 3.3.2 and 3.3.3 using `OperatorKey-A` as admin.

### Task 3.4: CLI for Admin Operations
- [x] **Sub-task 3.4.1:** Consider adding commands to `form-cli` for admin actions.

### Task 3.5: Testing Admin Account (Manual Multi-Machine Setup)

**Goal:** Verify that the configurable admin account (`initial_admin_public_key` and `Account.is_global_admin`) correctly grants authorization for admin-only API endpoints in `form-state`.

**Preamble:** Builds on the two-machine setup (`Node-A`, `Node-B`) from Task 1.3 and 2.4.
*   `Node-A` is the initial bootstrap node.
*   `form-state` is running on `Node-A`.

- [ ] **Sub-task 3.5.1: Configure and Initialize with a Specific Admin Key**
    - [ ] **Sub-sub-task 3.5.1.1:** On `Node-A`, use `form-config-wizard` to set a specific `initial_admin_public_key` (e.g., `AdminKey-A`). `Node-A`'s operator key is `OperatorKey-A`.
    - [ ] **Sub-sub-task 3.5.1.2:** Start `form-state` on `Node-A`.
    - [ ] **Sub-sub-task 3.5.1.3:** Start `formnet` on `Node-A`.
        - *Verification:* `formnet` calls `/bootstrap/ensure_admin_account` with `AdminKey-A`. `form-state` API for `GET /account/{AdminKey-A}/is_global_admin` returns `true`. `GET /account/{OperatorKey-A}/is_global_admin` (if different) returns `false`. `CrdtPeer` for `Node-A` has `is_admin: true`.

- [ ] **Sub-task 3.5.2: Test Admin-Only `form-state` API Endpoint with Admin Key**
    - [ ] **Sub-sub-task 3.5.2.1:** Choose an admin-only endpoint from `form-state` (e.g., `/node/create`).
    - [ ] **Sub-sub-task 3.5.2.2:** Construct a valid request for this endpoint.
    - [ ] **Sub-sub-task 3.5.2.3:** Sign request using private key for `AdminKey-A`.
    - [ ] **Sub-sub-task 3.5.2.4:** Send signed request to `form-state` API.
        - *Verification:* Request succeeds (HTTP 200/201).

- [ ] **Sub-task 3.5.3: Test Admin-Only `form-state` API Endpoint with Non-Admin Key**
    - [ ] **Sub-sub-task 3.5.3.1:** Use same endpoint and payload from 3.5.2.
    - [ ] **Sub-sub-task 3.5.3.2:** Sign request using a non-admin key (e.g., `OperatorKey-A` if not global admin, or a new random key).
    - [ ] **Sub-sub-task 3.5.3.3:** Send signed request to `form-state` API.
        - *Verification:* Request fails (HTTP 401 Unauthorized).

- [ ] **Sub-task 3.5.4: Test Non-Admin `form-state` API Endpoint**
    - [ ] **Sub-sub-task 3.5.4.1:** Choose a public endpoint (e.g., `/health`).
    - [ ] **Sub-sub-task 3.5.4.2:** Send request (unsigned if public).
        - *Verification:* Request succeeds.

- [ ] **Sub-task 3.5.5 (Optional): Test with Default Admin (First Node Operator)**
    - [ ] **Sub-sub-task 3.5.5.1:** On `Node-A` (clean setup), use `form-config-wizard`, leave `initial_admin_public_key` empty. `Node-A`'s `OperatorKey-A` becomes admin.
    - [ ] **Sub-sub-task 3.5.5.2:** Start `form-state` and `formnet`.
        - *Verification:* `ensure_admin_account` called with `OperatorKey-A`. `GET /account/{OperatorKey-A}/is_global_admin` shows `true`.
    - [ ] **Sub-sub-task 3.5.5.3:** Repeat 3.5.2 and 3.5.3 using `OperatorKey-A` as admin.

---

## Phase 4: Task 4 (Node State Communication & Task Selection with Proof of Claim)

**Goal:** Implement a deterministic task self-selection mechanism ("Proof of Claim") for image building and image hosting tasks within `form-state`. `form-state` will determine responsible node(s) and dispatch tasks directly to them.

**Preamble:**
*   "Proof of Claim": For a given task (`task_id`) and a set of capable nodes (`node_id`s), responsibility is determined by `XOR(task_id, node_id)`. Nodes with the lowest XOR result(s) are selected.
*   This applies specifically to "BuildImage" and "LaunchInstance" (image hosting) tasks.
*   `form-state` will house the PoC logic and dispatch tasks to responsible worker services.

### Task 4.1: (Already Completed - Verify and Enhance Node State Communication)
- [x] **Sub-task 4.1.1:** Ensure `Node` updates (capabilities, metrics, etc.) are reliably gossiped.
    - [x] **Sub-sub-task 4.1.1.1:** Review `form-p2p` gossip for `NodeOp`s; adjust if broader dissemination is needed.
- [x] **Sub-task 4.1.2:** Implement or verify periodic updates of `Node.metrics` and `Node.capacity` in `form-state`.

### Task 4.2: Task Definition and Representation (for Proof of Claim tasks)
- [x] **Sub-task 4.2.1:** Define specific "tasks" (e.g., "build image," "launch instance").
- [x] **Sub-task 4.2.2:** Decide how tasks are represented and stored.
    - *Decision:* Core tasks like `BuildImage` and `LaunchInstance` will be represented as CRDTs in `form-state`.
- [x] **Sub-task 4.2.3:** Define `Task` struct in `form-state` for Proof of Claim tasks.
    - [x] **Sub-sub-task 4.2.3.1:** Create `form-state/src/tasks.rs`.
    - [x] **Sub-sub-task 4.2.3.2:** Define `TaskId`.
    - [x] **Sub-sub-task 4.2.3.3:** Define `TaskStatus` enum.
    - [x] **Sub-sub-task 4.2.3.4:** Define `BuildImageParams`, `LaunchInstanceParams` (aligned with worker service inputs).
    - [x] **Sub-sub-task 4.2.3.5:** Define `TaskVariant` enum.
    - [x] **Sub-sub-task 4.2.3.6:** Define the main `Task` struct.
- [x] **Sub-task 4.2.4:** Implement `TaskState` CRDT in `form-state`.
    - [x] **Sub-sub-task 4.2.4.1:** Define `TaskOp`.
    - [x] **Sub-sub-task 4.2.4.2:** Define `TaskState` struct.
    - [x] **Sub-sub-task 4.2.4.3:** Implement methods in `TaskState` for `update_task_local` and `task_op`.
    - [x] **Sub-sub-task 4.2.4.4:** Add `TaskState` to `DataStore`.
    - [x] **Sub-sub-task 4.2.4.5:** Add `handle_task_op` and `handle_task_request` to `DataStore`.

### Task 4.3: Implement Proof of Claim and Task Dispatch in `form-state`
- [x] **Sub-task 4.3.1:** Create utility functions for "Proof of Claim" logic within `form-state/src/tasks.rs`.
    - [x] **Sub-sub-task 4.3.1.1:** Implement `fn calculate_poc_score(task_id: &str, node_id: &str) -> u64`.
    - [x] **Sub-sub-task 4.3.1.2:** Implement `fn determine_responsible_nodes(task: &Task, all_nodes: &[Node], datastore: &DataStore) -> BTreeSet<String>`.
- [x] **Sub-task 4.3.2:** Integrate PoC and Task Dispatch into Task lifecycle within `form-state`.
    - [x] **Sub-sub-task 4.3.2.1:** When a new PoC-eligible `Task` is created (`TaskRequest::Create` in `DataStore::handle_task_request`):
        - Set initial status (e.g., `PendingPoCAssessment`).
        - Create initial `Task` CRDT entry.
        - Run `determine_responsible_nodes`.
        - Update the `Task` CRDT entry with `responsible_nodes` and set status to `PoCAssigned` (this update is gossiped).
    - [x] **Sub-sub-task 4.3.2.2 (NEW):** After `Task` is `PoCAssigned`, `form-state` prepares and dispatches it to each responsible node.
        - [x] **Sub-sub-task 4.3.2.2.1 (Plan & Implement):** Define and implement storage for worker service API endpoint information (for `form-pack`, `form-vmm-service`, etc.) within `form-state`.
            - *Decision & Implementation:* Added `vmm_service_api_endpoint: Option<String>` and `pack_service_api_endpoint: Option<String>` to `NodeAnnotations` struct in `form-state/src/nodes.rs`. Nodes will report these during their registration/update.
        - [x] **Sub-sub-task 4.3.2.2.2 (Implement):** Implement the dispatch logic in `form-state/src/datastore.rs` (e.g., in the existing method `async fn dispatch_task_to_node(&self, task: &crate::tasks::Task, node: &crate::nodes::Node)` called from `handle_task_request` after PoC assignment).
            - *Detail:* This method will be called for each `node_id` in `task.responsible_nodes`.
            - **If Task is `LaunchInstance`:** (Implemented: constructs `LaunchTaskInfo`, dispatches via devnet API (signed) or prod queue (conceptual call to `write_to_queue` with node-specific topic)).
            - **If Task is `BuildImage`:** (Partially Implemented: calls stubbed `prepare_pack_build_request`, then dispatches resulting `PackBuildRequest` via devnet API (signed) or prod queue (conceptual call to `write_to_queue` with node-specific topic)).
                - *Note:* `prepare_pack_build_request` (fetching context, creating artifacts, generating Formfile, signing PackRequest) is a complex stub requiring full implementation.
            - *Authentication for devnet direct dispatch:* Implemented for both task types.

### Task 4.4: Testing Proof of Claim & Dispatch Mechanism (within `form-state`)
- [ ] **Sub-task 4.4.1:** Setup `form-state` with multiple mock `Node` entries with varying capabilities and service endpoints.
- [ ] **Sub-task 4.4.2:** Create a "BuildImage" task via `form-state` API.
    - *Verification:* `Task.responsible_nodes` populated by PoC. Status `PoCAssigned`.
    - *Verification:* `form-state` attempts to dispatch (logs API calls for `devnet` or queue writes for prod to correct responsible nodes).
- [ ] **Sub-task 4.4.3:** Test with `target_redundancy > 1`.

---

## Phase 5: Implement `devnet` Direct API Gossip

**Goal:** In `devnet` mode, bypass the `form-p2p` queue for CRDT operations from `form-state` and instead gossip these operations directly to other known peers via authenticated API calls.

### Task 5.1: Conditional Gossip Logic in `form-state`
- [x] **Sub-task 5.1.1:** Modify `form-state/src/datastore.rs` in methods like `handle_peer_op`, `handle_node_op`, `handle_account_op`, etc.
    - [x] **Sub-sub-task 5.1.1.1:** After a local CRDT `Op` is successfully applied, check if the `devnet`
    - [x] **Sub-sub-task 5.1.1.2:**
        - If `devnet` is enabled: Instead of calling `DataStore::write_to_queue`, initiate a new direct gossip mechanism for the `Op`.
        - If `devnet` is NOT enabled (production mode): Call `DataStore::write_to_queue` as it currently does.
- [x] **Sub-task 5.1.2:** Implement the direct gossip mechanism function within `DataStore` (e.g., `async fn gossip_op_directly<O: Serialize + Clone>(&self, operation: O, op_type_marker: &str)`).
    - [x] **Sub-sub-task 5.1.2.1:** This function will retrieve the list of active peers (their external `form-state` API endpoints) from its own `network_state.peers` or local `/peer/list_active` endpoint (excluding self).
    - [x] **Sub-sub-task 5.1.2.2:** For each peer, construct the target URL for the new `Op` application endpoint (e.g., `http://{peer_endpoint}/devnet_gossip/apply_op`).
    - [x] **Sub-sub-task 5.1.2.3:** Serialize the `operation` (as `DevnetGossipOpContainer`) and POST it to the target peer, **adding required authentication headers signed by the sending node's operator key.**
    - [x] **Sub-sub-task 5.1.2.4:** Log success/failure for each gossip attempt.

### Task 5.2: Create `Op` Application Endpoints in `form-state` API
- [x] **Sub-task 5.2.1:** In `form-state/src/api.rs`, define an API endpoint for checking task responsibility.
    - [x] **Sub-sub-task 5.2.1.1:** Implement the handler to fetch task, node, run PoC if needed, and return responsibility status.

## Phase 6: Implement Node-Side Task Reception & Execution (in `form-pack` and `form-vmm-service`)

**Goal:** Enable `form-pack` and `form-vmm-service` to receive dispatched tasks from `form-state` and execute them, updating status back to `form-state`.

### Task 6.1: Enhance `form-vmm-service` to Receive and Execute Dispatched `LaunchInstance` Tasks
- [ ] **Sub-task 6.1.1:** Define a mechanism for `form-vmm-service` to receive dispatched `LaunchTaskInfo` from `form-state`.
    - [ ] **Sub-sub-task 6.1.1.1 (`devnet`):** Implement an internal, authenticated HTTP API endpoint in `form-vmm-service` (e.g., `POST /internal/dispatch_launch_task`) that accepts `form_types::event::LaunchTaskInfo`.
        - *Handler will trigger `VmmEvent::ProcessLaunchTask`.*
    - [ ] **Sub-sub-task 6.1.1.2 (Production):** `VmManager` already has `api_rx` for `VmmEvent`. Ensure `form-state` can enqueue a `VmmEvent::ProcessLaunchTask` that `form-p2p` delivers to this existing channel, or adapt `VmmSubscriber` if a new topic is used.
- [x] **Sub-task 6.1.2:** When a `VmmEvent::ProcessLaunchTask(launch_info)` is received by `VmManager::handle_vmm_event`:
    - [x] **Sub-sub-task 6.1.2.1:** Update task status in `form-state` to `InProgress` via API call.
    - [x] **Sub-sub-task 6.1.2.2:** Extract `formfile_content`, `instance_name` from `launch_info`.
    - [x] **Sub-sub-task 6.1.2.3:** Construct `VmInstanceConfig` robustly (parsing `formfile_content`).
    - [x] **Sub-sub-task 6.1.2.4:** Call `VmManager::create()`.
    - [x] **Sub-sub-task 6.1.2.5:** Update final task status in `form-state`.

### Task 6.2: Enhance `form-pack` (or new agent) to Receive and Execute Dispatched `BuildImage` Tasks
- [ ] **Sub-task 6.2.1:** Define a mechanism for `form-pack` service/agent to receive dispatched build tasks (payload should align with or be transformable into `PackBuildRequest`, including `task_id`).
    - [ ] **Sub-sub-task 6.2.1.1 (`devnet`):** Implement an internal, authenticated API endpoint in `form-pack` service (e.g., `POST /internal/dispatch_build_task`).
        - *Handler will call existing build logic, e.g., `handle_pack_request`.*
    - [ ] **Sub-sub-task 6.2.1.2 (Production):** `FormPackManager` already polls a queue via `read_from_queue` and calls `handle_message` which processes `PackBuildRequest`. Ensure `form-state` can publish to this queue for the correct node, including `task_id` for correlation.
- [ ] **Sub-task 6.2.2:** When a build task (e.g., `PackBuildRequest` with `task_id`) is received:
    - [ ] **Sub-sub-task 6.2.2.1:** Update corresponding task status to `InProgress` in `form-state` (using the `task_id`).
    - [ ] **Sub-sub-task 6.2.2.2:** Execute image build via existing `handle_pack_request` or core build logic.
    - [ ] **Sub-sub-task 6.2.2.3:** Update final task status in `form-state` with `result_info` (e.g., resulting Formfile content/ID, artifact path/ID).

### Task 6.3: Testing Node-Side Task Reception & Execution (Manual Multi-Machine Setup)
- [ ] **Sub-task 6.3.1:** Setup `form-pack` and `form-vmm-service` with a mock `Node` and task.
- [ ] **Sub-task 6.3.2:** Verify `form-pack` and `form-vmm-service` can receive and execute a dispatched task.
    - *Verification:* `form-pack` and `form-vmm-service` should be able to process the task and update status in `form-state`.

---

## Phase 7: API Versioning for `form-state`

**Goal:** Introduce API versioning (v1) for all `form-state` HTTP API endpoints to ensure backward compatibility for future changes.

- [x] **Task 7.1: Plan API Path Updates**
    - [x] **Sub-task 7.1.1:** List all current and planned HTTP API endpoints in `form-state/src/api.rs`.
        - *Detail: All router groups (`public_api`, `network_writers_api`, `network_readers_api`, `account_api`, `instance_api`, `api_routes`, `devnet_gossip_api`) identified.*
    - [x] **Sub-task 7.1.2:** Define the new path structure, prefixing all existing routes with `/v1`.
        - *Detail: Strategy is `/{group}/{route}` becomes `/v1/{group}/{route}` or `/v1/{route}` for top-level merges.*
- [x] **Task 7.2: Update Axum Router Definitions in `form-state/src/api.rs`**
    - [x] **Sub-sub-task 7.2.1 (Plan):** Modify the main `app` function in `form-state/src/api.rs`.
    - [x] **Sub-sub-task 7.2.2 (Implement):** Consolidate all existing router merges and nests into a new intermediate router (e.g., `v1_routes`). Then, nest this `v1_routes` under a top-level router with the `/v1` prefix. Ensure Axum state is correctly applied.
        - *Example Structure:*
          ```rust
          // pub fn app(state: Arc<Mutex<DataStore>>) -> Router {
          //     // ... define individual routers like public_api, network_writers_api ...
          //     let devnet_gossip_api = Router::new() ... .layer(middleware::from_fn_with_state(state.clone(), ...));
          // 
          //     let v1_routes = Router::new()
          //         .merge(public_api)
          //         .merge(network_writers_api)
          //         // ... merge other specific api routers ...
          //         .nest("/devnet_gossip", devnet_gossip_api);
          // 
          //     Router::new().nest("/v1", v1_routes).with_state(state)
          // }
          ```
- [x] **Task 7.3: Update API Clients (Planning)**
    - [x] **Sub-task 7.3.1:** Identify all internal services (`form-net`, `form-p2p`, `form-vmm-service`, `form-pack`, etc.) and any external clients/tools that call `form-state` API endpoints.
    - [x] **Sub-task 7.3.2:** Plan to update the client-side code in these services/tools to use the new `/v1/...` paths for all calls to `form-state`.
    - [x] **Sub-task 7.3.3:** Plan to update any API documentation or user-facing examples for `form-state`.
- [ ] **Task 7.4: Testing API Versioning (Manual & Automated)**
    - [ ] **Sub-task 7.4.1:** After implementation, test all `form-state` API endpoints using the new `/v1/...` paths to ensure they function as before.
    - [ ] **Sub-task 7.4.2:** Verify that calls to old paths (without `/v1/`) now correctly result in 404 Not Found errors.
    - [ ] **Sub-task 7.4.3:** Conduct integration tests to ensure inter-service communication (where other services call `form-state`) works correctly with the versioned paths.

---

## Phase 8: API Versioning for Other Services

**Goal:** Introduce API versioning (v1) for other core service APIs like `form-pack`, `form-vmm-service`, etc., for consistency and future compatibility.

### Task 8.1: Version `form-pack` API Endpoints
- [x] **Sub-task 8.1.1 (Plan):** Identify `form-pack` API routes in `form-pack/src/helpers/api/mod.rs`.
    - *Detail: Routes `/ping`, `/health`, `/build`, `/:build_id/get_status` identified.*
- [x] **Sub-task 8.1.2 (Implement):** Modify `build_routes` in `form-pack/src/helpers/api/mod.rs` to nest all routes under `/v1`.
    - *Detail: Implemented by creating `core_api_routes` and returning `Router::new().nest("/v1", core_api_routes)`.*
- [ ] **Sub-task 8.1.3 (Planning):** Identify and plan updates for any clients of `form-pack` API (e.g., CLIs, other services if any).
- [ ] **Sub-task 8.1.4 (Test):** Test new `/v1/...` `form-pack` endpoints and ensure old paths are 404.

### Task 8.2: Version `form-vmm-service` API Endpoints
- [x] **Sub-task 8.2.1 (Plan):** Identify `form-vmm-service` API routes (likely in `form-vmm-service/src/api/mod.rs` or similar).
    - *Detail: `public_routes` and `protected_routes` identified in `start_api_server`.*
- [x] **Sub-sub-task 8.2.2 (Implement):** Plan to nest all its routes under `/v1` similar to `form-state` and `form-pack`.
    - *Detail: Implemented by merging existing routers into `v1_routes` and nesting under `/v1` in `start_api_server`.*
- [ ] **Sub-sub-task 8.2.3 (Planning):** Identify clients and plan for their updates.
    - *Detail: Clients like `form-state` (for `BootComplete`) or any direct CLI/UI tools would need updates.*

### Task 8.3: Version `form-dns` API Endpoints
- [x] **Sub-task 8.3.1 (Plan):** Identify `form-dns` API routes.
    - *Detail: Routes like `/record/...`, `/bootstrap/...` identified in `form-dns/src/api.rs`.*
- [ ] **Sub-sub-task 8.3.2 (Plan):** Plan to nest all its routes under `/v1` in `form-dns/src/api.rs`.
    - *Detail: Modify `build_routes` to consolidate current routes and nest under a new top-level `/v1` router.*
- [ ] **Sub-sub-task 8.3.3 (Planning):** Identify clients (`form-state`, `form-config-wizard`) and plan updates to their API calls to use `/v1` paths.
- [ ] **Sub-task 8.3.4 (Implement):** (To be done after planning approval)
    - Implement changes in `form-dns/src/api.rs`.
    - Implement changes in client call sites in `form-state` and `form-config`.
- [ ] **Sub-task 8.3.5 (Test):** Test new `/v1/...` `form-dns` endpoints and client interactions.

### Task 8.4: Version `form-net` API Endpoints (Planning - if applicable for `formnet-server`)
- [ ] **Sub-task 8.4.1 (Plan):** Identify any `formnet-server` API routes exposed for peer joining or management.
- [ ] **Sub-sub-task 8.4.2 (Plan):** Plan to nest these under `/v1` if applicable.
- [ ] **Sub-sub-task 8.4.3 (Planning):** Identify clients (e.g., `formnet` CLI, other nodes) and plan for their updates.