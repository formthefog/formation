#!/bin/bash

rm /var/lib/formnet/formnet.db
rm /etc/formnet/formnet.conf
/usr/local/bin/formnet-server uninstall formnet --yes
/usr/local/bin//formnet
