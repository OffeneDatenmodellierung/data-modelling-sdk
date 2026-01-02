# Data Flow Node and Relationship API Contracts: Enhanced Metadata

**Date**: 2026-01-27
**Feature**: Enhanced Data Flow Node and Relationship Metadata
**Phase**: 1 - Design & Contracts

## Overview

This document defines the API contracts for enhanced metadata on Data Flow nodes (Tables) and relationships. These are Rust API contracts (struct definitions, method signatures) rather than HTTP API contracts, as this is a library/SDK. Note: This is for Data Flow elements, NOT for ODCS Data Contracts. ODCS format is only for Data Models (tables). We use a lightweight Data Flow format separate from ODCS.

## Struct Definitions

### Table (Enhanced for Data Flow Nodes)

```rust
pub struct Table {
    // Existing fields...
    pub id: Uuid,
    pub name: String,
    pub columns: Vec<Column>,
    // ... other existing fields ...

    // New metadata fields for Data Flow nodes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub sla: Option<Vec<SlaProperty>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact_details: Option<ContactDetails>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub infrastructure_type: Option<InfrastructureType>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,

    // ... existing fields ...
}
```

### Relationship (Enhanced for Data Flow Relationships)

```rust
pub struct Relationship {
    // Existing fields...
    pub id: Uuid,
    pub source_table_id: Uuid,
    pub target_table_id: Uuid,
    // ... other existing fields ...

    // New metadata fields for Data Flow relationships
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub sla: Option<Vec<SlaProperty>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact_details: Option<ContactDetails>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub infrastructure_type: Option<InfrastructureType>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,

    // ... existing fields ...
}
```

### SlaProperty

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SlaProperty {
    pub property: String,
    pub value: serde_json::Value,
    pub unit: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub element: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub driver: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduler: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schedule: Option<String>,
}
```

### ContactDetails

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContactDetails {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub other: Option<String>,
}
```

### InfrastructureType

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "PascalCase")]
pub enum InfrastructureType {
    // ... all variants as defined in data-model.md
}
```

## Method Contracts

### Table::new()

```rust
impl Table {
    pub fn new(name: String, columns: Vec<Column>) -> Self {
        // Returns Table with all new metadata fields set to None
        // Existing behavior unchanged
    }
}
```

**Preconditions**: None
**Postconditions**:
- All new metadata fields are `None`
- Table is ready for use with existing functionality

### DataModel Filter Methods

```rust
impl DataModel {
    /// Filter Data Flow nodes (tables) by owner
    pub fn filter_nodes_by_owner(&self, owner: &str) -> Vec<&Table> {
        // Returns all tables where owner matches (case-sensitive exact match)
    }

    /// Filter Data Flow relationships by owner
    pub fn filter_relationships_by_owner(&self, owner: &str) -> Vec<&Relationship> {
        // Returns all relationships where owner matches (case-sensitive exact match)
    }

    /// Filter Data Flow nodes (tables) by infrastructure type
    pub fn filter_nodes_by_infrastructure_type(
        &self,
        infra_type: InfrastructureType
    ) -> Vec<&Table> {
        // Returns all tables with matching infrastructure_type
    }

    /// Filter Data Flow relationships by infrastructure type
    pub fn filter_relationships_by_infrastructure_type(
        &self,
        infra_type: InfrastructureType
    ) -> Vec<&Relationship> {
        // Returns all relationships with matching infrastructure_type
    }

