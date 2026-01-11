//! CLI binary entry point for odm (Open Data Modelling)

mod commands;
mod error;
mod output;
mod reference;

use clap::{Parser, Subcommand};
#[cfg(feature = "duckdb-backend")]
use commands::db::{
    DbExportArgs, DbInitArgs, DbStatusArgs, DbSyncArgs, handle_db_export, handle_db_init,
    handle_db_status, handle_db_sync,
};
use commands::export::{
    ExportArgs, ExportFormat, handle_export_avro, handle_export_branded_markdown,
    handle_export_json_schema, handle_export_markdown, handle_export_odcs, handle_export_odps,
    handle_export_pdf, handle_export_protobuf, handle_export_protobuf_descriptor,
};
#[cfg(feature = "odps-validation")]
use commands::import::handle_import_odps;
#[cfg(feature = "openapi")]
use commands::import::handle_import_openapi;
use commands::import::{
    ImportArgs, ImportFormat, InputSource, handle_import_avro, handle_import_json_schema,
    handle_import_odcl, handle_import_odcs, handle_import_protobuf, handle_import_sql,
};
#[cfg(all(feature = "inference", feature = "staging"))]
use commands::inference::{
    InferenceInferArgs, InferenceSchemasArgs, handle_inference_infer, handle_inference_schemas,
};
#[cfg(feature = "duckdb-backend")]
use commands::query::{QueryArgs, handle_query};
#[cfg(feature = "staging")]
use commands::staging::{
    StagingBatchesArgs, StagingIngestArgs, StagingInitArgs, StagingQueryArgs, StagingSampleArgs,
    StagingStatsArgs, handle_staging_batches, handle_staging_ingest, handle_staging_init,
    handle_staging_query, handle_staging_sample, handle_staging_stats,
};
use commands::validate::handle_validate;
#[cfg(feature = "staging")]
use data_modelling_core::staging::DedupStrategy;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "odm")]
#[command(about = "CLI tool for Open Data Modelling")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Import schemas from various formats
    Import {
        /// Format to import from
        #[arg(value_enum)]
        format: ImportFormatArg,
        /// Input source (file path, '-' for stdin, or SQL string). Optional when --jar is provided.
        #[arg(required_unless_present = "jar")]
        input: Option<String>,
        /// SQL dialect (required for SQL format)
        #[arg(short, long)]
        dialect: Option<String>,
        /// Override table UUID (only for single-table imports)
        #[arg(short, long)]
        uuid: Option<String>,
        /// Disable automatic external reference resolution
        #[arg(long)]
        no_resolve_references: bool,
        /// Skip schema validation before import
        #[arg(long)]
        no_validate: bool,
        /// Don't write .odcs.yaml file after import
        #[arg(long)]
        no_odcs: bool,
        /// Pretty-print output with detailed information
        #[arg(short, long)]
        pretty: bool,
        /// JAR file path (for Protobuf JAR imports). When provided, input is optional.
        #[arg(long)]
        jar: Option<PathBuf>,
        /// Filter by message type (for Protobuf JAR imports)
        #[arg(long)]
        message_type: Option<String>,
        /// Specify the root message for JAR imports. If not provided, auto-detects based on dependency analysis.
        #[arg(long)]
        root_message: Option<String>,
    },
    /// Export schemas to various formats
    Export {
        /// Format to export to
        #[arg(value_enum)]
        format: ExportFormatArg,
        /// Input file (.odcs.yaml, .madr.yaml, or .kb.yaml)
        input: PathBuf,
        /// Output file path
        output: PathBuf,
        /// Overwrite existing files without prompting
        #[arg(short, long)]
        force: bool,
        /// Custom path to protoc binary (for protobuf-descriptor format)
        #[arg(long)]
        protoc_path: Option<PathBuf>,
        /// Protobuf syntax version (proto2 or proto3, default: proto3)
        #[arg(long, default_value = "proto3")]
        protobuf_version: String,
        // Branding options for PDF and branded Markdown exports
        /// Logo URL for branding (PDF and branded-markdown formats)
        #[arg(long)]
        logo_url: Option<String>,
        /// Header text for branding (PDF and branded-markdown formats)
        #[arg(long)]
        header: Option<String>,
        /// Footer text for branding (PDF and branded-markdown formats)
        #[arg(long)]
        footer: Option<String>,
        /// Brand color in hex format, e.g., "#0066CC" (PDF and branded-markdown formats)
        #[arg(long)]
        brand_color: Option<String>,
        /// Company or organization name (PDF and branded-markdown formats)
        #[arg(long)]
        company_name: Option<String>,
        /// Include table of contents (branded-markdown format)
        #[arg(long)]
        include_toc: bool,
    },
    /// Validate a file against its schema
    Validate {
        /// Format to validate
        #[arg(value_enum)]
        format: ValidateFormatArg,
        /// Input file path or '-' for stdin
        #[arg(default_value = "-")]
        input: String,
    },

    /// Database management commands
    #[cfg(feature = "duckdb-backend")]
    Db {
        #[command(subcommand)]
        command: DbCommands,
    },

    /// Execute SQL queries against the workspace database
    #[cfg(feature = "duckdb-backend")]
    Query {
        /// SQL query to execute
        sql: String,
        /// Workspace path (default: current directory)
        #[arg(short, long, default_value = ".")]
        workspace: PathBuf,
        /// Output format (table, json, csv)
        #[arg(short, long, default_value = "table")]
        format: String,
    },

    /// Staging database for JSON data pipeline
    #[cfg(feature = "staging")]
    Staging {
        #[command(subcommand)]
        command: StagingCommands,
    },

    /// Schema inference from staged data
    #[cfg(all(feature = "inference", feature = "staging"))]
    Inference {
        #[command(subcommand)]
        command: InferenceCommands,
    },
}

