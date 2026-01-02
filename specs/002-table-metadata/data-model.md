# Data Model: Enhanced Data Flow Node and Relationship Metadata

**Date**: 2026-01-27
**Feature**: Enhanced Data Flow Node and Relationship Metadata
**Phase**: 1 - Design & Contracts

## Overview

This document defines the data structures for enhanced metadata for Data Flow nodes (Tables) and relationships including owner, SLA, contact details, infrastructure type, and notes. All structures extend the existing Table and Relationship models while maintaining backward compatibility. Note: This is for Data Flow elements, NOT for ODCS Data Contracts. ODCS format is only for Data Models (tables). We use a lightweight, cut-down specification format for Data Flow separate from ODCS.

## Core Entities

### Table (Enhanced for Data Flow Nodes)

The `Table` struct is enhanced with new optional metadata fields for use as Data Flow nodes:

```rust
pub struct Table {
    // ... existing fields ...

    /// Owner information (person, team, or organization name)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,

    /// SLA (Service Level Agreement) information (ODCS-inspired but lightweight format)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sla: Option<Vec<SlaProperty>>,

    /// Contact details for responsible parties
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact_details: Option<ContactDetails>,

    /// Infrastructure type (hosting platform, service, or tool)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub infrastructure_type: Option<InfrastructureType>,

    /// Additional notes and context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,

    // ... existing fields including odcl_metadata ...
}
```

**Relationships**:
- One Table (Data Flow node) has zero or one Owner (String)
- One Table (Data Flow node) has zero or many SLA Properties (Vec<SlaProperty>)
- One Table (Data Flow node) has zero or one ContactDetails
- One Table (Data Flow node) has zero or one InfrastructureType
- One Table (Data Flow node) has zero or one Notes (String)

**Validation Rules**:
- Owner: Max 255 characters, can contain any Unicode characters
- InfrastructureType: Must be from predefined enumeration (strict validation)
- Notes: Max 10,000 characters (SC-004)
- ContactDetails: All fields optional, email should be valid format if provided
- SLA: Array of valid SlaProperty objects

### Relationship (Enhanced for Data Flow Relationships)

The `Relationship` struct is enhanced with the same optional metadata fields for use as Data Flow relationships:

```rust
pub struct Relationship {
    // ... existing fields ...

    /// Owner information (person, team, or organization name)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,

    /// SLA (Service Level Agreement) information (ODCS-inspired but lightweight format)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sla: Option<Vec<SlaProperty>>,

    /// Contact details for responsible parties
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact_details: Option<ContactDetails>,

    /// Infrastructure type (hosting platform, service, or tool)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub infrastructure_type: Option<InfrastructureType>,

    /// Additional notes and context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,

    // ... existing fields ...
}
```

**Relationships**:
- One Relationship (Data Flow relationship) has zero or one Owner (String)
- One Relationship (Data Flow relationship) has zero or many SLA Properties (Vec<SlaProperty>)
- One Relationship (Data Flow relationship) has zero or one ContactDetails
- One Relationship (Data Flow relationship) has zero or one InfrastructureType
- One Relationship (Data Flow relationship) has zero or one Notes (String)

**Validation Rules**: Same as Table (above)

### SlaProperty

Represents a single SLA property using ODCS-inspired structure (lightweight format for Data Flow):

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SlaProperty {
    /// SLA attribute name (e.g., "latency", "availability", "throughput")
    pub property: String,

    /// Metric value (flexible type to support numbers, strings, etc.)
    pub value: serde_json::Value,

    /// Measurement unit (e.g., "hours", "percent", "requests_per_second")
    pub unit: String,

    /// Optional: Data elements this SLA applies to
    #[serde(skip_serializing_if = "Option::is_none")]
    pub element: Option<String>,

    /// Optional: Importance driver (e.g., "regulatory", "analytics", "operational")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub driver: Option<String>,

    /// Optional: Description of the SLA
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Optional: Scheduler type for monitoring
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduler: Option<String>,

    /// Optional: Schedule expression (e.g., cron format)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schedule: Option<String>,
}
```

**Fields**:
- `property`: Required - SLA attribute name
- `value`: Required - Metric value (flexible JSON value)
- `unit`: Required - Measurement unit
- `element`: Optional - Data elements SLA applies to
- `driver`: Optional - Importance/priority indicator
- `description`: Optional - Human-readable description
- `scheduler`: Optional - Monitoring scheduler type
- `schedule`: Optional - Schedule expression for monitoring

**Validation Rules**:
- Property: Max 100 characters, alphanumeric with underscores/hyphens
- Unit: Max 50 characters
- Description: Max 1,000 characters
- Schedule: Valid cron expression if scheduler provided

### ContactDetails

Structured contact information for Data Flow node/relationship owners/responsible parties:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContactDetails {
    /// Email address
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,

    /// Phone number
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,

    /// Contact name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Role or title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,

    /// Other contact methods or additional information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub other: Option<String>,
}
```

**Fields**: All optional to support partial contact information

