use crate::checks::Check;
use crate::document::HornDocument;
use crate::model::{CheckOutcome, CheckResult, Severity};
use anyhow::Result;

/// Checkpoint 16: List structure checks.
///
/// Validates that lists use correct L/LI/Lbl/LBody structure.
pub struct ListChecks;

impl Check for ListChecks {
    fn id(&self) -> &'static str {
        "16-lists"
    }

    fn checkpoint(&self) -> u8 {
        16
    }

    fn description(&self) -> &'static str {
        "Lists: L/LI/Lbl/LBody structure"
    }

    fn run(&self, doc: &mut HornDocument) -> Result<Vec<CheckResult>> {
        let mut results = Vec::new();
        let lopdf_doc = doc.lopdf();
        let catalog = doc.raw_catalog()?;

        let Some(struct_tree) = get_struct_tree(catalog, lopdf_doc) else {
            return Ok(results);
        };

        let mut list_count = 0;
        let mut errors_found = false;

        check_lists(
            lopdf_doc,
            struct_tree,
            &mut list_count,
            &mut errors_found,
            &mut results,
            0,
        );

        if list_count > 0 && !errors_found {
            results.push(CheckResult {
                rule_id: "16-001".to_string(),
                checkpoint: 16,
                description: format!("All {list_count} list(s) have valid L/LI structure"),
                severity: Severity::Info,
                outcome: CheckOutcome::Pass,
            });
        }

        Ok(results)
    }
}

fn check_lists(
    doc: &lopdf::Document,
    dict: &lopdf::Dictionary,
    list_count: &mut usize,
    errors_found: &mut bool,
    results: &mut Vec<CheckResult>,
    depth: usize,
) {
    if depth > 100 {
        return;
    }

    if let Ok(s_type) = dict.get(b"S").and_then(|o| o.as_name()) {
        if s_type == b"L" {
            *list_count += 1;
            let list_label = format!("List {}", *list_count);

            // 16-001: L must contain LI children
            let children = get_child_types(doc, dict);

            let li_count = children.iter().filter(|c| c.as_slice() == b"LI").count();
            // L can contain LI, Caption, and nested L (for nested lists)
            let non_li: Vec<_> = children
                .iter()
                .filter(|c| {
                    c.as_slice() != b"LI" && c.as_slice() != b"Caption" && c.as_slice() != b"L"
                })
                .collect();

            if li_count == 0 {
                *errors_found = true;
                results.push(CheckResult {
                    rule_id: "16-001".to_string(),
                    checkpoint: 16,
                    description: format!("{list_label}: List has no LI (list item) children"),
                    severity: Severity::Error,
                    outcome: CheckOutcome::Fail {
                        message: format!("{list_label}: L element must contain LI children"),
                        location: None,
                    },
                });
            }

            if !non_li.is_empty() {
                *errors_found = true;
                let types: Vec<String> = non_li
                    .iter()
                    .map(|t| String::from_utf8_lossy(t).to_string())
                    .collect();
                results.push(CheckResult {
                    rule_id: "16-002".to_string(),
                    checkpoint: 16,
                    description: format!(
                        "{list_label}: L contains non-LI children: {}",
                        types.join(", ")
                    ),
                    severity: Severity::Error,
                    outcome: CheckOutcome::Fail {
                        message: format!(
                            "{list_label}: L element contains invalid children ({}); only LI and Caption are allowed",
                            types.join(", ")
                        ),
                        location: None,
                    },
                });
            }

            // Check LI children have LBody
            check_li_structure(doc, dict, &list_label, errors_found, results);

            // Don't return — recurse to find nested lists
        }
    }

    walk_children(doc, dict, |child_dict| {
        check_lists(
            doc,
            child_dict,
            list_count,
            errors_found,
            results,
            depth + 1,
        );
    });
}

fn check_li_structure(
    doc: &lopdf::Document,
    list_dict: &lopdf::Dictionary,
    list_label: &str,
    errors_found: &mut bool,
    results: &mut Vec<CheckResult>,
) {
    let mut li_index = 0;

    walk_children(doc, list_dict, |child_dict| {
        if let Ok(s_type) = child_dict.get(b"S").and_then(|o| o.as_name()) {
            if s_type == b"LI" {
                li_index += 1;
                let child_types = get_child_types(doc, child_dict);

                // Only check for LBody if the LI has structured children.
                // If LI only has MCIDs (integer content refs) or no structured
                // children, it's direct content and LBody is implicit.
                if child_types.is_empty() {
                    return;
                }

                // Per ISO 32000-1 Table 336, LI should contain Lbl and/or LBody.
                // Either one (or both) is valid.
                let has_lbody = child_types.iter().any(|t| t.as_slice() == b"LBody");
                let has_lbl = child_types.iter().any(|t| t.as_slice() == b"Lbl");
                if !has_lbody && !has_lbl {
                    *errors_found = true;
                    results.push(CheckResult {
                        rule_id: "16-003".to_string(),
                        checkpoint: 16,
                        description: format!("{list_label}, item {li_index}: LI missing LBody"),
                        severity: Severity::Error,
                        outcome: CheckOutcome::Fail {
                            message: format!(
                                "{list_label}, item {li_index}: LI must contain an LBody element"
                            ),
                            location: None,
                        },
                    });
                }
            }
        }
    });
}

fn get_child_types(doc: &lopdf::Document, dict: &lopdf::Dictionary) -> Vec<Vec<u8>> {
    let mut types = Vec::new();

    walk_children(doc, dict, |child_dict| {
        if let Ok(s_type) = child_dict.get(b"S").and_then(|o| o.as_name()) {
            types.push(s_type.to_vec());
        }
    });

    types
}

fn walk_children(
    doc: &lopdf::Document,
    dict: &lopdf::Dictionary,
    mut callback: impl FnMut(&lopdf::Dictionary),
) {
    let Ok(kids) = dict.get(b"K") else { return };

    match kids {
        lopdf::Object::Array(arr) => {
            for kid in arr {
                if let Ok(kid_ref) = kid.as_reference() {
                    if let Ok(kid_obj) = doc.get_object(kid_ref) {
                        if let Ok(kid_dict) = kid_obj.as_dict() {
                            callback(kid_dict);
                        }
                    }
                } else if let Ok(kid_dict) = kid.as_dict() {
                    callback(kid_dict);
                }
            }
        }
        lopdf::Object::Reference(ref_id) => {
            if let Ok(obj) = doc.get_object(*ref_id) {
                if let Ok(d) = obj.as_dict() {
                    callback(d);
                }
            }
        }
        lopdf::Object::Dictionary(d) => {
            callback(d);
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
