#!/bin/bash

DOCKERFILE=/home/ans/projects/vrrb/protocol/compute/formation/Dockerfile.form-build-server

docker build --no-cache -f $DOCKERFILE -t form-build-server .
container_id=$(docker run --device=/dev/kvm -dit form-build-server)
docker exec -it $container_id /bin/bash
wait
docker kill $container_id
