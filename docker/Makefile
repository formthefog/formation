# Formation Platform Dockerfile
# Makefile for building Docker images

# Version information
VERSION ?= $(shell git describe --tags --always --dirty || echo "dev")
MAJOR_VERSION ?= $(shell echo $(VERSION) | cut -d. -f1)
MINOR_VERSION ?= $(shell echo $(VERSION) | cut -d. -f2 || echo "0")

# Docker registry settings (adjust as needed)
REGISTRY ?= formation
BASE_IMAGE = $(REGISTRY)/base
TAG ?= $(VERSION)

# List of all services
SERVICES = form-dns form-state vmm-service form-broker form-pack-manager formnet form-p2p

.PHONY: all base $(SERVICES) clean

# Build all images
all: base $(SERVICES)

# Build base image
base:
	@echo "Building base image $(BASE_IMAGE):$(TAG)"
	@cd base && docker build \
		--build-arg VERSION=$(VERSION) \
		-t $(BASE_IMAGE):latest \
		-t $(BASE_IMAGE):$(TAG) \
		-t $(BASE_IMAGE):$(MAJOR_VERSION).$(MINOR_VERSION) \
		.
	@echo "✅ Base image built successfully"

# Build individual service images
form-dns:
	@echo "Building form-dns image"
	@cd service-templates/form-dns && docker build \
		--build-arg SERVICE_VERSION=$(VERSION) \
		-t $(REGISTRY)/form-dns:latest \
		-t $(REGISTRY)/form-dns:$(TAG) \
		-t $(REGISTRY)/form-dns:$(MAJOR_VERSION).$(MINOR_VERSION) \
		.
	@echo "✅ form-dns image built successfully"

form-state:
	@echo "Building form-state image"
	@cd service-templates/form-state && docker build \
		--build-arg SERVICE_VERSION=$(VERSION) \
		-t $(REGISTRY)/form-state:latest \
		-t $(REGISTRY)/form-state:$(TAG) \
		-t $(REGISTRY)/form-state:$(MAJOR_VERSION).$(MINOR_VERSION) \
		.
	@echo "✅ form-state image built successfully"

vmm-service:
	@echo "Building vmm-service image"
	@cd service-templates/vmm-service && docker build \
		--build-arg SERVICE_VERSION=$(VERSION) \
		-t $(REGISTRY)/vmm-service:latest \
		-t $(REGISTRY)/vmm-service:$(TAG) \
		-t $(REGISTRY)/vmm-service:$(MAJOR_VERSION).$(MINOR_VERSION) \
		.
	@echo "✅ vmm-service image built successfully"

form-broker:
	@echo "Building form-broker image"
	@cd service-templates/form-broker && docker build \
		--build-arg SERVICE_VERSION=$(VERSION) \
		-t $(REGISTRY)/form-broker:latest \
		-t $(REGISTRY)/form-broker:$(TAG) \
		-t $(REGISTRY)/form-broker:$(MAJOR_VERSION).$(MINOR_VERSION) \
		.
	@echo "✅ form-broker image built successfully"

form-pack-manager:
	@echo "Building form-pack-manager image"
	@cd service-templates/form-pack-manager && docker build \
		--build-arg SERVICE_VERSION=$(VERSION) \
		-t $(REGISTRY)/form-pack-manager:latest \
		-t $(REGISTRY)/form-pack-manager:$(TAG) \
		-t $(REGISTRY)/form-pack-manager:$(MAJOR_VERSION).$(MINOR_VERSION) \
		.
	@echo "✅ form-pack-manager image built successfully"

formnet:
	@echo "Building formnet image"
	@cd service-templates/formnet && docker build \
		--build-arg SERVICE_VERSION=$(VERSION) \
		-t $(REGISTRY)/formnet:latest \
		-t $(REGISTRY)/formnet:$(TAG) \
		-t $(REGISTRY)/formnet:$(MAJOR_VERSION).$(MINOR_VERSION) \
		.
	@echo "✅ formnet image built successfully"

form-p2p:
	@echo "Building form-p2p image"
	@cd service-templates/form-p2p && docker build \
		--build-arg SERVICE_VERSION=$(VERSION) \
		-t $(REGISTRY)/form-p2p:latest \
		-t $(REGISTRY)/form-p2p:$(TAG) \
		-t $(REGISTRY)/form-p2p:$(MAJOR_VERSION).$(MINOR_VERSION) \
		.
	@echo "✅ form-p2p image built successfully"

# Clean up all images
clean:
	@echo "Removing all Formation Docker images"
	@docker images | grep $(REGISTRY) | awk '{print $$1":"$$2}' | xargs docker rmi || true
	@echo "✅ Images removed successfully"

# Push all images to registry
push: all
	@echo "Pushing all images to registry"
	@docker push $(BASE_IMAGE):latest
	@docker push $(BASE_IMAGE):$(TAG)
	@docker push $(BASE_IMAGE):$(MAJOR_VERSION).$(MINOR_VERSION)
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
	@echo "Available targets:"
	@echo "  all                Build all images"
	@echo "  base               Build base image only"
	@echo "  form-dns           Build form-dns service image"
	@echo "  form-state         Build form-state service image"
	@echo "  vmm-service        Build vmm-service service image"
	@echo "  form-broker        Build form-broker service image"
	@echo "  form-pack-manager  Build form-pack-manager service image"
	@echo "  formnet            Build formnet service image"
	@echo "  form-p2p           Build form-p2p service image"
	@echo "  clean              Remove all Formation images"
	@echo "  push               Push all images to registry"
	@echo ""
	@echo "Options:"
	@echo "  VERSION=x.y.z      Set specific version (default: git tag/commit)"
	@echo "  REGISTRY=name      Set registry prefix (default: formation)"
	@echo "" 