# Form State Service Setup

## Required Environment Variables

To run the Form State service, you need to set the following environment variables:

1. `PASSWORD` - The password to decrypt the operator config file.
2. `DYNAMIC_JWKS_URL` - The URL for the JSON Web Key Set used for authentication.

## Configuration File

The service requires an operator configuration file to be mounted at `/etc/formation/operator-config.json` inside the container.

## Running with Docker

### Method 1: Using the run script

```bash
# Set required environment variables
export PASSWORD="your-password-here"
export DYNAMIC_JWKS_URL="https://your-jwks-url"

# Create directories if they don't exist
mkdir -p $(pwd)/secrets
mkdir -p $(pwd)/state-data

# Copy your operator config file to the secrets directory
cp your-operator-config.json $(pwd)/secrets/operator-config.json

# Run the container using the script
./scripts/docker/run-form-state.sh
```

### Method 2: Using Docker Compose

1. Create a `.env` file in the project root with the following content:
```
PASSWORD=your-password-here
DYNAMIC_JWKS_URL=https://your-jwks-url
```

2. Create the necessary directories and copy your operator config:
```bash
mkdir -p $(pwd)/secrets
mkdir -p $(pwd)/state-data
cp your-operator-config.json $(pwd)/secrets/operator-config.json
```

3. Run using docker-compose:
```bash
docker-compose up form-state
```

## Verifying the Service

After starting the service, you can verify it's running correctly by accessing the health endpoint:

```bash
curl http://localhost:3004/health
```

## Troubleshooting

If the service fails to start:

1. Check that the operator config file exists at `$(pwd)/secrets/operator-config.json`
2. Verify that the correct password is provided
3. Ensure the JWKS URL is accessible
4. Check logs with `docker logs formation-state` 