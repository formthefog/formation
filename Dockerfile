FROM ubuntu:22.04

RUN apt-get update && apt-get upgrade
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
# Copy our formnet binary into /usr/local/bin 
COPY ./target/release/formnet /usr/local/bin/formnet
# Copy our formnet client CLI into /usr/local/bin 
COPY ./target/release/formnet-client /usr/local/bin/formnet-client
# Copy our formnet server CLI into /usr/local/bin 
COPY ./target/release/formnet-server /usr/local/bin/formnet-server
# Copy our formnet CLI into the formation library directory as this 
# needs to be added into VM instances so the instances can be 
# provisioned a CIDR within formnet
COPY ./target/release/formnet-client /var/lib/formation/formnet/formnet
# Copy our vmm-service binary into /usr/local/bin
COPY ./target/release/vmm-service /usr/local/bin/vmm-service
# Copy our message broker binary into /usr/local/bin
COPY ./target/release/form-broker /usr/local/bin/form-broker
# Copy our pack manager binary into /usr/local/bin
COPY ./target/release/form-pack-manager /usr/local/bin/form-pack-manager
# Copy our form-state binary into /usr/local/bin
COPY ./target/release/form-state /usr/local/bin/form-state
# Copy the VMM service run script to /usr/local/bin
COPY ./scripts/run-vmm-service.sh /usr/local/bin/run-vmm-service.sh
# Copy the formnet run script into /usr/local/bin
COPY ./scripts/run-formnet.sh /usr/local/bin/run-formnet.sh
# Copy the PackManager run script into /usr/local/bin
COPY ./scripts/run-pack-manager.sh /usr/local/bin/run-pack-manager.sh
# Copy he formnet run script into /usr/local/bin
COPY ./scripts/formation-minimal-entrypoint.sh /entrypoint.sh


# Provide the vmm-service run script with executable permission
RUN chmod +x /usr/local/bin/run-vmm-service.sh
# Provide the formnet run script with executable permission
RUN chmod +x /usr/local/bin/run-formnet.sh
# Provide the pack manage rrun script with executable permission
RUN chmod +x /usr/local/bin/run-pack-manager.sh
# Provide the entrypoint script with executable permission
RUN chmod +x /entrypoint.sh

EXPOSE 3001
EXPOSE 3002
EXPOSE 3003
EXPOSE 3004
EXPOSE 51820

WORKDIR /app
ENTRYPOINT ["/entrypoint.sh"]
