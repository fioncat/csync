all:
	@cargo build --release --locked --color=always --verbose

.PHONY: install
install:
	@cargo install --path . --force

.PHONY: clean
clean:
	@rm -rf ./target

.PHONY: cloc
cloc:
	cloc --exclude-dir target .
