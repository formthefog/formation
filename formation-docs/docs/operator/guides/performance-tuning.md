# Performance Tuning Guide for Formation Operators

This guide provides comprehensive recommendations for optimizing the performance of Formation operator nodes. By implementing these tuning strategies, you can maximize throughput, minimize latency, and ensure efficient resource utilization for your deployed instances.

## System Requirements and Baseline Performance

Before diving into specific tuning recommendations, ensure your system meets or exceeds the baseline requirements for running a Formation operator node:

**Minimum Requirements:**
- CPU: 4+ cores (8+ recommended)
- RAM: 16+ GB (32+ GB recommended)
- Storage: 500+ GB SSD (NVMe recommended)
- Network: 1 Gbps (10 Gbps recommended for production)

**Software Requirements:**
- Linux kernel 5.4+ (5.15+ recommended)
- QEMU/KVM 6.0+
- WireGuard support (kernel module or userspace)

## Performance Benchmarking

Before applying performance tuning, establish baseline performance metrics:

```bash
# Install benchmarking tools
sudo apt install -y sysbench fio iperf3

# CPU benchmark
sysbench cpu --cpu-max-prime=20000 run

# Memory benchmark
sysbench memory --memory-block-size=1K --memory-total-size=100G run

# Disk benchmark
fio --name=random-write --ioengine=posixaio --rw=randwrite --bs=4k --size=4g --numjobs=1 --direct=1

# Network benchmark (run iperf3 -s on another server first)
iperf3 -c <server-ip> -t 30
```

Record these baseline metrics to compare against after implementing performance tuning.

## CPU Optimization

### CPU Governor Configuration

The CPU governor controls how the processor scales frequency in response to load:

```bash
# Check current CPU governor
cat /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor

# Set performance governor (temporary)
echo performance | sudo tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor

# Make it permanent
sudo apt install -y cpufrequtils
sudo nano /etc/default/cpufrequtils
```

Add this line:

```
GOVERNOR="performance"
```

Then restart the service:

```bash
sudo systemctl restart cpufrequtils
```

### CPU Isolation for VM Workloads

Isolate specific CPU cores for VM workloads to reduce context switching:

```bash
# Edit the kernel boot parameters
sudo nano /etc/default/grub
```

Add CPU isolation parameter (replace X-Y with the core range you want to isolate):

```
GRUB_CMDLINE_LINUX="isolcpus=X-Y"
```

Update GRUB and reboot:

```bash
sudo update-grub
sudo reboot
```

Configure Formation to use these isolated cores:

```json
{
  "vm": {
    "cpu_pinning_enabled": true,
    "cpu_dedicated_set": "X-Y"
  }
}
```

### NUMA Considerations

For multi-socket systems, NUMA awareness is crucial for performance:

```bash
# Check NUMA topology
numactl --hardware

# Enable NUMA balancing
echo 1 | sudo tee /proc/sys/kernel/numa_balancing
```

Configure Formation to respect NUMA boundaries:

```json
{
  "vm": {
    "numa_aware": true
  }
}
```

## Memory Optimization

### System Memory Configuration

Configure system memory parameters for optimal VM performance:

```bash
# Add to /etc/sysctl.conf
vm.swappiness = 10
vm.dirty_ratio = 20
vm.dirty_background_ratio = 5
```

Apply changes:

```bash
sudo sysctl -p
```

### Huge Pages

Huge pages reduce TLB misses and improve memory access performance for VMs:

```bash
# Check huge page availability
cat /proc/meminfo | grep Huge

# Configure huge pages (number depends on your total RAM)
# For 2MB huge pages (allocate about 80% of RAM for huge pages)
echo 20480 | sudo tee /proc/sys/vm/nr_hugepages

# Make it persistent
echo "vm.nr_hugepages = 20480" | sudo tee -a /etc/sysctl.conf
```

Enable huge pages in Formation configuration:

```json
{
  "vm": {
    "use_hugepages": true,
    "hugepages_size_mb": 2,
    "hugepages_percentage": 80
  }
}
```

### Memory Ballooning

Memory ballooning allows dynamic adjustment of VM memory:

```json
{
  "vm": {
    "default_memory_ballooning": true,
    "memory_ballooning_statistics_period": 10
  }
}
```

## Storage Optimization

### I/O Scheduler

Select the appropriate I/O scheduler for your storage type:

