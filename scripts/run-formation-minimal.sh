#!/bin/bash

DOCKERFILE=/home/ans/projects/vrrb/protocol/compute/formation/Dockerfile
BUILD_SERVER_DOCKERFILE=/home/ans/projects/vrrb/protocol/compute/formation/Dockerfile.form-build-server

docker build -f $BUILD_SERVER_DOCKERFILE -t form-build-server .
docker build -f $DOCKERFILE -t formation-minimal .
container_id=$(docker run --rm --privileged --network=host \
    --device=/dev/kvm \
    --device=/dev/vhost-net \
    --device=/dev/null \
    --device=/dev/zero \
    --device=/dev/random \
    --device=/dev/urandom \
    -v /lib/modules:/lib/modules:ro \
    -v /var/run/docker.sock:/var/run/docker.sock \
    --mount type=tmpfs,destination=/dev/hugepages,tmpfs-mode=1770 \
    -dit formation-minimal:latest
)
docker exec -it $container_id /bin/bash
wait
