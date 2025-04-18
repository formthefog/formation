#!/bin/bash

# Create directories if they don't exist
mkdir -p $(pwd)/state-data
mkdir -p $(pwd)/secrets

# Check for a password in the environment
if [ -z "$PASSWORD" ]; then
  echo "WARNING: No PASSWORD environment variable set. Using default password."
  echo "For production use, please set the PASSWORD environment variable."
fi

# Run form-state container
docker run --rm --name formation-state -p 3004:3004 \
  -v $(pwd)/state-data:/var/lib/formation/db \
  -v $(pwd)/secrets:/etc/formation \
  -e DB_PATH=/var/lib/formation/db/formation.db \
  -e SECRET_PATH=$SECRET_PATH \
  -e PASSWORD=$PASSWORD \
  -e DEV_MODE=true \
  -e AUTH_MODE=development \
  -e DYNAMIC_JWKS_URL=$DYNAMIC_JWKS_URL \
  formationai/form-state:latest

# Verify service is running
echo "Checking form-state health..."
sleep 5
curl http://localhost:3004/health
