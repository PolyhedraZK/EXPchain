all: fmt lint

build:
	cargo build --bins

release:
	cargo build --release --bins

fmt:
	cargo fmt

fmtall:
	bash tools/fmt.sh

lint:
	cargo clippy