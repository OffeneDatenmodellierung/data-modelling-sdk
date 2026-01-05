//! CLI binary entry point for data-modelling-cli

#[cfg(feature = "cli")]
use clap::{Parser, Subcommand};
#[cfg(feature = "cli")]
use data_modelling_sdk::cli::commands::export::{
    ExportArgs, ExportFormat, handle_export_avro, handle_export_json_schema, handle_export_odcs,
    handle_export_odps, handle_export_protobuf, handle_export_protobuf_descriptor,
};
#[cfg(all(feature = "cli", feature = "openapi"))]
use data_modelling_sdk::cli::commands::import::handle_import_openapi;
#[cfg(feature = "cli")]
use data_modelling_sdk::cli::commands::import::{
    ImportArgs, ImportFormat, InputSource, handle_import_avro, handle_import_json_schema,
    handle_import_odcl, handle_import_odcs, handle_import_odps, handle_import_protobuf,
    handle_import_sql,
};
#[cfg(feature = "cli")]
use std::path::PathBuf;

#[cfg(feature = "cli")]
#[derive(Parser)]
#[command(name = "data-modelling-cli")]
#[command(about = "CLI wrapper for Data Modelling SDK")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[cfg(feature = "cli")]
#[derive(Subcommand)]
enum Commands {
    /// Import schemas from various formats
    Import {
        /// Format to import from
        #[arg(value_enum)]
        format: ImportFormatArg,
        /// Input source (file path, '-' for stdin, or SQL string)
        input: String,
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
        /// JAR file path (for Protobuf JAR imports)
        #[arg(long)]
        jar: Option<PathBuf>,
        /// Filter by message type (for Protobuf JAR imports)
        #[arg(long)]
        message_type: Option<String>,
    },
    /// Export schemas to various formats
    Export {
        /// Format to export to
        #[arg(value_enum)]
        format: ExportFormatArg,
        /// Input ODCS YAML file (.odcs.yaml)
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
    },
}

#[cfg(feature = "cli")]
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

#[cfg(feature = "cli")]
#[derive(clap::ValueEnum, Clone, Debug)]
enum ExportFormatArg {
    Odcs,
    Avro,
    JsonSchema,
    Protobuf,
    ProtobufDescriptor,
    Odps,
}

#[cfg(feature = "cli")]
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

#[cfg(feature = "cli")]
fn convert_export_format(format: ExportFormatArg) -> ExportFormat {
    match format {
        ExportFormatArg::Odcs => ExportFormat::Odcs,
        ExportFormatArg::Avro => ExportFormat::Avro,
        ExportFormatArg::JsonSchema => ExportFormat::JsonSchema,
        ExportFormatArg::Protobuf => ExportFormat::Protobuf,
        ExportFormatArg::ProtobufDescriptor => ExportFormat::ProtobufDescriptor,
        ExportFormatArg::Odps => ExportFormat::Odps,
    }
}

#[cfg(feature = "cli")]
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

#[cfg(feature = "cli")]
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
        } => {
            let import_format = convert_import_format(format);
            let input_source = parse_input_source(&input, &import_format);

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
                        use data_modelling_sdk::cli::error::CliError;
                        Err(CliError::InvalidArgument(
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
                        use data_modelling_sdk::cli::error::CliError;
                        Err(CliError::InvalidArgument(
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
        } => {
            let export_format = convert_export_format(format);

            let args = ExportArgs {
                format: export_format,
                input,
                output,
                force,
                protoc_path,
                protobuf_version: Some(protobuf_version),
            };

            match args.format {
                ExportFormat::Odcs => handle_export_odcs(&args),
                ExportFormat::Avro => handle_export_avro(&args),
                ExportFormat::JsonSchema => handle_export_json_schema(&args),
                ExportFormat::Protobuf => handle_export_protobuf(&args),
                ExportFormat::ProtobufDescriptor => handle_export_protobuf_descriptor(&args),
                ExportFormat::Odps => handle_export_odps(&args),
            }
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

#[cfg(not(feature = "cli"))]
fn main() {
    eprintln!("CLI feature is not enabled. Build with --features cli");
    std::process::exit(1);
}
