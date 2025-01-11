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
RUN mkdir -p /var/lib/formation/netplan
RUN mkdir -p /var/lib/formation/vm-images/ubuntu/22.04/
RUN mkdir -p /run/form-vm
RUN mkdir -p /var/log/formation

COPY ./artifacts/hypervisor-fw /var/lib/formation/kernel/hypervisor-fw
COPY ./target/release/formnet /usr/local/bin/formnet
COPY ./target/release/formnet-client /usr/local/bin/formnet-client
COPY ./target/release/formnet-server /usr/local/bin/formnet-server
COPY ./target/release/formnet-client /var/lib/formation/formnet/formnet
COPY ./target/release/vmm-service-test /usr/local/bin/vmm-service-test
COPY ./target/release/vmm-service /usr/local/bin/vmm-service
COPY ./target/release/form-broker /usr/local/bin/form-broker
COPY ./artifacts/base.raw /var/lib/formation/vm-images/ubuntu/22.04/base.raw

EXPOSE 3001
EXPOSE 3002
EXPOSE 51820

WORKDIR /app
ENTRYPOINT ["/bin/bash"]
