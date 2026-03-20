use crate::checks::Check;
use crate::document::HornDocument;
use crate::model::{CheckOutcome, CheckResult, Severity};
use anyhow::Result;

/// Checkpoint 21: Embedded Files.
///
/// PDF/UA-1 requires that file specification dictionaries for embedded files
/// contain both `/F` and `/UF` (Unicode filename) entries so assistive
/// technologies can present filenames correctly.
pub struct EmbeddedFileChecks;

impl Check for EmbeddedFileChecks {
    fn id(&self) -> &'static str {
        "21-embedded-files"
    }

    fn checkpoint(&self) -> u8 {
        21
    }

    fn description(&self) -> &'static str {
        "Embedded files: file specification completeness"
    }

    fn run(&self, doc: &mut HornDocument) -> Result<Vec<CheckResult>> {
        let mut results = Vec::new();
        let Ok(catalog) = doc.raw_catalog() else {
            return Ok(results);
        };
        let lopdf_doc = doc.lopdf();

        // Navigate to /Names/EmbeddedFiles in catalog
        let names_dict = match catalog.get_deref(b"Names", lopdf_doc) {
            Ok(obj) => match obj.as_dict() {
                Ok(d) => d,
                Err(_) => return Ok(results),
            },
            Err(_) => return Ok(results),
        };

        let ef_tree = match names_dict.get_deref(b"EmbeddedFiles", lopdf_doc) {
            Ok(obj) => match obj.as_dict() {
                Ok(d) => d,
                Err(_) => return Ok(results),
            },
            Err(_) => return Ok(results), // No embedded files
        };

        // Collect file spec references from the name tree
        let file_specs = collect_file_specs(ef_tree, lopdf_doc);

        if file_specs.is_empty() {
            return Ok(results);
        }

        let mut all_valid = true;

        for (i, fs_dict) in file_specs.iter().enumerate() {
            let file_num = i + 1;
            let has_filename = fs_dict.get(b"F").is_ok();
            let has_unicode_filename = fs_dict.get(b"UF").is_ok();

            if !has_filename {
                all_valid = false;
                results.push(fail(
                    "21-001",
                    &format!("Embedded file {file_num}: file specification missing /F (file name)"),
                ));
            }

            if !has_unicode_filename {
                all_valid = false;
                results.push(fail(
                    "21-002",
                    &format!(
                        "Embedded file {file_num}: file specification missing /UF (Unicode file name)"
                    ),
                ));
            }
        }

        if all_valid {
            results.push(pass(
                "21-001",
                &format!(
                    "All {} embedded file specification(s) have /F and /UF entries",
                    file_specs.len()
                ),
            ));
        }

        Ok(results)
    }
}

/// Collect file specification dictionaries from a name tree.
///
/// Name trees can have /Names (leaf) or /Kids (intermediate) arrays.
fn collect_file_specs<'a>(
    tree: &'a lopdf::Dictionary,
    doc: &'a lopdf::Document,
) -> Vec<&'a lopdf::Dictionary> {
    let mut specs = Vec::new();
    collect_from_tree(tree, doc, &mut specs, 0);
    specs
}

fn collect_from_tree<'a>(
    node: &'a lopdf::Dictionary,
    doc: &'a lopdf::Document,
    specs: &mut Vec<&'a lopdf::Dictionary>,
    depth: usize,
) {
    if depth > 20 {
        return;
    }

    // Leaf node: /Names [key1 value1 key2 value2 ...]
    if let Ok(names_arr) = node.get(b"Names").and_then(|o| o.as_array()) {
        let mut i = 1; // values are at odd indices
        while i < names_arr.len() {
            let resolved = if let Ok(ref_id) = names_arr[i].as_reference() {
                doc.get_object(ref_id).ok()
            } else {
                Some(&names_arr[i])
            };
            if let Some(obj) = resolved {
                if let Ok(dict) = obj.as_dict() {
                    specs.push(dict);
                }
            }
            i += 2;
        }
    }

    // Intermediate node: /Kids [ref1 ref2 ...]
    if let Ok(kids) = node.get(b"Kids").and_then(|o| o.as_array()) {
        for kid in kids {
            let resolved = if let Ok(ref_id) = kid.as_reference() {
                doc.get_object(ref_id).ok()
            } else {
                Some(kid)
            };
            if let Some(obj) = resolved {
                if let Ok(d) = obj.as_dict() {
                    collect_from_tree(d, doc, specs, depth + 1);
                }
            }
        }
    }
}

fn pass(rule_id: &str, description: &str) -> CheckResult {
    CheckResult {
        rule_id: rule_id.to_string(),
        checkpoint: 21,
        description: description.to_string(),
        severity: Severity::Info,
        outcome: CheckOutcome::Pass,
    }
}

fn fail(rule_id: &str, message: &str) -> CheckResult {
    CheckResult {
        rule_id: rule_id.to_string(),
        checkpoint: 21,
        description: message.to_string(),
        severity: Severity::Error,
        outcome: CheckOutcome::Fail {
            message: message.to_string(),
            location: None,
        },
    }
}
