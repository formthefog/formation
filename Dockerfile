FROM ubuntu:22.04

RUN apt-get update -y 
RUN apt-get upgrade -y
RUN apt-get install -y protobuf-compiler libprotobuf-dev build-essential \
    clang llvm pkg-config iputils-ping wget curl \
    linux-headers-$(uname -r) qemu-kvm libvirt-dev \
    protobuf-compiler libprotobuf-dev libdbus-1-dev \
    libudev-dev libfuse-dev libseccomp-dev cloud-utils \
    libnetfilter-queue-dev libnl-3-dev libnl-route-3-dev \
    zlib1g-dev libbpf-dev liburing-dev libssl-dev \
    iproute2 bridge-utils ssh socat libguestfs-tools \
    qemu-utils

RUN mkdir -p /usr/local/bin
RUN mkdir -p /var/lib/formation/formnet
RUN mkdir -p /var/lib/formation/kernel
RUN mkdir -p /var/lib/formation/vm-images
RUN mkdir -p /run/form-vm
RUN mkdir -p /var/log/formation

# Copy our VPS kernel into the correct dir
COPY ./artifacts/hypervisor-fw /var/lib/formation/kernel/hypervisor-fw

# Copy our form-dns binary into /usr/local/bin 
COPY ./target/release/form-dns /usr/local/bin/form-dns

# Copy our form-state binary into /usr/local/bin
COPY ./target/release/form-state /usr/local/bin/form-state

# Copy our vmm-service binary into /usr/local/bin
COPY ./target/release/vmm-service /usr/local/bin/vmm-service

# Copy our message broker binary into /usr/local/bin
COPY ./target/release/form-broker /usr/local/bin/form-broker

# Copy our pack manager binary into /usr/local/bin
COPY ./target/release/form-pack-manager /usr/local/bin/form-pack-manager

# Copy our formnet binary into /usr/local/bin 
COPY ./target/release/formnet /usr/local/bin/formnet

# Copy our formnet binary into /usr/local/bin 
COPY ./target/release/form-p2p /usr/local/bin/form-p2p

# Copy the form-state run script into /usr/local/bin
COPY ./scripts/run-form-state.sh /usr/local/bin/run-form-state.sh

# Copy the form-dns run script into /usr/local/bin
COPY ./scripts/run-form-state.sh /usr/local/bin/run-form-dns.sh

# Copy the VMM service run script to /usr/local/bin
COPY ./scripts/run-vmm-service.sh /usr/local/bin/run-vmm-service.sh

# Copy the formnet run script into /usr/local/bin
COPY ./scripts/run-formnet.sh /usr/local/bin/run-formnet.sh

# Copy the PackManager run script into /usr/local/bin
COPY ./scripts/run-pack-manager.sh /usr/local/bin/run-pack-manager.sh

COPY ./scripts/run-form-p2p.sh /usr/local/bin/run-form-p2p.sh

# Copy he formnet run script into /usr/local/bin
COPY ./scripts/formation-minimal-entrypoint.sh /entrypoint.sh

# Provide the vmm-service run script with executable permission
RUN chmod +x /usr/local/bin/run-vmm-service.sh
# Provide the formnet run script with executable permission
RUN chmod +x /usr/local/bin/run-formnet.sh
# Provide the pack-manager run script with executable permission
RUN chmod +x /usr/local/bin/run-pack-manager.sh
# Provide the form-state run script with executable permission
RUN chmod +x /usr/local/bin/run-form-state.sh
RUN chmod +x /usr/local/bin/run-form-p2p.sh

# Provide the entrypoint script with executable permission
RUN chmod +x /entrypoint.sh

# Copy our formnet CLI into the formation library directory as this 
# needs to be added into VM instances so the instances can be 
# provisioned a CIDR within formnet
COPY ./target/release/formnet /var/lib/formation/formnet/formnet

EXPOSE 3001
EXPOSE 3002
EXPOSE 3003
EXPOSE 3004
EXPOSE 3005
EXPOSE 53333
EXPOSE 51820

WORKDIR /app
ENTRYPOINT ["/entrypoint.sh"]
