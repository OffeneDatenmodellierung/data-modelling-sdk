# Research: Enhanced Data Flow Node and Relationship Metadata

**Date**: 2026-01-27
**Feature**: Enhanced Data Flow Node and Relationship Metadata
**Phase**: 0 - Research & Design Decisions

## Overview

This document captures research findings and design decisions for adding comprehensive metadata fields to Data Flow nodes (Tables) and relationships. All clarifications from the specification phase have been resolved. This is for Data Flow elements, NOT for ODCS Data Contracts (ODCS is only for Data Models/tables). We will create a lightweight, cut-down specification format for Data Flow separate from ODCS.

## Research Findings

### 1. SLA Format (Inspired by ODCS, but Lightweight)

**Decision**: Use ODCS-inspired servicelevels format (slaProperties array structure) but in a lightweight Data Flow format separate from ODCS

**Rationale**:
- ODCS specification defines servicelevels as an array of objects with property, value, unit, and optional fields (element, driver, description, scheduler, schedule)
- ODCS format is ONLY for Data Models (tables), NOT for Data Flow nodes/relationships
- We can use ODCS structure as inspiration but create a lightweight format for Data Flow
- Lightweight format keeps Data Flow metadata simple and focused

**Alternatives Considered**:
- Free-form text: Rejected - lacks structure for automated SLA monitoring and validation
- Full ODCS format: Rejected - ODCS is only for Data Models, Data Flow needs separate lightweight format
- Custom structured format: Accepted - lightweight format inspired by ODCS but separate

**Implementation Notes**:
- Store as `Vec<SlaProperty>` struct in Table and Relationship models
- Each SlaProperty contains: property (String), value (serde_json::Value for flexibility), unit (String), and optional fields
- Serialize/deserialize using serde for JSON/YAML compatibility
- Use lightweight Data Flow format (separate from ODCS) for import/export

### 2. Contact Details Structure

**Decision**: Structured object with standard fields (email, phone, name, role, other)

**Rationale**:
- Provides structure while maintaining flexibility
- Supports both individual contacts (name, email) and team contacts (role, other)
- Aligns with common contact information patterns
- All fields optional to support partial information

**Alternatives Considered**:
- Single string field: Rejected - lacks structure for parsing and validation
- Completely flexible key-value object: Rejected - too unstructured, makes querying difficult

**Implementation Notes**:
- Create `ContactDetails` struct with optional fields
- Use `#[serde(skip_serializing_if = "Option::is_none")]` to omit empty fields in serialization
- Store as `Option<ContactDetails>` in Table struct

### 3. Infrastructure Type Enumeration

**Decision**: Strict enumeration with comprehensive list (70+ types) covering all major cloud services

**Rationale**:
- Ensures consistency across data models
- Enables reliable filtering and search operations
- Prevents typos and inconsistent naming
- Comprehensive coverage supports diverse infrastructure environments

**Alternatives Considered**:
- Allow custom values: Rejected - would reduce consistency and make filtering unreliable
- Minimal list with "Other" option: Rejected - comprehensive list better serves user needs

**Implementation Notes**:
- Create `InfrastructureType` enum in `src/models/enums.rs`
- Use `#[derive(Serialize, Deserialize)]` with string representation
- Include all types from specification: Traditional DBs, NoSQL, AWS Services, Azure Services, GCP Services, Message Queues, Containers, Data Warehouses, BI Tools, Storage
- Validation: Reject values not in enumeration during deserialization

### 4. Metadata Storage Strategy

**Decision**: Add dedicated fields to Table struct (for Data Flow nodes) and Relationship struct (for Data Flow relationships) while maintaining existing metadata structures for backward compatibility

**Rationale**:
- Dedicated fields provide type safety and better API ergonomics
- Tables and Relationships can both be used in Data Flow contexts
- Backward compatible - existing code continues to work
- Lightweight Data Flow format import/export can use dedicated fields

