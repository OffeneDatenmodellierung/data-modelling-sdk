# Feature Specification: Enhanced Table Metadata

**Feature Branch**: `002-table-metadata`
**Created**: 2026-01-27
**Status**: Draft
**Input**: User description: "Enhance node metadata to include Owner, SLA, Contact Details, Infrastructure Type (Kafka|Pulsar|EKS|Postgres|MySQL|MSSQL|other DB Types), Notes, Description, and Tags. This is for Data Flow nodes and relationships, NOT for ODCS Data Contracts. ODCS spec is only for Data Models (tables). Review ODCS meta information for suggestions but keep this lightweight info, not a full table descriptor. We will create a lightweight, cut-down spec for Data Flow nodes and relationships separate from ODCS."

## Clarifications

### Session 2026-01-27

- Q: What infrastructure types should be included in the enumeration? → A: Expand list to include DynamoDB, Cassandra, ElasticSearch in addition to Kafka, Pulsar, EKS, Postgres, MySQL, MSSQL, and other database/infrastructure types
- Q: Should contact details be stored as a single string or structured object? → A: Structured object with separate fields (e.g., email, phone, other)
- Q: Should infrastructure type be a strict enumeration or allow custom values? → A: Strict enumeration with comprehensive list covering all major cloud databases, container platforms, data warehouses, message queues, BI/analytics tools, and storage systems from AWS, Azure, and GCP
- Q: Should SLA be stored as free-form text or structured format? → A: Structured format following ODCS specification for servicelevels (slaProperties array with property, value, unit, element, driver, description, scheduler, schedule fields)
- Q: What are the exact field names in the contact details structured object? → A: Standard fields: email, phone, name, role (all optional), plus optional other field for additional contact methods
- Q: Is this metadata for ODCS Data Contracts (tables) or for Data Flow nodes/relationships? → A: This metadata is for Data Flow nodes and relationships, NOT for ODCS Data Contracts. ODCS spec is only for Data Models (tables). We will create a lightweight, cut-down spec for Data Flow nodes and relationships separate from ODCS.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Add Metadata Fields to Data Flow Nodes and Relationships (Priority: P1)

Data modelers and data engineers need to associate operational and governance metadata with Data Flow nodes and relationships to support data governance, operations, and collaboration workflows. Users need to record who owns a node/relationship, what service level agreements apply, how to contact responsible parties, what infrastructure type hosts the data, and additional notes for context. Note: This is for Data Flow nodes and relationships, NOT for ODCS Data Contracts (tables). ODCS spec is only for Data Models.

**Why this priority**: This is foundational functionality that enables data governance, operational support, and team collaboration. Without these metadata fields, users cannot properly document ownership, SLAs, or contact information, making it difficult to manage data assets effectively.

**Independent Test**: Can be fully tested by creating a Data Flow node or relationship, setting metadata fields (owner, SLA, contact details, infrastructure type, notes), and verifying that all metadata is stored correctly and can be retrieved. This delivers immediate value by enabling comprehensive metadata management for Data Flow elements.

**Acceptance Scenarios**:

1. **Given** a user creates a new Data Flow node or relationship, **When** they set owner, SLA, contact details, infrastructure type, and notes, **Then** all metadata fields are stored and can be retrieved with the node/relationship
2. **Given** a Data Flow node or relationship with existing metadata, **When** a user updates any metadata field, **Then** the updated value is stored and previous values are replaced
3. **Given** a Data Flow node or relationship with metadata, **When** a user exports it to a lightweight Data Flow format (separate from ODCS), **Then** the metadata fields are included in the exported format
4. **Given** a Data Flow format file containing metadata fields, **When** a user imports it, **Then** the metadata is preserved and available in the node/relationship structure

---

### User Story 2 - Search and Filter Data Flow Nodes and Relationships by Metadata (Priority: P2)

Users need to find Data Flow nodes and relationships based on metadata criteria such as owner, infrastructure type, or tags to support data discovery, governance audits, and operational tasks.

**Why this priority**: Once metadata exists, users need to leverage it for discovery and management. This enables users to answer questions like "What Data Flow nodes does team X own?" or "What Kafka nodes do we have?"

**Independent Test**: Can be fully tested by creating multiple tables with different metadata values, then searching/filtering by owner, infrastructure type, or tags, and verifying that only matching tables are returned. This delivers value by enabling efficient data asset discovery.

**Acceptance Scenarios**:

1. **Given** multiple Data Flow nodes/relationships with different owners, **When** a user searches for nodes/relationships by owner, **Then** only nodes/relationships matching that owner are returned
2. **Given** Data Flow nodes/relationships with different infrastructure types, **When** a user filters by infrastructure type (e.g., "Kafka"), **Then** only nodes/relationships with that infrastructure type are returned
3. **Given** Data Flow nodes/relationships with tags, **When** a user filters by tag, **Then** all nodes/relationships with that tag are returned

