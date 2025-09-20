use anyhow::Result;
use clap::{Parser, Subcommand};
use f_ck::{QueryPlan, CachedEngine, DataWriter};
use std::fs;

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
        /// JSON file containing the query plan
        #[arg(short, long, value_name = "FILE")]
        query: String,
        
        /// Output file path
        #[arg(short, long, value_name = "FILE")]
        output: String,
        
        /// Output format (csv, tsv, xlsx, sqlite)
        #[arg(short, long, value_name = "FORMAT", default_value = "csv")]
        format: String,
        
        /// Preview the result without writing to file
        #[arg(short, long)]
        preview: bool,
        
        /// Limit preview to N rows
        #[arg(short, long, value_name = "N")]
        limit: Option<usize>,
        
        /// Disable caching for this execution
        #[arg(long)]
        no_cache: bool,
        
        /// Show cache statistics
        #[arg(long)]
        cache_stats: bool,
    },
    /// Generate JSON schema for query plan validation
    Schema {
        /// Output file for the schema (defaults to stdout)
        #[arg(short, long, value_name = "FILE")]
        output: Option<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run {
            query,
            output,
            format,
            preview,
            limit,
            no_cache,
            cache_stats,
        } => {
            // Read and parse the query plan
            let query_json = fs::read_to_string(&query)
                .map_err(|e| anyhow::anyhow!("Failed to read query file '{}': {}", query, e))?;
            
            let query_plan = QueryPlan::from_json(&query_json)
                .map_err(|e| anyhow::anyhow!("Failed to parse query: {}", e))?;

            println!("Executing query with {} sources...", query_plan.sources.len());
            
            // Create cached engine
            let mut engine = CachedEngine::new();
            
            // Show cache stats if requested
            if cache_stats {
                let stats = engine.cache_stats();
                println!("Cache Statistics:");
                for (key, value) in stats {
                    println!("  {}: {}", key, value);
                }
            }
            
            // Execute the query with or without caching
            let result = if no_cache {
                use f_ck::JoinEngine;
                JoinEngine::execute_query(&query_plan)
            } else {
                engine.execute_query_cached(&query_plan)
            }.map_err(|e| anyhow::anyhow!("Query execution failed: {}", e))?;

            if preview {
                // Preview mode - just print the results
                let preview_output = DataWriter::preview_data(result, limit)?;
                println!("Preview:\n{}", preview_output);
            } else {
                // Write to output file
                DataWriter::write_with_format(result, &output, &format)?;
                println!("Results written to: {}", output);
            }
        }
        Commands::Schema { output } => {
            let schema = QueryPlan::json_schema()?;
            
            if let Some(output_file) = output {
                fs::write(&output_file, &schema)
                    .map_err(|e| anyhow::anyhow!("Failed to write schema to '{}': {}", output_file, e))?;
                println!("JSON schema written to: {}", output_file);
            } else {
                println!("{}", schema);
            }
        }
    }

    Ok(())
}
