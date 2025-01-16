#!/bin/bash

IMAGE_NAME="jammy-server-cloudimg-amd64.raw"

virt-customize -v -a "/img/$IMAGE_NAME" \
    --run-command 'growpart /dev/sda 1' \
    --run-command 'resize2fs /dev/sda1' \
    --run-command 'useradd bigdog -m -g sudo -p $6$rounds=4096$EIdAq84D9192B0cc$JzIjsDI8CuS5DvIhUfEmQnCsIz7gfQMLoHZiPLKm6nlOGKillEXiSRzF66yWexzqi0k8.jgAmWqX5/FN5BbOc.' \
    --install npm \
    --ssh-inject bigdog:string:"ssh-rsa AAAAA...user@localhost" \
    --run-command 'echo Hello, World' \
    --run-command 'ls -a /home/bigdog/' \
    --run-command 'which npm' \
    --run-command 'cat /home/bigdog/.ssh/authorized_keys'
