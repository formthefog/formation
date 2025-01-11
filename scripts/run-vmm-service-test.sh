#!/bin/bash

rm /run/form-vm/test-vm-1.sock
/usr/local/bin/vmm-service-test -t 1
