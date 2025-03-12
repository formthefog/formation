# Economic Infrastructure: Integration Plan

This document provides guidance for teams integrating with the Formation compute backend economic infrastructure. It outlines the architecture, interfaces, and responsibilities for each team to create a cohesive economic system.

## System Architecture Overview

The Formation economic infrastructure follows an event-driven architecture with the following components:

1. **Compute Backend (This Repository)** - Responsible for resource usage monitoring and frequent event emission (every 30 seconds)
2. **Message Queue** - Handles reliable delivery of usage events to consuming services
3. **Usage Database** - Stores historical usage data and provides aggregation capabilities
4. **Accounts Database** - Manages user accounts, permissions, and credit balances
5. **Blockchain Integration** - Handles tokenization, staking, and on-chain settlements
6. **Billing Service** - Processes usage events and generates invoices
7. **Admin Dashboard** - Provides management interface for system administration
8. **User Portal** - Offers self-service billing management for end-users

```
┌────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│ Compute Backend│────►│  Message Queue  │────►│ Usage Database  │
└────────────────┘     └─────────────────┘     └─────────────────┘
                              │                        ▲
                              │                        │
                              ▼                        │
┌────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│ Accounts DB    │◄───►│ Billing Service │◄────┤ Admin Dashboard │
└────────────────┘     └─────────────────┘     └─────────────────┘
        ▲                      ▲                        ▲
        │                      │                        │
        │                      │                        │
┌────────────────┐             │               ┌─────────────────┐
│ Blockchain     │◄────────────┘               │ User Portal     │
└────────────────┘                             └─────────────────┘
```

## Team Responsibilities

### Compute Backend Team

**Provides:**
- Point-in-time resource usage metrics (CPU, memory, storage, network, GPU)
- Frequent usage event emission (every 30 seconds)
- Current usage data API (no historical data)
- Stateless threshold checking against current metrics

**Consumes:**
- User account information (from Accounts DB)
- Threshold configuration (from Admin Dashboard)

### Usage Database Team

**Provides:**
- Long-term storage of all usage events
- Time-series aggregation capabilities (hourly, daily, monthly)
- User and organization level aggregation
- Historical usage data querying
- Data retention policies and optimization

**Consumes:**
- Usage events (from Message Queue)
- User and organization structure (from Accounts DB)

### Accounts Database Team

**Provides:**
- User identity and authentication
- Organization structure and permissions
- Account balance information
- Account activity history

**Consumes:**
- Usage-based credit deductions (from Billing Service)
- Account status updates (from Blockchain)

### Blockchain Integration Team

**Provides:**
- Token contract implementation on Ethereum
- On-chain settlement of large transactions
- Staking mechanisms for node operators
- Cryptographic verification of payments

**Consumes:**
- Account deposit/withdrawal requests (from User Portal)
- Large settlement requests (from Billing Service)

### Billing Service Team

**Provides:**
- Usage-to-invoice processing
- Credit deduction calculations
- Resource tier pricing management
- Discount and promotion management

**Consumes:**
- Usage events (from Message Queue)
- Historical usage data (from Usage Database)
- Account status and balances (from Accounts DB)
- Payment processing results (from Payment Processor)

### User Portal Team

**Provides:**
- Self-service billing management interface
- Usage and cost visualization
- Payment method management
- Invoice access and export

**Consumes:**
- Current usage data (from Compute Backend)
- Historical usage data (from Usage Database)
- Account information (from Accounts DB)
- Invoice data (from Billing Service)

### Admin Dashboard Team

**Provides:**
- System-wide economic metrics
- Manual intervention tools
- Threshold configuration interface
- Pricing management tools

**Consumes:**
- Current usage data (from Compute Backend)
- Historical usage data (from Usage Database)
- System status information (from all services)
- Account administration (from Accounts DB)

## Integration Points

### 1. Usage Events

**Publisher:** Compute Backend
**Consumer:** Message Queue → Usage Database, Billing Service
**Format:** JSON over Message Queue
**Frequency:** Every 30 seconds

