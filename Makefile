SCHEMA_PATH := schemas
SCHEMA_DEST_PATH := src/generated

schema:
	moleculec --language rust --schema-file ${SCHEMA_PATH}/eth_header_cell.mol > ${SCHEMA_DEST_PATH}/eth_header_cell.rs
	cargo fmt

ci: check-fmt clippy

fmt:
	cargo fmt

check-fmt:
	cargo fmt -- --check

test-cli:
	sh test.sh

build:
	cargo build

clippy:
	cargo clippy -- -D warnings
