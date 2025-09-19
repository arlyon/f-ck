use anyhow::Result;
use clap::{Arg, Command};
use f_ck::{QueryPlan, JoinEngine, DataWriter};
use std::fs;

fn main() -> Result<()> {
    let matches = Command::new("f-ck")
        .about("f*ck - fields combined with columnar keys")
        .version("0.1.0")
        .arg(
            Arg::new("query")
                .short('q')
                .long("query")
                .value_name("FILE")
                .help("JSON file containing the query plan")
                .required(true),
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .value_name("FILE")
                .help("Output file path")
                .required(true),
        )
        .arg(
            Arg::new("format")
                .short('f')
                .long("format")
                .value_name("FORMAT")
                .help("Output format (csv, tsv, xlsx, sqlite)")
                .default_value("csv"),
        )
        .arg(
            Arg::new("preview")
                .short('p')
                .long("preview")
                .help("Preview the result without writing to file")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("limit")
                .short('l')
                .long("limit")
                .value_name("N")
                .help("Limit preview to N rows")
                .value_parser(clap::value_parser!(usize)),
        )
        .get_matches();

    let query_file = matches.get_one::<String>("query").unwrap();
    let output_file = matches.get_one::<String>("output").unwrap();
    let format = matches.get_one::<String>("format").unwrap();
    let preview_mode = matches.get_flag("preview");
    let limit = matches.get_one::<usize>("limit").copied();

    // Read and parse the query plan
    let query_json = fs::read_to_string(query_file)
        .map_err(|e| anyhow::anyhow!("Failed to read query file '{}': {}", query_file, e))?;
    
    let query = QueryPlan::from_json(&query_json)
        .map_err(|e| anyhow::anyhow!("Failed to parse query: {}", e))?;

    println!("Executing query with {} sources...", query.sources.len());
    
    // Execute the query
    let result = JoinEngine::execute_query(&query)
        .map_err(|e| anyhow::anyhow!("Query execution failed: {}", e))?;

    if preview_mode {
        // Preview mode - just print the results
        let preview = DataWriter::preview_data(result, limit)?;
        println!("Preview:\n{}", preview);
    } else {
        // Write to output file
        DataWriter::write_with_format(result, output_file, format)?;
        println!("Results written to: {}", output_file);
    }

    Ok(())
}
