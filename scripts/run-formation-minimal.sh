#!/bin/bash

DOCKERFILE=/home/ans/projects/vrrb/protocol/formation/Dockerfile

docker build --no-cache -f $DOCKERFILE -t formation-minimal .
container_id=$(docker run --rm --privileged --network=host --device=/dev/kvm \
    -v /var/run/docker.sock:/var/run/docker.sock -dit formation-minimal:latest
)
docker exec -it $container_id /bin/bash
wait
