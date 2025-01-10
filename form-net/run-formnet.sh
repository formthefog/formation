#!/bin/bash

sudo rm /var/lib/formnet/formnet.db
sudo rm /etc/formnet/formnet.conf
sudo ./target/release/innernet-server uninstall formnet --yes
sudo ./target/release/formnet