    /// Filter by tag (works for both nodes and relationships)
    pub fn filter_by_tags(&self, tag: &str) -> (Vec<&Table>, Vec<&Relationship>) {
        // Returns tables and relationships containing the specified tag
    }
}
```

**Preconditions**:
- DataModel contains tables and relationships
- For infrastructure type filters: infra_type is valid enum value

**Postconditions**:
- Returns vector of references to matching nodes/relationships
- Empty vector if no matches
- Performance: Returns in <1 second for up to 10,000 nodes/relationships (SC-003)

**Error Handling**: No errors - returns empty vector if no matches

## Lightweight Data Flow Format Import/Export Contracts

### DataFlowImporter::import()

**New Contract** (lightweight format, separate from ODCS):
- Extracts new metadata fields from lightweight Data Flow format YAML/JSON
- Stores in Table and Relationship struct dedicated fields
- Format is lightweight and focused on Data Flow needs (separate from ODCS)

**Metadata Extraction**:
- `owner`: From metadata.owner field
- `sla`: From metadata.sla array (ODCS-inspired structure)
- `contact_details`: From metadata.contact_details object
- `infrastructure_type`: From metadata.infrastructure_type (validated against enum)
- `notes`: From metadata.notes field

**Error Handling**:
- Invalid infrastructure_type: Returns ImportError with validation message
- Invalid SLA structure: Logs warning, stores partial data if possible
- Missing fields: Leaves as None (no error)

### DataFlowExporter::export()

**New Contract** (lightweight format, separate from ODCS):
- Exports new metadata fields to lightweight Data Flow format YAML/JSON
- Uses dedicated fields from Table and Relationship structs
- Format is lightweight and focused on Data Flow needs

**Metadata Export**:
- `owner`: Exported to metadata.owner
- `sla`: Exported to metadata.sla array (ODCS-inspired structure)
- `contact_details`: Exported to metadata.contact_details object
- `infrastructure_type`: Exported to metadata.infrastructure_type
- `notes`: Exported to metadata.notes

**Serialization**:
- None values omitted (skip_serializing_if)
- Empty arrays/objects omitted
- PascalCase for InfrastructureType enum
- Lightweight structure (no ODCS overhead)

## Serialization Contracts

### JSON Serialization

```rust
// Table with metadata serializes to:
{
  "id": "uuid",
  "name": "table_name",
  "columns": [...],
  "owner": "Team Name",  // omitted if None
  "sla": [               // omitted if None
    {
      "property": "latency",
      "value": 4,
      "unit": "hours",
      "description": "Data must be available within 4 hours"
    }
  ],
  "contact_details": {   // omitted if None
    "email": "team@example.com",
    "name": "Data Team"
  },
  "infrastructure_type": "Kafka",  // omitted if None, PascalCase
  "notes": "Additional context",   // omitted if None
  // ... other fields ...
}
```

### YAML Serialization (Lightweight Data Flow Format)

```yaml
# Lightweight Data Flow format (separate from ODCS)
nodes:
  - id: uuid
    name: node_name
    type: table
    metadata:
      owner: "Team Name"
      infrastructure_type: "Kafka"
      notes: "Additional context"
      sla:
        - property: latency
          value: 4
          unit: hours
          description: "Data must be available within 4 hours"
      contact_details:
        email: "team@example.com"
        name: "Data Team"

relationships:
  - id: uuid
    source_node_id: uuid
    target_node_id: uuid
    metadata:
      owner: "Team Name"
      infrastructure_type: "Kafka"
      notes: "Connection notes"
```

## Validation Contracts

### InfrastructureType Validation

```rust
impl InfrastructureType {
    /// Validate string value and convert to enum
    pub fn from_str(s: &str) -> Result<Self, InfrastructureTypeError> {
        // Returns Ok(InfrastructureType) if valid
        // Returns Err(InfrastructureTypeError) if invalid
    }
}
```

**Error Type**:
```rust
#[derive(Debug, thiserror::Error)]
pub enum InfrastructureTypeError {
    #[error("Invalid infrastructure type: {0}")]
    InvalidType(String),
}
```

### ContactDetails Validation

```rust
impl ContactDetails {
    /// Validate email format if provided
    pub fn validate(&self) -> Result<(), ContactDetailsError> {
        // Validates email format if email is Some
        // Returns Ok(()) if valid or email is None
        // Returns Err(ContactDetailsError) if email format invalid
    }
}
```

## Backward Compatibility Guarantees

1. **Existing Code**: All existing code using Table and Relationship structs continues to work without modification
2. **Serialization**: Tables and relationships without new metadata fields serialize identically to before
3. **Deserialization**: Old serialized tables/relationships deserialize correctly (new fields are None)
4. **Data Flow Format Import**: Old Data Flow format files import correctly (new fields extracted if present, otherwise None)
5. **Data Flow Format Export**: Export format maintains compatibility (new fields included when present)

## Performance Contracts

- **Search/Filter**: Returns results in <1 second for data models with up to 10,000 tables (SC-003)
- **Serialization**: Metadata fields add minimal overhead (<10% increase in serialization time)
- **Memory**: Optional fields use minimal memory when None (Option<T> overhead only)
- **Deserialization**: Handles missing fields gracefully (no performance penalty)
