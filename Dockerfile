FROM ubuntu:22.04

RUN sudo apt-get update && apt-get upgrade
RUN sudo -y protobuf-compiler libprotobuf-dev build-essential \
    clang llvm pkg-config \
    linux-headers-$(uname -r) qemu-kvm libvirt-dev \
    protobuf-compiler libprotobuf-dev libdbus-1-dev \
    libudev-dev libfuse-dev libseccomp-dev \
    libnetfilter-queue-dev libnl-3-dev libnl-route-3-dev \
    zlib1g-dev libbpf-dev liburing-dev libssl-dev

RUN mkdir -p /usr/local/bin
RUN mkdir -p /var/lib/formation/formnet
RUN mkdir -p /var/lib/formation/kernel
RUN mkdir -p /var/lib/formation/netplan
RUN mkdir -p /var/lib/formation/vm-images


COPY ./target/release/formnet /usr/local/bin/formnet
COPY ./target/release/formnet-client /usr/local/bin/formnet-client
COPY ./target/release/formnet-server /usr/local/bin/formnet-server
COPY ./target/release/formnet-client /var/lib/formnet/formnet
COPY ./target/release/vmm-service-test /usr/local/bin/vmm-service-test
COPY ./target/release/vmm-service /usr/local/bin/vmm-service
