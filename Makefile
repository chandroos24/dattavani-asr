# Dattavani ASR Rust - Makefile

.PHONY: help build test clean install run fmt lint check audit docker-build docker-run release

# Default target
help:
	@echo "Dattavani ASR Rust - Available targets:"
	@echo "  build        - Build the project in debug mode"
	@echo "  release      - Build the project in release mode"
	@echo "  test         - Run all tests"
	@echo "  clean        - Clean build artifacts"
	@echo "  install      - Install the binary to ~/.cargo/bin"
	@echo "  run          - Run the application with default args"
	@echo "  fmt          - Format the code"
	@echo "  lint         - Run clippy linter"
	@echo "  check        - Check the code without building"
	@echo "  audit        - Run security audit"
	@echo "  docker-build - Build Docker image"
	@echo "  docker-run   - Run Docker container"
	@echo "  setup        - Setup development environment"
	@echo "  benchmark    - Run benchmarks"

# Build targets
build:
	cargo build

release:
	cargo build --release

# Test targets
test:
	cargo test

test-verbose:
	cargo test -- --nocapture

# Development targets
clean:
	cargo clean
	rm -rf logs/
	rm -rf /tmp/dattavani_asr/
	rm -rf /tmp/dattavani_cache/

install:
	cargo install --path .

run:
	cargo run -- --help

# Code quality targets
fmt:
	cargo fmt

lint:
	cargo clippy -- -D warnings

check:
	cargo check

audit:
	cargo audit

# Docker targets
docker-build:
	docker build -t dattavani-asr-rust .

docker-run:
	docker run --rm -it \
		-v $(PWD)/service-account-key.json:/app/service-account-key.json:ro \
		dattavani-asr-rust

# Setup development environment
setup:
	rustup component add clippy rustfmt
	cargo install cargo-audit cargo-watch cargo-tarpaulin
	@echo "Development environment setup complete!"

# Benchmark
benchmark:
	cargo bench

# Watch for changes and run tests
watch:
	cargo watch -x test

# Coverage report
coverage:
	cargo tarpaulin --out Html --output-dir coverage/

# Performance profiling
profile:
	cargo build --release
	perf record --call-graph=dwarf ./target/release/dattavani-asr stream-process test-file.mp3
	perf report

# Create release package
package: release
	mkdir -p dist/
	cp target/release/dattavani-asr dist/
	cp README.md LICENSE .env.template dist/
	tar -czf dist/dattavani-asr-rust-$(shell git describe --tags --always).tar.gz -C dist/ .

# Health check
health-check:
	./target/release/dattavani-asr health-check

# Example commands
example-single:
	./target/release/dattavani-asr stream-process examples/sample.mp3

example-batch:
	./target/release/dattavani-asr stream-batch examples/

example-gdrive:
	./target/release/dattavani-asr stream-process "https://drive.google.com/file/d/EXAMPLE_ID/view"

# Generate documentation
docs:
	cargo doc --open

# All quality checks
quality: fmt lint test audit
	@echo "All quality checks passed!"

# CI pipeline simulation
ci: quality benchmark
	@echo "CI pipeline completed successfully!"