```bash
# Check current scheduler for a device
cat /sys/block/sda/queue/scheduler

# For SSDs, use mq-deadline or none
echo mq-deadline | sudo tee /sys/block/sda/queue/scheduler

# Make it persistent
echo 'ACTION=="add|change", KERNEL=="sda", ATTR{queue/scheduler}="mq-deadline"' | sudo tee /etc/udev/rules.d/60-scheduler.rules
```

Configure in Formation:

```json
{
  "performance": {
    "io_scheduler": "mq-deadline"
  }
}
```

### File System Tuning

Optimize the file system for VM storage:

```bash
# For XFS (recommended for VM storage)
sudo mkfs.xfs -f -d agcount=16 -i size=512 /dev/sdb
sudo mount -o noatime,nodiratime,discard /dev/sdb /var/lib/formation/vms
```

Add to `/etc/fstab`:

```
/dev/sdb /var/lib/formation/vms xfs noatime,nodiratime,discard 0 0
```

### VM Image Format and Caching

Configure optimal VM image settings:

```json
{
  "vm": {
    "default_disk_format": "qcow2",
    "disk_cache_mode": "writeback",
    "disk_io_mode": "native",
    "vm_image_cache_size_gb": 50
  }
}
```

## Network Optimization

### Network Stack Tuning

Optimize the network stack for high-throughput applications:

```bash
# Add to /etc/sysctl.conf
net.core.rmem_max = 16777216
net.core.wmem_max = 16777216
net.ipv4.tcp_rmem = 4096 87380 16777216
net.ipv4.tcp_wmem = 4096 65536 16777216
net.core.netdev_max_backlog = 50000
net.ipv4.tcp_max_syn_backlog = 30000
net.ipv4.tcp_slow_start_after_idle = 0
net.ipv4.tcp_tw_reuse = 1
net.ipv4.ip_local_port_range = 1024 65535
```

Apply changes:

```bash
sudo sysctl -p
```

### WireGuard Optimization

Tune WireGuard for optimal formation networking:

```bash
# Adjust MTU for better performance
ip link set mtu 1420 dev wg0

# Increase txqueuelen
ip link set txqueuelen 2000 dev wg0
```

Configure in Formation:

```json
{
  "network": {
    "wireguard_mtu": 1420,
    "wireguard_fwmark": 51820,
    "wireguard_tx_queue_length": 2000
  }
}
```

### TCP BBR Congestion Control

Enable TCP BBR for improved throughput and reduced latency:

```bash
# Check if BBR is available
sysctl net.ipv4.tcp_available_congestion_control

# Enable BBR
echo "net.core.default_qdisc = fq" | sudo tee -a /etc/sysctl.conf
echo "net.ipv4.tcp_congestion_control = bbr" | sudo tee -a /etc/sysctl.conf
sudo sysctl -p

# Verify
sysctl net.ipv4.tcp_congestion_control
```

Enable in Formation configuration:

```json
{
  "performance": {
    "network_tuning": {
      "enabled": true,
      "tcp_bbr": true,
      "tcp_window_scaling": true
    }
  }
}
```

## Virtualization Optimization

### QEMU/KVM Configuration

Optimize the QEMU/KVM hypervisor settings for better performance:

```bash
# Edit libvirt configuration
sudo nano /etc/libvirt/qemu.conf
```

Add or modify these settings:

```
set_process_name = 1
max_files = 32768
max_processes = 32768
namespaces = [ "mount", "network" ]
```

Restart libvirt:

```bash
sudo systemctl restart libvirtd
```

### VM CPU Configuration

Configure optimal CPU settings for VMs:

```json
{
  "vm": {
    "default_cpu_model": "host",
    "enable_nested_virtualization": false,
    "cpu_pinning_enabled": true,
    "emulator_thread_policy": "isolate"
  }
}
```

### VM Network Configuration

Optimize VM networking:

```json
{
  "vm": {
    "default_network_model": "virtio",
    "network_multi_queue": true,
    "network_queue_size": 1024
  }
}
```

## Resource Pool Configuration

Organize VM resources into optimized pools to ensure fair resource distribution:

```json
{
  "resource_pools": [
    {
      "name": "compute-optimized",
      "description": "High CPU performance pool",
      "cpus": 16,
      "memory_mb": 32768,
      "disk_gb": 500,
      "priority": "high",
      "allow_overcommit": false,
      "cpu_pinning_enabled": true
    },
    {
      "name": "balanced",
      "description": "Balanced resource pool",
      "cpus": 16,
      "memory_mb": 65536,
      "disk_gb": 1000,
      "priority": "normal",
      "allow_overcommit": true,
      "allow_overcommit_ratio": 1.5
    }
  ]
}
```

