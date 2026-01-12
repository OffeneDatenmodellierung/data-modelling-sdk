//! Enums for data modeling
//!
//! # Serde Casing Conventions
//!
//! The enums in this module use different serde `rename_all` strategies based on their
//! semantic meaning and external schema requirements:
//!
//! - `SCREAMING_SNAKE_CASE`: Technical/database constants (DatabaseType, SCDPattern)
//! - `lowercase`: Simple layer/level keywords (MedallionLayer, ModelingLevel)
//! - `PascalCase`: Type names and relationships (Cardinality, RelationshipType, InfrastructureType)
//! - No rename: Values that match Rust conventions (DataVaultClassification)
//!
//! These conventions ensure compatibility with ODCS, CADS, and other external schemas.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DatabaseType {
    DatabricksDelta,
    DatabricksIceberg,
    AwsGlue,
    DatabricksLakebase,
    Postgres,
    Mysql,
    SqlServer,
    Dynamodb,
    Cassandra,
    Kafka,
    Pulsar,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MedallionLayer {
    Bronze,
    Silver,
    Gold,
    Operational,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SCDPattern {
    Type1,
    Type2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataVaultClassification {
    Hub,
    Link,
    Satellite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelingLevel {
    Conceptual,
    Logical,
    Physical,
}

/// Legacy cardinality enum (for backward compatibility)
/// Consider using EndpointCardinality for more precise crow's feet notation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Cardinality {
    OneToOne,
    OneToMany,
    ManyToOne,
    ManyToMany,
}

/// Crow's feet notation endpoint cardinality
///
/// Defines the cardinality at one end of a relationship using standard
/// crow's feet notation symbols:
/// - ZeroOrOne: Optional single (0..1) - circle with single line
/// - ExactlyOne: Required single (1..1) - single line with perpendicular bar
/// - ZeroOrMany: Optional multiple (0..*) - circle with crow's foot
/// - OneOrMany: Required multiple (1..*) - perpendicular bar with crow's foot
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EndpointCardinality {
    /// Zero or one (optional single) - 0..1
    ZeroOrOne,
    /// Exactly one (required single) - 1..1
    ExactlyOne,
    /// Zero or many (optional multiple) - 0..*
    ZeroOrMany,
    /// One or many (required multiple) - 1..*
    OneOrMany,
}

/// Flow direction for data flow relationships
///
/// Defines the direction of data movement between nodes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FlowDirection {
    /// Data flows from source to target only
    SourceToTarget,
    /// Data flows from target to source only
    TargetToSource,
    /// Data flows in both directions
    Bidirectional,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RelationshipType {
    DataFlow,
    Dependency,
    ForeignKey,
    /// ETL transformation (maps to "etl" in JSON)
    #[serde(rename = "etl")]
    EtlTransformation,
}

/// Infrastructure type for Data Flow nodes and relationships
///
/// Comprehensive enumeration covering major cloud databases, container platforms,
/// data warehouses, message queues, BI/analytics tools, and storage systems
/// from AWS, Azure, and GCP.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
    // GCP Services
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
    // Message Queues
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
