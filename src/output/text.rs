use crate::model::{CheckOutcome, Severity, Standard, ValidationReport};
use anyhow::Result;
use std::io::Write;

pub fn write_text(report: &ValidationReport, w: &mut dyn Write) -> Result<()> {
    for file_report in &report.files {
        let path = file_report.path.display();
        writeln!(w, "\n{path}")?;
        writeln!(w, "{}", "=".repeat(path.to_string().len()))?;

        if let Some(error) = &file_report.error {
            writeln!(w, "  ERROR: Could not process file: {error}")?;
            continue;
        }

        // Show detected standard
        match file_report.standard {
            Standard::Ua1 => writeln!(w, "  Standard: PDF/UA-1")?,
            Standard::Ua2 => writeln!(w, "  Standard: PDF/UA-2")?,
            Standard::Unknown => writeln!(w, "  Standard: Unknown (no pdfuaid:part found)")?,
        }

        let failures: Vec<_> = file_report
            .results
            .iter()
            .filter(|r| r.is_failure())
            .collect();

        if failures.is_empty() {
            writeln!(
                w,
                "  PASS: No issues found ({} checks run)",
                file_report.results.len()
            )?;
        } else {
            for result in &failures {
                let severity_label = match result.severity {
                    Severity::Error => "FAIL",
                    Severity::Warning => "WARN",
                    Severity::Info => "INFO",
                };
                if let CheckOutcome::Fail { message, location } = &result.outcome {
                    let loc = location
                        .as_ref()
                        .and_then(|l| l.element.as_deref())
                        .unwrap_or("");
                    let loc_str = if loc.is_empty() {
                        String::new()
                    } else {
                        format!(" [{loc}]")
                    };
                    writeln!(
                        w,
                        "  [{severity_label}] {id}: {message}{loc_str}",
                        id = result.rule_id,
                    )?;
                }
            }
        }

        writeln!(
            w,
            "\n  Summary: {} passed, {} failed, {} needs review",
            file_report.passed(),
            file_report.failed(),
            file_report.needs_review(),
        )?;
    }

    // Overall summary
    let total_files = report.files.len();
    let compliant = report.files.iter().filter(|f| f.is_compliant()).count();
    writeln!(w, "\n---")?;
    writeln!(w, "Total: {compliant}/{total_files} files compliant")?;

    Ok(())
}
