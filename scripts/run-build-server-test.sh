#!/bin/bash

DOCKERFILE=/home/ans/projects/vrrb/protocol/compute/formation/Dockerfile.form-build-server

docker build -f $DOCKERFILE -t form-builder .
container_id=$(docker run --device=/dev/kvm -dit form-builder)
docker exec -it $container_id /bin/bash
wait
docker kill $container_id
