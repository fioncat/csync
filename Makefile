.PHONY: all
all:
	@cargo build

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
