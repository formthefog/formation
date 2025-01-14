FROM ubuntu:22.04

RUN apt-get update
RUN apt-get install -y wget qemu-utils nbd-client

RUN mkdir -p /mnt/cloudimg
RUN mkdir -p /img
RUN wget https://cloud-images.ubuntu.com/jammy/current/jammy-server-cloudimg-amd64.img -O /img/jammy-server-cloudimg-amd64.img && \
    ls -l /img/jammy-server-cloudimg-amd64.img
RUN qemu-img convert -O raw /img/jammy-server-cloudimg-amd64.img /img/jammy-server-cloudimg-amd64.raw
#RUN modprobe nbd && \
#    qemu-nbd --connect=/dev/nbd0 /img/jammy-server-cloudimg-amd64.raw && \
#    mkdir -p /mnt/cloudimg && \
#    mount /dev/nbd0p1 /mnt/cloudimg && \

ENTRYPOINT ["/bin/bash"]
