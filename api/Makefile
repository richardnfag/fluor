.PHONY: dev test lint clean build

dev:
	cargo run

test:
	cargo test

lint:
	cargo clippy -- -D warnings

clean:
	cargo clean

build:
	cargo build --release

coverage:
	cargo llvm-cov --html --open

install-tools:
	cargo install cargo-llvm-cov

build-modules:
	$(MAKE) -C modules/healthz build
