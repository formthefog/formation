# Formation System Architecture Diagram

```
+---------------------+                     +---------------------+
|     form-cli        |                     |   External APIs     |
|  (User Interface)   |                     |                     |
+-----+---------------+                     +-----------+---------+
      |                                                 |
      |  Commands/Events                                |
      v                                                 v
+-----+---------------------------------------------------+
|                                                         |
|           form-p2p Message Queue                        |
|     (Central Communication Infrastructure)              |
|                                                         |
+---+----------------+---------------+------------------+-+
    |                |               |                  |
    |                |               |                  |
    v                v               v                  v
+---+----+      +---+----+     +----+-----+      +-----+-----+
|form-vmm |      |form-net|     |form-state|      | form-pack |
| Service |      |Service |     | Service  |      |  Service  |
+---+----+      +---+----+     +----+-----+      +-----+-----+
    |                |               |                  |
    |                |               |                  |
    |                |               |                  |
    v                v               v                  v
+---+----+      +---+----+     +----+-----+      +-----+-----+
|  VMs    |      |Network |     |Distributed|     |VM Images  |
|Management|     |Interface|     |  State   |     |& Packages |
+---+----+      +--------+     +-----------+     +-----------+


Node Architecture (expanded view of a single node):

+------------------------------------------------------+
|                      Node                            |
|                                                      |
|  +------------------------+                          |
|  |      form-vmm          |                          |
|  | +--------------------+ |                          |
|  | |  VMM Service       | |                          |
|  | | +-----------------+| |                          |
|  | | |VM1| |VM2| |VM3| || |                          |
|  | | +-----------------+| |                          |
|  | +--------------------+ |                          |
|  +------------------------+                          |
|                                                      |
|  +------------------------+  +-------------------+   |
|  |      form-net          |  |    form-pack      |   |
|  | (WireGuard Networking) |  | (VM Image Mgmt)   |   |
|  +------------------------+  +-------------------+   |
|                                                      |
|  +------------------------+  +-------------------+   |
|  |      form-p2p          |  |    form-state     |   |
|  | (Message Queue)        |  | (State Store)     |   |
|  +------------------------+  +-------------------+   |
|                                                      |
|  +-------------------------------------------+       |
|  |  Supporting Services                      |       |
|  |  - form-dns - Network Name Resolution     |       |
|  |  - form-rplb - Load Balancing            |       |
|  |  - form-metrics - Monitoring             |       |
|  +-------------------------------------------+       |
+------------------------------------------------------+

Communication Patterns:

+-------------+                      +-------------+
| User/Client |                      | Service A   |
+------+------+                      +------+------+
       |                                    |
       | 1. Command/Request                 |
       v                                    |
+------+------+     2. Event Publication    |
| form-p2p    |<----------------------------|
| Message Q   |                             |
+------+------+                             |
       |                                    |
       | 3. Event Consumption               |
       v                                    |
+------+------+     4. Direct API Call      |
| Service B   |<----------------------------|
+-------------+     (when immediate         |
                     response needed)       |
```

## Key Changes in the Architecture

1. **Central Role of form-p2p Message Queue**: The message queue serves as the primary communication infrastructure connecting all services.

2. **form-pack Service**: The diagram includes the form-pack service which handles VM image management, including the pack-manager and image-builder components.

3. **Dual Communication Patterns**: The diagram shows both the primary asynchronous event-based communication and the secondary direct API calls used when immediate responses are required.

## Component Relationships

1. **form-cli & External APIs**: User interfaces for system interaction, publishing commands as events to the message queue.

2. **form-p2p Message Queue**: Central nervous system of the architecture, routing events between services.

3. **form-vmm**: Manages virtual machines using Cloud Hypervisor, consuming VM lifecycle events.

4. **form-net**: Handles networking, configuring connections between VMs and nodes.

5. **form-state**: Maintains distributed state across the network.

6. **form-pack**: Component that manages VM image creation, packaging, and distribution.

7. **Supporting Services**: Additional services that enhance the system's functionality.

## Communication Flow

1. Users interact with the system through the CLI or APIs
2. Commands are either:
   - Published as events to the form-p2p message queue (primary pattern)
   - Sent as direct API calls to services (secondary pattern, for immediate responses)
3. Services consume relevant events from the message queue
4. The form-pack system prepares VM images based on specifications
5. The VMM service deploys and manages VMs using these images
6. The networking layer establishes connections between VMs
7. The state management system maintains global state consistency

## Benefits of this Architecture

1. **Reduced Complexity**: No broker middleware layer keeps the system simple.
2. **Direct Communication**: Services directly read from the message queue.
3. **Flexibility**: The dual communication approach allows for both asynchronous and synchronous patterns.
4. **Decentralization**: No single point of failure in the communication infrastructure.
5. **Scalability**: Services can be added or updated independently.