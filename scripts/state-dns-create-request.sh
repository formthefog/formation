#!/bin/bash

# Create DNS record with IP addresses
curl -X POST http://localhost:3004/dns/create \
  -H "Content-Type: application/json" \
  -d '{
    "Create": {
      "domain": "test.example.com",
      "record_type": "A",
      "public_ip": ["203.0.113.1", "212.22.43.44", "200.48.222.128"],
      "formnet_ip": ["10.0.0.47", "10.0.0.48", "10.0.0.49"],
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
      "public_ip": [],
      "formnet_ip": [], 
      "cname_target": "target.example.com",
      "ttl": 3600
    }
  }'
