# Quickstart: Enhanced Data Flow Node and Relationship Metadata

**Date**: 2026-01-27
**Feature**: Enhanced Data Flow Node and Relationship Metadata
**Phase**: 1 - Design & Contracts

## Overview

This quickstart guide demonstrates how to use the enhanced metadata features for Data Flow nodes (Tables) and relationships including owner, SLA, contact details, infrastructure type, and notes. Note: This is for Data Flow elements, NOT for ODCS Data Contracts. ODCS format is only for Data Models (tables). We use a lightweight Data Flow format separate from ODCS.

## Basic Usage

### Creating a Data Flow Node (Table) with Metadata

```rust
use data_modelling_sdk::models::{Table, Column, InfrastructureType, ContactDetails, SlaProperty};
use serde_json::json;

// Create a Data Flow node (table)
let mut table = Table::new(
    "user_events".to_string(),
    vec![
        Column::new("id".to_string(), "UUID".to_string()),
        Column::new("event_type".to_string(), "VARCHAR(50)".to_string()),
        Column::new("timestamp".to_string(), "TIMESTAMP".to_string()),
    ],
);

// Set owner
table.owner = Some("Data Engineering Team".to_string());

// Set infrastructure type
table.infrastructure_type = Some(InfrastructureType::Kafka);

// Set contact details
table.contact_details = Some(ContactDetails {
    email: Some("data-team@example.com".to_string()),
    phone: Some("+1-555-0123".to_string()),
    name: Some("Data Engineering Team".to_string()),
    role: Some("Data Owner".to_string()),
    other: None,
});

// Set SLA
table.sla = Some(vec![
    SlaProperty {
        property: "latency".to_string(),
        value: json!(4),
        unit: "hours".to_string(),
        description: Some("Data must be available within 4 hours of generation".to_string()),
        element: None,
        driver: Some("operational".to_string()),
        scheduler: Some("cron".to_string()),
        schedule: Some("0 */2 * * *".to_string()), // Check every 2 hours
    },
    SlaProperty {
        property: "availability".to_string(),
        value: json!(99.9),
        unit: "percent".to_string(),
        description: Some("99.9% uptime SLA".to_string()),
        element: None,
        driver: Some("regulatory".to_string()),
        scheduler: None,
        schedule: None,
    },
]);

// Set notes
table.notes = Some("This table contains user interaction events from the web application.".to_string());
```

### Creating a Data Flow Relationship with Metadata

```rust
use data_modelling_sdk::models::{Relationship, InfrastructureType, ContactDetails, SlaProperty};
use serde_json::json;
use uuid::Uuid;

// Create a Data Flow relationship
let mut relationship = Relationship::new(
    source_table_id,
    target_table_id,
);

// Set owner
relationship.owner = Some("Data Engineering Team".to_string());

// Set infrastructure type
relationship.infrastructure_type = Some(InfrastructureType::Kafka);

// Set contact details
relationship.contact_details = Some(ContactDetails {
    email: Some("data-team@example.com".to_string()),
    name: Some("Data Engineering Team".to_string()),
    role: Some("Data Owner".to_string()),
    phone: None,
    other: None,
});

// Set SLA
relationship.sla = Some(vec![
    SlaProperty {
        property: "latency".to_string(),
        value: json!(2),
        unit: "hours".to_string(),
        description: Some("Data flow must complete within 2 hours".to_string()),
        element: None,
        driver: Some("operational".to_string()),
        scheduler: None,
        schedule: None,
    },
]);

// Set notes
relationship.notes = Some("ETL pipeline from source to target".to_string());
```

### Reading Metadata

