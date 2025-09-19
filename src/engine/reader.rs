use crate::dsl::Source;
use anyhow::Result;
use polars::lazy::frame::LazyFrame;
use polars::prelude::*;

pub struct DataReader;

impl DataReader {
    pub fn read_source(source: &Source) -> Result<LazyFrame> {
        let path = &source.path;

        if !path.exists() {
            return Err(anyhow::anyhow!("File not found: {}", path.display()));
        }

        match source.format.as_str() {
            "csv" => Self::read_csv(&path.to_string_lossy()),
            "tsv" => Self::read_tsv(&path.to_string_lossy()),
            "xlsx" => Self::read_xlsx(&path.to_string_lossy()),
            "sqlite" => Self::read_sqlite(&path.to_string_lossy()),
            _ => Err(anyhow::anyhow!("Unsupported format: {}", source.format)),
        }
    }

    fn read_csv(path: &str) -> Result<LazyFrame> {
        let df = LazyCsvReader::new(PlPath::new(path)).finish().unwrap();
        Ok(df)
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
