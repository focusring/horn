use crate::model::{CheckOutcome, ValidationReport};
use anyhow::Result;
use std::io::Write;

/// Write a `JUnit` XML report for CI dashboard integration.
pub fn write_junit(report: &ValidationReport, w: &mut dyn Write) -> Result<()> {
    writeln!(w, r#"<?xml version="1.0" encoding="UTF-8"?>"#)?;

    let total_tests: usize = report.files.iter().map(|f| f.results.len()).sum();
    let total_failures: usize = report
        .files
        .iter()
        .map(super::super::model::FileReport::failed)
        .sum();
    let total_errors: usize = report.files.iter().filter(|f| f.error.is_some()).count();

    writeln!(
        w,
        r#"<testsuites name="horn" tests="{total_tests}" failures="{total_failures}" errors="{total_errors}">"#,
    )?;

    for file_report in &report.files {
        let file_path = file_report.path.display().to_string();
        let suite_name = xml_escape(&file_path);
        let tests = file_report.results.len();
        let failures = file_report.failed();
        let errors = i32::from(file_report.error.is_some());

        writeln!(
            w,
            r#"  <testsuite name="{suite_name}" tests="{tests}" failures="{failures}" errors="{errors}">"#,
        )?;

        if let Some(error) = &file_report.error {
            writeln!(w, r#"    <testcase name="parse" classname="{suite_name}">"#,)?;
            writeln!(
                w,
                r#"      <error message="{}">{}</error>"#,
                xml_escape(error),
                xml_escape(error),
            )?;
            writeln!(w, "    </testcase>")?;
        }

        for result in &file_report.results {
            let test_name = xml_escape(&result.rule_id);
            let classname = xml_escape(&format!("checkpoint-{:02}", result.checkpoint));

            write!(
                w,
                r#"    <testcase name="{test_name}" classname="{classname}""#,
            )?;

            match &result.outcome {
                CheckOutcome::Fail { message, location } => {
                    writeln!(w, ">")?;
                    // JUnit only supports "failure" and "error" — map all severities to "failure"
                    let severity = "failure";
                    let loc_str = location
                        .as_ref()
                        .and_then(|l| l.element.as_deref())
                        .unwrap_or("");
                    let full_msg = if loc_str.is_empty() {
                        message.clone()
                    } else {
                        format!("{message} [{loc_str}]")
                    };
                    writeln!(
                        w,
                        r#"      <{severity} message="{}" type="{}">{}</{severity}>"#,
                        xml_escape(&full_msg),
                        xml_escape(&result.rule_id),
                        xml_escape(&full_msg),
                    )?;
                    writeln!(w, "    </testcase>")?;
                }
                CheckOutcome::NeedsReview { reason } => {
                    writeln!(w, ">")?;
                    writeln!(w, r"      <system-out>{}</system-out>", xml_escape(reason),)?;
                    writeln!(w, "    </testcase>")?;
                }
                _ => {
                    writeln!(w, " />")?;
                }
            }
        }

        writeln!(w, "  </testsuite>")?;
    }

    writeln!(w, "</testsuites>")?;
    Ok(())
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
