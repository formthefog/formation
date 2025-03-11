# Economic Infrastructure: Backend Implementation Tasks

## 1. Resource Usage Measurement System

### Metrics Collection
- [x] Implement CPU usage tracking per VM instance *(implemented in form-vm-metrics/cpu.rs)*
- [x] Implement memory usage tracking per VM instance *(implemented in form-vm-metrics/mem.rs)*
- [x] Implement storage usage tracking per VM instance *(implemented in form-vm-metrics/disk.rs)*
- [x] Implement network bandwidth tracking *(implemented in form-vm-metrics/network.rs)*
- [x] Implement GPU usage tracking *(implemented in form-vm-metrics/gpu.rs)*
- [x] Create configurable sampling intervals *(implemented in form-vm-metrics/main.rs, set to 30 seconds)*
- [x] Extend point-in-time metrics to include instance and account information *(implemented in form-vm-metrics/system.rs)*

### Short-Term Metrics Buffer
- [x] Implement minimal short-term buffer for latest metrics *(implemented in form-vm-metrics/main.rs using an Arc<Mutex<SystemMetrics>>)*
- [ ] Create sliding window mechanism to handle collection failures
- [ ] Implement efficient buffer pruning to prevent state growth
- [ ] Build recovery mechanism for missed measurement intervals

## 2. Usage Event Emission System

### Event Structure
- [x] Design lightweight usage event schema with essential properties only *(implemented in form-usage-events/src/events.rs)*
- [x] Implement versioning for event schema to support future changes *(implemented with version field in UsageEvent)*
- [x] Create efficient serialization mechanisms for events *(implemented using serde in form-usage-events)*
- [x] Include user ID and organization ID in event structure for external aggregation *(implemented in UsageEvent struct)*

### Event Publishing
- [x] Implement reliable event emission at regular intervals *(implemented in form-vm-metrics/src/main.rs, emitting every 30 seconds)*
- [x] Create retry mechanisms for failed event publishing *(implemented in form-usage-events/src/retry.rs with exponential backoff and jitter)*
- [x] Implement circuit breaking for event destination outages *(implemented in form-usage-events/src/circuit_breaker.rs)*
- [ ] Implement batching for failed events to ensure delivery
- [ ] Build dead-letter queue for unprocessable events
- [ ] Create event emission metrics and monitoring 

## 3. Threshold Detection (Stateless)

### Configuration
- [x] Implement configuration for threshold definitions from external source *(implemented in form-usage-events/src/threshold.rs)*
- [x] Create API for accepting threshold updates *(implemented via ThresholdManager in form-usage-events)*
- [x] Build lightweight caching of threshold configurations *(implemented using Arc<RwLock<HashMap>> in ThresholdManager)*

### Monitoring and Alerting
- [x] Implement stateless threshold checking against current metrics *(implemented in ThresholdManager::check_thresholds)*
- [x] Create notification event emission when thresholds are approached/exceeded *(implemented in ThresholdManager::process_violations)*
- [x] Implement circuit breaking for notification services *(reused the same circuit breaking pattern)*
- [x] Build throttling mechanisms to prevent notification storms *(managed by time-based thresholds in the checking logic)*

## 4. Minimal API Layer

### Current API Improvements
- [x] Design RESTful API endpoints for retrieving current usage data *(basic endpoint exists at /get in form-vm-metrics/main.rs)*
- [x] Build health check endpoints for monitoring system components *(implemented basic /health and detailed /api/v1/health/status endpoints)*
- [x] Create API documentation with examples and usage guidelines *(created comprehensive docs in API_DOCUMENTATION.md files)*

### Integration API
- [x] Create webhook registration for real-time usage events *(implemented registration, listing, and deletion endpoints at /api/v1/webhooks)*

## 5. Message Queue Integration
- [x] Leverage existing message queue infrastructure *(form-p2p/queue.rs has the necessary components)*
- [x] Create a topic specifically for usage events *(implemented in form-usage-events/src/publish.rs)*
- [x] Implement reliable publishing to the queue *(implemented in form-usage-events/src/publish.rs and integrated with form-vm-metrics)*

## 6. Future Improvements

### Testing and Validation
- [ ] Write comprehensive tests for all metric collection methods
- [ ] Create tests for event emission and serialization
- [ ] Implement threshold detection tests
- [ ] Build API endpoint tests
- [ ] Create end-to-end tests with mock event consumers
- [ ] Implement performance tests for high-frequency event emission
- [ ] Build tests for handling event destination failures
- [ ] Create metrics collection accuracy validation tests

### Deployment and Operations
- [ ] Implement health checks for event emission pipeline
- [ ] Create metrics for tracking event emission success rates
- [ ] Build alerting for event publishing failures
- [ ] Create documentation for system configuration
- [ ] Build deployment scripts and CI/CD integration

### Account Service Integration
- [ ] Integrate with the existing account service API *(form-state/src/accounts.rs)*
- [ ] Retrieve user/organization information for instances
- [ ] Verify instance ownership when serving metrics

### Enhanced API Features
- [ ] Implement authentication and authorization for API access
- [ ] Create filtering parameters for VM-specific current usage
- [ ] Implement rate limiting for API endpoints
- [ ] Implement advanced filtering and aggregation in the API
- [ ] Create query language for complex metric queries
- [ ] Build visualization endpoints for dashboard integration

### Enhanced Event Publishing
- [ ] Build dead-letter queue for unprocessable events
- [ ] Create event emission metrics and monitoring
- [ ] Implement batching for failed events to ensure delivery

### Advanced Buffering
- [ ] Implement configurable retention policies for metrics buffer
- [ ] Create data compaction for long-running instances
- [ ] Build disk-based fallback for memory buffer overflow 