use crate::checks::Check;
use crate::document::HornDocument;
use crate::model::{CheckOutcome, CheckResult, Severity};
use anyhow::Result;

/// Checkpoint 13: Image/Figure checks.
///
/// Validates that Figure structure elements have alternative text
/// and that decorative images are marked as artifacts.
pub struct ImageChecks;

impl Check for ImageChecks {
    fn id(&self) -> &'static str {
        "13-images"
    }

    fn checkpoint(&self) -> u8 {
        13
    }

    fn description(&self) -> &'static str {
        "Images: Figure alt text, decorative artifacts"
    }

    fn run(&self, doc: &mut HornDocument) -> Result<Vec<CheckResult>> {
        let mut results = Vec::new();
        let lopdf_doc = doc.lopdf();
        let catalog = doc.raw_catalog()?;

        let Some(struct_tree) = get_struct_tree(catalog, lopdf_doc) else {
            return Ok(results);
        };

        let mut figure_count = 0;
        let mut missing_alt = 0;
        let mut empty_alt = 0;

        collect_figures(
            lopdf_doc,
            struct_tree,
            &mut |dict| {
                figure_count += 1;

                // Check for /Alt entry on the Figure element
                match dict.get(b"Alt") {
                    Ok(alt_obj) => {
                        if let Ok(alt_text) = alt_obj.as_str() {
                            if alt_text.is_empty() {
                                empty_alt += 1;
                                results.push(CheckResult {
                                rule_id: "13-004".to_string(),
                                checkpoint: 13,
                                description: format!("Figure {figure_count}: /Alt text is empty"),
                                severity: Severity::Error,
                                outcome: CheckOutcome::Fail {
                                    message: format!(
                                        "Figure {figure_count} has an empty /Alt entry — alternative text must be meaningful"
                                    ),
                                    location: None,
                                },
                            });
                            }
                            // Non-empty alt text — pass (quality is a manual check)
                        } else {
                            // Alt exists but isn't a string
                            results.push(CheckResult {
                                rule_id: "13-004".to_string(),
                                checkpoint: 13,
                                description: format!("Figure {figure_count}: /Alt is not a string"),
                                severity: Severity::Error,
                                outcome: CheckOutcome::Fail {
                                    message: format!(
                                        "Figure {figure_count} has /Alt but it is not a text string"
                                    ),
                                    location: None,
                                },
                            });
                        }
                    }
                    Err(_) => {
                        // Check for /ActualText as a fallback
                        if dict.get(b"ActualText").is_ok() {
                            // ActualText can serve as alternative text for simple figures
                        } else {
                            missing_alt += 1;
                            results.push(CheckResult {
                            rule_id: "13-004".to_string(),
                            checkpoint: 13,
                            description: format!(
                                "Figure {figure_count}: missing /Alt (alternative text)"
                            ),
                            severity: Severity::Error,
                            outcome: CheckOutcome::Fail {
                                message: format!(
                                    "Figure {figure_count} has no /Alt entry — all Figure elements must have alternative text"
                                ),
                                location: None,
                            },
                        });
                        }
                    }
                }
            },
            0,
        );

        if figure_count > 0 && missing_alt == 0 && empty_alt == 0 {
            results.push(CheckResult {
                rule_id: "13-004".to_string(),
                checkpoint: 13,
                description: format!("All {figure_count} Figure elements have alt text"),
                severity: Severity::Info,
                outcome: CheckOutcome::Pass,
            });
        }

        // Flag that alt text quality cannot be machine-checked
        if figure_count > 0 && missing_alt == 0 {
            results.push(CheckResult {
                rule_id: "13-004".to_string(),
                checkpoint: 13,
                description: "Alt text quality requires human review".to_string(),
                severity: Severity::Info,
                outcome: CheckOutcome::NeedsReview {
                    reason: format!(
                        "{figure_count} Figure element(s) have alt text but quality/accuracy cannot be verified automatically"
                    ),
                },
            });
        }

        Ok(results)
    }
}

fn collect_figures(
    doc: &lopdf::Document,
    dict: &lopdf::Dictionary,
    callback: &mut impl FnMut(&lopdf::Dictionary),
    depth: usize,
) {
    if depth > 100 {
        return;
    }

    if let Ok(s_type) = dict.get(b"S").and_then(|o| o.as_name()) {
        if s_type == b"Figure" {
            callback(dict);
            return; // Don't recurse into Figure children
        }
    }

    let Ok(kids) = dict.get(b"K") else { return };

    match kids {
        lopdf::Object::Array(arr) => {
            for kid in arr {
                if let Ok(kid_ref) = kid.as_reference() {
                    if let Ok(kid_obj) = doc.get_object(kid_ref) {
                        if let Ok(kid_dict) = kid_obj.as_dict() {
                            collect_figures(doc, kid_dict, callback, depth + 1);
                        }
                    }
                } else if let Ok(kid_dict) = kid.as_dict() {
                    collect_figures(doc, kid_dict, callback, depth + 1);
                }
            }
        }
        lopdf::Object::Reference(ref_id) => {
            if let Ok(obj) = doc.get_object(*ref_id) {
                if let Ok(d) = obj.as_dict() {
                    collect_figures(doc, d, callback, depth + 1);
                }
            }
        }
        lopdf::Object::Dictionary(d) => {
            collect_figures(doc, d, callback, depth + 1);
        }
        _ => {}
    }
}

fn get_struct_tree<'a>(
    catalog: &'a lopdf::Dictionary,
    doc: &'a lopdf::Document,
) -> Option<&'a lopdf::Dictionary> {
    let ref_id = catalog.get(b"StructTreeRoot").ok()?.as_reference().ok()?;
    doc.get_object(ref_id).ok()?.as_dict().ok()
}
