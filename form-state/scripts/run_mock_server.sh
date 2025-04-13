#!/bin/bash

# Colors for better output
RED="\033[0;31m"
GREEN="\033[0;32m"
YELLOW="\033[0;33m"
BLUE="\033[0;34m"
CYAN="\033[0;36m"
RESET="\033[0m"

# Default values
PORT=3004
VERBOSE=false
SKIP_JWT=false

# Parse command line arguments
while [[ $# -gt 0 ]]; do
  case $1 in
    --port=*)
      PORT="${1#*=}"
      shift
      ;;
    --jwt-audience=*)
      JWT_AUDIENCE="${1#*=}"
      shift
      ;;
    --jwt-issuer=*)
      JWT_ISSUER="${1#*=}"
      shift
      ;;
    --jwks-url=*)
      JWKS_URL="${1#*=}"
      shift
      ;;
    --jwt-leeway=*)
      JWT_LEEWAY="${1#*=}"
      shift
      ;;
    --env-file=*)
      ENV_FILE="${1#*=}"
      shift
      ;;
    --skip-jwt)
      SKIP_JWT=true
      shift
      ;;
    --verbose)
      VERBOSE=true
      shift
      ;;
    --help|-h)
      echo -e "${CYAN}Form-State Mock Server Runner${RESET}"
      echo ""
      echo "Usage: $0 [OPTIONS]"
      echo ""
      echo "Options:"
      echo "  --port=NUM             Port to listen on (default: 3004)"
      echo "  --jwt-audience=STR     JWT audience for auth validation"
      echo "  --jwt-issuer=STR       JWT issuer for auth validation"
      echo "  --jwks-url=URL         JWKS URL for auth validation"
      echo "  --jwt-leeway=NUM       JWT leeway in seconds (default: 60)"
      echo "  --env-file=FILE        Path to .env file"
      echo "  --skip-jwt             Skip JWT validation (for local development)"
      echo "  --verbose              Generate verbose logs"
      echo "  --help, -h             Display this help message"
      exit 0
      ;;
    *)
      echo -e "${RED}Unknown option: $1${RESET}"
      echo -e "Use ${CYAN}--help${RESET} to see available options"
      exit 1
      ;;
  esac
done

# Build command with options
CMD="cargo run --example mock-server --features=devnet"
CMD+=" -- --port $PORT"

if [ -n "$JWT_AUDIENCE" ]; then
  CMD+=" --jwt-audience \"$JWT_AUDIENCE\""
fi

if [ -n "$JWT_ISSUER" ]; then
  CMD+=" --jwt-issuer \"$JWT_ISSUER\""
fi

if [ -n "$JWKS_URL" ]; then
  CMD+=" --jwks-url \"$JWKS_URL\""
fi

if [ -n "$JWT_LEEWAY" ]; then
  CMD+=" --jwt-leeway $JWT_LEEWAY"
fi

if [ -n "$ENV_FILE" ]; then
  CMD+=" --env-file \"$ENV_FILE\""
fi

if [ "$SKIP_JWT" = true ]; then
  CMD+=" --skip-jwt"
fi

if [ "$VERBOSE" = true ]; then
  CMD+=" --verbose"
fi

# Display info
echo -e "${GREEN}Starting Form-State Mock Server${RESET}"
echo -e "${BLUE}Port:${RESET} $PORT"

if [ "$SKIP_JWT" = true ]; then
  echo -e "${YELLOW}WARNING: JWT validation is disabled!${RESET}"
fi

echo -e "${CYAN}Running command:${RESET} $CMD"
echo ""

# Run the command
eval $CMD 