---

### User Story 3 - Preserve Metadata During Import/Export Operations (Priority: P1)

Users need assurance that metadata is preserved when Data Flow nodes and relationships are imported from or exported to various formats (lightweight Data Flow format, JSON, etc.), ensuring metadata continuity across different workflows. Note: This is separate from ODCS format, which is only for Data Models (tables).

**Why this priority**: Metadata preservation is critical for maintaining data governance across import/export cycles. Without this, users would lose important operational information when converting between formats.

**Independent Test**: Can be fully tested by creating a Data Flow node or relationship with metadata, exporting it to a lightweight Data Flow format, importing it back, and verifying that all metadata fields are preserved. This delivers value by ensuring metadata continuity.

**Acceptance Scenarios**:

1. **Given** a Data Flow node or relationship with owner, SLA, contact details, infrastructure type, and notes, **When** it is exported to a lightweight Data Flow format and then imported back, **Then** all metadata fields are preserved
2. **Given** a Data Flow format file with metadata, **When** it is imported, **Then** the metadata is extracted and stored in the node/relationship structure
3. **Given** a Data Flow node or relationship with metadata, **When** it is exported to formats that don't support custom metadata, **Then** the metadata is preserved in the SDK structure even if not included in the exported format

---

### Edge Cases

- What happens when a user sets an infrastructure type that doesn't match the database_type field?
- How does the system handle very long notes or contact details?
- What happens when metadata fields contain special characters or multi-line text?
- How does the system handle Data Flow nodes/relationships imported from format files that have metadata in different formats (e.g., team vs owner)?
- What happens when a user tries to set an infrastructure type value that is not in the predefined enumeration? (System must reject invalid values and only accept values from the strict enumeration)
- How does the system handle empty or null metadata values during export?
- How does the lightweight Data Flow format differ from ODCS format? (ODCS is only for Data Models/tables)

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST support storing owner information for each Data Flow node and relationship (single value: person, team, or organization name)
- **FR-002**: System MUST support storing SLA (Service Level Agreement) information for each Data Flow node and relationship as a structured format inspired by ODCS specification (lightweight, cut-down version): array of SLA property objects, each containing property (SLA attribute name like "latency", "availability"), value (metric value), unit (measurement unit like "hours", "percent"), and optional fields: element (data elements SLA applies to), driver (importance like "regulatory", "analytics", "operational"), description, scheduler, and schedule
- **FR-003**: System MUST support storing contact details for each Data Flow node and relationship as a structured object with standard fields: email (optional), phone (optional), name (optional), role (optional), and other (optional field for additional contact methods)
- **FR-004**: System MUST support storing infrastructure type for each Data Flow node and relationship as a strict enumeration from the following comprehensive list: **Traditional Databases**: PostgreSQL, MySQL, MSSQL, Oracle, SQLite, MariaDB; **NoSQL Databases**: DynamoDB, Cassandra, MongoDB, Redis, ElasticSearch, CouchDB, Neo4j; **AWS Services**: RDS (PostgreSQL, MySQL, MariaDB, Oracle, SQL Server), Redshift, Aurora, DocumentDB, Neptune, ElastiCache, S3, EKS, ECS, Lambda, Kinesis, SQS, SNS, Glue, Athena, QuickSight; **Azure Services**: Azure SQL Database, Cosmos DB, Azure Synapse Analytics, Azure Data Lake Storage, Azure Blob Storage, AKS, ACI, Azure Functions, Event Hubs, Service Bus, Azure Data Factory, PowerBI; **Google Cloud Services**: Cloud SQL (PostgreSQL, MySQL, SQL Server), BigQuery, Cloud Spanner, Firestore, Cloud Storage, GKE, Cloud Run, Cloud Functions, Pub/Sub, Dataflow, Looker; **Message Queues/Streaming**: Kafka, Pulsar, RabbitMQ, ActiveMQ; **Container Platforms**: Kubernetes, Docker, EKS, ECS, AKS, GKE; **Data Warehouses**: Snowflake, Databricks, Teradata, Vertica; **BI/Analytics Tools**: Tableau, Qlik, Metabase, Apache Superset, Grafana; **Other Storage**: HDFS, MinIO
- **FR-005**: System MUST support storing notes for each Data Flow node and relationship (free-form text for additional context)
- **FR-006**: System MUST support storing description for each Data Flow node and relationship (ensure it remains supported)
- **FR-007**: System MUST support storing tags for each Data Flow node and relationship (ensure it remains supported)
- **FR-008**: System MUST store all metadata fields as optional (nullable) to support nodes/relationships without complete metadata
- **FR-009**: System MUST preserve metadata fields during Data Flow format import operations (lightweight format, separate from ODCS)
- **FR-010**: System MUST include metadata fields in Data Flow format export operations (lightweight format, separate from ODCS)
- **FR-011**: System MUST allow users to search and filter Data Flow nodes and relationships by owner, infrastructure type, and tags
- **FR-012**: System MUST handle metadata fields independently from database_type field (infrastructure type is separate from database engine type)
- **FR-013**: System MUST support metadata values containing multi-line text, special characters, and unicode characters
- **FR-014**: System MUST maintain backward compatibility with existing Data Flow nodes and relationships that don't have the new metadata fields
- **FR-015**: System MUST use a lightweight, cut-down specification format for Data Flow nodes and relationships, separate from ODCS (which is only for Data Models/tables)

