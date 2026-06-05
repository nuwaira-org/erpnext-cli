.PHONY: build build-linux check test clippy fmt clean

CARGO  := $(HOME)/.cargo/bin/cargo
SHELL  := /bin/bash

# Ensure cargo-zigbuild can find zig
export PATH := /opt/homebrew/bin:$(HOME)/.cargo/bin:/usr/bin:/bin

build:
	$(CARGO) build --release

build-linux:
	$(CARGO) zigbuild --release --target x86_64-unknown-linux-musl

check:
	$(CARGO) check

test:
	$(CARGO) test

clippy:
	$(CARGO) clippy -- -D warnings

fmt:
	$(CARGO) fmt --check

clean:
	$(CARGO) clean
