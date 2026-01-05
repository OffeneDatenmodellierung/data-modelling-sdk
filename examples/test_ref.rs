use data_modelling_sdk::import::odcs::ODCSImporter;

fn main() {
    let yaml = r#"
apiVersion: v3.1.0
kind: DataContract
id: test-contract-id
version: 1.0.0
schema:
  - id: test_schema
    name: test_table
    properties:
      - id: col1_prop
        name: complete_column
        logicalType: string
        physicalType: varchar(100)
        required: true
        description: This column has all three field types
        $ref: '#/definitions/order_id'
definitions:
  order_id:
    logicalType: string
"#;

    let mut importer = ODCSImporter::new();
    let result = importer.import(yaml).unwrap();

    println!("Tables: {}", result.tables.len());
    for table in &result.tables {
        println!("Table: {:?}", table.name);
        for col in &table.columns {
            println!(
                "  Column: {} - relationships: {:?}",
                col.name, col.relationships
            );
        }
    }
}