Example event:
```json
{
  "event_type": "resource_usage",
  "version": "1.0",
  "timestamp": "2023-07-15T14:30:00Z",
  "user_id": "user_123456",
  "org_id": "org_789012",
  "instance_id": "instance_abcdef",
  "metrics": {
    "cpu_seconds": 30,
    "cpu_percent_avg": 12.5,
    "memory_gb": 4.2,
    "memory_percent": 52.5,
    "storage_gb": 25.7,
    "network_egress_mb": 15.2,
    "network_ingress_mb": 8.7,
    "gpu_seconds": 0
  },
  "period": {
    "start": "2023-07-15T14:29:30Z",
    "end": "2023-07-15T14:30:00Z"
  }
}
```

### 2. Current Usage API

**Provider:** Compute Backend
**Consumer:** User Portal, Admin Dashboard
**Format:** RESTful API
**Authentication:** JWT or API Key
**Scope:** Current point-in-time metrics only (no history)

Example endpoints:
- `GET /api/v1/usage/instances/{instance_id}/current` - Get current usage for specific instance
- `GET /api/v1/usage/users/{user_id}/current` - Get current usage across all user instances

### 3. Historical Usage API

**Provider:** Usage Database
**Consumer:** User Portal, Admin Dashboard, Billing Service
**Format:** RESTful API
**Authentication:** JWT or API Key
**Scope:** Historical aggregated data with filtering capabilities

Example endpoints:
- `GET /api/v1/usage/instances/{instance_id}/history` - Get historical usage for specific instance
- `GET /api/v1/usage/users/{user_id}/history` - Get historical usage across all user instances
- `GET /api/v1/usage/organizations/{org_id}/history` - Get organization-wide historical usage

### 4. Account Information API

**Provider:** Accounts DB
**Consumer:** Compute Backend, Billing Service, Usage Database
**Format:** RESTful API or gRPC
**Authentication:** Service-to-service auth

Example endpoints:
- `GET /api/v1/accounts/{account_id}` - Get account details
- `GET /api/v1/accounts/{account_id}/balance` - Get current balance
- `POST /api/v1/accounts/{account_id}/deduct` - Deduct credits

### 5. Threshold Configuration

**Provider:** Admin Dashboard
**Consumer:** Compute Backend
**Format:** Configuration API
**Authentication:** Admin credentials

Example threshold configuration:
```json
{
  "resource_type": "storage",
  "threshold_type": "absolute",
  "threshold_value": 500,
  "units": "GB",
  "action": "notify",
  "user_id": "user_123456",
  "notification_channels": ["email", "api_callback"]
}
```

## Implementation Timeline and Dependencies

### Phase 1: Core Metering and Events (Week 1-2)
- Compute Backend: Implement basic usage metrics collection and event emission
- Message Queue: Set up infrastructure
- Usage Database: Implement initial storage schema and ingestion

### Phase 2: Integration Interfaces (Week 3-4)
- Compute Backend: Implement current usage API
- Usage Database: Implement historical data API
- Billing Service: Develop event consumption

### Phase 3: Advanced Features (Week 5-8)
- Usage Database: Implement aggregation capabilities
- Blockchain Integration: Implement token contracts
- Admin Dashboard: Build reporting and management tools
- All Teams: End-to-end integration testing

## Testing Strategy

1. **Unit Tests** - Each component team tests their own services
2. **Integration Tests** - Pairs of teams test direct integrations
3. **End-to-End Tests** - Full system tests with all components
4. **Synthetic Load Tests** - Simulated usage patterns at scale
5. **Data Consistency Tests** - Verify data integrity across systems

## Communication Channels

- **API Documentation:** Swagger/OpenAPI hosted at [internal-docs-url]
- **Event Schema Registry:** Available at [schema-registry-url]
- **Integration Questions:** #economic-infra Slack channel
- **Weekly Sync:** Thursdays at 10am (all teams) 