```rust
// Check if metadata exists
if let Some(owner) = &table.owner {
    println!("Owner: {}", owner);
}

// Access infrastructure type
if let Some(infra_type) = &table.infrastructure_type {
    match infra_type {
        InfrastructureType::Kafka => println!("Using Kafka for streaming"),
        InfrastructureType::PostgreSQL => println!("Using PostgreSQL database"),
        _ => println!("Infrastructure: {:?}", infra_type),
    }
}

// Access contact details
if let Some(contact) = &table.contact_details {
    if let Some(email) = &contact.email {
        println!("Contact email: {}", email);
    }
    if let Some(name) = &contact.name {
        println!("Contact name: {}", name);
    }
}

// Access SLA properties
if let Some(sla_properties) = &table.sla {
    for sla in sla_properties {
        println!("SLA: {} = {} {}", sla.property, sla.value, sla.unit);
    }
}

// Access notes
if let Some(notes) = &table.notes {
    println!("Notes: {}", notes);
}

// Access relationship metadata
if let Some(relationship) = &some_relationship {
    if let Some(owner) = &relationship.owner {
        println!("Relationship owner: {}", owner);
    }
    if let Some(infra_type) = &relationship.infrastructure_type {
        println!("Relationship infrastructure: {:?}", infra_type);
    }
}
```

## Filtering Tables by Metadata

### Filter Data Flow Nodes by Owner

```rust
use data_modelling_sdk::models::DataModel;

let model = DataModel::new(
    "MyModel".to_string(),
    "/path/to/git".to_string(),
    "control.yaml".to_string(),
);

// Add tables (Data Flow nodes) to model...
// model.tables.push(table1);
// model.tables.push(table2);

// Filter Data Flow nodes by owner
let owned_nodes = model.filter_nodes_by_owner("Data Engineering Team");
println!("Found {} Data Flow nodes owned by Data Engineering Team", owned_nodes.len());
```

### Filter Data Flow Relationships by Owner

```rust
// Filter Data Flow relationships by owner
let owned_relationships = model.filter_relationships_by_owner("Data Engineering Team");
println!("Found {} Data Flow relationships owned by Data Engineering Team", owned_relationships.len());
```

### Filter by Infrastructure Type

```rust
// Filter Data Flow nodes by infrastructure type
let kafka_nodes = model.filter_nodes_by_infrastructure_type(InfrastructureType::Kafka);
println!("Found {} Kafka nodes", kafka_nodes.len());

let postgres_nodes = model.filter_nodes_by_infrastructure_type(InfrastructureType::PostgreSQL);
println!("Found {} PostgreSQL nodes", postgres_nodes.len());

// Filter Data Flow relationships by infrastructure type
let kafka_relationships = model.filter_relationships_by_infrastructure_type(InfrastructureType::Kafka);
println!("Found {} Kafka relationships", kafka_relationships.len());
```

### Filter by Tags

```rust
// Filter Data Flow nodes and relationships by tag
let (tagged_nodes, tagged_relationships) = model.filter_by_tags("production");
println!("Found {} nodes and {} relationships tagged 'production'", tagged_nodes.len(), tagged_relationships.len());
```

## Lightweight Data Flow Format Import/Export

### Importing Data Flow Format with Metadata

```rust
use data_modelling_sdk::import::DataFlowImporter;

let yaml_content = r#"
nodes:
  - id: 123e4567-e89b-12d3-a456-426614174000
    name: user_events
    type: table
    metadata:
      owner: "Data Engineering Team"
      infrastructure_type: "Kafka"
      notes: "User interaction events"
      sla:
        - property: latency
          value: 4
          unit: hours
          description: "Data must be available within 4 hours"
    columns:
      - name: id
        type: UUID
      - name: event_type
        type: VARCHAR(50)
"#;

let importer = DataFlowImporter::new();
let result = importer.import(yaml_content)?;

// Metadata is automatically extracted
if let Some(table) = result.nodes.first() {
    assert_eq!(table.owner, Some("Data Engineering Team".to_string()));
    assert_eq!(table.infrastructure_type, Some(InfrastructureType::Kafka));
    assert_eq!(table.notes, Some("User interaction events".to_string()));
    assert!(table.sla.is_some());
}
```

### Exporting to Data Flow Format with Metadata

```rust
use data_modelling_sdk::export::DataFlowExporter;

// Create table with metadata (as shown above)
let table = create_table_with_metadata();

// Export to lightweight Data Flow format
let exporter = DataFlowExporter::new();
let yaml_output = exporter.export_node(&table)?;

// Metadata is included in the output
println!("{}", yaml_output);
// Output includes:
// - metadata.owner
// - metadata.infrastructure_type
// - metadata.notes
// - metadata.sla array with SLA properties
// - metadata.contact_details
```