**Alternatives Considered**:
- Store only in existing metadata structures: Rejected - lacks type safety and makes API less ergonomic
- Replace existing metadata entirely: Rejected - would break backward compatibility
- Only add to Table: Rejected - Relationships also need metadata for Data Flow

**Implementation Notes**:
- Add new fields to Table: `owner: Option<String>`, `sla: Option<Vec<SlaProperty>>`, `contact_details: Option<ContactDetails>`, `infrastructure_type: Option<InfrastructureType>`, `notes: Option<String>`
- Add same fields to Relationship struct
- Lightweight Data Flow format export: Use dedicated fields
- Lightweight Data Flow format import: Extract to dedicated fields when possible
- Note: This is separate from ODCS format (ODCS is only for Data Models/tables)

### 5. Backward Compatibility Strategy

**Decision**: All new metadata fields are optional (Option<T>) with default values

**Rationale**:
- Existing tables without metadata continue to work without modification
- No breaking changes to Table struct API
- Serialization omits None values (skip_serializing_if)
- Import/export handles missing fields gracefully

**Implementation Notes**:
- Use `#[serde(skip_serializing_if = "Option::is_none")]` for all new fields
- Default to `None` in Table::new() constructor
- ODCS import: Extract metadata if present, otherwise leave as None
- ODCS export: Only include fields that have values

### 6. Search and Filter Implementation

**Decision**: Implement search/filter as methods on DataModel that iterate tables and relationships

**Rationale**:
- Simple linear search sufficient for 10,000 nodes/relationships (performance requirement: <1 second)
- Data Flow nodes are Tables, Data Flow relationships are Relationships
- No need for complex indexing for initial implementation
- Can be optimized later with indexing if needed
- Keeps implementation simple and maintainable

**Alternatives Considered**:
- Database indexing: Rejected - SDK is in-memory, no database
- Hash maps for indexing: Considered but deferred - linear search meets performance requirements
- Separate methods for nodes vs relationships: Considered but unified approach simpler

**Implementation Notes**:
- Add methods to DataModel: `filter_nodes_by_owner()`, `filter_relationships_by_owner()`, `filter_nodes_by_infrastructure_type()`, `filter_relationships_by_infrastructure_type()`, `filter_by_tags()` (works for both)
- Use iterator patterns for efficient filtering
- Return `Vec<&Table>` for nodes, `Vec<&Relationship>` for relationships

## Design Decisions Summary

| Decision | Rationale | Impact |
|----------|-----------|--------|
| ODCS-inspired SLA format (lightweight) | Structure for SLA monitoring, separate from ODCS | High - provides structure while keeping Data Flow format lightweight |
| Structured contact details | Type safety, queryability | Medium - improves API ergonomics |
| Strict infrastructure enum | Consistency, reliability | High - enables reliable filtering |
| Dedicated fields in Table and Relationship | Type safety, supports both nodes and relationships | High - balances structure and extensibility |
| Optional fields | Backward compatibility | High - no breaking changes |
| Lightweight Data Flow format | Separate from ODCS, focused on Data Flow needs | High - correct separation of concerns |
| Linear search | Simplicity, meets performance reqs | Low - can optimize later if needed |

## Open Questions Resolved

All clarifications from specification phase have been resolved:
- ✅ Infrastructure types: Comprehensive strict enumeration
- ✅ Contact details format: Structured object with standard fields
- ✅ SLA format: ODCS-inspired structure (lightweight, separate from ODCS)
- ✅ Storage strategy: Dedicated fields in Table and Relationship structs
- ✅ Scope: Data Flow nodes/relationships, NOT ODCS Data Contracts
- ✅ Format: Lightweight Data Flow format separate from ODCS

## Next Steps

Proceed to Phase 1: Design & Contracts
- Define data model structures (SlaProperty, ContactDetails, InfrastructureType)
- Update Table struct with new fields (for Data Flow nodes)
- Update Relationship struct with new fields (for Data Flow relationships)
- Design lightweight Data Flow format import/export (separate from ODCS)
- Create API contracts for search/filter operations
