DOCKER_CMD ?= docker
BUILD_IMAGE ?= rust:latest
BASE_IMAGE ?= debian:latest
IMAGE ?= fioncat/csync:latest

.PHONY: all
all:
	cargo build

.PHONY: release
release:
	cargo build --release --locked

.PHONY: docker
docker:
	$(DOCKER_CMD) build \
		--build-arg BUILD_IMAGE=$(BUILD_IMAGE) \
		--build-arg BASE_IMAGE=$(BASE_IMAGE) \
		-t $(IMAGE) .

.PHONY: install
install:
	$(CARGO_CMD) install --path .

.PHONY: clean
clean:
	$(CARGO_CMD) clean
