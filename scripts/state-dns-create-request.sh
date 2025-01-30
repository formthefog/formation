#!/bin/bash

# Create DNS record with IP addresses
curl -X POST http://localhost:3004/dns/create \
  -H "Content-Type: application/json" \
  -d '{
    "Create": {
      "domain": "test.example.com",
      "record_type": "A",
      "public_ip": "203.0.113.1",
      "formnet_ip": "10.0.0.47",
      "cname_target": null,
      "ttl": 3600
    }
  }'

# Create DNS record with CNAME
curl -X POST http://localhost:3004/dns/create \
  -H "Content-Type: application/json" \
  -d '{
    "Create": {
      "domain": "alias.example.com",
      "record_type": "CNAME",
      "public_ip": null,
      "formnet_ip": null, 
      "cname_target": "target.example.com",
      "ttl": 3600
    }
  }'
