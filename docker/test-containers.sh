#!/bin/bash
# Script to test Formation Docker containers
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
ACTION="check"
SERVICE=""
VERBOSE=0

function print_usage() {
    echo "Usage: $0 [OPTIONS] [ACTION] [SERVICE]"
    echo ""
    echo "Test Formation Docker containers by building, running, and checking them."
    echo ""
    echo "OPTIONS:"
    echo "  -v, --verbose          Show detailed output from commands"
    echo "  -h, --help             Show this help message"
    echo ""
    echo "ACTIONS:"
    echo "  build                  Build container(s)"
    echo "  run                    Run container(s)"
    echo "  check                  Run health checks on container(s) (default)"
    echo "  stop                   Stop container(s)"
    echo "  clean                  Remove container(s)"
    echo "  all                    Build, run, and check container(s)"
    echo ""
    echo "SERVICE:"
    echo "  [service_name]         Operate on a specific service (e.g., form-dns)"
    echo "  (empty)                Operate on all services"
    echo ""
    echo "Examples:"
    echo "  $0 build form-dns      Build the form-dns container"
    echo "  $0 run                 Run all containers"
    echo "  $0 check form-state    Check health of form-state container"
    echo "  $0 all form-dns        Build, run, and check form-dns container"
}

# Process command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -v|--verbose)
            VERBOSE=1
            shift
            ;;
        -h|--help)
            print_usage
            exit 0
            ;;
        build|run|check|stop|clean|all)
            ACTION="$1"
            shift
            ;;
        *)
            # If this is the first positional argument, treat it as the service name
            if [ -z "$SERVICE" ]; then
                SERVICE="$1"
                shift
            else
                echo "Unknown option: $1"
                print_usage
                exit 1
            fi
            ;;
    esac
done

# Function to execute a command with or without verbose output
function run_command() {
    if [ $VERBOSE -eq 1 ]; then
        "$@"
    else
        "$@" > /dev/null 2>&1
    fi
}

# Function to build a service
function build_service() {
    local service="$1"
    echo "Building $service container..."
    run_command make -C "$SCRIPT_DIR" "$service"
    echo "✅ $service built successfully"
}

# Function to run a service
function run_service() {
    local service="$1"
    echo "Running $service container..."
    
    # Check if container is already running
    if docker ps | grep -q "formation-$service"; then
        echo "⚠️ Container formation-$service is already running"
        return 0
    fi
    
    # Run the container with appropriate settings based on service type
    case "$service" in
        form-dns)
            run_command docker run -d --name "formation-$service" \
                -p 53:53/udp -p 53:53/tcp \
                "formation/$service:latest"
            ;;
        form-state)
            run_command docker run -d --name "formation-$service" \
                -p 3004:3004 \
                -v "$(pwd)/form-state/data:/var/lib/formation/db" \
                "formation/$service:latest"
            ;;
        vmm-service)
            run_command docker run -d --name "formation-$service" \
                -p 3002:3002 \
                --privileged \
                -v "$(pwd)/form-vmm/data:/var/lib/formation/vm-images" \
                -v "/run/form-vm:/run/form-vm" \
                "formation/$service:latest"
            ;;
        form-broker)
            run_command docker run -d --name "formation-$service" \
                -p 3005:3005 -p 5672:5672 -p 1883:1883 \
                "formation/$service:latest"
            ;;
        form-pack-manager)
            run_command docker run -d --name "formation-$service" \
                -p 8080:8080 \
                "formation/$service:latest"
            ;;
        formnet)
            run_command docker run -d --name "formation-$service" \
                -p 8081:8080 -p 51820:51820/udp \
                --privileged \
                --cap-add NET_ADMIN \
                --cap-add SYS_MODULE \
                "formation/$service:latest"
            ;;
        form-p2p)
            run_command docker run -d --name "formation-$service" \
                -p 3003:3003 \
                "formation/$service:latest"
            ;;
        *)
            echo "⚠️ No run configuration defined for $service"
            return 1
            ;;
    esac
    
    echo "✅ $service started successfully"
    return 0
}

# Function to check service health
function check_service() {
    local service="$1"
    echo "Checking $service container..."
    
    # Run the health check script
    if [ $VERBOSE -eq 1 ]; then
        "$SCRIPT_DIR/health-checks/run-all-checks.sh" -v -s "$service"
    else
        "$SCRIPT_DIR/health-checks/run-all-checks.sh" -s "$service"
    fi
    
    local result=$?
    if [ $result -eq 0 ]; then
        echo "✅ $service health check passed"
    else
        echo "❌ $service health check failed"
        return 1
    fi
    
    return 0
}

# Function to stop a service
function stop_service() {
    local service="$1"
    echo "Stopping $service container..."
    
    # Check if container is running
    if ! docker ps | grep -q "formation-$service"; then
        echo "⚠️ Container formation-$service is not running"
        return 0
    fi
    
    run_command docker stop "formation-$service"
    echo "✅ $service stopped successfully"
}

# Function to clean up a service
function clean_service() {
    local service="$1"
    echo "Cleaning up $service container..."
    
    # Stop the container if it's running
    if docker ps | grep -q "formation-$service"; then
        run_command docker stop "formation-$service"
    fi
    
    # Remove the container if it exists
    if docker ps -a | grep -q "formation-$service"; then
        run_command docker rm "formation-$service"
    fi
    
    echo "✅ $service cleaned up successfully"
}

# Main function to process the request
function process_request() {
    local action="$1"
    local service="$2"
    
    # If no specific service is provided, process all services
    if [ -z "$service" ]; then
        for s in "${SERVICES[@]}"; do
            process_service "$action" "$s"
            echo ""
        done
    else
        # Verify if the service is valid
        local valid=0
        for s in "${SERVICES[@]}"; do
            if [ "$s" = "$service" ]; then
                valid=1
                break
            fi
        done
        
        if [ $valid -eq 1 ]; then
            process_service "$action" "$service"
        else
            echo "❌ Unknown service: $service"
            echo "Valid services: ${SERVICES[*]}"
            exit 1
        fi
    fi
}

# Function to process a specific action on a specific service
function process_service() {
    local action="$1"
    local service="$2"
    
    echo "====================================================="
    echo "Processing $action for $service"
    echo "====================================================="
    
    case "$action" in
        build)
            build_service "$service"
            ;;
        run)
            run_service "$service"
            ;;
        check)
            check_service "$service"
            ;;
        stop)
            stop_service "$service"
            ;;
        clean)
            clean_service "$service"
            ;;
        all)
            build_service "$service" && \
            run_service "$service" && \
            # Wait a bit for the service to start up
            sleep 5 && \
            check_service "$service"
            ;;
        *)
            echo "Unknown action: $action"
            print_usage
            exit 1
            ;;
    esac
    
    local result=$?
    if [ $result -ne 0 ]; then
        echo "❌ Action $action failed for $service"
        return 1
    fi
    
    return 0
}

# Process the request
process_request "$ACTION" "$SERVICE"

exit $? 