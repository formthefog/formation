#!/bin/bash

sudo curl --unix-socket /run/form-vm/test-vm-$1.sock -i -X $2 "http://localhost/api/v1/$3)"
