# Form Broker Service

The Form Broker Service is a core component of the Formation platform that provides reliable message broker capabilities for inter-service communication. It supports multiple messaging protocols including AMQP and MQTT to facilitate different communication patterns between microservices.

## Features

- Dual protocol support: AMQP 0-9-1 (RabbitMQ compatible) and MQTT 3.1.1
- Message persistence with configurable durability
- Topic-based routing and filtering
- Quality of Service (QoS) levels for reliable message delivery
- Horizontal scalability through clustering
- Integration with the Formation state service for service discovery
- RESTful API for management and monitoring

## Building the Service

### Prerequisites

- Rust toolchain (1.65 or newer)
- pkg-config
- OpenSSL development libraries
- Docker (for containerized builds)

### Build Commands

```bash
# Build in debug mode
cargo build --package form-broker

# Build in release mode
cargo build --release --package form-broker

# Build Docker image
docker build -t formation/form-broker:latest -f form-broker/Dockerfile .
```

## Configuration

The form-broker service can be configured using:

1. A configuration file (default location: `/etc/formation/broker/default.conf`)
2. Environment variables
3. Command line arguments

### Key Configuration Parameters

| Parameter | Description | Default |
|-----------|-------------|---------|
| `api_port` | HTTP API port | 3005 |
| `amqp_port` | AMQP protocol port | 5672 |
| `mqtt_port` | MQTT protocol port | 1883 |
| `data_dir` | Directory for persistent storage | /var/lib/formation/broker |
| `state_url` | URL of the state service | http://form-state:3004 |
| `log_level` | Logging verbosity | info |

See `form-broker/config/default.conf` for the complete list of configuration options.

## Running the Service

### Direct Execution

```bash
# Run with default configuration
./target/release/form-broker

# Run with custom configuration file
./target/release/form-broker --config /path/to/config.conf

# Run with custom settings
./target/release/form-broker --api-port 3005 --amqp-port 5672 --mqtt-port 1883
```

### Docker Execution

```bash
docker run -d --name form-broker \
  -p 3005:3005 -p 5672:5672 -p 1883:1883 \
  -v broker-data:/var/lib/formation/broker \
  -v /path/to/config:/etc/formation/broker \
  -e BROKER_LOG_LEVEL=info \
  formation/form-broker:latest
```

## API Documentation

The Form Broker provides a RESTful API for management and monitoring:

### Health Check
- `GET /health` - Service health status

### Broker Statistics
- `GET /stats` - Get broker statistics
- `GET /stats/connections` - Get connection statistics
- `GET /stats/messages` - Get message statistics

### Queue Management
- `GET /queues` - List all queues
- `GET /queues/{queue}` - Get queue information
- `DELETE /queues/{queue}` - Delete a queue

### Topic Management
- `GET /topics` - List all topics
- `GET /topics/{topic}` - Get topic information

### Message Operations
- `POST /publish` - Publish a message to a topic or queue
- `GET /messages/{queue}` - Get messages from a queue

## Client Libraries

The form-broker service can be accessed using standard AMQP and MQTT client libraries:

- For AMQP: RabbitMQ clients, AMQP 0-9-1 clients
- For MQTT: Any MQTT 3.1.1 compatible client

## Monitoring

The broker service exposes metrics in Prometheus format at `/metrics` for monitoring its health and performance.

## Troubleshooting

Common issues:

1. **Connection refused**: Ensure the broker is running and ports are properly exposed.
2. **Authentication failure**: Check credentials if authentication is enabled.
3. **Message not delivered**: Verify queue existence and routing configuration.
4. **Performance issues**: Adjust worker threads and memory settings based on load.

## Security Considerations

- Enable TLS for production deployments
- Configure authentication for sensitive environments
- Set appropriate authorization rules for topics and queues
- Regularly update to the latest version for security patches

## License

This component is part of the Formation platform and is licensed under the appropriate license terms of the project.
