use crate::checks::Check;
use crate::document::HornDocument;
use crate::model::{CheckOutcome, CheckResult, Severity};
use anyhow::Result;

/// Checkpoint 14: Heading hierarchy checks.
///
/// Validates that headings follow a logical sequence (no skipped levels)
/// and that the document uses numbered headings consistently.
pub struct HeadingChecks;

impl Check for HeadingChecks {
    fn id(&self) -> &'static str {
        "14-headings"
    }

    fn checkpoint(&self) -> u8 {
        14
    }

    fn description(&self) -> &'static str {
        "Headings: hierarchy, no skipped levels"
    }

    fn run(&self, doc: &mut HornDocument) -> Result<Vec<CheckResult>> {
        let mut results = Vec::new();
        let lopdf_doc = doc.lopdf();
        let catalog = doc.raw_catalog()?;

        let Some(struct_tree) = get_struct_tree(catalog, lopdf_doc) else {
            return Ok(results);
        };

        let mut headings: Vec<HeadingInfo> = Vec::new();
        collect_headings(lopdf_doc, struct_tree, &mut headings, 0);

        // Check for sibling generic H headings (same parent, both H)
        let has_generic_h_sibling_violation = check_generic_h_siblings(lopdf_doc, struct_tree, 0);

        if headings.is_empty() {
            // No headings found — this might be okay for simple documents
            return Ok(results);
        }

        // 14-002: First heading should be H1
        if let Some(first) = headings.first() {
            if first.level > 1 {
                results.push(CheckResult {
                    rule_id: "14-002".to_string(),
                    checkpoint: 14,
                    description: format!(
                        "First heading is H{} — should start with H1",
                        first.level
                    ),
                    severity: Severity::Warning,
                    outcome: CheckOutcome::Fail {
                        message: format!("Document starts with H{} instead of H1", first.level),
                        location: None,
                    },
                });
            }
        }

        // 14-006: No skipped heading levels (e.g., H1 -> H3 without H2)
        let mut prev_level: u8 = 0;
        let mut skip_found = false;

        for heading in &headings {
            if heading.level > prev_level + 1 && prev_level > 0 {
                results.push(CheckResult {
                    rule_id: "14-006".to_string(),
                    checkpoint: 14,
                    description: format!(
                        "Heading level skipped: H{prev_level} followed by H{}",
                        heading.level
                    ),
                    severity: Severity::Error,
                    outcome: CheckOutcome::Fail {
                        message: format!(
                            "Heading level skipped from H{prev_level} to H{} — levels must not be skipped",
                            heading.level
                        ),
                        location: None,
                    },
                });
                skip_found = true;
            }
            prev_level = heading.level;
        }

        if !skip_found {
            results.push(CheckResult {
                rule_id: "14-006".to_string(),
                checkpoint: 14,
                description: "Heading hierarchy has no skipped levels".to_string(),
                severity: Severity::Info,
                outcome: CheckOutcome::Pass,
            });
        }

        // 14-003: Generic H headings must use nesting to convey hierarchy.
        // Two sibling generic H headings (same parent element) means the document
        // doesn't properly distinguish heading levels through nesting.
        if has_generic_h_sibling_violation {
            results.push(CheckResult {
                rule_id: "14-003".to_string(),
                checkpoint: 14,
                description: "Generic H headings at the same nesting level are ambiguous"
                    .to_string(),
                severity: Severity::Error,
                outcome: CheckOutcome::Fail {
                    message: "Multiple generic H headings found as siblings under the same parent element — use nesting (e.g., Sect > H) to distinguish hierarchy levels".to_string(),
                    location: None,
                },
            });
        } else {
            // Already checked — no violation
        }

        // 14-007: Check for mixing numbered (H1-H6) and generic (H) headings
        let has_numbered = headings.iter().any(|h| h.level > 0);
        let has_generic = headings.iter().any(|h| h.is_generic);
        if has_numbered && has_generic {
            results.push(CheckResult {
                rule_id: "14-007".to_string(),
                checkpoint: 14,
                description: "Document mixes numbered (H1-H6) and generic (H) headings".to_string(),
                severity: Severity::Error,
                outcome: CheckOutcome::Fail {
                    message: "Document must use either numbered headings (H1-H6) or generic headings (H), not both".to_string(),
                    location: None,
                },
            });
        }

        Ok(results)
    }
}

struct HeadingInfo {
    level: u8,
    is_generic: bool,
}

