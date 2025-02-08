#!/bin/bash

rm /etc/formnet/formnet.conf
/usr/local/bin/formnet operator leave --yes
/usr/local/bin/formnet operator join -C $SECRET_PATH -p $PASSWORD 
