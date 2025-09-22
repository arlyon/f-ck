use anyhow::Result;
use polars::lazy::frame::LazyFrame;
use polars::prelude::file::DynWriteable;
use polars::prelude::*;
use std::io::Write;

#[cfg(feature = "csv-support")]
use polars::prelude::CsvWriter;

pub struct DataWriter;

struct WriteWrapper<W: Write + Send> {
    writer: W,
}

impl<'a, W: Write + Send> Write for WriteWrapper<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        tracing::debug!("writing {}", String::from_utf8_lossy(buf));
        self.writer.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }
}
impl<W: Write + Send + 'static> DynWriteable for WriteWrapper<W> {
    fn as_dyn_write(&self) -> &(dyn std::io::Write + Send + 'static) {
        &self.writer
    }

    fn as_mut_dyn_write(&mut self) -> &mut (dyn std::io::Write + Send + 'static) {
        &mut self.writer
    }

    fn close(self: Box<Self>) -> std::io::Result<()> {
        Ok(())
    }

    fn sync_on_close(
        &mut self,
        _sync_on_close: sync_on_close::SyncOnCloseType,
    ) -> std::io::Result<()> {
        Ok(())
    }
}

impl DataWriter {
    pub fn write_csv<F: IntoLazy>(df: F, output_path: &str) -> Result<()> {
        let file = std::fs::File::create(output_path)?;
        Self::write_csv_to_writer(df, file)
    }

    #[tracing::instrument(skip(df, writer))]
    pub fn write_csv_to_writer<W: Write + Send + 'static, F: IntoLazy>(
        df: F,
        writer: W,
    ) -> Result<()> {
        #[cfg(feature = "csv-support")]
        {
            use std::sync::Mutex;

            let start = std::time::Instant::now();

            // let df = df
            //     .lazy()
            //     .sink_csv(
            //         SinkTarget::Dyn(SpecialEq::new(Arc::new(Mutex::new(Some(Box::new(
            //             WriteWrapper { writer },
            //         )))))),
            //         CsvWriterOptions::default(),
            //         None,
            //         SinkOptions::default(),
            //     )?
            //     .collect()?;

            // println!("collected {}", df.size());

            CsvWriter::new(writer).finish(&mut df.lazy().collect()?)?;

            tracing::debug!("CSV writing took {:?}", std::time::Instant::now() - start);
            Ok(())
        }
        #[cfg(not(feature = "csv-support"))]
        {
            Err(anyhow::anyhow!("CSV support not enabled for WASM build"))
        }
    }

    pub fn write_with_format<F: IntoLazy>(df: F, output_path: &str, format: &str) -> Result<()> {
        match format.to_lowercase().as_str() {
            "csv" => Self::write_csv(df, output_path),
            "tsv" => Self::write_tsv(df, output_path),
            "xlsx" => Self::write_xlsx(df, output_path),
            "sqlite" => Self::write_sqlite(df, output_path),
            _ => Err(anyhow::anyhow!("Unsupported output format: {}", format)),
        }
    }

    fn write_tsv<F: IntoLazy>(df: F, output_path: &str) -> Result<()> {
        let mut file = std::fs::File::create(output_path)?;
        Self::write_tsv_to_writer(df, &mut file)
    }

    pub fn write_tsv_to_writer<W: Write, F: IntoLazy>(df: F, writer: &mut W) -> Result<()> {
        #[cfg(feature = "csv-support")]
        {
            let collected = df.lazy().collect()?;
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

    fn write_xlsx<F: IntoLazy>(_df: F, _output_path: &str) -> Result<()> {
        // XLSX support would require additional dependencies like calamine
        // For now, we'll defer this implementation
        Err(anyhow::anyhow!("XLSX output not yet implemented"))
    }

    fn write_sqlite<F: IntoLazy>(_df: F, _output_path: &str) -> Result<()> {
        // SQLite output support would require additional SQL handling
        // For now, we'll defer this implementation
        Err(anyhow::anyhow!("SQLite output not yet implemented"))
    }

    pub fn preview_data<F: IntoLazy>(df: F, limit: Option<usize>) -> Result<String> {
        let mut collected = df.lazy().collect()?;

        if let Some(n) = limit {
            collected = collected.head(Some(n));
        }

        Ok(format!("{}", collected))
    }

    pub fn write_to_writer<W: Write + Send + 'static, F: IntoLazy>(
        df: F,
        mut writer: W,
        format: &str,
    ) -> Result<()> {
        match format.to_lowercase().as_str() {
            "csv" => Self::write_csv_to_writer(df, writer),
            "tsv" => Self::write_tsv_to_writer(df, &mut writer),
            "xlsx" => Err(anyhow::anyhow!("XLSX output to writer not supported")),
            "sqlite" => Err(anyhow::anyhow!("SQLite output to writer not supported")),
            _ => Err(anyhow::anyhow!("Unsupported output format: {}", format)),
        }
    }
}