### Key Entities *(include if feature involves data)*

- **Data Flow Node**: Represents a node in a Data Flow diagram (can be a table, service, or other data processing element). Enhanced with new metadata fields: owner, sla (structured array inspired by ODCS servicelevels format with property, value, unit, element, driver, description, scheduler, schedule), contact_details (structured object with email, phone, name, role, other fields), infrastructure_type, notes. Note: This is separate from ODCS Data Contracts which are only for Data Models (tables).
- **Data Flow Relationship**: Represents a relationship/connection between Data Flow nodes. Enhanced with the same metadata fields as nodes. Note: This is separate from ODCS format.
- **Metadata**: Lightweight operational and governance information associated with Data Flow nodes and relationships. Stored in node/relationship structure, not as separate entities. Includes owner, SLA, contact details, infrastructure type, notes, description, and tags. Uses a lightweight, cut-down specification format separate from ODCS.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Users can set and retrieve all metadata fields (owner, SLA, contact details, infrastructure type, notes) for Data Flow nodes and relationships with 100% accuracy
- **SC-002**: Metadata fields are preserved with 100% accuracy during Data Flow format import/export roundtrips (export then import preserves all fields) - note: this is separate from ODCS format
- **SC-003**: Users can search and filter Data Flow nodes and relationships by owner, infrastructure type, or tags with results returned in under 1 second for data models with up to 10,000 nodes/relationships
- **SC-004**: All metadata fields support text values up to 10,000 characters without data loss or performance degradation
- **SC-005**: Data Flow nodes and relationships imported from existing format files without new metadata fields continue to function correctly (100% backward compatibility)
- **SC-006**: Metadata fields are included in Data Flow format exports in a format that can be round-tripped (exported and re-imported) without loss - note: this is a lightweight format separate from ODCS

## Assumptions

- Metadata fields are optional and can be null/empty for tables that don't have complete metadata
- Infrastructure type is distinct from database_type (database_type indicates the database engine like PostgreSQL, while infrastructure_type indicates the hosting platform, service, or tool like Kafka, EKS, PowerBI, Snowflake, etc.) and must be selected from a strict predefined enumeration covering all major cloud services, databases, containers, and BI/analytics tools
- Owner can be a person name, team name, or organization name (single string value)
- SLA is stored as a structured format following ODCS specification (servicelevels/slaProperties): array of objects with property, value, unit, and optional element, driver, description, scheduler, schedule fields to support comprehensive SLA documentation aligned with ODCS standard
- Contact details are stored as a structured object with standard fields: email, phone, name, role (all optional), plus optional other field for additional contact methods, allowing flexible contact information per table
- Notes field supports multi-line text for detailed context
- Description field already exists in odcl_metadata and should continue to be supported
- Tags field already exists as a separate array field on Table and should continue to be supported
- Data Flow format (lightweight, separate from ODCS) supports storing metadata for nodes and relationships
- ODCS format is ONLY for Data Models (tables), NOT for Data Flow nodes and relationships
- Metadata is lightweight operational information, not a full node descriptor (no need to duplicate schema details)
- We will create a lightweight, cut-down specification format for Data Flow nodes and relationships, separate from ODCS

## Dependencies

- Existing Data Flow node and relationship model structures
- Existing lightweight Data Flow format import/export functionality (separate from ODCS)
- ODCS specification (for reference/inspiration only - ODCS is only for Data Models/tables, not Data Flow)
- Existing metadata storage structures for flexible metadata

## Out of Scope

- ODCS Data Contract format (ODCS is only for Data Models/tables, not Data Flow nodes/relationships)
- Full node descriptor information (this is lightweight metadata, not schema details)
- Metadata validation rules or constraints beyond basic type checking
- Metadata versioning or change tracking
- Metadata inheritance or templating
- Complex metadata relationships or hierarchies
- Metadata access control or permissions
