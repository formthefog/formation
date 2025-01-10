#!/bin/bash

sudo curl --unix-socket /tmp/cloud-hypervisor-1.sock -i \
     -X PUT 'http://localhost/api/v1/vm.create'  \
     -H 'Accept: application/json'               \
     -H 'Content-Type: application/json'         \
     -d '{
         "cpus":{"boot_vcpus": 4, "max_vcpus": 4},
         "payload":{"kernel":"./linux-cloud-hypervisor/arch/x86/boot/compressed/vmlinux.bin", "cmdline":"console=ttyS0 console=hvc0 root=/dev/vda1 rw"},
         "disks":[{"path":"/var/lib/formation/vm-images/test-vm-2/disk.raw"},{"path":"/var/lib/formation/vm-images/default/cloud_init/test/test-vm-2"}],
         "net":[{"tap":"vmnet1"}],
         "serial":{"mode":"Tty"},
         "console":{"mode":"Off"}
         }'
