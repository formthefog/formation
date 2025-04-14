# Formation Platform Dockerfile
# Makefile for building Docker images

# --- Variables ---
# VERSION: Sets the version for image tags. 
#          Defaults to git describe, or 'dev' if not in a git repo.
#          Used to derive TAG, MAJOR_VERSION, MINOR_VERSION.
# Example: make all VERSION=1.2.3
VERSION ?= $(shell git describe --tags --always --dirty || echo "dev")
MAJOR_VERSION ?= $(shell echo $(VERSION) | cut -d. -f1)
MINOR_VERSION ?= $(shell echo $(VERSION) | cut -d. -f2 || echo "0")

# REGISTRY: Docker registry/organization prefix for image tags.
# Example: make all REGISTRY=myrepo
REGISTRY ?= formation

# TAG: Specific tag used, derived from VERSION.
TAG ?= $(VERSION)

# --- Config ---
# List of all services
SERVICES = form-dns form-state vmm-service form-broker form-pack-manager formnet form-p2p

.PHONY: all $(SERVICES) clean push help

# --- Targets ---
# Build all images
all: $(SERVICES)

# Build individual service images
form-dns:
	@echo "Building form-dns image"
	@docker build \
		-t $(REGISTRY)/form-dns:latest \
		-t $(REGISTRY)/form-dns:$(TAG) \
		-t $(REGISTRY)/form-dns:$(MAJOR_VERSION).$(MINOR_VERSION) \
		-f ../form-dns/Dockerfile \
		..
	@echo "✅ form-dns image built successfully"

form-state:
	@echo "Building form-state image"
	@docker build \
		-t $(REGISTRY)/form-state:latest \
		-t $(REGISTRY)/form-state:$(TAG) \
		-t $(REGISTRY)/form-state:$(MAJOR_VERSION).$(MINOR_VERSION) \
		-f ../form-state/Dockerfile \
		..
	@echo "✅ form-state image built successfully"

vmm-service:
	@echo "Building vmm-service image"
	@docker build \
		-t $(REGISTRY)/vmm-service:latest \
		-t $(REGISTRY)/vmm-service:$(TAG) \
		-t $(REGISTRY)/vmm-service:$(MAJOR_VERSION).$(MINOR_VERSION) \
		-f ../form-vmm/vmm-service/Dockerfile \
		..
	@echo "✅ vmm-service image built successfully"

form-broker:
	@echo "Building form-broker image"
	@docker build \
		-t $(REGISTRY)/form-broker:latest \
		-t $(REGISTRY)/form-broker:$(TAG) \
		-t $(REGISTRY)/form-broker:$(MAJOR_VERSION).$(MINOR_VERSION) \
		-f ../form-broker/Dockerfile \
		..
	@echo "✅ form-broker image built successfully"

form-pack-manager:
	@echo "Building form-pack-manager image"
	@docker build \
		-t $(REGISTRY)/form-pack-manager:latest \
		-t $(REGISTRY)/form-pack-manager:$(TAG) \
		-t $(REGISTRY)/form-pack-manager:$(MAJOR_VERSION).$(MINOR_VERSION) \
		-f ../form-pack-manager/Dockerfile \
		..
	@echo "✅ form-pack-manager image built successfully"

formnet:
	@echo "Building formnet image"
	@docker build \
		-t $(REGISTRY)/formnet:latest \
		-t $(REGISTRY)/formnet:$(TAG) \
		-t $(REGISTRY)/formnet:$(MAJOR_VERSION).$(MINOR_VERSION) \
		-f ../form-net/Dockerfile \
		..
	@echo "✅ formnet image built successfully"

form-p2p:
	@echo "Building form-p2p image"
	@docker build \
		-t $(REGISTRY)/form-p2p:latest \
		-t $(REGISTRY)/form-p2p:$(TAG) \
		-t $(REGISTRY)/form-p2p:$(MAJOR_VERSION).$(MINOR_VERSION) \
		-f ../form-p2p/Dockerfile \
		..
	@echo "✅ form-p2p image built successfully"

# Clean up all images
clean:
	@echo "Removing all Formation Docker images"
	@docker images | grep $(REGISTRY) | awk '{print $$1":"$$2}' | xargs docker rmi || true
	@echo "✅ Images removed successfully"

# Push all images to registry
push: all
	@echo "Pushing all images to registry"
	@for service in $(SERVICES); do \
		docker push $(REGISTRY)/$$service:latest; \
		docker push $(REGISTRY)/$$service:$(TAG); \
		docker push $(REGISTRY)/$$service:$(MAJOR_VERSION).$(MINOR_VERSION); \
	done
	@echo "✅ All images pushed successfully"

# Display help
help:
	@echo "Formation Docker Build System"
	@echo "============================="
	@echo ""
	@echo "Usage: make [TARGET] [VARIABLE=value]"
	@echo ""
	@echo "Available targets:"
	@echo "  all                Build images for all services: $(SERVICES)"
	@echo "  <service_name>   Build image for a specific service (e.g., make form-dns)"
	@echo "  clean              Remove all locally built Formation images tagged with registry '$(REGISTRY)'"
	@echo "  push               Build and push all images to registry '$(REGISTRY)'"
	@echo "  help               Show this help message"
	@echo ""
	@echo "Available variables:"
	@echo "  VERSION=x.y.z      Set specific version tag (default: $(VERSION))"
	@echo "  REGISTRY=name      Set registry/organization prefix (default: $(REGISTRY))"
	@echo "" 