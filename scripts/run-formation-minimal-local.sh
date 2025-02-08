#!/bin/bash

./target/release/form-p2p --config ./secrets/.operator-config.json -e -p fatdog run > ./logs/form-p2p.log 2>&1 &
sleep 1
./target/release/form-state -e -p fatdog -C ./secrets/.operator-config.json > ./logs/form-state.log 2>&1 &
sleep 1
sudo ./target/release/formnet operator join -C ./secrets/.operator-config.json -e -p fatdog > ./logs/form-net.log 2>&1 &
sleep 1
sudo ./target/release/form-pack-manager -c ./secrets/.operator-config.json -e -P fatdog -i all -p 3003 > ./logs/form-pack.log 2>&1 &
sudo ./target/release/vmm-service -C ./secrets/.operator-config.json -e -p fatdog run > ./logs/form-vmm.log 2>&1 &

wait
