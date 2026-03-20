pub mod junit;
pub mod sarif;
pub mod text;

use crate::model::ValidationReport;
use anyhow::Result;
use std::io::Write;

/// Supported output formats.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "cli", derive(clap::ValueEnum))]
pub enum OutputFormat {
    Text,
    Json,
    Sarif,
    Junit,
}

/// Write a validation report in the specified format.
pub fn write_report(
    report: &ValidationReport,
    format: OutputFormat,
    writer: &mut dyn Write,
) -> Result<()> {
    match format {
        OutputFormat::Text => text::write_text(report, writer),
        OutputFormat::Json => {
            serde_json::to_writer_pretty(&mut *writer, report)?;
            writeln!(writer)?;
            Ok(())
        }
        OutputFormat::Sarif => sarif::write_sarif(report, writer),
        OutputFormat::Junit => junit::write_junit(report, writer),
    }
}
