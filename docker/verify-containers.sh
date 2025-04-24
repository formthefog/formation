#!/bin/bash
# Script to verify each Formation container independently
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
KEEP_RUNNING=0
LOG_DIR="$SCRIPT_DIR/verification-logs"

# Process command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -v|--verbose)
            VERBOSE=1
            shift
            ;;
        -k|--keep-running)
            KEEP_RUNNING=1
            shift
            ;;
        -s|--service)
            SPECIFIC_SERVICE="$2"
            shift 2
            ;;
        *)
            echo "Unknown option: $1"
            echo "Usage: $0 [-v|--verbose] [-k|--keep-running] [-s|--service SERVICE_NAME]"
            exit 1
            ;;
    esac
done

# Create log directory if it doesn't exist
mkdir -p "$LOG_DIR"

# Helper function to run test-containers.sh with the right options
function run_test_container() {
    local action="$1"
    local service="$2"
    local verbose_flag=""
    
    if [ $VERBOSE -eq 1 ]; then
        verbose_flag="-v"
    fi
    
    "$SCRIPT_DIR/test-containers.sh" $verbose_flag "$action" "$service"
    return $?
}

# Function to verify a specific service
function verify_service() {
    local service="$1"
    local log_file="$LOG_DIR/${service}-verification.log"
    
    echo "====================================================="
    echo "Verifying $service container"
    echo "====================================================="
    echo "Log file: $log_file"
    
    # Start fresh with a clean log file
    echo "Verification log for $service container" > "$log_file"
    echo "Date: $(date)" >> "$log_file"
    echo "====================================================" >> "$log_file"
    
    # First, clean up any existing instances of the container
    echo "Cleaning up any existing $service containers..." | tee -a "$log_file"
    run_test_container "clean" "$service" >> "$log_file" 2>&1
    
    # Build the container
    echo "Building $service container..." | tee -a "$log_file"
    run_test_container "build" "$service" >> "$log_file" 2>&1
    
    if [ $? -ne 0 ]; then
        echo "❌ Build failed for $service container!" | tee -a "$log_file"
        return 1
    fi
    echo "✅ Build successful for $service container" | tee -a "$log_file"
    
    # Run the container
    echo "Running $service container..." | tee -a "$log_file"
    run_test_container "run" "$service" >> "$log_file" 2>&1
    
    if [ $? -ne 0 ]; then
        echo "❌ Failed to run $service container!" | tee -a "$log_file"
        return 1
    fi
    echo "✅ $service container is running" | tee -a "$log_file"
    
    # Wait for the container to be fully ready
    echo "Waiting for $service container to fully initialize..." | tee -a "$log_file"
    sleep 10
    
    # Check the container
    echo "Performing health check for $service container..." | tee -a "$log_file"
    run_test_container "check" "$service" >> "$log_file" 2>&1
    
    if [ $? -ne 0 ]; then
        echo "❌ Health check failed for $service container!" | tee -a "$log_file"
        
        # Dump container logs for debugging
        echo "Container logs:" >> "$log_file"
        docker logs "formation-$service" >> "$log_file" 2>&1
        
        # Clean up unless --keep-running was specified
        if [ $KEEP_RUNNING -eq 0 ]; then
            echo "Cleaning up failed $service container..." | tee -a "$log_file"
            run_test_container "clean" "$service" >> "$log_file" 2>&1
        else
            echo "Container kept running for debugging" | tee -a "$log_file"
        fi
        
        return 1
    fi
    
    echo "✅ $service container health check passed" | tee -a "$log_file"
    
    # Run service-specific verification tests if they exist
    if [ -f "$SCRIPT_DIR/verification-tests/${service}-verify.sh" ]; then
        echo "Running extended verification tests for $service..." | tee -a "$log_file"
        bash "$SCRIPT_DIR/verification-tests/${service}-verify.sh" >> "$log_file" 2>&1
        
        if [ $? -ne 0 ]; then
            echo "❌ Extended verification tests failed for $service!" | tee -a "$log_file"
            
            # Clean up unless --keep-running was specified
            if [ $KEEP_RUNNING -eq 0 ]; then
                echo "Cleaning up $service container after failed verification..." | tee -a "$log_file"
                run_test_container "clean" "$service" >> "$log_file" 2>&1
            else
                echo "Container kept running for debugging" | tee -a "$log_file"
            fi
            
            return 1
        fi
        
        echo "✅ Extended verification tests passed for $service" | tee -a "$log_file"
    fi
    
    # Clean up unless --keep-running was specified
    if [ $KEEP_RUNNING -eq 0 ]; then
        echo "Cleaning up $service container..." | tee -a "$log_file"
        run_test_container "clean" "$service" >> "$log_file" 2>&1
    else
        echo "Container kept running as requested" | tee -a "$log_file"
    fi
    
    echo "✅ Verification complete for $service container" | tee -a "$log_file"
    echo "====================================================" | tee -a "$log_file"
    return 0
}

# Main function to verify services
function verify_services() {
    # Create directory for service-specific tests if it doesn't exist
    mkdir -p "$SCRIPT_DIR/verification-tests"
    
    # If a specific service was requested, only verify that one
    if [ -n "$SPECIFIC_SERVICE" ]; then
        local valid=0
        for s in "${SERVICES[@]}"; do
            if [ "$s" = "$SPECIFIC_SERVICE" ]; then
                valid=1
                break
            fi
        done
        
        if [ $valid -eq 1 ]; then
            verify_service "$SPECIFIC_SERVICE"
            exit $?
        else
            echo "❌ Unknown service: $SPECIFIC_SERVICE"
            echo "Valid services: ${SERVICES[*]}"
            exit 1
        fi
    fi
    
    # Otherwise, verify all services
    echo "Verifying all Formation containers..."
    echo "===================================================="
    
    local failed_services=()
    for service in "${SERVICES[@]}"; do
        verify_service "$service"
        if [ $? -ne 0 ]; then
            failed_services+=("$service")
        fi
        echo ""
    done
    
    # Report final results
    echo "===================================================="
    echo "Verification Summary"
    echo "===================================================="
    
    if [ ${#failed_services[@]} -eq 0 ]; then
        echo "✅ All containers verified successfully!"
    else
        echo "❌ The following containers failed verification:"
        for failed in "${failed_services[@]}"; do
            echo "  - $failed"
        done
        return 1
    fi
    
    return 0
}

# Run the verification
verify_services
exit $? 