//! Benchmarks for import operations
//!
//! Run with: cargo bench -p data-modelling-core

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use data_modelling_core::import::{ODCSImporter, SQLImporter};

/// Sample ODCS contract for benchmarking
fn sample_odcs_contract() -> &'static str {
    r#"
kind: DataContract
apiVersion: v3.0.0
id: "550e8400-e29b-41d4-a716-446655440000"
uuid: "550e8400-e29b-41d4-a716-446655440000"
version: 1.0.0
info:
  title: Customer Orders
  description: Customer order information
  owner: data-team
  dataProduct: sales-analytics
schema:
  - name: orders
    description: Customer orders table
    type: table
    physicalName: customer_orders
    columns:
      - name: order_id
        dataType: bigint
        description: Unique order identifier
        isPrimaryKey: true
        isNullable: false
      - name: customer_id
        dataType: bigint
        description: Customer reference
        isNullable: false
      - name: order_date
        dataType: timestamp
        description: Order placement date
        isNullable: false
      - name: total_amount
        dataType: decimal(18,2)
        description: Order total
        isNullable: false
      - name: status
        dataType: varchar(50)
        description: Order status
        isNullable: false
      - name: shipping_address
        dataType: text
        description: Delivery address
        isNullable: true
      - name: notes
        dataType: text
        description: Order notes
        isNullable: true
      - name: created_at
        dataType: timestamp
        description: Record creation time
        isNullable: false
      - name: updated_at
        dataType: timestamp
        description: Record update time
        isNullable: true
"#
}

/// Sample SQL DDL for benchmarking
fn sample_sql_ddl() -> &'static str {
    r#"
CREATE TABLE customer_orders (
    order_id BIGINT PRIMARY KEY NOT NULL,
    customer_id BIGINT NOT NULL,
    order_date TIMESTAMP NOT NULL,
    total_amount DECIMAL(18,2) NOT NULL,
    status VARCHAR(50) NOT NULL,
    shipping_address TEXT,
    notes TEXT,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP
);

CREATE TABLE customers (
    customer_id BIGINT PRIMARY KEY NOT NULL,
    email VARCHAR(255) NOT NULL UNIQUE,
    name VARCHAR(255) NOT NULL,
    phone VARCHAR(50),
    created_at TIMESTAMP NOT NULL
);

CREATE TABLE order_items (
    item_id BIGINT PRIMARY KEY NOT NULL,
    order_id BIGINT NOT NULL REFERENCES customer_orders(order_id),
    product_id BIGINT NOT NULL,
    quantity INTEGER NOT NULL,
    unit_price DECIMAL(18,2) NOT NULL,
    discount DECIMAL(5,2) DEFAULT 0
);
"#
}

/// Generate ODCS with varying column counts
fn generate_odcs_with_columns(column_count: usize) -> String {
    let mut columns = String::new();
    for i in 0..column_count {
        columns.push_str(&format!(
            r#"      - name: column_{i}
        dataType: varchar(255)
        description: Column {i} description
        isNullable: true
"#
        ));
    }

    format!(
        r#"
kind: DataContract
apiVersion: v3.0.0
id: "550e8400-e29b-41d4-a716-446655440000"
uuid: "550e8400-e29b-41d4-a716-446655440000"
version: 1.0.0
info:
  title: Wide Table
  description: Table with many columns
  owner: data-team
schema:
  - name: wide_table
    description: Wide table for benchmarking
    type: table
    columns:
{columns}"#
    )
}

/// Generate SQL DDL with varying column counts
fn generate_sql_with_columns(column_count: usize) -> String {
    let columns: Vec<String> = (0..column_count)
        .map(|i| format!("    column_{} VARCHAR(255)", i))
        .collect();

    format!(
        "CREATE TABLE wide_table (\n    id BIGINT PRIMARY KEY,\n{}\n);",
        columns.join(",\n")
    )
}

/// Benchmark ODCS parsing
fn bench_odcs_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("odcs_parsing");

    let contract = sample_odcs_contract();
    group.throughput(Throughput::Bytes(contract.len() as u64));

    group.bench_function("parse_single_table", |b| {
        b.iter(|| {
            let mut importer = ODCSImporter::new();
            black_box(importer.parse_table(contract))
        });
    });

    // Benchmark with varying column counts
    for column_count in [10, 50, 100, 200].iter() {
        let contract = generate_odcs_with_columns(*column_count);
        group.throughput(Throughput::Elements(*column_count as u64));

        group.bench_with_input(
            BenchmarkId::new("parse_columns", column_count),
            &contract,
            |b, contract| {
                b.iter(|| {
                    let mut importer = ODCSImporter::new();
                    black_box(importer.parse_table(contract))
                });
            },
        );
    }

    group.finish();
}

/// Benchmark SQL parsing
fn bench_sql_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("sql_parsing");

    let ddl = sample_sql_ddl();
    group.throughput(Throughput::Bytes(ddl.len() as u64));

    group.bench_function("parse_multi_table", |b| {
        b.iter(|| {
            let importer = SQLImporter::new("generic");
            black_box(importer.parse(ddl))
        });
    });

    // Benchmark with varying column counts
    for column_count in [10, 50, 100, 200].iter() {
        let ddl = generate_sql_with_columns(*column_count);
        group.throughput(Throughput::Elements(*column_count as u64));

        group.bench_with_input(
            BenchmarkId::new("parse_columns", column_count),
            &ddl,
            |b, ddl| {
                b.iter(|| {
                    let importer = SQLImporter::new("generic");
                    black_box(importer.parse(ddl))
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_odcs_parsing, bench_sql_parsing);
criterion_main!(benches);
