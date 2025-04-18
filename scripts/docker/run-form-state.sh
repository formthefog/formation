#!/bin/bash

# Create directories if they don't exist
mkdir -p $(pwd)/state-data
mkdir -p $(pwd)/secrets

# Run form-state container
docker run --name formation-state -p 3004:3004 \
  -v $(pwd)/state-data:/var/lib/formation/db \
  -v $(pwd)/secrets:/var/lib/formation/secrets:ro \
  -e STATE_LOG_LEVEL=info \
  -e STATE_DB_PATH=/var/lib/formation/db/formation.db \
  -e STATE_API_PORT=3004 \
  -e AUTH_MODE=development \
  -e SECRET_PATH=/var/lib/formation/secrets/.operator-config.json \
  -e PASSWORD=$PASSWORD \
  formationai/form-state:latest

# Verify service is running
echo "Checking form-state health..."
sleep 5
curl http://localhost:3004/health 