fn collect_headings(
    doc: &lopdf::Document,
    dict: &lopdf::Dictionary,
    headings: &mut Vec<HeadingInfo>,
    depth: usize,
) {
    if depth > 100 {
        return; // Prevent infinite recursion
    }

    // Check if this element is a heading
    if let Ok(s_type) = dict.get(b"S").and_then(|o| o.as_name()) {
        match s_type {
            b"H1" => headings.push(HeadingInfo {
                level: 1,
                is_generic: false,
            }),
            b"H2" => headings.push(HeadingInfo {
                level: 2,
                is_generic: false,
            }),
            b"H3" => headings.push(HeadingInfo {
                level: 3,
                is_generic: false,
            }),
            b"H4" => headings.push(HeadingInfo {
                level: 4,
                is_generic: false,
            }),
            b"H5" => headings.push(HeadingInfo {
                level: 5,
                is_generic: false,
            }),
            b"H6" => headings.push(HeadingInfo {
                level: 6,
                is_generic: false,
            }),
            b"H" => headings.push(HeadingInfo {
                level: 0,
                is_generic: true,
            }),
            _ => {}
        }
    }

    // Recurse into children
    let Ok(kids) = dict.get(b"K") else { return };

    match kids {
        lopdf::Object::Array(arr) => {
            for kid in arr {
                if let Ok(kid_ref) = kid.as_reference() {
                    if let Ok(kid_obj) = doc.get_object(kid_ref) {
                        if let Ok(kid_dict) = kid_obj.as_dict() {
                            collect_headings(doc, kid_dict, headings, depth + 1);
                        }
                    }
                } else if let Ok(kid_dict) = kid.as_dict() {
                    collect_headings(doc, kid_dict, headings, depth + 1);
                }
            }
        }
        lopdf::Object::Reference(ref_id) => {
            if let Ok(obj) = doc.get_object(*ref_id) {
                if let Ok(dict) = obj.as_dict() {
                    collect_headings(doc, dict, headings, depth + 1);
                }
            }
        }
        lopdf::Object::Dictionary(dict) => {
            collect_headings(doc, dict, headings, depth + 1);
        }
        _ => {}
    }
}

/// Check if any element in the tree has multiple generic H children (siblings).
/// This means the document uses generic headings but doesn't nest them to convey hierarchy.
fn check_generic_h_siblings(doc: &lopdf::Document, dict: &lopdf::Dictionary, depth: usize) -> bool {
    if depth > 100 {
        return false;
    }

    // Count generic H children of this element
    let Ok(kids) = dict.get(b"K") else {
        return false;
    };

    let mut h_count = 0;
    let mut found_in_children = false;

    let process_child = |child_dict: &lopdf::Dictionary, h_count: &mut usize, found: &mut bool| {
        if let Ok(s_type) = child_dict.get(b"S").and_then(|o| o.as_name()) {
            if s_type == b"H" {
                *h_count += 1;
            }
        }
        // Recurse
        if check_generic_h_siblings(doc, child_dict, depth + 1) {
            *found = true;
        }
    };

    match kids {
        lopdf::Object::Array(arr) => {
            for kid in arr {
                if let Ok(kid_ref) = kid.as_reference() {
                    if let Ok(kid_obj) = doc.get_object(kid_ref) {
                        if let Ok(kid_dict) = kid_obj.as_dict() {
                            process_child(kid_dict, &mut h_count, &mut found_in_children);
                        }
                    }
                } else if let Ok(kid_dict) = kid.as_dict() {
                    process_child(kid_dict, &mut h_count, &mut found_in_children);
                }
            }
        }
        lopdf::Object::Reference(ref_id) => {
            if let Ok(obj) = doc.get_object(*ref_id) {
                if let Ok(d) = obj.as_dict() {
                    process_child(d, &mut h_count, &mut found_in_children);
                }
            }
        }
        lopdf::Object::Dictionary(d) => {
            process_child(d, &mut h_count, &mut found_in_children);
        }
        _ => {}
    }

    h_count >= 2 || found_in_children
}

fn get_struct_tree<'a>(
    catalog: &'a lopdf::Dictionary,
    doc: &'a lopdf::Document,
) -> Option<&'a lopdf::Dictionary> {
    let ref_id = catalog.get(b"StructTreeRoot").ok()?.as_reference().ok()?;
    doc.get_object(ref_id).ok()?.as_dict().ok()
}