**Validation Rules**:
- Email: Valid email format if provided (RFC 5322)
- Phone: Max 50 characters
- Name: Max 255 characters
- Role: Max 100 characters
- Other: Max 500 characters

### InfrastructureType

Strict enumeration of supported infrastructure types:

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "PascalCase")]
pub enum InfrastructureType {
    // Traditional Databases
    PostgreSQL,
    MySQL,
    Mssql,
    Oracle,
    Sqlite,
    MariaDB,

    // NoSQL Databases
    DynamoDB,
    Cassandra,
    MongoDB,
    Redis,
    ElasticSearch,
    CouchDB,
    Neo4j,

    // AWS Services
    RdsPostgreSQL,
    RdsMySQL,
    RdsMariaDB,
    RdsOracle,
    RdsSqlServer,
    Redshift,
    Aurora,
    DocumentDB,
    Neptune,
    ElastiCache,
    S3,
    Eks,
    Ecs,
    Lambda,
    Kinesis,
    Sqs,
    Sns,
    Glue,
    Athena,
    QuickSight,

    // Azure Services
    AzureSqlDatabase,
    CosmosDB,
    AzureSynapseAnalytics,
    AzureDataLakeStorage,
    AzureBlobStorage,
    Aks,
    Aci,
    AzureFunctions,
    EventHubs,
    ServiceBus,
    AzureDataFactory,
    PowerBI,

    // Google Cloud Services
    CloudSqlPostgreSQL,
    CloudSqlMySQL,
    CloudSqlSqlServer,
    BigQuery,
    CloudSpanner,
    Firestore,
    CloudStorage,
    Gke,
    CloudRun,
    CloudFunctions,
    PubSub,
    Dataflow,
    Looker,

    // Message Queues/Streaming
    Kafka,
    Pulsar,
    RabbitMQ,
    ActiveMQ,

    // Container Platforms
    Kubernetes,
    Docker,

    // Data Warehouses
    Snowflake,
    Databricks,
    Teradata,
    Vertica,

    // BI/Analytics Tools
    Tableau,
    Qlik,
    Metabase,
    ApacheSuperset,
    Grafana,

    // Other Storage
    Hdfs,
    MinIO,
}
```

**Validation**: Must be one of the predefined values. Invalid values rejected during deserialization.

**Serialization**: Serialized as PascalCase strings (e.g., "PostgreSQL", "DynamoDB", "AzureSqlDatabase")

## Data Model Relationships

```
Table (Data Flow Node)
├── owner: Option<String>
├── sla: Option<Vec<SlaProperty>>
├── contact_details: Option<ContactDetails>
├── infrastructure_type: Option<InfrastructureType>
├── notes: Option<String>
└── ... existing fields ...

Relationship (Data Flow Relationship)
├── owner: Option<String>
├── sla: Option<Vec<SlaProperty>>
├── contact_details: Option<ContactDetails>
├── infrastructure_type: Option<InfrastructureType>
├── notes: Option<String>
└── ... existing fields ...
```

## State Transitions

### Table Creation (Data Flow Node)
1. Table created with `Table::new(name, columns)`
2. All new metadata fields default to `None`
3. Existing behavior unchanged

### Relationship Creation (Data Flow Relationship)
1. Relationship created with `Relationship::new(source_table_id, target_table_id)`
2. All new metadata fields default to `None`
3. Existing behavior unchanged

### Metadata Addition
1. User sets metadata fields (owner, sla, contact_details, infrastructure_type, notes) on Table or Relationship
2. Fields stored in respective struct
3. Metadata applies to Data Flow context (separate from ODCS)

### Metadata Update
1. User updates any metadata field
2. Previous value replaced (no versioning)
3. Updated value stored immediately

### Import/Export (Lightweight Data Flow Format)
1. Data Flow Format Import: Extract metadata from lightweight Data Flow format file
2. Store in dedicated fields if structure matches
3. Format is lightweight and separate from ODCS (ODCS is only for Data Models/tables)
4. Data Flow Format Export: Export metadata using dedicated fields to lightweight format

## Validation Rules

### Table Level
- All metadata fields optional (can be None)
- InfrastructureType must be valid enum value if provided
- Notes max 10,000 characters
- Owner max 255 characters

### SlaProperty Level
- Property required, max 100 characters
- Value required (any JSON value)
- Unit required, max 50 characters
- All other fields optional

### ContactDetails Level
- All fields optional
- Email must be valid format if provided
- Max lengths: phone (50), name (255), role (100), other (500)

### InfrastructureType Level
- Must be from predefined enumeration
- Invalid values rejected with clear error message

## Backward Compatibility

- Existing tables and relationships without metadata continue to work
- All new fields are `Option<T>` with `skip_serializing_if = "Option::is_none"`
- Lightweight Data Flow format import/export handles missing fields gracefully
- Existing metadata structures preserved for backward compatibility

## Performance Considerations

- Metadata fields stored inline in Table struct (no indirection)
- Optional fields use minimal memory when None
- Search/filter operations iterate tables linearly (meets <1s requirement for 10k tables)
- Serialization optimized with skip_serializing_if to reduce JSON/YAML size
