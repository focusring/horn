use crate::checks::Check;
use crate::document::HornDocument;
use crate::model::{CheckOutcome, CheckResult, Location, Severity};
use anyhow::Result;

/// Checkpoint 28: Annotation checks.
///
/// Validates that annotations are tagged, links have destinations,
/// and form fields are accessible.
pub struct AnnotationChecks;

impl Check for AnnotationChecks {
    fn id(&self) -> &'static str {
        "28-annotations"
    }

    fn checkpoint(&self) -> u8 {
        28
    }

    fn description(&self) -> &'static str {
        "Annotations: link structure, form fields, tab order"
    }

    fn run(&self, doc: &mut HornDocument) -> Result<Vec<CheckResult>> {
        let mut results = Vec::new();
        let lopdf_doc = doc.lopdf();
        let pages = lopdf_doc.get_pages();

        for (page_num, page_id) in &pages {
            check_tab_order(lopdf_doc, *page_id, *page_num, &mut results);
            check_annotations_on_page(lopdf_doc, *page_id, *page_num, &mut results);
        }

        Ok(results)
    }
}

/// 28-001: Tab order must be set to /S (structure order) on pages with annotations.
fn check_tab_order(
    doc: &lopdf::Document,
    page_id: lopdf::ObjectId,
    page_num: u32,
    results: &mut Vec<CheckResult>,
) {
    let Ok(page) = doc.get_dictionary(page_id) else {
        return;
    };

    // Only check if the page has annotations
    let has_annots = page.get(b"Annots").is_ok();
    if !has_annots {
        return;
    }

    match page.get(b"Tabs") {
        Ok(obj) => {
            if let Ok(tabs) = obj.as_name() {
                if tabs == b"S" {
                    results.push(CheckResult {
                        rule_id: "28-001".to_string(),
                        checkpoint: 28,
                        description: format!("Page {page_num}: Tab order is /S (structure)"),
                        severity: Severity::Info,
                        outcome: CheckOutcome::Pass,
                    });
                } else {
                    let tab_val = String::from_utf8_lossy(tabs);
                    results.push(CheckResult {
                        rule_id: "28-001".to_string(),
                        checkpoint: 28,
                        description: format!(
                            "Page {page_num}: Tab order is /{tab_val}, should be /S"
                        ),
                        severity: Severity::Error,
                        outcome: CheckOutcome::Fail {
                            message: format!(
                                "Page {page_num}: Tab order is /{tab_val} — must be /S (structure order) for PDF/UA"
                            ),
                            location: Some(Location {
                                page: Some(page_num),
                                element: None,
                            }),
                        },
                    });
                }
            }
        }
        Err(_) => {
            results.push(CheckResult {
                rule_id: "28-001".to_string(),
                checkpoint: 28,
                description: format!("Page {page_num}: Missing /Tabs entry"),
                severity: Severity::Error,
                outcome: CheckOutcome::Fail {
                    message: format!(
                        "Page {page_num} has annotations but no /Tabs entry — must be set to /S"
                    ),
                    location: Some(Location {
                        page: Some(page_num),
                        element: None,
                    }),
                },
            });
        }
    }
}

/// 28-004/28-006: Check individual annotations on a page.
fn check_annotations_on_page(
    doc: &lopdf::Document,
    page_id: lopdf::ObjectId,
    page_num: u32,
    results: &mut Vec<CheckResult>,
) {
    let Ok(annots) = doc.get_page_annotations(page_id) else {
        return;
    };

    for (i, annot) in annots.iter().enumerate() {
        let annot_label = format!("Page {page_num}, annotation {}", i + 1);

        let subtype = annot
            .get_deref(b"Subtype", doc)
            .ok()
            .and_then(|o| o.as_name().ok())
            .map(<[u8]>::to_vec);

        match subtype.as_deref() {
            Some(b"Link") => {
                check_link_annotation(doc, annot, &annot_label, page_num, results);
            }
            Some(b"Widget" | b"Form") => {
                check_widget_annotation(doc, annot, &annot_label, page_num, results);
            }
            _ => {
                // Other annotations should have /Contents for accessibility
                check_annotation_contents(annot, &annot_label, page_num, results);
            }
        }
    }
}

