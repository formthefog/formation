# Formation Service Discovery Design

This document outlines the service discovery approach for the Formation microservices platform, enabling dynamic registration, discovery, and health monitoring of services.

## 1. Service Discovery Approach

### 1.1 Client-side Service Discovery with Registry

The Formation platform will implement a client-side service discovery pattern with a centralized service registry:

- **Centralized Registry**: A dedicated service registry will maintain information about all available services
- **Client-side Resolution**: Clients will query the registry to locate services
- **Dynamic Updates**: Services will register and update their status in real-time
- **Metadata Support**: Registry will store service metadata, including capabilities and dependencies

### 1.2 Registry Implementation

The service registry will be implemented as a dedicated component within the Formation Admin service with the following characteristics:

- **High Availability**: Registry service designed for high availability with redundancy
- **Performance Optimized**: Fast read access with caching mechanisms
- **Consistency**: Eventual consistency model with conflict resolution
- **API-Driven**: RESTful API for registration and discovery operations
- **Event Notifications**: Change events for service status updates

### 1.3 Service Resolution Strategies

Multiple resolution strategies will be supported:

- **Direct Resolution**: Lookup by service ID or name
- **Tag-based Resolution**: Lookup by service tags or capabilities
- **Dependency Resolution**: Lookup based on service dependencies
- **Weighted Resolution**: Support for load balancing with service weighting
- **Health-aware Routing**: Consideration of service health status in resolution

## 2. Service Registration Process

### 2.1 Initial Registration

Services will register with the discovery system during startup through the following process:

1. **Service Initialization**: Service starts and prepares registration information
2. **Registry Connection**: Service establishes connection to the registry service
3. **Registration Request**: Service sends registration payload with metadata
4. **Validation**: Registry validates the registration data
5. **Confirmation**: Registry confirms successful registration
6. **Service Activation**: Service marks itself as ready to receive requests

### 2.2 Registration Payload

```json
{
  "serviceId": "form-state-01",
  "name": "form-state",
  "instanceId": "i-1234567890abcdef0",
  "version": "1.5.0",
  "status": "starting",
  "addresses": [
    {
      "type": "ipv4",
      "address": "172.28.0.3",
      "network": "docker"
    }
  ],
  "endpoints": [
    {
      "name": "api",
      "protocol": "http",
      "port": 3004,
      "path": "/",
      "healthCheck": "/health"
    }
  ],
  "metadata": {
    "startTime": "2023-06-15T10:30:45Z",
    "capabilities": ["database", "state-management"],
    "environment": "production",
    "region": "us-west-1"
  },
  "dependencies": [
    {
      "name": "form-dns",
      "required": true
    }
  ],
  "healthCheck": {
    "path": "/health",
    "interval": 30,
    "timeout": 5,
    "retries": 3
  }
}
```

### 2.3 Registration Maintenance

- **Heartbeats**: Services will send periodic heartbeats to maintain registration
- **TTL-based Expiry**: Registrations expire if not renewed within TTL period
- **Metadata Updates**: Services can update their metadata without full re-registration
- **Graceful Deregistration**: Services deregister when shutting down gracefully

### 2.4 Registry Persistence

- **In-memory Cache**: Fast access through in-memory data structure
- **Persistent Storage**: Backup to database for recovery purposes
- **Change Log**: Journal of registration changes for auditing
- **Snapshot Mechanism**: Periodic state snapshots for faster recovery

## 3. Health Check Requirements

### 3.1 Health Check Types

Multiple health check mechanisms will be supported:

- **HTTP Check**: Regular HTTP(S) requests to a health endpoint
- **TCP Check**: TCP connection establishment test
- **Command Check**: Execution of a command within the service container
- **Dependency Check**: Verification of dependency health status
- **Custom Checks**: Support for service-specific health validation logic

### 3.2 Health Status Definitions

- **Passing**: Service is healthy and operating normally
- **Warning**: Service is operating with degraded capabilities
- **Critical**: Service is not functioning properly
- **Unknown**: Health status could not be determined
- **Starting**: Service is in startup process
- **Stopping**: Service is in shutdown process

### 3.3 Health Check Configuration

```json
{
  "healthCheck": {
    "id": "form-state-api",
    "name": "Form State API Check",
    "serviceId": "form-state-01",
    "type": "http",
    "endpoint": "http://172.28.0.3:3004/health",
    "interval": 30,
    "timeout": 5,
    "retries": 3,
    "startPeriod": 60,
    "successCodes": [200],
    "method": "GET",
    "headers": {},
    "expectedOutput": "",
    "tlsSkipVerify": false
  }
}
```

### 3.4 Health Check Process

1. **Scheduled Execution**: Health checks run at configured intervals
2. **Result Evaluation**: Check results evaluated against success criteria
3. **Status Update**: Service status updated based on check results
4. **Status Propagation**: Status changes propagated to interested parties
5. **Alerting**: Critical status changes trigger alerts
6. **Automatic Recovery**: Optional automatic recovery for failed services

## 4. Fallback Mechanisms

### 4.1 Registry Unavailability

When the service registry is temporarily unavailable, the system will employ these fallback mechanisms:

- **Local Cache**: Clients maintain a local cache of service locations
- **Last Known Good**: Use last known good configuration when registry is unavailable
- **Static Configuration**: Fall back to static configuration as last resort
- **Exponential Backoff**: Retry registry connection with backoff
- **Circuit Breaking**: Prevent cascading failures with circuit breakers

### 4.2 Service Unavailability

When a required service is unavailable, the following strategies will be applied:

- **Retry with Backoff**: Attempt to reconnect with exponential backoff
- **Alternative Instances**: Try alternative instances of the same service
- **Degraded Operation**: Continue with reduced functionality if possible
- **Graceful Degradation**: Disable non-critical features dependent on the service
- **Clear Error Reporting**: Provide clear error messages about unavailable dependencies

### 4.3 Network Partitioning

In case of network partitioning:

- **Partition Awareness**: System detects and adapts to network partitions
- **Split-brain Prevention**: Consensus protocols to prevent split-brain scenarios
- **Reconciliation**: Data reconciliation when partitions heal
- **Local-first Operations**: Prioritize operations that can complete locally
- **Eventual Consistency**: Ensure eventual consistency across partitions

### 4.4 Recovery Procedures

- **Automatic Recovery**: Self-healing mechanisms for common failures
- **Service Resurrection**: Automatic restart of failed services
- **State Recovery**: Procedures for state recovery after failures
- **Warm Standby**: Maintain warm standby instances for critical services
- **Recovery Coordination**: Coordinated recovery to maintain system integrity

## 5. Implementation Technologies

### 5.1 Registry Implementation

The service registry will be implemented using:

- **Backend**: Rust-based microservice with high-performance data structures
- **Storage**: SQLite for persistence with in-memory caching
- **API**: RESTful API with WebSocket for real-time updates
- **Integration**: Direct integration with Formation Admin Tool

### 5.2 Client Libraries

Client libraries will be provided for service integration:

- **Rust Client**: Native Rust client for Formation services
- **Language-specific SDKs**: Clients for other languages as needed
- **Configuration Adapters**: Integration with common configuration systems
- **Health Check Implementations**: Standard health check implementations

### 5.3 Monitoring Integration

Integration with monitoring systems:

- **Prometheus Integration**: Metrics exported in Prometheus format
- **Logging**: Structured logging of discovery events
- **Alerting**: Integration with alerting systems
- **Visualization**: Dashboard visualizations of service topology
- **Audit Trail**: Comprehensive audit trail of service lifecycle events 