use crate::model::{CheckOutcome, Severity, ValidationReport};
use anyhow::Result;
use serde_json::{Value, json};
use std::io::Write;

/// Write a SARIF v2.1.0 report for GitHub Code Scanning integration.
pub fn write_sarif(report: &ValidationReport, w: &mut dyn Write) -> Result<()> {
    let mut rules: Vec<Value> = Vec::new();
    let mut rule_ids: Vec<String> = Vec::new();
    let mut results: Vec<Value> = Vec::new();

    for file_report in &report.files {
        let file_path = file_report.path.display().to_string();

        for check_result in &file_report.results {
            if let CheckOutcome::Fail { message, location } = &check_result.outcome {
                // Register the rule if not already seen
                if !rule_ids.contains(&check_result.rule_id) {
                    rule_ids.push(check_result.rule_id.clone());
                    rules.push(json!({
                        "id": check_result.rule_id,
                        "name": check_result.rule_id,
                        "shortDescription": {
                            "text": check_result.description
                        },
                        "defaultConfiguration": {
                            "level": sarif_level(check_result.severity)
                        },
                        "properties": {
                            "checkpoint": check_result.checkpoint
                        }
                    }));
                }

                let rule_index = rule_ids
                    .iter()
                    .position(|id| id == &check_result.rule_id)
                    .unwrap_or(0);

                let mut sarif_location = json!({
                    "physicalLocation": {
                        "artifactLocation": {
                            "uri": file_path
                        }
                    }
                });

                if let Some(loc) = location {
                    if let Some(element) = &loc.element {
                        sarif_location["logicalLocations"] = json!([{
                            "name": element,
                            "kind": "element"
                        }]);
                    }
                }

                results.push(json!({
                    "ruleId": check_result.rule_id,
                    "ruleIndex": rule_index,
                    "level": sarif_level(check_result.severity),
                    "message": {
                        "text": message
                    },
                    "locations": [sarif_location]
                }));
            }
        }

        if let Some(error) = &file_report.error {
            results.push(json!({
                "ruleId": "parse-error",
                "level": "error",
                "message": {
                    "text": format!("Failed to process: {error}")
                },
                "locations": [{
                    "physicalLocation": {
                        "artifactLocation": {
                            "uri": file_path
                        }
                    }
                }]
            }));
        }
    }

    let sarif = json!({
        "$schema": "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/main/sarif-2.1/schema/sarif-schema-2.1.0.json",
        "version": "2.1.0",
        "runs": [{
            "tool": {
                "driver": {
                    "name": "horn",
                    "version": env!("CARGO_PKG_VERSION"),
                    "informationUri": "https://github.com/focusring/horn",
                    "rules": rules
                }
            },
            "results": results
        }]
    });

    serde_json::to_writer_pretty(&mut *w, &sarif)?;
    writeln!(w)?;
    Ok(())
}

fn sarif_level(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "note",
    }
}