#[cfg(feature = "staging")]
#[derive(Subcommand)]
enum StagingCommands {
    /// Initialize a new staging database
    Init {
        /// Path to the staging database file
        #[arg(default_value = "staging.duckdb")]
        database: PathBuf,
    },

    /// Ingest JSON/JSONL files into the staging database
    Ingest {
        /// Path to the staging database file
        #[arg(short, long, default_value = "staging.duckdb")]
        database: PathBuf,
        /// Source directory containing files to ingest
        source: PathBuf,
        /// File pattern to match (e.g., "*.json", "**/*.jsonl")
        #[arg(short, long, default_value = "*.json")]
        pattern: String,
        /// Partition key for organizing data
        #[arg(short = 'k', long)]
        partition: Option<String>,
        /// Deduplication strategy (none, by-path, by-content, both)
        #[arg(long, default_value = "by-path", value_parser = parse_dedup_strategy)]
        dedup: DedupStrategy,
        /// Batch size for database inserts
        #[arg(long, default_value = "1000")]
        batch_size: usize,
        /// Resume a previous batch
        #[arg(short, long)]
        resume: bool,
        /// Batch ID to resume (required with --resume)
        #[arg(long)]
        batch_id: Option<String>,
    },

    /// Show staging database statistics
    Stats {
        /// Path to the staging database file
        #[arg(short, long, default_value = "staging.duckdb")]
        database: PathBuf,
        /// Partition to filter by
        #[arg(short = 'k', long)]
        partition: Option<String>,
    },

    /// List processing batches
    Batches {
        /// Path to the staging database file
        #[arg(short, long, default_value = "staging.duckdb")]
        database: PathBuf,
        /// Maximum number of batches to show
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },

    /// Execute SQL query against staging database
    Query {
        /// Path to the staging database file
        #[arg(short, long, default_value = "staging.duckdb")]
        database: PathBuf,
        /// SQL query to execute
        sql: String,
        /// Output format (table, json)
        #[arg(short, long, default_value = "table")]
        format: String,
    },

    /// Get sample records from staging database
    Sample {
        /// Path to the staging database file
        #[arg(short, long, default_value = "staging.duckdb")]
        database: PathBuf,
        /// Number of samples to retrieve
        #[arg(short, long, default_value = "5")]
        limit: usize,
        /// Partition to sample from
        #[arg(short = 'k', long)]
        partition: Option<String>,
    },
}

#[cfg(feature = "staging")]
fn parse_dedup_strategy(s: &str) -> Result<DedupStrategy, String> {
    s.parse().map_err(|_| {
        format!(
            "Invalid dedup strategy: {}. Valid values: none, by-path, by-content, both",
            s
        )
    })
}

