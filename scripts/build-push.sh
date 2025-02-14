#!/bin/bash

cargo build --release

echo "starting docker build for formation-minimal"
docker build -t cryptonomikhan/formation-minimal:latest --no-cache -f Dockerfile . &
build_pid1=$!
echo "docker build for formation-minimal pid: $build_pid1"

echo "starting docker build for form-build-server"
docker build -t cryptonomikhan/form-build-server:latest --no-cache -f Dockerfile.form-build-server . &
build_pid2=$!
echo "docker build for form-build-server pid: $build_pid2"

wait $build_pid1
echo "starting docker push for formation-minimal"
docker push cryptonomikhan/formation-minimal:latest &

wait $build_pid2
echo "starting docker push for form-build-server"
docker push cryptonomikhan/form-build-server:latest &

wait

echo "Docker build and push processes complete"
