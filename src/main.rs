use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use f_ck::{DataWriter, JoinEngine, QueryPlan, Source, dsl::Query};
use std::{fs, path::PathBuf, time::Instant};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser)]
#[command(name = "f-ck")]
#[command(about = "f*ck - fields combined with columnar keys")]
#[command(version = "0.1.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Execute a query plan
    Run {
        /// Query string (JSON for DSL, plain text for SQL)
        #[arg(long, value_name = "QUERY", group = "input_mode")]
        query: Query,
        /// Source files in format "id:path" (e.g., "customers:data/customers.csv")
        #[arg(long, value_name = "ID:PATH", action = clap::ArgAction::Append)]
        source: Vec<Source>,

        #[command(flatten)]
        rest: RunArgs,
    },
    File {
        /// JSON file containing the complete query plan
        #[arg(long, value_name = "FILE", group = "input_mode")]
        file: PathBuf,

        #[command(flatten)]
        rest: RunArgs,
    },
    /// Generate JSON schema for query plan validation
    Schema {
        /// Output file for the schema (defaults to stdout)
        #[arg(short, long, value_name = "FILE")]
        output: Option<String>,
    },
}

#[derive(Args, Clone)]
struct RunArgs {
    /// Output file path (defaults to stdout)
    #[arg(short, long, value_name = "FILE")]
    output: Option<String>,

    /// Output format (csv, tsv, xlsx, sqlite)
    #[arg(short, long, value_name = "FORMAT", default_value = "csv")]
    format: String,

    /// Preview the result without writing to file
    #[arg(short, long)]
    preview: bool,

    /// Limit preview to N rows
    #[arg(short, long, value_name = "N")]
    limit: Option<usize>,
}

#[tracing::instrument]
fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    let start = std::time::Instant::now();

    let cli = Cli::parse();

    let (
        query_plan,
        RunArgs {
            format,
            limit,
            output,
            preview,
            ..
        },
    ) = match cli.command {
        Commands::Schema { output } => {
            let schema = QueryPlan::json_schema()?;

            if let Some(output_file) = output {
                fs::write(&output_file, &schema).map_err(|e| {
                    anyhow::anyhow!("Failed to write schema to '{}': {}", output_file, e)
                })?;
                tracing::debug!("JSON schema written to: {}", output_file);
            } else {
                println!("{}", schema);
            }

            return Ok(());
        }
        Commands::Run {
            query,
            source,
            rest,
        } => (
            QueryPlan {
                sources: source,
                query,
            },
            rest,
        ),
        Commands::File { file, rest } => {
            let file = std::fs::read_to_string(&file).unwrap();
            (QueryPlan::from_json(&file)?, rest)
        }
    };

    let parse = std::time::Instant::now();
    tracing::debug!("Parsed in {:?}", parse - start);

    tracing::debug!(
        "Executing query with {} sources...",
        query_plan.sources.len()
    );

    // Execute the query with or without caching
    let result = JoinEngine::execute_query(&query_plan)
        .map_err(|e| anyhow::anyhow!("Query execution failed: {}", e))?;

    if preview {
        // Preview mode - just print the results
        let preview_output = DataWriter::preview_data(result, limit)?;
        println!("Preview:\n{}", preview_output);
    } else {
        // Write to output file or stdout
        if let Some(output_file) = output {
            DataWriter::write_with_format(result, &output_file, &format)?;
            tracing::debug!("Results written to: {}", output_file);
        } else {
            let stdout = std::io::stdout();
            DataWriter::write_to_writer(result, stdout, &format)?;
        }
    }

    let written = Instant::now();

    tracing::debug!("Query executed in {:?}", written - start);

    Ok(())
}
