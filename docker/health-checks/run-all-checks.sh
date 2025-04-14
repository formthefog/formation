#!/bin/bash
# Run health checks for all Formation services
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SERVICES=(
    "form-dns"
    "form-state"
    "vmm-service"
    "form-broker"
    "form-pack-manager"
    "formnet"
    "form-p2p"
)

# Command line arguments
VERBOSE=0
SPECIFIC_SERVICE=""

# Process command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -v|--verbose)
            VERBOSE=1
            shift
            ;;
        -s|--service)
            SPECIFIC_SERVICE="$2"
            shift 2
            ;;
        *)
            echo "Unknown option: $1"
            echo "Usage: $0 [-v|--verbose] [-s|--service SERVICE_NAME]"
            exit 1
            ;;
    esac
done

# Function to run a specific service health check
run_service_check() {
    local service="$1"
    local script="${SCRIPT_DIR}/${service}-healthcheck.sh"
    
    if [ -f "$script" ]; then
        echo "Running health check for $service..."
        if [ $VERBOSE -eq 1 ]; then
            bash "$script" "formation-$service"
        else
            bash "$script" "formation-$service" > /dev/null 2>&1
            if [ $? -eq 0 ]; then
                echo "✅ $service health check passed"
            else
                echo "❌ $service health check failed"
                return 1
            fi
        fi
    else
        echo "⚠️ No health check script found for $service"
        # Create a basic health check for services without specific checks
        echo "Running basic health check..."
        docker ps | grep -q "formation-$service"
        if [ $? -ne 0 ]; then
            echo "❌ Container formation-$service is not running!"
            return 1
        fi
        echo "✅ Container is running"
    fi
    
    return 0
}

echo "=================================================="
echo "Formation Services Health Check"
echo "=================================================="
echo ""

if [ -n "$SPECIFIC_SERVICE" ]; then
    run_service_check "$SPECIFIC_SERVICE"
    exit $?
fi

# Run all health checks
FAILED=0
for service in "${SERVICES[@]}"; do
    run_service_check "$service"
    if [ $? -ne 0 ]; then
        FAILED=1
    fi
    echo ""
done

if [ $FAILED -eq 0 ]; then
    echo "=================================================="
    echo "✅ All service health checks passed!"
    echo "=================================================="
    exit 0
else
    echo "=================================================="
    echo "❌ One or more service health checks failed"
    echo "=================================================="
    exit 1
fi 