/// Check that Link annotations have a valid destination or action.
fn check_link_annotation(
    _doc: &lopdf::Document,
    annot: &lopdf::Dictionary,
    label: &str,
    page_num: u32,
    results: &mut Vec<CheckResult>,
) {
    let has_action = annot.get(b"A").is_ok();
    let has_dest = annot.get(b"Dest").is_ok();

    if !has_action && !has_dest {
        results.push(CheckResult {
            rule_id: "28-004".to_string(),
            checkpoint: 28,
            description: format!("{label}: Link has no destination or action"),
            severity: Severity::Error,
            outcome: CheckOutcome::Fail {
                message: format!(
                    "{label}: Link annotation has neither /A (action) nor /Dest (destination)"
                ),
                location: Some(Location {
                    page: Some(page_num),
                    element: Some("Link".to_string()),
                }),
            },
        });
    }

    // Check for /Contents on the link annotation.
    // Links commonly get their accessible text from the Link structure
    // element in the tag tree rather than /Contents on the annotation,
    // so a missing /Contents is only a review flag, not a hard fail.
    let has_contents = annot.get(b"Contents").is_ok();
    if !has_contents {
        results.push(CheckResult {
            rule_id: "28-006".to_string(),
            checkpoint: 28,
            description: format!("{label}: Link has no /Contents (may be provided via structure tree)"),
            severity: Severity::Info,
            outcome: CheckOutcome::NeedsReview {
                reason: format!(
                    "{label}: Link annotation has no /Contents — verify link text is provided via the Link structure element"
                ),
            },
        });
    }
}

/// Check that Widget (form field) annotations are accessible.
///
/// In PDF forms, /T (field name) and /TU (tooltip) may be on the widget
/// annotation itself, or inherited from a parent field dictionary via /Parent.
/// We walk up the /Parent chain to find these entries.
fn check_widget_annotation(
    doc: &lopdf::Document,
    annot: &lopdf::Dictionary,
    label: &str,
    page_num: u32,
    results: &mut Vec<CheckResult>,
) {
    // 28-009: Form fields must have /TU (tooltip/alternative text)
    // Check the annotation itself and walk up /Parent chain for inheritance
    let has_tu = has_inherited_key(doc, annot, b"TU", 10);
    let has_t = has_inherited_key(doc, annot, b"T", 10);

    if !has_tu && !has_t {
        results.push(CheckResult {
            rule_id: "28-009".to_string(),
            checkpoint: 28,
            description: format!("{label}: Widget has no /T or /TU (tooltip)"),
            severity: Severity::Error,
            outcome: CheckOutcome::Fail {
                message: format!(
                    "{label}: Form field must have /T (field name) or /TU (tooltip) for accessibility"
                ),
                location: Some(Location {
                    page: Some(page_num),
                    element: Some("Widget".to_string()),
                }),
            },
        });
    }
}

/// Check if a dictionary key exists on the given dict or any ancestor via /Parent.
///
/// PDF form fields use /Parent to build a hierarchy. Properties like /T (field name)
/// and /TU (tooltip) can be inherited from parent field dictionaries.
fn has_inherited_key(
    doc: &lopdf::Document,
    dict: &lopdf::Dictionary,
    key: &[u8],
    max_depth: u8,
) -> bool {
    if dict.get(key).is_ok() {
        return true;
    }
    if max_depth == 0 {
        return false;
    }
    // Walk up /Parent chain
    if let Ok(parent_obj) = dict.get(b"Parent") {
        let parent_dict = match parent_obj {
            lopdf::Object::Reference(ref_id) => {
                doc.get_object(*ref_id).ok().and_then(|o| o.as_dict().ok())
            }
            lopdf::Object::Dictionary(d) => Some(d),
            _ => None,
        };
        if let Some(parent) = parent_dict {
            return has_inherited_key(doc, parent, key, max_depth - 1);
        }
    }
    false
}

/// Generic check for annotation /Contents.
fn check_annotation_contents(
    annot: &lopdf::Dictionary,
    label: &str,
    _page_num: u32,
    results: &mut Vec<CheckResult>,
) {
    // Skip annotations that don't require /Contents:
    // - Popup: inherits from parent
    // - PrinterMark: not user-visible
    // - Markup annotations: reference underlying text, /Contents optional
    // - TrapNet, Caret, FreeText: special types
    let subtype = annot
        .get(b"Subtype")
        .ok()
        .and_then(|o| o.as_name().ok())
        .unwrap_or(b"");

    if matches!(
        subtype,
        b"Popup"
            | b"PrinterMark"
            | b"Caret"
            | b"TrapNet"
            | b"Highlight"
            | b"Underline"
            | b"Squiggly"
            | b"StrikeOut"
            | b"Redact"
            | b"FreeText"
    ) {
        return;
    }

    // Check /F (flags) for hidden annotation (bit 2)
    if let Ok(flags) = annot.get(b"F").and_then(lopdf::Object::as_i64) {
        if flags & 0x02 != 0 {
            return; // Hidden annotation — no contents needed
        }
    }

    if annot.get(b"Contents").is_err() {
        // When an annotation is tagged in the structure tree, its accessible text
        // comes from the structure element — /Contents is a fallback, not required.
        // Use NeedsReview instead of Fail to avoid false positives.
        results.push(CheckResult {
            rule_id: "28-006".to_string(),
            checkpoint: 28,
            description: format!("{label}: Annotation missing /Contents"),
            severity: Severity::Warning,
            outcome: CheckOutcome::NeedsReview {
                reason: format!(
                    "{label}: Annotation of type /{} has no /Contents — verify accessible text is provided via the structure tree",
                    String::from_utf8_lossy(subtype)
                ),
            },
        });
    }
}
