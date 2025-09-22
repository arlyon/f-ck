use crate::dsl::Source;
use anyhow::Result;
use polars::lazy::frame::LazyFrame;
use polars::prelude::*;

use polars_io::prelude::CsvReader;

use polars_io::SerReader;
#[cfg(feature = "csv-support")]
use polars_lazy::prelude::LazyCsvReader;

pub struct DataReader;

impl DataReader {
    pub fn read_source(source: &Source) -> Result<LazyFrame> {
        match source {
            #[cfg(not(feature = "wasm"))]
            Source::File { path, .. } if !path.exists() => {
                Err(anyhow::anyhow!("File not found: {}", path.display()))
            }
            #[cfg(not(feature = "wasm"))]
            Source::File { path, format, .. } => match format.as_str() {
                "csv" => Self::read_csv(&path.to_string_lossy()),
                "tsv" => Self::read_tsv(&path.to_string_lossy()),
                "xlsx" => Self::read_xlsx(&path.to_string_lossy()),
                "sqlite" => Self::read_sqlite(&path.to_string_lossy()),
                fmt => Err(anyhow::anyhow!("Unsupported format: {}", fmt)),
            },
            Source::Url { url, .. } => {
                // download the blob and then produce a dataframe wrapped in a lazy frame
                // let body = reqwest::blocking::get(url)?.bytes()?;
                // let cursor = std::io::Cursor::new(body.to_vec());
                // Ok(CsvReader::new(cursor).finish()?.lazy())
                todo!()
            }
            Source::Blob { blob, .. } => {
                let cursor = std::io::Cursor::new(blob);
                Ok(CsvReader::new(cursor).finish()?.lazy())
            }
        }
    }

    fn read_csv(path: &str) -> Result<LazyFrame> {
        #[cfg(feature = "csv-support")]
        {
            let df = LazyCsvReader::new(PlPath::new(path)).finish().unwrap();
            Ok(df)
        }
        #[cfg(not(feature = "csv-support"))]
        {
            Err(anyhow::anyhow!("CSV support not enabled for WASM build"))
        }
    }

    fn read_tsv(_path: &str) -> Result<LazyFrame> {
        Err(anyhow::anyhow!("TSV support not yet implemented"))
    }

    fn read_xlsx(_path: &str) -> Result<LazyFrame> {
        // For now, we'll implement basic XLSX support
        // This is a simplified implementation - in practice you'd want more robust Excel handling
        Err(anyhow::anyhow!("XLSX support not yet implemented"))
    }

    fn read_sqlite(_path: &str) -> Result<LazyFrame> {
        // SQLite support would require additional SQL parsing or table specification
        // For now, we'll defer this implementation
        Err(anyhow::anyhow!("SQLite support not yet implemented"))
    }

    pub fn get_schema(source: &Source) -> Result<Schema> {
        let mut df = Self::read_source(source)?;
        Ok((*df.collect_schema()?).clone())
    }

    pub fn infer_types(source: &Source) -> Result<Vec<(String, DataType)>> {
        let schema = Self::get_schema(source)?;
        let types: Vec<(String, DataType)> = schema
            .iter()
            .map(|(name, dtype)| (name.to_string(), dtype.clone()))
            .collect();
        Ok(types)
    }
}
