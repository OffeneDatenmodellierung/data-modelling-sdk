# Contracts Directory

This directory contains API contracts and interface definitions for the Enhanced Table Metadata feature.

## Files

- **table-api.md**: Rust API contracts for Table struct, metadata structs, and related methods
- **README.md**: This file

## Contract Types

These contracts define:
- Struct definitions (Table, SlaProperty, ContactDetails, InfrastructureType)
- Method signatures (Table::new(), DataModel filter methods)
- ODCS import/export behavior
- Serialization formats (JSON, YAML)
- Validation rules
- Performance guarantees
- Backward compatibility guarantees

## Usage

These contracts serve as the specification for:
- Implementation developers
- Test writers
- API consumers
- Documentation writers

All implementations must conform to these contracts.