## Resource Overcommitment

Intelligently configure resource overcommitment for increased density:

```json
{
  "hardware": {
    "overcommit_ratio": 1.5
  },
  "vm": {
    "cpu_overcommit_ratio": 2.0,
    "memory_overcommit_ratio": 1.2,
    "dynamic_resource_allocation": true
  }
}
```

## Monitoring and Performance Analysis

Set up comprehensive monitoring to track performance:

```json
{
  "monitoring": {
    "prometheus_enabled": true,
    "prometheus_port": 9100,
    "node_exporter_enabled": true,
    "libvirt_exporter_enabled": true,
    "performance_metrics_collection_interval": 15,
    "detailed_vm_metrics": true
  }
}
```

Install additional performance monitoring tools:

```bash
# Install Prometheus and Grafana
sudo apt install -y prometheus prometheus-node-exporter
wget https://github.com/prometheus/libvirt_exporter/releases/download/v0.1.1/libvirt_exporter-0.1.1.linux-amd64.tar.gz
tar xvfz libvirt_exporter-0.1.1.linux-amd64.tar.gz
sudo mv libvirt_exporter-0.1.1.linux-amd64/libvirt_exporter /usr/local/bin/

# Create a systemd service for libvirt_exporter
sudo nano /etc/systemd/system/libvirt_exporter.service
```

Add the following content:

```
[Unit]
Description=Libvirt Exporter
After=network.target

[Service]
Type=simple
User=root
ExecStart=/usr/local/bin/libvirt_exporter --libvirt.uri "qemu:///system"

[Install]
WantedBy=multi-user.target
```

Start the service:

```bash
sudo systemctl daemon-reload
sudo systemctl enable libvirt_exporter
sudo systemctl start libvirt_exporter
```

## GPU Acceleration

For nodes with GPUs, optimize for GPU passthrough performance:

### NVIDIA GPU Passthrough

```bash
# Add the nvidia-vgpu manager module
sudo apt install -y nvidia-vgpu-ubuntu-driver

# Configure IOMMU groups for passthrough
sudo nano /etc/default/grub
```

Add these parameters to `GRUB_CMDLINE_LINUX`:

```
intel_iommu=on iommu=pt
```

Or for AMD processors:

```
amd_iommu=on iommu=pt
```

Update GRUB and reboot:

```bash
sudo update-grub
sudo reboot
```

Configure in Formation:

```json
{
  "hardware": {
    "gpu_enabled": true,
    "gpu_devices": [
      {
        "id": "GPU-1234567890abcdef",
        "name": "NVIDIA GeForce RTX 3080",
        "pass_through_mode": "vfio"
      }
    ]
  },
  "vm": {
    "gpu_driver_vfio": true,
    "gpu_performance_mode": "max_performance"
  }
}
```

## Performance Tuning by Workload Type

### Web Server Workloads

```json
{
  "instance_types": [
    {
      "name": "web-server",
      "vcpu_range": [2, 8],
      "memory_range_mb": [2048, 8192],
      "disk_range_gb": [10, 100],
      "default_vcpus": 2,
      "default_memory_mb": 4096,
      "default_disk_gb": 20,
      "cpu_shares": 1024,
      "io_weight": 500,
      "network_priority": "high"
    }
  ]
}
```

### Database Workloads

```json
{
  "instance_types": [
    {
      "name": "database",
      "vcpu_range": [4, 16],
      "memory_range_mb": [8192, 65536],
      "disk_range_gb": [50, 1000],
      "default_vcpus": 4,
      "default_memory_mb": 16384,
      "default_disk_gb": 100,
      "cpu_shares": 2048,
      "io_weight": 800,
      "cpu_pinning_enabled": true,
      "memory_dedicated": true
    }
  ]
}
```

### Compute-Intensive Workloads

```json
{
  "instance_types": [
    {
      "name": "compute",
      "vcpu_range": [8, 32],
      "memory_range_mb": [16384, 131072],
      "disk_range_gb": [20, 500],
      "default_vcpus": 8,
      "default_memory_mb": 32768,
      "default_disk_gb": 50,
      "cpu_shares": 4096,
      "io_weight": 400,
      "cpu_pinning_enabled": true,
      "numa_aware": true
    }
  ]
}
```

