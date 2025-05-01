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
