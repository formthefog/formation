#!/bin/bash

/usr/local/bin/form-broker > /var/log/services/form-broker.log 2>&1 &
/usr/local/bin/run-vmm-service-test.sh > /var/log/services/vmm-service-test.log 2>&1 &
/usr/local/bin/run-formnet.sh > /var/log/services/formnet.log 2>&1 &

wait