## Scaling Performance

### Handling Multiple VMs Efficiently

Optimize your system to handle numerous VMs simultaneously:

```json
{
  "vm": {
    "max_concurrent_operations": 10,
    "max_queued_operations": 50,
    "vm_start_timeout_seconds": 300,
    "concurrent_io_operations": 32
  }
}
```

### Load Balancing

Configure load balancing across multiple nodes:

```json
{
  "advanced": {
    "load_balancing": {
      "enabled": true,
      "strategy": "least_loaded",
      "check_interval_seconds": 60,
      "migration_threshold_cpu_percent": 80,
      "migration_threshold_memory_percent": 85
    }
  }
}
```

## Kernel Parameter Optimization

Fine-tune kernel parameters for VM host performance:

```bash
# Add to /etc/sysctl.conf
# Memory management
vm.min_free_kbytes = 1048576
vm.overcommit_memory = 0
vm.overcommit_ratio = 50
vm.zone_reclaim_mode = 0

# File system
fs.file-max = 2097152
fs.aio-max-nr = 1048576

# Networking
net.core.somaxconn = 65535
net.ipv4.tcp_mem = 16777216 16777216 16777216
net.ipv4.tcp_retries2 = 8
net.ipv4.tcp_keepalive_time = 60
net.ipv4.tcp_keepalive_intvl = 10
net.ipv4.tcp_keepalive_probes = 6

# Apply changes
sudo sysctl -p
```

## Benchmarking and Performance Testing

After implementing performance tuning, re-run your benchmarks to measure improvement:

```bash
# Run the same benchmarks as before
sysbench cpu --cpu-max-prime=20000 run
sysbench memory --memory-block-size=1K --memory-total-size=100G run
fio --name=random-write --ioengine=posixaio --rw=randwrite --bs=4k --size=4g --numjobs=1 --direct=1
iperf3 -c <server-ip> -t 30

# VM-specific benchmarks
# Create a test VM with fixed resources
form deploy --file benchmark.form

# Then run benchmark inside the VM and compare to bare metal performance
```

## Performance Tuning Checklist

Use this checklist to ensure you've implemented all applicable optimizations:

### System-level Optimizations
- [ ] CPU governor set to "performance"
- [ ] Swappiness reduced to appropriate level
- [ ] I/O scheduler optimized for SSD/NVMe
- [ ] Huge pages enabled and configured
- [ ] Network stack parameters tuned
- [ ] TCP BBR congestion control enabled
- [ ] File system parameters optimized

### Virtualization Optimizations
- [ ] CPU pinning configured
- [ ] Nested virtualization disabled (unless required)
- [ ] Memory ballooning enabled
- [ ] VM image format and caching optimized
- [ ] NUMA topology respected
- [ ] KSM configured for memory deduplication
- [ ] VM network configuration optimized

### Resource Allocation
- [ ] Resource pools appropriately defined
- [ ] Overcommitment ratios set reasonably
- [ ] Instance types optimized for workloads
- [ ] GPU passthrough properly configured (if applicable)

### Monitoring
- [ ] Performance metrics collection enabled
- [ ] Monitoring dashboards set up
- [ ] Performance alerts configured

## Troubleshooting Performance Issues

### High CPU Utilization

Check which processes are consuming CPU:

```bash
top -c
```

Check if CPU frequency scaling is working:

```bash
cat /sys/devices/system/cpu/cpu*/cpufreq/scaling_cur_freq
```

### Memory Issues

Check memory statistics:

```bash
free -h
vmstat 1
```

Check VM memory allocation:

```bash
sudo virsh dommemstat <vm-name>
```

### I/O Performance Problems

Check disk I/O:

```bash
iostat -x 1
```

Check for slow disk operations:

```bash
sudo iotop
```

### Network Latency

Check network latency:

```bash
ping <destination>
traceroute <destination>
```

Monitor network traffic:

```bash
sudo iftop
```

## Conclusion

Optimizing a Formation operator node for performance is an ongoing process that requires careful monitoring and adjustment. By implementing the recommended tuning strategies in this guide, you can significantly improve the performance, stability, and efficiency of your Formation nodes, resulting in better service for your users.

Remember that performance requirements vary based on workload types, and no single configuration fits all scenarios. Always validate tuning changes with appropriate benchmarks and adjust based on your specific requirements.

For further assistance with performance tuning, consult the Formation support team or reach out to the community for additional guidance. 