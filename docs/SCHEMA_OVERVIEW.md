# Schema Overview Guide

This guide provides an overview of the different schemas supported by the Data Modelling SDK and how they are used.

## Table of Contents

1. [ODCS (Open Data Contract Standard)](#odcs-open-data-contract-standard)
2. [ODCL (Open Data Contract Language)](#odcl-open-data-contract-language)
3. [ODPS (Open Data Product Standard)](#odps-open-data-product-standard)
4. [CADS (Compute Asset Description Specification)](#cads-compute-asset-description-specification)
5. [Business Domain Schema](#business-domain-schema)
6. [Other Formats](#other-formats)
7. [Universal Converter](#universal-converter)

---

## ODCS (Open Data Contract Standard)

**Version**: v3.1.0
**Purpose**: Data Contracts (tables/schemas)
**Status**: Primary format for data models

### Overview

ODCS is the primary format for defining data contracts (tables). It provides comprehensive metadata about data structures, including:

- Schema definitions with properties/fields
- Quality rules and validation checks
- Service level agreements (SLAs)
- Tags and metadata
- References to external definitions

### Key Features

- **Full Schema Coverage**: Supports all ODCS v3.1.0 fields including `description`, `quality` arrays, and `$ref` references
- **Field Preservation**: All metadata is preserved during import/export operations
- **Enhanced Tags**: Supports Simple, Pair, and List tag formats

### Usage

```rust
use data_modelling_sdk::import::ODCSImporter;
use data_modelling_sdk::export::ODCSExporter;

// Import ODCS YAML
let mut importer = ODCSImporter::new();
let result = importer.import(odcs_yaml)?;

// Export to ODCS YAML
let exporter = ODCSExporter::new();
let yaml = exporter.export_table(&table, "odcs_v3_1_0")?;
```

### When to Use

- Defining data contracts (tables)
- Sharing schema definitions between systems
- Data governance and documentation
- Quality assurance and validation

---

## ODCL (Open Data Contract Language)

**Version**: v1.2.1 (Last Supported)
**Purpose**: Legacy data contract format
**Status**: Legacy format, full support maintained

### Overview

ODCL is the legacy format for data contracts. It's similar to ODCS but uses a different structure. The SDK provides full backward compatibility.

### Key Features

- **Legacy Support**: Full support for ODCL v1.2.1
- **Field Preservation**: All fields including `description`, `quality`, and `$ref` are preserved
- **Auto-Detection**: Automatically detected during import

### Usage

```rust
use data_modelling_sdk::import::ODCSImporter; // Same importer handles both ODCS and ODCL
use data_modelling_sdk::export::ODCLExporter;

// Import ODCL YAML (automatically detected)
let mut importer = ODCSImporter::new();
let result = importer.import(odcl_yaml)?;

// Export to ODCL format
let exporter = ODCLExporter::new();
let yaml = exporter.export_table(&table, "odcl")?;
```

### When to Use

- Working with legacy ODCL files
- Migrating from ODCL to ODCS
- Maintaining backward compatibility

---

## ODPS (Open Data Product Standard)

**Version**: Latest
**Purpose**: Data Products
**Status**: Full import/export support

### Overview

ODPS defines data products that link to ODCS Tables via `contractId` references. Data products represent higher-level abstractions that consume or produce data contracts.

### Key Features

- **Data Product Definition**: Complete support for ODPS data product structure
- **Contract Linking**: Links to ODCS Tables via `contractId` in input/output ports
- **Validation**: Validates `contractId` references against known ODCS Tables
- **Full Metadata**: Supports all ODPS fields including ports, support, team, and custom properties

### Usage

```rust
use data_modelling_sdk::import::ODPSImporter;
use data_modelling_sdk::export::ODPSExporter;

// Import ODPS YAML
let importer = ODPSImporter::new();
let product = importer.import(odps_yaml)?;

// Export to ODPS YAML
let exporter = ODPSExporter::new();
let yaml = exporter.export_product(&product)?;
```

### When to Use

- Defining data products
- Linking multiple data contracts together
- Product-level governance and documentation
- API and service definitions

---

## CADS (Compute Asset Description Specification)

**Version**: v1.0
**Purpose**: AI/ML models, applications, pipelines
**Status**: Full import/export support

### Overview

CADS describes computational assets including AI/ML models, ML pipelines, traditional applications, ETL pipelines, and source/destination systems. It focuses on governance, risk management, and operational clarity without embedding data schemas.

### Key Features

- **Asset Kinds**: Supports AIModel, MLPipeline, Application, ETLPipeline, SourceSystem, DestinationSystem
- **Governance-First**: Risk, compliance, and ownership are first-class concepts
- **Runtime Context**: Describes where and how assets execute
- **SLA Support**: Service level agreements for operational guarantees
- **Validation Profiles**: Defines expected checks based on asset type or risk

### Usage

```rust
use data_modelling_sdk::import::CADSImporter;
use data_modelling_sdk::export::CADSExporter;

// Import CADS YAML
let importer = CADSImporter::new();
let asset = importer.import(cads_yaml)?;

// Export to CADS YAML
let exporter = CADSExporter::new();
let yaml = exporter.export(&asset)?;
```

### When to Use

- Describing AI/ML models
- Documenting applications and pipelines
- Governance and risk management
- Operational documentation

---

## Business Domain Schema

**Version**: Custom (SDK-specific)
**Purpose**: Organize systems, CADS nodes, and ODCS nodes
**Status**: Full support

### Overview

The Business Domain schema is a top-level organizational structure that groups systems, CADS nodes, and ODCS nodes within business domains. It provides:

- **Systems**: Physical infrastructure entities (Kafka, Cassandra, EKS, EC2, etc.)
- **CADS Nodes**: References to CADS assets (AI/ML models, applications, pipelines)
- **ODCS Nodes**: References to ODCS Tables (data contracts)
- **Connections**: ERD-style connections between systems, Crowsfeet notation for ODCS nodes

### Key Features

- **System Metadata**: Systems inherit DataFlow metadata (owner, SLA, contact_details, infrastructure_type, notes)
- **Shared References**: Systems, CADS nodes, and ODCS nodes can be shared across domains
- **Relationship Types**: ERD-style for systems, Crowsfeet notation for ODCS nodes
- **Versioning**: Systems have version fields for cross-domain sharing

### Usage

```rust
use data_modelling_sdk::models::{Domain, System, InfrastructureType};
use data_modelling_sdk::models::domain::{CADSNode, ODCSNode, CADSKind};
use uuid::Uuid;

// Create a domain
let mut domain = Domain::new("customer-service".to_string());

// Add a system
let system = System::new(
    "kafka-cluster".to_string(),
    InfrastructureType::Kafka,
    domain.id,
);
domain.add_system(system);

// Import/Export Domain YAML
let yaml = domain.to_yaml()?;
let domain2 = Domain::from_yaml(&yaml)?;
```

### When to Use

- Organizing infrastructure within business domains
- Mapping data flow across systems
- Cross-domain data sharing
- Enterprise architecture documentation

---

## Other Formats

### SQL

**Purpose**: SQL DDL statements
**Support**: Import and export

```rust
use data_modelling_sdk::import::SQLImporter;
use data_modelling_sdk::export::SQLExporter;

let importer = SQLImporter::new("postgresql");
let result = importer.parse(sql_ddl)?;

let exporter = SQLExporter;
let sql = exporter.export(&tables, Some("postgresql"))?;
```

### JSON Schema

**Purpose**: JSON Schema definitions
**Support**: Import and export

```rust
use data_modelling_sdk::import::JSONSchemaImporter;
use data_modelling_sdk::export::JSONSchemaExporter;

let importer = JSONSchemaImporter::new();
let result = importer.import(json_schema)?;

let exporter = JSONSchemaExporter;
let json = exporter.export(&tables)?;
```

### AVRO

**Purpose**: AVRO schema definitions
**Support**: Import and export

```rust
use data_modelling_sdk::import::AvroImporter;
use data_modelling_sdk::export::AvroExporter;

let importer = AvroImporter::new();
let result = importer.import(avro_schema)?;

let exporter = AvroExporter;
let avro = exporter.export(&tables)?;
```

### Protobuf

**Purpose**: Protocol Buffer definitions
**Support**: Import and export

```rust
use data_modelling_sdk::import::ProtobufImporter;
use data_modelling_sdk::export::ProtobufExporter;

let importer = ProtobufImporter::new();
let result = importer.import(protobuf_content)?;

let exporter = ProtobufExporter;
let proto = exporter.export(&tables)?;
```

---

## Universal Converter

The universal converter (`convert_to_odcs`) can convert any supported format to ODCS v3.1.0 format.

### Supported Formats

- SQL
- ODCS v3.1.0
- ODCL v1.2.1
- JSON Schema
- AVRO
- Protobuf
- CADS v1.0
- ODPS (Latest)
- Domain Schema

### Usage

```rust
use data_modelling_sdk::convert::convert_to_odcs;

// Auto-detect format
let odcs_yaml = convert_to_odcs(input_content, None)?;

// Explicit format
let odcs_yaml = convert_to_odcs(input_content, Some("sql"))?;
```

### Format Detection

The converter automatically detects formats based on content:

- **ODCS**: Contains `apiVersion:` and `kind: DataContract`
- **ODCL**: Contains `dataContractSpecification:`
- **SQL**: Contains `CREATE TABLE`
- **JSON Schema**: JSON object with `$schema` or `type`
- **AVRO**: JSON with `type`, `fields`, `name`
- **Protobuf**: Contains `syntax`, `message`, or `service`
- **CADS**: Contains `apiVersion:` and `kind: AIModel|MLPipeline|Application|...`
- **ODPS**: Contains `apiVersion:` and `kind: DataProduct`
- **Domain**: Contains `systems:` and `odcs_nodes:` or `cads_nodes:`

### Conversion Notes

- **CADS → ODCS**: Returns an error explaining that CADS assets represent compute resources, not data contracts
- **ODPS → ODCS**: Requires `contractId` references and ODCS Table definitions
- **Domain → ODCS**: Requires Table definitions (Domain only stores references)

---

## Schema Comparison

| Schema | Purpose | Primary Use Case | Data Contracts | Compute Assets | Products |
|--------|---------|------------------|----------------|---------------|----------|
| **ODCS** | Data Contracts | Tables/Schemas | ✅ | ❌ | ❌ |
| **ODCL** | Data Contracts (Legacy) | Legacy Tables | ✅ | ❌ | ❌ |
| **ODPS** | Data Products | Products linking Tables | ✅ (via refs) | ❌ | ✅ |
| **CADS** | Compute Assets | AI/ML/Applications | ❌ | ✅ | ❌ |
| **Domain** | Organization | Systems & Nodes | ✅ (via refs) | ✅ (via refs) | ❌ |

---

## Migration Guide

### DataFlow → Domain Schema

The DataFlow format has been migrated to the Domain schema. Use the migration utility:

```rust
use data_modelling_sdk::convert::migrate_dataflow::migrate_dataflow_to_domain;

let domain = migrate_dataflow_to_domain(dataflow_yaml, Some("domain-name"))?;
```

**Migration Mapping**:
- DataFlow nodes → Systems (with all metadata preserved)
- DataFlow relationships → SystemConnections (ERD-style)

---

## Best Practices

1. **Use ODCS for Data Contracts**: ODCS v3.1.0 is the primary format for tables/schemas
2. **Use CADS for Compute Assets**: CADS is designed for AI/ML models and applications
3. **Use ODPS for Data Products**: ODPS links multiple data contracts together
4. **Use Domain Schema for Organization**: Domain schema organizes systems and nodes within business domains
5. **Preserve Metadata**: Always use import/export functions to preserve metadata during conversions
6. **Validate References**: When using ODPS, validate `contractId` references against known ODCS Tables

---

## Enhanced Tag Support

All schemas support enhanced tag formats:

- **Simple**: `"finance"` - Single word tags
- **Pair**: `"Environment:Dev"` - Key:Value pairs
- **List**: `"SecondaryDomains:[XXXXX, PPPP]"` - Key:[Value1, Value2, ...] lists

Tags are automatically detected and parsed during import, and serialized as strings during export.

---

## Further Reading

- [ODCS Specification](https://github.com/bitol-io/open-data-contract-standard)
- [ODPS Specification](https://github.com/bitol-io/open-data-product-standard)
- [CADS Specification](https://github.com/your-org/cads-spec) (when available)
- SDK Documentation: See `README.md` and `LLM.txt` for detailed API documentation
