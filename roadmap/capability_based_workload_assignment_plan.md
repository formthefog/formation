# Capability-Based Workload Assignment Plan

## Overview

This document outlines the plan for implementing the capability-based node selection system for workloads in the Formation network. This involves two key projects:

1. **Primary Project**: Utilize node metrics data to determine which nodes are capable of handling a specific workload and implementing a deterministic selection mechanism to decide which node is responsible for execution.

2. **Future Project** (roadmap only): Enable nodes to cluster together to support workloads that exceed the capacity of a single node.

## Current Architecture

- `form-node-metrics`: Collects and reports node capabilities and capacity (CPU, memory, storage, GPU, etc.) to `form-state` regularly.
- `form-pack`: Handles build requests and processes Docker/image creation.
- `form-state`: Stores and manages node metrics data from all nodes in the network.

## Primary Project Implementation Plan

### Goals

1. Use the capabilities and metrics data already being collected to filter nodes capable of handling a workload.
2. Implement a deterministic algorithm to select which node is responsible for a given workload.
3. Ensure the form-pack-manager only processes workloads it is capable of and responsible for.

### Node Selection Algorithm

The algorithm for selecting nodes will work as follows:

1. Filter all nodes based on both **capability** (what the node can do) and **capacity** (what resources are currently available)
2. For each node that passes the filter, compute XOR(build_id, node_id)
3. Sort the results by value (lowest to highest)
4. By default, the node with the lowest XOR value is responsible for the workload
5. For clustering (future implementation):
   - If there are fewer than 3 capable nodes, all available nodes form the cluster
   - If there are 3 or more capable nodes, the 3 nodes with the lowest XOR values form the cluster

This approach is:
- **Fast**: Simple bitwise operation with minimal computation
- **Scalable**: Works with any number of nodes
- **Deterministic**: Same inputs always produce the same node selection
- **Consistent**: All nodes make the same decision due to shared CRDT-based state
- **Flexible**: Works with any size network, including development environments with few nodes

### Tasks

#### Task 1: Define Workload Requirements Structure
- Create a structure to define resource requirements for a workload
- Define CPU, memory, storage, GPU, and other requirements
- Ensure these requirements can be compared against node capabilities and capacity

**Acceptance Criteria**:
- A well-defined data structure for workload requirements
- Requirements structure can be serialized/deserialized for network transmission
- Documentation for the requirements structure

#### Task 2: Implement Capability and Capacity Matching Logic in form-pack-manager
- Create a module to evaluate if a node can handle specific workload requirements
- Add logic to check CPU, memory, storage, GPU requirements against local node capabilities
- Add logic to check if currently available resources (capacity) are sufficient
- Provide clear logging for capability and capacity decisions

**Acceptance Criteria**:
- Logic correctly evaluates if a node can handle a workload based on requirements
- Both capabilities (what the node can do) and capacity (currently available resources) are checked
- All relevant resource types (CPU, memory, storage, GPU) are checked
- Log output clearly indicates why a node is or isn't capable/available

#### Task 3: Implement Node Selection Algorithm in form-pack-manager
- Implement the XOR-based algorithm for node selection
- Retrieve node data from form-state to identify all capable nodes
- Determine if the local node is responsible based on having the lowest XOR value
- Prepare for future clustering by ranking nodes (lowest XOR values first)

**Acceptance Criteria**:
- Algorithm correctly identifies the responsible node using XOR(build_id, node_id)
- Node with lowest XOR value is selected as responsible
- Even distribution of workloads across capable nodes is achieved
- Foundation is in place for future clustering (identifying top 3 or more nodes)

#### Task 4: Integrate with form-pack Build Process
- Update the PackRequest structure to include workload requirements
- Modify form-pack-manager to check capability, capacity, and responsibility before processing
- Add proper error handling and status reporting

**Acceptance Criteria**:
- form-pack-manager only processes workloads it should handle
- Build requests include workload requirements
- Local node correctly determines if it is responsible for a given workload
- Proper errors are returned for incapable nodes
- Status is reported correctly for capability-based decisions

#### Task 5: Testing
- Unit tests for capability and capacity matching logic
- Unit tests for node selection algorithm
- Integration tests for the complete workflow
- Performance testing to ensure minimal overhead

**Acceptance Criteria**:
- All tests pass
- Coverage of key capability and selection logic
- Verified behavior in various scenarios (capable/not capable, responsible/not responsible)
- Performance impact is within acceptable limits

## Future Project: Node Clustering for Workloads

*Note: This is for roadmap planning only; implementation will come later.*

### Goals

1. Enable multiple nodes to collaborate on workloads that exceed single-node capacity
2. Distribute workload processing across multiple nodes efficiently
3. Manage shared state and communication between collaborating nodes

### Clustering Implementation Approach

The clustering implementation will build on the primary project's node selection algorithm:

1. **Node Selection**: Use the same XOR(build_id, node_id) algorithm
2. **Cluster Formation**: 
   - In networks with fewer than 3 capable nodes, all available capable nodes form the cluster
   - By default in larger networks, the 3 nodes with the lowest XOR values form the cluster
   - Make cluster size configurable (4, 5, or more nodes as needed) for larger networks
   - Each node in the network can determine if it's part of a cluster without coordination

3. **Workload Distribution**:
   - Partition workloads based on cluster node capabilities
   - Assign specific tasks to specific cluster members

4. **State Management**:
   - Leverage existing CRDT-based state sharing mechanism
   - Ensure consistent view of cluster state across all members

### High-Level Tasks

1. **Implement configurable cluster size selection**
   - Allow specifying how many nodes should form a cluster
   - Enforce minimum capability requirements for all cluster members

2. **Create workload partitioning strategies**
   - Methods to divide workloads into processable chunks
   - Assignment of chunks to appropriate nodes within the cluster

3. **Implement failure recovery mechanisms**
   - Handle node departures/failures
   - Reassign work as needed

4. **Performance and resource optimization**
   - Balance load across cluster nodes
   - Minimize inter-node communication overhead

## Implementation Approach

### Phase 1: Primary Project
1. Start with Task 1 (Define Workload Requirements)
2. Implement Tasks 2 and 3 in parallel (Capability/Capacity Matching and Node Selection)
3. Complete Task 4 (Integration with form-pack)
4. Finalize with Task 5 (Testing)

### Phase 2: Future Clustering Project
- Detailed planning to be done after primary project completion
- Will build on the foundations established in the primary project

## Dependencies

- `form-node-metrics`: Must continue collecting accurate capability and capacity data
- `form-state`: Must provide reliable access to node metrics data
- `form-pack`: Will be modified to incorporate capability checks

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Node metrics data could be stale | Incorrect capability assessment | Implement freshness checks on metrics data |
| Network partitions could affect node list | Incorrect responsibility decisions | Add fallback mechanisms for unreachable form-state |
| Workload requirements may be difficult to specify accurately | Improper node selection | Create clear guidelines and examples for requirement specification |
| Performance overhead of capability checking | Slower build processing | Optimize queries and caching of capability data |
| XOR distribution might not be even across all possible node IDs | Uneven workload distribution | Monitor distribution and adjust algorithm if needed |

## Success Metrics

- Workloads are consistently assigned to nodes with sufficient resources
- Even distribution of workloads across capable nodes 
- No failed builds due to insufficient resources
- Minimal overhead added to the build process
- Clear deterministic decision-making that all nodes can independently verify 