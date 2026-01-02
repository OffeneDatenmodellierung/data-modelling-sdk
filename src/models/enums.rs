//! Enums for data modeling

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum Cardinality {
    OneToOne,
    OneToMany,
    ManyToOne,
    ManyToMany,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum RelationshipType {
    DataFlow,
    Dependency,
    ForeignKey,
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