#[cfg(all(feature = "inference", feature = "staging"))]
#[derive(Subcommand)]
enum InferenceCommands {
    /// Infer schema from staged JSON data
    Infer {
        /// Path to the staging database file
        #[arg(short, long, default_value = "staging.duckdb")]
        database: PathBuf,
        /// Partition to infer schema from
        #[arg(short = 'k', long)]
        partition: Option<String>,
        /// Sample size for inference
        #[arg(short, long, default_value = "1000")]
        sample_size: usize,
        /// Minimum field frequency (0.0-1.0)
        #[arg(long, default_value = "0.01")]
        min_frequency: f64,
        /// Maximum depth for nested objects
        #[arg(long, default_value = "10")]
        max_depth: usize,
        /// Disable format detection
        #[arg(long)]
        no_formats: bool,
        /// Output format (json, yaml, json-schema)
        #[arg(short, long, default_value = "json")]
        format: String,
        /// Output file path (stdout if not provided)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Analyze and group schemas across partitions
    Schemas {
        /// Path to the staging database file
        #[arg(short, long, default_value = "staging.duckdb")]
        database: PathBuf,
        /// Similarity threshold for grouping (0.0-1.0)
        #[arg(short, long, default_value = "0.8")]
        threshold: f64,
        /// Output format (table, json)
        #[arg(short, long, default_value = "table")]
        format: String,
    },
}

#[cfg(feature = "duckdb-backend")]
#[derive(Subcommand)]
enum DbCommands {
    /// Initialize database for a workspace
    Init {
        /// Workspace path
        #[arg(default_value = ".")]
        workspace: PathBuf,
        /// Database backend (duckdb, postgres)
        #[arg(short, long, default_value = "duckdb")]
        backend: String,
        /// PostgreSQL connection string (required for postgres backend)
        #[arg(long)]
        connection_string: Option<String>,
    },

    /// Sync YAML files to database
    Sync {
        /// Workspace path
        #[arg(default_value = ".")]
        workspace: PathBuf,
        /// Force full resync (ignore change detection)
        #[arg(short, long)]
        force: bool,
    },

    /// Show sync status
    Status {
        /// Workspace path
        #[arg(default_value = ".")]
        workspace: PathBuf,
    },

