#!/bin/bash

/usr/local/bin/run-form-p2p.sh > /var/log/formation/form-p2p.log 2>&1 &
sleep 1
/usr/local/bin/run-form-state.sh > /var/log/formation/form-state.log 2>&1 &
sleep 1
/usr/local/bin/run-formnet.sh > /var/log/formation/formnet.log 2>&1 &
/usr/local/bin/run-pack-manager.sh > /var/log/formation/form-pack.log 2>&1 &
/usr/local/bin/run-vmm-service-test.sh > /var/log/formation/vmm-service-test.log 2>&1 &

wait
