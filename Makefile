.PHONY: build
build:
	cargo build --release

.PHONY: install
install:
	cargo install --path .

.PHONY: uninstall
uninstall:
	cargo uninstall mak

.PHONY: clean
clean:
	rm -rf ./target

.PHONY: reset
reset: clean

.PHONY: publish
publish:
	cargo publish

.PHONY: test
test: test-build

.PHONY: test-build
test-build: build
	./target/release/mak --completions fish
	./target/release/mak --file Makefile-examples/hello.Makefile --print-completion-targets
	./target/release/mak --file Makefile-examples/cubing.js.Makefile --print-graph
	./target/release/mak --file Makefile-examples/hello.Makefile

.PHONY: lint
lint:
	cargo clippy -- --deny warnings
	cargo fmt --check

.PHONY: format
format:
	cargo clippy --fix --allow-no-vcs
	cargo fmt
