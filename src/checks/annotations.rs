use crate::checks::Check;
use crate::document::HornDocument;
use crate::model::{CheckOutcome, CheckResult, Location, Severity};
use anyhow::Result;

/// Checkpoint 28: page-level annotation checks.
///
/// - 28-008 / 28-009: pages with annotations must have `/Tabs /S`
/// - 28-x01 (Horn extension): link annotations must have an action or destination
///
/// Annotation-to-structure-tree rules (28-002, 28-004, 28-005, 28-010 … 28-017)
/// live in `annot_struct.rs`.
pub struct AnnotationChecks;

impl Check for AnnotationChecks {
    fn id(&self) -> &'static str {
        "28-annotations"
    }

    fn checkpoint(&self) -> u8 {
        28
    }

    fn rules(&self) -> &'static [&'static str] {
        &["28-008", "28-009", "28-x01"]
    }

    fn description(&self) -> &'static str {
        "Annotations: tab order, link destinations"
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

/// 28-008 / 28-009: Tab order must be set to /S (structure order) on pages with annotations.
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
            let tabs = obj.as_name().unwrap_or(b"");
            {
                if tabs == b"S" {
                    results.push(CheckResult {
                        rule_id: "28-009".to_string(),
                        checkpoint: 28,
                        description: format!("Page {page_num}: Tab order is /S (structure)"),
                        severity: Severity::Info,
                        outcome: CheckOutcome::Pass,
                    });
                } else {
                    let tab_val = String::from_utf8_lossy(tabs);
                    results.push(CheckResult {
                        rule_id: "28-009".to_string(),
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
                rule_id: "28-008".to_string(),
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

/// 28-x01: Check individual annotations on a page.
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

        if let Some(b"Link") = subtype.as_deref() {
            check_link_annotation(doc, annot, &annot_label, page_num, results);
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
            rule_id: "28-x01".to_string(),
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
}
