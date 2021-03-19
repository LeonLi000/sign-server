SCHEMA_PATH := schemas
SCHEMA_DEST_PATH := src/generated

schema:
	moleculec --language rust --schema-file ${SCHEMA_PATH}/eth_header_cell.mol > ${SCHEMA_DEST_PATH}/eth_header_cell.rs
	cargo fmt
