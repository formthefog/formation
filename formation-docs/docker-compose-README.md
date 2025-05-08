# Formation Docker Compose

This directory contains the Docker Compose configuration for the Formation platform, which allows running all services together as a complete system.

## Requirements

- Docker Engine 20.10.0+
- Docker Compose 2.0.0+
- At least 4GB of free RAM
- At least 10GB of free disk space

## Getting Started

### 1. Build all service images

Before running the Docker Compose setup, ensure all service images are built:

```bash
cd docker
make all
```

### 2. Start all services

To start the entire Formation platform:

```bash
docker-compose up -d
```

This will:
- Create all required Docker volumes
- Create the Formation network
- Start all services in the correct order
- Configure healthchecks for each service

### 3. Check service status

```bash
docker-compose ps
```

This will show the status of all services. All services should be in the "Up" state.

### 4. View logs

To view logs from all services:

```bash
docker-compose logs
```

Or to follow logs and see only specific services:

```bash
docker-compose logs -f form-dns form-state
```

## Service Dependencies

The services depend on each other in the following order:

1. `form-dns` - DNS service (no dependencies)
2. `form-state` - State service (no dependencies)
3. `vmm-service` - VM Manager (depends on form-state)
4. `form-broker` - Message Broker (depends on form-state)
5. `form-pack-manager` - Package Manager (depends on form-state)
6. `formnet` - Network Service (depends on form-state)
7. `form-p2p` - P2P Communication (depends on form-state)

## Ports

The following ports are exposed on the host:

| Service | Port | Protocol | Purpose |
|---------|------|----------|---------|
| form-dns | 53 | TCP/UDP | DNS |
| form-state | 3004 | TCP | State API |
| vmm-service | 3002 | TCP | VM Management API |
| form-broker | 3005, 5672, 1883 | TCP | API, AMQP, MQTT |
| form-pack-manager | 8080 | TCP | Package Manager API |
| formnet | 8081, 51820 | TCP, UDP | API, WireGuard |
| form-p2p | 3003 | TCP | P2P API |

## Volumes

Each service has its own volumes for data and configuration:

- DNS: `dns-data`, `dns-config`
- State: `state-data`, `state-config`
- VM: `vm-images`, `vm-kernel`
- Broker: `broker-data`, `broker-config`
- Package Manager: `pack-data`, `pack-config`
- Network: `net-data`, `net-config`
- P2P: `p2p-data`, `p2p-db`

## Network

All services are connected to the `formation-net` network with subnet `172.28.0.0/16`.

## Management

### Start specific services

```bash
docker-compose up -d form-dns form-state
```

### Stop specific services

```bash
docker-compose stop form-broker form-p2p
```

### Restart a service

```bash
docker-compose restart form-state
```

### Remove all services and volumes

```bash
docker-compose down -v
```

## Troubleshooting

If services fail to start:

1. Check logs: `docker-compose logs [service-name]`
2. Verify images exist: `docker images | grep formation`
3. Check if ports are already in use: `netstat -tuln`
4. Verify volume permissions: `ls -la $(docker volume inspect [volume-name] | jq -r '.[0].Mountpoint')`
5. Check healthchecks: `docker inspect [container-name] | jq '.[0].State.Health'`

## Next Steps

After starting all services, you can:

1. Access the Package Manager UI at http://localhost:8080
2. Configure DNS resolution by adding entries through the form-dns API
3. Create virtual machines using the vmm-service API
4. Set up network connections using the formnet API 