    /// Export database back to YAML files
    Export {
        /// Workspace path
        #[arg(default_value = ".")]
        workspace: PathBuf,
        /// Output directory (default: same as workspace)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum ImportFormatArg {
    Sql,
    Avro,
    JsonSchema,
    Protobuf,
    Openapi,
    Odcs,
    Odcl,
    Odps,
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum ExportFormatArg {
    Odcs,
    Avro,
    JsonSchema,
    Protobuf,
    ProtobufDescriptor,
    Odps,
    /// PDF export for decision records and knowledge articles
    Pdf,
    /// Markdown export for decision records and knowledge articles
    Markdown,
    /// Branded Markdown export with logo, header, footer
    BrandedMarkdown,
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum ValidateFormatArg {
    /// ODCS v3.1.0 (Open Data Contract Standard)
    Odcs,
    /// Legacy ODCL format
    Odcl,
    /// ODPS (Open Data Product Standard)
    Odps,
    /// CADS (Compute Asset Description Specification)
    Cads,
    /// OpenAPI 3.1.1 specification
    Openapi,
    /// Protocol Buffers
    Protobuf,
    /// Apache Avro
    Avro,
    /// JSON Schema
    JsonSchema,
    /// SQL (CREATE TABLE statements)
    Sql,
    /// MADR Decision Record (.madr.yaml)
    Decision,
    /// Knowledge Base Article (.kb.yaml)
    Knowledge,
    /// Decisions Index (decisions.yaml)
    DecisionsIndex,
    /// Knowledge Index (knowledge.yaml)
    KnowledgeIndex,
}

fn convert_import_format(format: ImportFormatArg) -> ImportFormat {
    match format {
        ImportFormatArg::Sql => ImportFormat::Sql,
        ImportFormatArg::Avro => ImportFormat::Avro,
        ImportFormatArg::JsonSchema => ImportFormat::JsonSchema,
        ImportFormatArg::Protobuf => ImportFormat::Protobuf,
        ImportFormatArg::Openapi => ImportFormat::OpenApi,
        ImportFormatArg::Odcs => ImportFormat::Odcs,
        ImportFormatArg::Odcl => ImportFormat::Odcl,
        ImportFormatArg::Odps => ImportFormat::Odps,
    }
}

fn convert_export_format(format: ExportFormatArg) -> ExportFormat {
    match format {
        ExportFormatArg::Odcs => ExportFormat::Odcs,
        ExportFormatArg::Avro => ExportFormat::Avro,
        ExportFormatArg::JsonSchema => ExportFormat::JsonSchema,
        ExportFormatArg::Protobuf => ExportFormat::Protobuf,
        ExportFormatArg::ProtobufDescriptor => ExportFormat::ProtobufDescriptor,
        ExportFormatArg::Odps => ExportFormat::Odps,
        ExportFormatArg::Pdf => ExportFormat::Pdf,
        ExportFormatArg::Markdown => ExportFormat::BrandedMarkdown, // Use same handler, no branding
        ExportFormatArg::BrandedMarkdown => ExportFormat::BrandedMarkdown,
    }
}

fn parse_input_source(input: &str, format: &ImportFormat) -> InputSource {
    if input == "-" {
        InputSource::Stdin
    } else if matches!(format, ImportFormat::Sql) && !std::path::Path::new(input).exists() {
        // For SQL, if the input doesn't exist as a file, treat it as a SQL string
        InputSource::String(input.to_string())
    } else {
        InputSource::File(PathBuf::from(input))
    }
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Import {
            format,
            input,
            dialect,
            uuid,
            no_resolve_references,
            no_validate,
            no_odcs,
            pretty,
            jar,
            message_type,
            root_message,
        } => {
            let import_format = convert_import_format(format);

            // When --jar is provided, input is optional. Use a placeholder if not provided.
            let input_str = input.unwrap_or_else(|| "-".to_string());
            let input_source = parse_input_source(&input_str, &import_format);

            let args = ImportArgs {
                format: import_format,
                input: input_source,
                dialect,
                uuid_override: uuid,
                resolve_references: !no_resolve_references,
                validate: !no_validate,
                pretty,
                jar_path: jar,
                message_type,
                no_odcs,
                root_message,
            };

            match args.format {
                ImportFormat::Sql => handle_import_sql(&args),
                ImportFormat::Avro => handle_import_avro(&args),
                ImportFormat::JsonSchema => handle_import_json_schema(&args),
                ImportFormat::Protobuf => handle_import_protobuf(&args),
                ImportFormat::OpenApi => {
                    #[cfg(feature = "openapi")]
                    {
                        handle_import_openapi(&args)
                    }
                    #[cfg(not(feature = "openapi"))]
                    {
                        Err(error::CliError::InvalidArgument(
                            "OpenAPI support not enabled. Enable 'openapi' feature.".to_string(),
                        ))
                    }
                }
                ImportFormat::Odcs => handle_import_odcs(&args),
                ImportFormat::Odcl => handle_import_odcl(&args),
                ImportFormat::Odps => {
                    #[cfg(feature = "odps-validation")]
                    {
                        handle_import_odps(&args)
                    }
                    #[cfg(not(feature = "odps-validation"))]
                    {
                        Err(error::CliError::InvalidArgument(
                            "ODPS support not enabled. Enable 'odps-validation' feature."
                                .to_string(),
                        ))
                    }
                }
            }
        }
        Commands::Export {
            format,
            input,
            output,
            force,
            protoc_path,
            protobuf_version,
            logo_url,
            header,
            footer,
            brand_color,
            company_name,
            include_toc,
        } => {
            let export_format = convert_export_format(format.clone());

            let args = ExportArgs {
                format: export_format,
                input,
                output,
                force,
                protoc_path,
                protobuf_version: Some(protobuf_version),
                logo_url,
                header,
                footer,
                brand_color,
                company_name,
                include_toc,
            };

            match args.format {
                ExportFormat::Odcs => handle_export_odcs(&args),
                ExportFormat::Avro => handle_export_avro(&args),
                ExportFormat::JsonSchema => handle_export_json_schema(&args),
                ExportFormat::Protobuf => handle_export_protobuf(&args),
                ExportFormat::ProtobufDescriptor => handle_export_protobuf_descriptor(&args),
                ExportFormat::Odps => handle_export_odps(&args),
                ExportFormat::Pdf => handle_export_pdf(&args),
                ExportFormat::BrandedMarkdown => {
                    // If no branding options provided, use standard markdown export
                    if args.logo_url.is_none()
                        && args.header.is_none()
                        && args.footer.is_none()
                        && args.company_name.is_none()
                        && !args.include_toc
                        && matches!(format, ExportFormatArg::Markdown)
                    {
                        handle_export_markdown(&args)
                    } else {
                        handle_export_branded_markdown(&args)
                    }
                }
            }
        }
        Commands::Validate { format, input } => {
            let validate_format = match format {
                ValidateFormatArg::Odcs => "odcs",
                ValidateFormatArg::Odcl => "odcl",
                ValidateFormatArg::Odps => "odps",
                ValidateFormatArg::Cads => "cads",
                ValidateFormatArg::Openapi => "openapi",
                ValidateFormatArg::Protobuf => "protobuf",
                ValidateFormatArg::Avro => "avro",
                ValidateFormatArg::JsonSchema => "json-schema",
                ValidateFormatArg::Sql => "sql",
                ValidateFormatArg::Decision => "decision",
                ValidateFormatArg::Knowledge => "knowledge",
                ValidateFormatArg::DecisionsIndex => "decisions-index",
                ValidateFormatArg::KnowledgeIndex => "knowledge-index",
            };
            handle_validate(validate_format, &input)
        }

        #[cfg(feature = "duckdb-backend")]
        Commands::Db { command } => match command {
            DbCommands::Init {
                workspace,
                backend,
                connection_string,
            } => {
                let args = DbInitArgs {
                    workspace,
                    backend,
                    connection_string,
                };
                handle_db_init(&args)
            }
            DbCommands::Sync { workspace, force } => {
                let args = DbSyncArgs { workspace, force };
                handle_db_sync(&args)
            }
            DbCommands::Status { workspace } => {
                let args = DbStatusArgs { workspace };
                handle_db_status(&args)
            }
            DbCommands::Export { workspace, output } => {
                let args = DbExportArgs { workspace, output };
                handle_db_export(&args)
            }
        },

        #[cfg(feature = "duckdb-backend")]
        Commands::Query {
            sql,
            workspace,
            format,
        } => {
            let args = QueryArgs {
                sql,
                workspace,
                format,
            };
            handle_query(&args)
        }

        #[cfg(feature = "staging")]
        Commands::Staging { command } => match command {
            StagingCommands::Init { database } => {
                let args = StagingInitArgs { database };
                handle_staging_init(&args)
            }
            StagingCommands::Ingest {
                database,
                source,
                pattern,
                partition,
                dedup,
                batch_size,
                resume,
                batch_id,
            } => {
                let args = StagingIngestArgs {
                    database,
                    source,
                    pattern,
                    partition,
                    dedup,
                    batch_size,
                    resume,
                    batch_id,
                };
                handle_staging_ingest(&args)
            }
            StagingCommands::Stats {
                database,
                partition,
            } => {
                let args = StagingStatsArgs {
                    database,
                    partition,
                };
                handle_staging_stats(&args)
            }
            StagingCommands::Batches { database, limit } => {
                let args = StagingBatchesArgs { database, limit };
                handle_staging_batches(&args)
            }
            StagingCommands::Query {
                database,
                sql,
                format,
            } => {
                let args = StagingQueryArgs {
                    database,
                    sql,
                    format,
                };
                handle_staging_query(&args)
            }
            StagingCommands::Sample {
                database,
                limit,
                partition,
            } => {
                let args = StagingSampleArgs {
                    database,
                    limit,
                    partition,
                };
                handle_staging_sample(&args)
            }
        },

        #[cfg(all(feature = "inference", feature = "staging"))]
        Commands::Inference { command } => match command {
            InferenceCommands::Infer {
                database,
                partition,
                sample_size,
                min_frequency,
                max_depth,
                no_formats,
                format,
                output,
            } => {
                let args = InferenceInferArgs {
                    database,
                    partition,
                    sample_size,
                    min_frequency,
                    max_depth,
                    detect_formats: !no_formats,
                    format,
                    output,
                };
                handle_inference_infer(&args)
            }
            InferenceCommands::Schemas {
                database,
                threshold,
                format,
            } => {
                let args = InferenceSchemasArgs {
                    database,
                    threshold,
                    format,
                };
                handle_inference_schemas(&args)
            }
        },
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
