#!/bin/bash

curl -X POST 'http://localhost:3005/record/create' \
-H 'Content-Type: application/json' \
-d '{
  "Create": {
    "domain": "example.formation.cloud",
    "record_type": "A",
    "ip_addr": ["127.0.0.1:8081", "127.0.0.1:8082"],
    "cname_target": null,
    "ssl_cert": true
  }
}'
