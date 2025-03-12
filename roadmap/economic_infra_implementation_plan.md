# Economic Infrastructure: Detailed Implementation Plan

This document provides a comprehensive implementation plan for the Economic Infrastructure backend, based on analysis of the existing codebase. It identifies what components are already implemented, what needs to be built, and how everything will integrate.

## 1. Existing Components Analysis

### A. Resource Usage Measurement (Mostly Implemented)

The `form-vm-metrics` crate already implements most of the basic resource measurement functionality:

- ✅ CPU usage tracking (via `cpu.rs`)
- ✅ Memory usage tracking (via `mem.rs`)
- ✅ Disk usage tracking (via `disk.rs`)
- ✅ Network bandwidth tracking (via `network.rs`)
- ✅ GPU usage tracking (via `gpu.rs`)
- ✅ System load tracking (via `load.rs`)
- ✅ Collection interval (set to 30 seconds in `main.rs`)
- ✅ RESTful API endpoint for current metrics (via `/get` endpoint in `main.rs`)

### B. Event Emission (Partially Implemented)

Several event emission mechanisms exist in the codebase, though not specifically for usage metrics:

- ✅ Event logging framework exists in `form-vmm/event_monitor`
- ✅ Message queue integration exists in `form-p2p/queue.rs`
- ✅ `write_to_queue` utility in `form-node-metrics/src/util.rs`
- ❌ No specific usage event schema for billing/usage tracking
- ❌ No retry/reliability mechanisms for usage event emission

### C. Threshold Detection (Not Implemented)

- ❌ No specific implementation for threshold configuration and detection

### D. API Layer (Partially Implemented)

- ✅ Basic RESTful API exists for current metrics in `form-vm-metrics/main.rs`
- ✅ Account management API exists in `form-state/src/datastore.rs`
- ❌ No specific integration between metrics and account/instance ownership

## 2. Required New Components

### A. Resource Usage Event Schema

We need to create a new event schema specifically for usage events that includes:

```rust
pub struct UsageEvent {
    // Event metadata
    pub event_type: String,  // "resource_usage"
    pub version: String,     // "1.0"
    pub timestamp: i64,      // Unix timestamp
    
    // Identity information
    pub instance_id: String,
    pub user_id: String,     // From account information
    pub org_id: Option<String>, // If available
    
    // Metrics data
    pub metrics: UsageMetrics,
    
    // Time period
    pub period: UsagePeriod,
}

pub struct UsageMetrics {
    pub cpu_seconds: u64,
    pub cpu_percent_avg: f64,
    pub memory_gb: f64,
    pub memory_percent: f64,
    pub storage_gb: f64,
    pub network_egress_mb: f64,
    pub network_ingress_mb: f64,
    pub gpu_seconds: u64,
}

pub struct UsagePeriod {
    pub start: i64, // Unix timestamp
    pub end: i64,   // Unix timestamp
}
```

### B. Message Queue Integration

We'll need to enhance the existing queue mechanism to:

1. Convert SystemMetrics to UsageEvent format
2. Add instance and user/org identification
3. Implement retry logic and circuit breaking
4. Add monitoring for event emission

### C. Threshold Configuration System

We'll need to create:

1. Threshold configuration storage
2. API for setting/updating thresholds
3. Stateless threshold checking
4. Notification event emission

## 3. Integration Points

### A. Account Service Integration

- Integrate with existing account management API in `form-state/src/accounts.rs`
- Use the account information to include user/org IDs in usage events
- Implement instance ownership verification via the accounts system

### B. Message Queue Integration

- Use the existing message queue in `form-p2p/queue.rs`
- Create a new topic specifically for usage events
- Implement the publishing interface

### C. Current Metrics API

- Extend the existing API in `form-vm-metrics/main.rs`
- Add authentication and authorization
- Implement filtering by instance/user

## 4. Implementation Plan by Module

### A. `form-vm-metrics` Enhancements

1. **Update SystemMetrics struct to include additional fields**
   - `instance_id: String`
   - `account_id: Option<String>`

2. **Implement event emission**
   - Create new module `events.rs`
   - Implement conversion from SystemMetrics to UsageEvent
   - Add retry logic for event publication

3. **Add threshold detection**
   - Create new module `thresholds.rs`
   - Implement configuration loading
   - Add stateless threshold checking

### B. New Crate: `form-usage-events`

1. **Define event schema**
   - Create all necessary structs for usage events
   - Implement serialization/deserialization
   - Add versioning support

2. **Implement publication mechanism**
   - Create abstraction over message queue
   - Implement retry and circuit breaking
   - Add monitoring

### C. API Enhancements

1. **Add authentication to metrics API**
   - Integrate with existing auth mechanism
   - Add authorization checks based on instance ownership

2. **Implement new API endpoints**
   - Add user/org filtering capabilities
   - Implement health check endpoint

## 5. Dependencies and Infrastructure

### Required Libraries:
- `serde` and `serde_json` - Already used throughout the codebase
- `tokio` - Already used for async runtime
- `axum` - Already used for API in form-vm-metrics
- Circuit breaker library - Either implement custom or use `circuit_breaker` crate

### Infrastructure Requirements:
- Message queue - Already implemented in `form-p2p`
- Account service - Already implemented in `form-state`

## 6. Implementation Sequence

### Phase 1: Core Measurement and Event Schema (Week 1)
1. Create usage event schema in new `form-usage-events` crate
2. Update `form-vm-metrics` to collect instance/account info
3. Implement basic event emission without retries

### Phase 2: Reliability and Integration (Week 2)
1. Implement retry mechanism for event emission
2. Add circuit breaking for failure handling
3. Integrate with account service for user/org information

### Phase 3: Threshold Detection and API (Week 3)
1. Implement threshold configuration and detection
2. Enhance API with auth and filtering
3. Add health check endpoints

### Phase 4: Testing and Validation (Week 4)
1. Implement comprehensive unit tests
2. Create integration tests with mock consumers
3. Set up performance testing for event emission

## 7. Testing Strategy

### Unit Tests:
- Test metrics collection accuracy
- Test event serialization/deserialization
- Test threshold detection logic
- Test retry mechanisms

### Integration Tests:
- Test end-to-end flow with mock event consumers
- Test account integration
- Test threshold notifications

### Performance Tests:
- Test high-frequency event emission
- Test event pipeline under load
- Test recovery from failure scenarios 