## Common Patterns

### Setting Multiple SLA Properties

```rust
table.sla = Some(vec![
    SlaProperty {
        property: "latency".to_string(),
        value: json!(4),
        unit: "hours".to_string(),
        description: Some("Data latency SLA".to_string()),
        driver: Some("operational".to_string()),
        element: None,
        scheduler: None,
        schedule: None,
    },
    SlaProperty {
        property: "availability".to_string(),
        value: json!(99.9),
        unit: "percent".to_string(),
        description: Some("Uptime SLA".to_string()),
        driver: Some("regulatory".to_string()),
        element: None,
        scheduler: None,
        schedule: None,
    },
]);
```

### Working with Infrastructure Types

```rust
// Check infrastructure type
match table.infrastructure_type {
    Some(InfrastructureType::Kafka) => {
        println!("Streaming data source");
    },
    Some(InfrastructureType::PostgreSQL) | Some(InfrastructureType::MySQL) => {
        println!("Relational database");
    },
    Some(InfrastructureType::S3) | Some(InfrastructureType::AzureBlobStorage) => {
        println!("Object storage");
    },
    Some(InfrastructureType::PowerBI) | Some(InfrastructureType::Tableau) => {
        println!("BI/Analytics tool");
    },
    Some(_) => {
        println!("Other infrastructure type");
    },
    None => {
        println!("Infrastructure type not specified");
    },
}
```

### Updating Metadata

```rust
// Update owner
table.owner = Some("New Team Name".to_string());

// Update contact details (replace entire struct)
table.contact_details = Some(ContactDetails {
    email: Some("new-email@example.com".to_string()),
    phone: None,
    name: Some("New Contact".to_string()),
    role: Some("Data Steward".to_string()),
    other: None,
});

// Add to SLA (append to existing)
if let Some(ref mut sla_properties) = table.sla {
    sla_properties.push(SlaProperty {
        property: "throughput".to_string(),
        value: json!(1000),
        unit: "records_per_second".to_string(),
        description: Some("Minimum throughput requirement".to_string()),
        element: None,
        driver: Some("operational".to_string()),
        scheduler: None,
        schedule: None,
    });
} else {
    // Create new SLA array
    table.sla = Some(vec![/* ... */]);
}

// Clear metadata
table.owner = None;
table.contact_details = None;
table.sla = None;
```

## Error Handling

### Invalid Infrastructure Type

```rust
use data_modelling_sdk::models::InfrastructureType;

// This will fail at compile time if using enum directly
let infra_type = InfrastructureType::Kafka; // ✅ Valid

// When parsing from string (e.g., from Data Flow format import)
match InfrastructureType::from_str("InvalidType") {
    Ok(infra_type) => {
        table.infrastructure_type = Some(infra_type);
    },
    Err(e) => {
        eprintln!("Invalid infrastructure type: {}", e);
        // Handle error - maybe store in odcl_metadata as fallback
    },
}
```

### Missing Metadata

```rust
// Always check for None before accessing
if let Some(owner) = &table.owner {
    // Use owner
} else {
    // Handle missing owner
    println!("Owner not specified");
}

// Or use unwrap_or for defaults
let owner = table.owner.as_ref().unwrap_or(&"Unknown".to_string());
```

## Best Practices

1. **Always use Option types**: Check for None before accessing metadata fields
2. **Validate infrastructure types**: Use the enum directly or validate strings before conversion
3. **Preserve Data Flow format compatibility**: When importing, metadata is preserved in dedicated fields
4. **Use structured SLA**: Follow ODCS-inspired servicelevels format (lightweight, for Data Flow)
5. **Remember scope**: This is for Data Flow nodes/relationships, NOT for ODCS Data Contracts
5. **Keep notes concise**: While 10,000 characters are supported, concise notes are more maintainable
6. **Update timestamps**: Consider updating `updated_at` when modifying metadata (if your use case requires it)

## Next Steps

- See [data-model.md](./data-model.md) for detailed data structure definitions
- See [contracts/table-api.md](./contracts/table-api.md) for complete API reference
- See [spec.md](./spec.md) for feature requirements and success criteria
