# For convenience

.PHONY: release
release:
	cargo build --release --target x86_64-unknown-linux-musl --out-dir . -Z unstable-options
