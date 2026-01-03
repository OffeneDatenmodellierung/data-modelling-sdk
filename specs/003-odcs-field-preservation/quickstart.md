# Quickstart Guide: ODCS/ODCL Field Preservation & Universal Format Conversion

This guide provides quick examples for using the enhanced features in the Data Modelling SDK.

## Table of Contents

1. [ODCS/ODCL Field Preservation](#odcsodcl-field-preservation)
2. [Enhanced Tag Support](#enhanced-tag-support)
3. [CADS Import/Export](#cads-importexport)
4. [ODPS Import/Export](#odps-importexport)
5. [Domain Schema Usage](#domain-schema-usage)
6. [Universal Converter](#universal-converter)
7. [WASM Usage](#wasm-usage)

---

## ODCS/ODCL Field Preservation

### Import ODCS with Complete Field Preservation

```rust
use data_modelling_sdk::import::ODCSImporter;

let odcs_yaml = r#"
apiVersion: v3.1.0
kind: DataContract
id: users-contract
schema:
  properties:
    id:
      type: integer
      description: User identifier
      quality:
        - type: sql
          description: Must be positive
          query: SELECT COUNT(*) FROM users WHERE id <= 0
          mustBeGreaterThan: 0
    email:
      type: string
      format: email
      description: User email address
      $ref: '#/definitions/email'
"#;

let mut importer = ODCSImporter::new();
let result = importer.import(odcs_yaml)?;

// All fields are preserved: description, quality, $ref
for table_data in &result.tables {
    for column in &table_data.columns {
        println!("Column: {}", column.name);
        println!("  Description: {:?}", column.description);
        println!("  Quality rules: {:?}", column.quality);
        println!("  Reference: {:?}", column.ref_path);
    }
}
```

### Round-Trip Preservation

```rust
use data_modelling_sdk::import::ODCSImporter;
use data_modelling_sdk::export::ODCSExporter;

// Import
let mut importer = ODCSImporter::new();
let result = importer.import(odcs_yaml)?;

// Convert to Table
let table = result.tables[0].to_table()?;

// Export back
let exporter = ODCSExporter::new();
let exported_yaml = exporter.export_table(&table, "odcs_v3_1_0")?;

// All fields (description, quality, $ref) are preserved
```

---

## Enhanced Tag Support

### Simple Tags

```rust
use data_modelling_sdk::models::{Table, Column, Tag};

let mut table = Table::new("users".to_string(), vec![
    Column::new("id".to_string(), "INTEGER".to_string())
]);

// Add simple tags
table.tags.push(Tag::Simple("finance".to_string()));
table.tags.push(Tag::Simple("production".to_string()));
```

### Pair Tags (Key:Value)

```rust
use data_modelling_sdk::models::Tag;

// Parse pair tag
let tag = Tag::from_str("Environment:Dev").unwrap();
assert_eq!(tag, Tag::Pair("Environment".to_string(), "Dev".to_string()));

// Serialize
assert_eq!(tag.to_string(), "Environment:Dev");
```

### List Tags (Key:[Value1, Value2])

```rust
use data_modelling_sdk::models::Tag;

// Parse list tag
let tag = Tag::from_str("SecondaryDomains:[Finance, Sales]").unwrap();
assert_eq!(
    tag,
    Tag::List("SecondaryDomains".to_string(), vec!["Finance".to_string(), "Sales".to_string()])
);

// Serialize
assert_eq!(tag.to_string(), "SecondaryDomains:[Finance, Sales]");
```

### Auto-Detection

```rust
use data_modelling_sdk::models::Tag;
use std::str::FromStr;

// Automatically detects format
let simple = Tag::from_str("finance").unwrap();
let pair = Tag::from_str("Environment:Dev").unwrap();
let list = Tag::from_str("Domains:[A, B]").unwrap();

// Malformed tags gracefully degrade to Simple
let malformed = Tag::from_str("key:value:extra").unwrap();
assert_eq!(malformed, Tag::Simple("key:value:extra".to_string()));
```

---

## CADS Import/Export

### Import CADS Asset

```rust
use data_modelling_sdk::import::CADSImporter;

let cads_yaml = r#"
apiVersion: v1.0
kind: AIModel
id: 550e8400-e29b-41d4-a716-446655440000
name: customer-churn-predictor
version: 1.0.0
status: active
description:
  purpose: Predict customer churn probability
  usage: Used by customer success team for proactive outreach
  limitations: Not suitable for real-time inference
runtime:
  environment: production
  location: s3://models/customer-churn/v1.0
sla:
  availability:
    percentage: 99.9%
team:
  owner: ML Engineering Team
  contact:
    email: ml-team@example.com
"#;

let importer = CADSImporter::new();
let asset = importer.import(cads_yaml)?;

println!("Asset: {}", asset.name);
println!("Kind: {:?}", asset.kind);
println!("Status: {:?}", asset.status);
```

### Export CADS Asset

```rust
use data_modelling_sdk::export::CADSExporter;
use data_modelling_sdk::models::cads::{CADSAsset, CADSKind, CADSStatus};

let asset = CADSAsset {
    api_version: "v1.0".to_string(),
    kind: CADSKind::AIModel,
    id: uuid::Uuid::new_v4(),
    name: "my-model".to_string(),
    version: Some("1.0.0".to_string()),
    status: CADSStatus::Draft,
    // ... other fields
};

let exporter = CADSExporter;
let yaml = exporter.export(&asset)?;
println!("{}", yaml);
```

---

## ODPS Import/Export

### Import ODPS Data Product

```rust
use data_modelling_sdk::import::ODPSImporter;

let odps_yaml = r#"
apiVersion: v1.0.0
kind: DataProduct
id: 550e8400-e29b-41d4-a716-446655440000
name: customer-analytics-product
version: 1.0.0
status: active
inputPorts:
  - name: customer-data
    contractId: customer-table-id
    description: Customer data input
outputPorts:
  - name: analytics-results
    contractId: analytics-table-id
    description: Analytics results output
"#;

let importer = ODPSImporter::new();
let product = importer.import(odps_yaml)?;

println!("Product: {}", product.name);
println!("Input ports: {:?}", product.input_ports);
println!("Output ports: {:?}", product.output_ports);
```

### Export ODPS Data Product

```rust
use data_modelling_sdk::export::ODPSExporter;
use data_modelling_sdk::models::odps::{ODPSDataProduct, ODPSStatus, ODPSInputPort, ODPSOutputPort};

let product = ODPSDataProduct {
    api_version: "v1.0.0".to_string(),
    kind: "DataProduct".to_string(),
    id: uuid::Uuid::new_v4(),
    name: Some("my-product".to_string()),
    version: Some("1.0.0".to_string()),
    status: ODPSStatus::Draft,
    input_ports: Some(vec![ODPSInputPort {
        name: "input".to_string(),
        contract_id: "contract-id".to_string(),
        // ... other fields
    }]),
    output_ports: Some(vec![ODPSOutputPort {
        name: "output".to_string(),
        contract_id: Some("contract-id".to_string()),
        // ... other fields
    }]),
    // ... other fields
};

let exporter = ODPSExporter;
let yaml = exporter.export(&product)?;
println!("{}", yaml);
```

---

## Domain Schema Usage

### Create a Domain

```rust
use data_modelling_sdk::models::domain::{Domain, System};
use data_modelling_sdk::models::enums::InfrastructureType;

// Create domain
let mut domain = Domain::new("customer-service".to_string());
domain.description = Some("Customer service domain".to_string());

// Add a system
let system = System::new(
    "kafka-cluster".to_string(),
    InfrastructureType::Kafka,
    domain.id,
);
domain.add_system(system);

// Export to YAML
let yaml = domain.to_yaml()?;
println!("{}", yaml);
```

### Import Domain from YAML

```rust
use data_modelling_sdk::models::domain::Domain;

let domain_yaml = r#"
id: 550e8400-e29b-41d4-a716-446655440000
name: customer-service
description: Customer service domain
systems:
  - id: 660e8400-e29b-41d4-a716-446655440001
    name: kafka-cluster
    infrastructure_type: Kafka
    domain_id: 550e8400-e29b-41d4-a716-446655440000
"#;

let domain = Domain::from_yaml(domain_yaml)?;
println!("Domain: {}", domain.name);
println!("Systems: {}", domain.systems.len());
```

### Domain Operations in DataModel

```rust
use data_modelling_sdk::models::{DataModel, domain::{Domain, System}};
use data_modelling_sdk::models::enums::InfrastructureType;

let mut model = DataModel::new(
    "MyModel".to_string(),
    "/path/to/git".to_string(),
    "control.yaml".to_string(),
);

// Create and add domain
let domain = Domain::new("customer-service".to_string());
model.add_domain(domain.clone());

// Add system to domain
let system = System::new(
    "kafka-cluster".to_string(),
    InfrastructureType::Kafka,
    domain.id,
);
model.add_system_to_domain(domain.id, system)?;
```

---

## Universal Converter

### Convert Any Format to ODCS

```rust
use data_modelling_sdk::convert::convert_to_odcs;

// Auto-detect format
let sql = "CREATE TABLE users (id INT, name VARCHAR(100));";
let odcs_yaml = convert_to_odcs(sql, None)?;

// Explicit format
let json_schema = r#"{"type": "object", "properties": {"id": {"type": "integer"}}}"#;
let odcs_yaml = convert_to_odcs(json_schema, Some("json_schema"))?;
```

### Supported Formats

- `sql` - SQL DDL statements
- `odcs` - ODCS v3.1.0 format
- `odcl` - ODCL v1.2.1 format
- `json_schema` - JSON Schema format
- `avro` - AVRO schema format
- `protobuf` - Protobuf .proto format
- `cads` - CADS v1.0 format (returns informative error)
- `odps` - ODPS format (returns informative error)
- `domain` - Domain schema format (returns informative error)

---

## WASM Usage

### CADS Import/Export

```javascript
import init, { importFromCads, exportToCads } from './pkg/data_modelling_sdk.js';

await init();

// Import CADS
const cadsYaml = `
apiVersion: v1.0
kind: AIModel
id: test-model
name: Test Model
version: 1.0.0
status: draft
`;

const assetJson = importFromCads(cadsYaml);
const asset = JSON.parse(assetJson);
console.log('CADS Asset:', asset.name);

// Export CADS
const exportedYaml = exportToCads(assetJson);
console.log('Exported YAML:', exportedYaml);
```

### ODPS Import/Export

```javascript
import init, { importFromOdps, exportToOdps } from './pkg/data_modelling_sdk.js';

await init();

// Import ODPS
const odpsYaml = `
apiVersion: v1.0.0
kind: DataProduct
id: test-product
name: Test Product
version: 1.0.0
status: draft
`;

const productJson = importFromOdps(odpsYaml);
const product = JSON.parse(productJson);
console.log('ODPS Product:', product.name);

// Export ODPS
const exportedYaml = exportToOdps(productJson);
console.log('Exported YAML:', exportedYaml);
```

### Domain Operations

```javascript
import init, { createDomain, importFromDomain, exportToDomain, addSystemToDomain } from './pkg/data_modelling_sdk.js';

await init();

// Create domain
const domainJson = createDomain("customer-service");
const domain = JSON.parse(domainJson);
console.log('Domain ID:', domain.id);

// Add system to domain
const system = {
    id: "660e8400-e29b-41d4-a716-446655440001",
    name: "kafka-cluster",
    infrastructure_type: "Kafka",
    domain_id: domain.id
};

const workspace = {
    tables: [],
    relationships: [],
    domains: [domain]
};

const updatedWorkspaceJson = addSystemToDomain(
    JSON.stringify(workspace),
    domain.id,
    JSON.stringify(system)
);
const updatedWorkspace = JSON.parse(updatedWorkspaceJson);
console.log('Systems:', updatedWorkspace.domains[0].systems.length);
```

### Tag Operations

```javascript
import init, { parseTag, serializeTag } from './pkg/data_modelling_sdk.js';

await init();

// Parse tags
const simpleTagJson = parseTag("finance");
const pairTagJson = parseTag("Environment:Dev");
const listTagJson = parseTag("Domains:[Finance, Sales]");

console.log('Simple tag:', JSON.parse(simpleTagJson));
console.log('Pair tag:', JSON.parse(pairTagJson));
console.log('List tag:', JSON.parse(listTagJson));

// Serialize tags
const simpleTagStr = serializeTag(simpleTagJson);
console.log('Serialized:', simpleTagStr); // "finance"
```

### Universal Converter

```javascript
import init, { convertToOdcs } from './pkg/data_modelling_sdk.js';

await init();

// Auto-detect format
const sql = "CREATE TABLE users (id INT);";
const odcsYaml = convertToOdcs(sql, null);
console.log('Converted to ODCS:', odcsYaml);

// Explicit format
const jsonSchema = '{"type": "object", "properties": {"id": {"type": "integer"}}}';
const odcsYaml2 = convertToOdcs(jsonSchema, "json_schema");
console.log('Converted to ODCS:', odcsYaml2);
```

---

## Migration: DataFlow to Domain

### Migrate DataFlow YAML to Domain Schema

```rust
use data_modelling_sdk::convert::migrate_dataflow::migrate_dataflow_to_domain;

let dataflow_yaml = r#"
nodes:
  - name: kafka-cluster
    type: Kafka
    metadata:
      owner: Data Engineering Team
      infrastructure_type: Kafka
relationships:
  - source: kafka-cluster
    target: postgres-db
    type: data-flow
"#;

let domain = migrate_dataflow_to_domain(dataflow_yaml, Some("customer-service"))?;

// DataFlow nodes become Systems
println!("Systems: {}", domain.systems.len());

// DataFlow relationships become SystemConnections
println!("Connections: {}", domain.system_connections.len());
```

### WASM Migration

```javascript
import init, { migrateDataflowToDomain } from './pkg/data_modelling_sdk.js';

await init();

const dataflowYaml = `
nodes:
  - name: kafka-cluster
    type: Kafka
`;

const domainJson = migrateDataflowToDomain(dataflowYaml, "customer-service");
const domain = JSON.parse(domainJson);
console.log('Migrated Domain:', domain.name);
console.log('Systems:', domain.systems.length);
```

---

## Best Practices

1. **Use ODCS for Data Contracts**: ODCS v3.1.0 is the primary format for tables/schemas
2. **Use Enhanced Tags**: Leverage Simple, Pair, and List tag formats for flexible metadata
3. **Preserve Metadata**: Always use import/export functions to preserve all fields
4. **Validate References**: When using ODPS, validate `contractId` references against known ODCS Tables
5. **Use Domain Schema**: Organize systems and nodes within business domains for better structure
6. **Universal Converter**: Use `convert_to_odcs()` for format conversion, but note that CADS/ODPS/Domain require additional context for full conversion

---

For more detailed information, see the [Schema Overview Guide](../../docs/SCHEMA_OVERVIEW.md).
