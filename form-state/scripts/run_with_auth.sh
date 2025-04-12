#!/bin/bash

# Run Formation State server with JWT authentication settings
# This script configures the JWT parameters for the Dynamic auth provider

# Parse command line arguments
usage() {
  echo "Usage: $0 [options]"
  echo ""
  echo "Options:"
  echo "  -e, --env-id ID          Dynamic Environment ID"
  echo "  -a, --audience URL       JWT audience value"
  echo "  -f, --env-file FILE      Use environment file (.env)"
  echo "  -h, --help               Show this help message"
  echo ""
  echo "Examples:"
  echo "  $0 --env-id 3f53e601-17c7-419b-8a13-4c5e25c0bde9"
  echo "  $0 --env-file custom.env"
  exit 1
}

# Default values
AUDIENCE="https://formation-cloud-git-dynamic-versatus.vercel.app"
ENV_FILE=""

# Parse arguments
while [[ $# -gt 0 ]]; do
  case $1 in
    -e|--env-id)
      DYNAMIC_ENV_ID="$2"
      shift 2
      ;;
    -a|--audience)
      AUDIENCE="$2"
      shift 2
      ;;
    -f|--env-file)
      ENV_FILE="$2"
      shift 2
      ;;
    -h|--help)
      usage
      ;;
    *)
      echo "Unknown option: $1"
      usage
      ;;
  esac
done

# Execute with env file if specified
if [ -n "$ENV_FILE" ]; then
  echo "Starting Formation State server with environment file: $ENV_FILE"
  
  # Check if the file exists
  if [ ! -f "$ENV_FILE" ]; then
    echo "Error: Environment file not found: $ENV_FILE"
    exit 1
  fi
  
  cargo run -- --env-file "$ENV_FILE"
  exit 0
fi

# Check if DYNAMIC_ENV_ID is provided when not using env file
if [ -z "$DYNAMIC_ENV_ID" ]; then
  echo "Error: Dynamic Environment ID is required when not using an environment file"
  usage
fi

echo "Starting Formation State server with Dynamic Auth configuration..."
echo "Environment ID: $DYNAMIC_ENV_ID"
echo "Audience: $AUDIENCE"

# Build the JWKS URL from the environment ID
JWKS_URL="https://app.dynamic.xyz/api/v0/sdk/$DYNAMIC_ENV_ID/.well-known/jwks"
JWT_ISSUER="app.dynamicauth.com/$DYNAMIC_ENV_ID"

echo "JWKS URL: $JWKS_URL"
echo "JWT Issuer: $JWT_ISSUER"

# Run the application with the JWT parameters
cargo run -- \
  --jwt-audience "$AUDIENCE" \
  --jwt-issuer "$JWT_ISSUER" \
  --jwks-url "$JWKS_URL" \
  --jwt-leeway 60

# For development/testing, you can also use the standalone example:
# cargo run --example standalone-server-auth 