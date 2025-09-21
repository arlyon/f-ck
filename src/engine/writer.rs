use anyhow::Result;
use polars::lazy::frame::LazyFrame;
use polars::prelude::*;
use std::io::Write;

#[cfg(feature = "csv-support")]
use polars::prelude::CsvWriter;

pub struct DataWriter;

impl DataWriter {
    pub fn write_csv(df: LazyFrame, output_path: &str) -> Result<()> {
        let mut file = std::fs::File::create(output_path)?;
        Self::write_csv_to_writer(df, &mut file)
    }

    pub fn write_csv_to_writer<W: Write>(df: LazyFrame, writer: &mut W) -> Result<()> {
        #[cfg(feature = "csv-support")]
        {
            let collected = df.collect()?;
            CsvWriter::new(writer)
                .include_header(true)
                .finish(&mut collected.clone())?;
            Ok(())
        }
        #[cfg(not(feature = "csv-support"))]
        {
            Err(anyhow::anyhow!("CSV support not enabled for WASM build"))
        }
    }

    pub fn write_with_format(df: LazyFrame, output_path: &str, format: &str) -> Result<()> {
        match format.to_lowercase().as_str() {
            "csv" => Self::write_csv(df, output_path),
            "tsv" => Self::write_tsv(df, output_path),
            "xlsx" => Self::write_xlsx(df, output_path),
            "sqlite" => Self::write_sqlite(df, output_path),
            _ => Err(anyhow::anyhow!("Unsupported output format: {}", format)),
        }
    }

    fn write_tsv(df: LazyFrame, output_path: &str) -> Result<()> {
        let mut file = std::fs::File::create(output_path)?;
        Self::write_tsv_to_writer(df, &mut file)
    }

    pub fn write_tsv_to_writer<W: Write>(df: LazyFrame, writer: &mut W) -> Result<()> {
        #[cfg(feature = "csv-support")]
        {
            let collected = df.collect()?;
            CsvWriter::new(writer)
                .include_header(true)
                .with_separator(b'\t')
                .finish(&mut collected.clone())?;
            Ok(())
        }
        #[cfg(not(feature = "csv-support"))]
        {
            Err(anyhow::anyhow!("TSV support not enabled for WASM build"))
        }
    }

    fn write_xlsx(_df: LazyFrame, _output_path: &str) -> Result<()> {
        // XLSX support would require additional dependencies like calamine
        // For now, we'll defer this implementation
        Err(anyhow::anyhow!("XLSX output not yet implemented"))
    }

    fn write_sqlite(_df: LazyFrame, _output_path: &str) -> Result<()> {
        // SQLite output support would require additional SQL handling
        // For now, we'll defer this implementation
        Err(anyhow::anyhow!("SQLite output not yet implemented"))
    }

    pub fn preview_data(df: LazyFrame, limit: Option<usize>) -> Result<String> {
        let mut collected = df.collect()?;

        if let Some(n) = limit {
            collected = collected.head(Some(n));
        }

        Ok(format!("{}", collected))
    }

    pub fn write_to_writer<W: Write>(df: LazyFrame, writer: &mut W, format: &str) -> Result<()> {
        match format.to_lowercase().as_str() {
            "csv" => Self::write_csv_to_writer(df, writer),
            "tsv" => Self::write_tsv_to_writer(df, writer),
            "xlsx" => Err(anyhow::anyhow!("XLSX output to writer not supported")),
            "sqlite" => Err(anyhow::anyhow!("SQLite output to writer not supported")),
            _ => Err(anyhow::anyhow!("Unsupported output format: {}", format)),
        }
    }
}
