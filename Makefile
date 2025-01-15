WHAT ?= debug
CARGO_CMD ?= cargo
DOCKER_CMD ?= docker
CARGO_TARGET ?=
BUILD_IMAGE ?= rust:latest
BASE_IMAGE ?= debian:latest
TARGET ?= fioncat/csync:latest

.PHONY: all
all: build

.PHONY: build
build:
ifeq ($(WHAT),release)
	$(if $(CARGO_TARGET), \
		$(CARGO_CMD) build --release --target $(CARGO_TARGET), \
		$(CARGO_CMD) build --release)
else ifeq ($(WHAT),docker)
	$(DOCKER_CMD) build \
		--build-arg BUILD_IMAGE=$(BUILD_IMAGE) \
		--build-arg BASE_IMAGE=$(BASE_IMAGE) \
		-t $(TARGET) .
else
	$(if $(CARGO_TARGET), \
		$(CARGO_CMD) build --target $(CARGO_TARGET), \
		$(CARGO_CMD) build)
endif

.PHONY: install
install:
	$(CARGO_CMD) install --path .

.PHONY: clean
clean:
	$(CARGO_CMD) clean

.PHONY: help
help:
	@echo "Usage:"
	@echo "  make [WHAT=<target>] [options]"
	@echo ""
	@echo "Targets:"
	@echo "  debug    - Build debug version (default)"
	@echo "  release  - Build release version"
	@echo "  docker   - Build docker image"
	@echo "  install  - Install using cargo install"
	@echo ""
	@echo "Options:"
	@echo "  CARGO_CMD    - Specify cargo command (default: cargo)"
	@echo "  CARGO_TARGET - Specify build target"
	@echo "  DOCKER_CMD   - Specify docker command (default: docker)"
	@echo "  BUILD_IMAGE  - Specify builder image (default: rust:alpine)"
	@echo "  BASE_IMAGE   - Specify base image (default: alpine:latest)"
	@echo "  TARGET       - Specify docker image tag (default: fioncat/csync:latest)"
