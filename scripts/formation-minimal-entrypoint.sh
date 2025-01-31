#!/bin/bash

/usr/local/bin/form-broker > /var/log/formation/form-broker.log 2>&1 &
/usr/local/bin/run-form-state.sh > /var/log/formation/form-state.log 2>&1 &
/usr/local/bin/run-form-dns.sh > /var/log/formation/form-dns.log 2>&1 &
sleep(1)
/usr/local/bin/run-vmm-service.sh > /var/log/formation/vmm-service.log 2>&1 &
/usr/local/bin/run-pack-manager.sh > /var/log/formation/pack-manager.log 2>&1 &
/usr/local/bin/run-formnet.sh > /var/log/formation/formnet.log 2>&1 &

wait
