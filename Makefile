DOCKER_CMD ?= docker
BUILD_IMAGE ?= rust:latest
BASE_IMAGE ?= debian:latest
IMAGE ?= fioncat/csync:latest

.PHONY: all
all:
	@cargo build

.PHONY: docker
docker:
	$(DOCKER_CMD) build \
		--build-arg BUILD_IMAGE=$(BUILD_IMAGE) \
		--build-arg BASE_IMAGE=$(BASE_IMAGE) \
		-t $(IMAGE) .

.PHONY: test
test:
	@cargo test --package csync_misc --package csync-server

.PHONY: test-server
test-server:
	@cargo build --package csync-server
	@CSYNC_CONFIG_PATH="./testdata/config" CSYNC_DATA_PATH="./testdata/data" ./target/debug/csync-server

.PHONY: clean-testdata
clean-testdata:
	@rm -rf ./testdata/data
	@rm -rf ./testdata/config
