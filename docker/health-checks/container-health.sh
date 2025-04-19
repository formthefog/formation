#!/bin/bash
# container-health.sh - General health check script for Formation containers
set -e

function check_service_process() {
    local service_name=$1
    local process_pattern=$2
    echo "Checking if process is running for $service_name..."
    
    # Check if the process is running
    if ps aux | grep -v grep | grep -q "$process_pattern"; then
        echo "✅ Process is running for $service_name"
        return 0
    else
        echo "❌ Process is NOT running for $service_name"
        return 1
    fi
}

function check_service_port() {
    local service_name=$1
    local port=$2
    local protocol=${3:-tcp}
    echo "Checking if port $port/$protocol is open for $service_name..."
    
    # Check if port is open
    if netstat -tuln | grep -q ":$port"; then
        echo "✅ Port $port/$protocol is open for $service_name"
        return 0
    else
        echo "❌ Port $port/$protocol is NOT open for $service_name"
        return 1
    fi
}

function check_http_endpoint() {
    local service_name=$1
    local endpoint=$2
    local expected_code=${3:-200}
    echo "Checking HTTP endpoint $endpoint for $service_name..."
    
    # Check if HTTP endpoint returns expected status code
    local status_code=$(curl -s -o /dev/null -w "%{http_code}" "$endpoint")
    if [ "$status_code" = "$expected_code" ]; then
        echo "✅ Endpoint $endpoint returned $status_code as expected for $service_name"
        return 0
    else
        echo "❌ Endpoint $endpoint returned $status_code, expected $expected_code for $service_name"
        return 1
    fi
}

function check_dns_resolution() {
    local service_name=$1
    local domain=$2
    local expected_ip=${3:-}
    echo "Checking DNS resolution for $domain using $service_name..."
    
    # Check if domain resolves
    if dig +short "$domain" > /dev/null; then
        if [ -n "$expected_ip" ]; then
            resolved_ip=$(dig +short "$domain")
            if [ "$resolved_ip" = "$expected_ip" ]; then
                echo "✅ Domain $domain resolved to expected IP $expected_ip using $service_name"
                return 0
            else
                echo "❌ Domain $domain resolved to $resolved_ip, expected $expected_ip using $service_name"
                return 1
            fi
        else
            echo "✅ Domain $domain resolved using $service_name"
            return 0
        fi
    else
        echo "❌ Domain $domain did NOT resolve using $service_name"
        return 1
    fi
}

function check_file_exists() {
    local service_name=$1
    local file_path=$2
    echo "Checking if file $file_path exists for $service_name..."
    
    # Check if file exists
    if [ -f "$file_path" ]; then
        echo "✅ File $file_path exists for $service_name"
        return 0
    else
        echo "❌ File $file_path does NOT exist for $service_name"
        return 1
    fi
}

function check_directory_exists() {
    local service_name=$1
    local dir_path=$2
    echo "Checking if directory $dir_path exists for $service_name..."
    
    # Check if directory exists
    if [ -d "$dir_path" ]; then
        echo "✅ Directory $dir_path exists for $service_name"
        return 0
    else
        echo "❌ Directory $dir_path does NOT exist for $service_name"
        return 1
    fi
}

# Display usage information if no arguments provided
if [ $# -eq 0 ]; then
    echo "Usage: $0 [service_name] [check_type] [parameters...]"
    echo ""
    echo "Check types:"
    echo "  process [process_pattern]              - Check if process is running"
    echo "  port [port_number] [protocol?]         - Check if port is open (protocol defaults to tcp)"
    echo "  http [endpoint_url] [expected_code?]   - Check if HTTP endpoint returns expected status code (defaults to 200)"
    echo "  dns [domain] [expected_ip?]            - Check if DNS resolution works"
    echo "  file [file_path]                       - Check if file exists"
    echo "  directory [dir_path]                   - Check if directory exists"
    echo ""
    echo "Example: $0 form-dns process form-dns"
    echo "Example: $0 form-state port 3004"
    echo "Example: $0 form-broker http http://localhost:3005/health"
    exit 1
fi

service_name=$1
check_type=$2

case "$check_type" in
    process)
        check_service_process "$service_name" "$3"
        ;;
    port)
        check_service_port "$service_name" "$3" "${4:-tcp}"
        ;;
    http)
        check_http_endpoint "$service_name" "$3" "${4:-200}"
        ;;
    dns)
        check_dns_resolution "$service_name" "$3" "$4"
        ;;
    file)
        check_file_exists "$service_name" "$3"
        ;;
    directory)
        check_directory_exists "$service_name" "$3"
        ;;
    *)
        echo "Unknown check type: $check_type"
        exit 1
        ;;
esac

exit $? 