# Commit Command Implementation Plan

## Overview

The `commit` command allows a developer to commit changes made to a VM instance and propagate those changes to other instances in the same cluster. This provides a mechanism for developers to make changes to one instance and then replicate those changes across all replicated instances, maintaining consistency within the cluster.

## Workflow

1. Developer makes changes to a VM instance (e.g., through console access)
2. Developer executes `form manage commit` with the ID of the modified instance
3. The system:
   - Takes a snapshot of the modified instance
   - Identifies other instances in the same cluster
   - Distributes the snapshot to the nodes hosting those instances
   - Replaces the existing instances with the updated snapshot

## Implementation Components

### 1. CLI Command Structure

```rust
#[derive(Clone, Debug, Args)]
pub struct CommitCommand {
    /// The ID of the instance that has been modified
    #[clap(long, short)]
    pub id: Option<String>,
    
    /// The name of the instance that has been modified, an alternative to ID
    #[clap(long, short)]
    pub name: Option<String>,
    
    /// A hexadecimal or base64 representation of a valid private key for 
    /// signing the request
    #[clap(long, short)]
    pub private_key: Option<String>,
    
    /// An alternative to private key or mnemonic
    #[clap(long, short)]
    pub keyfile: Option<String>,
    
    /// An alternative to private key or keyfile - BIP39 mnemonic phrase
    #[clap(long, short)]
    pub mnemonic: Option<String>,
    
    /// Description for the commit (optional)
    #[clap(long)]
    pub description: Option<String>,
}
```

### 2. Backend API Endpoint

Add a new endpoint to the VMM service API:

```rust
// In form-vmm/vmm-service/src/api/mod.rs
#[post("/api/v1/commit")]
async fn commit(
    State(state): State<AppState>,
    Json(request): Json<CommitVmRequest>,
) -> impl IntoResponse {
    // Implementation
}
```

### 3. Types and Structures

```rust
// In form-types/src/lib.rs
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommitVmRequest {
    pub id: Option<String>,
    pub name: Option<String>,
    pub signature: Option<String>, 
    pub address: Option<String>,
}

// In form-vmm/vmm-service/src/service/vmm.rs
pub enum VmmEvent {
    // ...existing events...
    Commit { id: String, snapshot_id: String },
}
```

### 4. Implementation Phases

#### Phase 1: Instance Validation and Preparation

1. Validate the instance ID/name and ownership (using signature)
2. Verify the instance exists and is in a valid state for commit
3. Pause the VM to prepare for snapshot
4. Add metadata to the commit (description, timestamp, etc.)

#### Phase 2: Snapshot Creation

1. Use the VMM's snapshot functionality to create a snapshot of the instance
2. Store the snapshot with appropriate metadata
3. Update the instance's `last_snapshot` timestamp
4. Record the snapshot in the instance's `snapshots` history

#### Phase 3: Cluster Instance Discovery

1. Query the form-state datastore to retrieve information about the instance
2. Use the `cluster_members()` method to get all instances in the cluster
3. Filter out the instance being committed (source instance)
4. Group instances by node to optimize distribution

#### Phase 4: Snapshot Distribution

1. For each target node:
   - Establish connection to the node
   - Transfer the snapshot files
   - Send a request to apply the snapshot to all instances on that node

#### Phase 5: Instance Replacement

1. For each target instance:
   - Pause the target instance
   - Apply the snapshot
   - Resume the instance
   - Verify the replacement succeeded
   - Update the instance state in form-state

#### Phase 6: Status Updates and Cleanup

1. Update the form-state database with the new snapshot information
2. Add commit information to the instance's metadata
3. Clean up temporary snapshot files if needed
4. Return success/failure status to the user

## Key Considerations

### Security

1. Only the instance owner or authorized users should be able to commit changes
2. Signatures should be validated to ensure proper authorization
3. Sensitive information in snapshots should be handled securely

### Performance

1. Snapshots may be large - consider compression and chunked transfers
2. Distribute snapshots in parallel where possible
3. Implement timeout mechanisms for long-running operations

### Reliability

1. Implement proper error handling for each phase
2. Add recovery mechanisms for failed snapshot transfers
3. Consider a transactional approach to ensure all instances are updated together or none are
4. Add logging throughout the process for debugging

### User Experience

1. Provide clear progress indicators during the potentially long-running operation
2. Supply detailed error messages if the process fails
3. Allow for rollback of commits if issues arise

## Implementation Strategy

1. Start with a basic implementation of the CLI command and handler
2. Add the backend API endpoint that creates and manages a snapshot
3. Implement the snapshot distribution mechanism
4. Add cluster instance discovery and replacement logic
5. Implement proper error handling and status reporting
6. Add security measures and validation
7. Test extensively with various cluster configurations

## Future Enhancements

1. Support commit rollbacks
2. Allow selective commits to specific instances in a cluster
3. Add scheduling for commits during low-traffic periods
4. Implement differential snapshots to reduce data transfer
5. Add history tracking for commits with the ability to view and revert changes

## Integration Points

1. CLI command in `form-cli/src/dev/manage/commit.rs`
2. API endpoint in `form-vmm/vmm-service/src/api/mod.rs`
3. VMM events handling in `form-vmm/vmm-service/src/service/vmm.rs`
4. Snapshot functionality in `form-vmm/vmm/src/vm.rs`
5. Form-state interaction for cluster information
6. Node-to-node communication for snapshot distribution 