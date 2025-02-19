# Get the current date and time precise to the second
DATE_TAG := $(shell date +%Y%m%d%H%M%S)

# Docker image name
IMAGE_NAME := yautz/cube
STORE_IMAGE_NAME := yautz/cube_store
IMAGE_VERSION := 0.1

# Default target
all: build

# Build the Docker image with the current date tag
build:
	docker build --no-cache -t $(IMAGE_NAME):$(DATE_TAG) --progress=plain --build-arg IMAGE_VERSION=${IMAGE_VERSION} -f packages/cubejs-docker/release.Dockerfile . 2>&1 | tee build.log

store-build:
	docker build --no-cache -t $(STORE_IMAGE_NAME):$(DATE_TAG) --progress=plain --build-arg IMAGE_VERSION=${IMAGE_VERSION} -f rust/cubestore/Dockerfile ./rust 2>&1 | tee store_build.log

dev-tag:
	docker tag $(IMAGE_NAME):dev $(IMAGE_NAME):$(DATE_TAG)
	docker push $(IMAGE_NAME):$(DATE_TAG)

# Build the Docker image with a fixed tag for development
dev:
	# Update DEV_BUILD_TAG with current timestamp
	sed -i 's/ENV DEV_BUILD_TAG=.*/ENV DEV_BUILD_TAG=$(DATE_TAG)/' packages/cubejs-docker/release.Dockerfile
	docker build -t $(IMAGE_NAME):dev --progress=plain --build-arg IMAGE_VERSION=${IMAGE_VERSION} -f packages/cubejs-docker/release.Dockerfile . 2>&1 | tee build_dev.log

# Prune Docker system
prune:
	docker system prune --all --volumes -f

# Clean up all Docker images built by this Makefile
clean:
	docker rmi $$(docker images $(IMAGE_NAME) -q) 2>/dev/null || true

# Scan for critical CVEs
scan-cve:
	trivy image --severity CRITICAL --format json $(IMAGE_NAME):dev 2>&1 | tee scan-cve.log

.PHONY: all build dev prune clean scan-cve
