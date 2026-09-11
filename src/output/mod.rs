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

/// Presentation options for report writers.
#[derive(Debug, Clone, Copy, Default)]
pub struct OutputOptions {
    /// Text output: also list the conditions that need manual review.
    pub show_review: bool,
}

/// Write a validation report in the specified format.
pub fn write_report(
    report: &ValidationReport,
    format: OutputFormat,
    writer: &mut dyn Write,
) -> Result<()> {
    write_report_with(report, format, OutputOptions::default(), writer)
}

/// Write a validation report in the specified format with presentation options.
pub fn write_report_with(
    report: &ValidationReport,
    format: OutputFormat,
    options: OutputOptions,
    writer: &mut dyn Write,
) -> Result<()> {
    match format {
        OutputFormat::Text => text::write_text(report, options, writer),
        OutputFormat::Json => {
            serde_json::to_writer_pretty(&mut *writer, report)?;
            writeln!(writer)?;
            Ok(())
        }
        OutputFormat::Sarif => sarif::write_sarif(report, writer),
        OutputFormat::Junit => junit::write_junit(report, writer),
    }
}
