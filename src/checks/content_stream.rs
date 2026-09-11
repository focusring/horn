use crate::checks::Check;
use crate::document::HornDocument;
use crate::model::{CheckOutcome, CheckResult, Severity};
use anyhow::Result;
use lopdf::content::Content;

/// Content stream analysis checks.
///
/// Parses PDF page content streams to detect:
/// - 01-003: Artifact content nested inside tagged content
/// - 01-004: Tagged content nested inside Artifact content
/// - 01-005: Content not wrapped in marked content sequences (untagged text/images)
pub struct ContentStreamChecks;

impl Check for ContentStreamChecks {
    fn id(&self) -> &'static str {
        "cs-content-stream"
    }

    fn checkpoint(&self) -> u8 {
        1
    }

    fn rules(&self) -> &'static [&'static str] {
        &["01-003", "01-004", "01-005"]
    }

    fn description(&self) -> &'static str {
        "Content stream: untagged content, artifact nesting"
    }

    #[allow(clippy::too_many_lines)]
    fn run(&self, doc: &mut HornDocument) -> Result<Vec<CheckResult>> {
        let mut results = Vec::new();
        let lopdf_doc = doc.lopdf();
        let pages = lopdf_doc.get_pages();

        let mut total_text_ops = 0u32;
        let mut untagged_text_ops = 0u32;
        let mut untagged_xobject_ops = 0u32;
        let mut artifact_in_tagged = 0u32;
        let mut tagged_in_artifact = 0u32;
        let mut pages_analyzed = 0u32;

        for page_id in pages.values() {
            let content_data = lopdf_doc.get_page_content(*page_id);
            if content_data.is_empty() {
                continue;
            }

            let Ok(content) = Content::decode(&content_data) else {
                continue;
            };

            pages_analyzed += 1;
            let resources = crate::content::page_resources(lopdf_doc, *page_id);
            let properties = properties_of(lopdf_doc, resources);
            let page_result = analyze_page_content(lopdf_doc, &content.operations, properties);

            total_text_ops += page_result.total_text_ops;
            untagged_text_ops += page_result.untagged_text_ops;
            untagged_xobject_ops += page_result.untagged_xobject_ops;
            artifact_in_tagged += page_result.artifact_inside_tagged;
            tagged_in_artifact += page_result.tagged_inside_artifact;
        }

        // 01-005: Untagged content detection
        if total_text_ops > 0 {
            if untagged_text_ops > 0 {
                results.push(CheckResult {
                    rule_id: "01-005".to_string(),
                    checkpoint: 1,
                    description: format!(
                        "{untagged_text_ops} of {total_text_ops} text operation(s) are outside marked content"
                    ),
                    severity: Severity::Error,
                    outcome: CheckOutcome::Fail {
                        message: format!(
                            "{untagged_text_ops} text operation(s) across {pages_analyzed} page(s) are not inside BMC/BDC..EMC marked content sequences"
                        ),
                        location: None,
                    },
                });
            } else {
                results.push(CheckResult {
                    rule_id: "01-005".to_string(),
                    checkpoint: 1,
                    description: "All text content is inside marked content sequences".to_string(),
                    severity: Severity::Info,
                    outcome: CheckOutcome::Pass,
                });
            }
        }

        // 01-005: Untagged XObject (image/form) invocations
        if untagged_xobject_ops > 0 {
            results.push(CheckResult {
                rule_id: "01-005".to_string(),
                checkpoint: 1,
                description: format!(
                    "{untagged_xobject_ops} XObject invocation(s) are outside marked content"
                ),
                severity: Severity::Error,
                outcome: CheckOutcome::Fail {
                    message: format!(
                        "{untagged_xobject_ops} XObject (Do) operation(s) are not inside BMC/BDC..EMC marked content — images and form XObjects must be tagged or marked as artifacts"
                    ),
                    location: None,
                },
            });
        }

        // 01-004: Tagged content inside Artifact content
        if tagged_in_artifact > 0 {
            results.push(CheckResult {
                rule_id: "01-004".to_string(),
                checkpoint: 1,
                description: format!(
                    "{tagged_in_artifact} tagged marked-content sequence(s) found nested inside Artifact content"
                ),
                severity: Severity::Error,
                outcome: CheckOutcome::Fail {
                    message: format!(
                        "{tagged_in_artifact} BDC sequence(s) with an MCID are nested inside /Artifact marked content — real content must not be inside artifacts"
                    ),
                    location: None,
                },
            });
        }

        // 01-003: Artifact content inside tagged content
        if artifact_in_tagged > 0 {
            results.push(CheckResult {
                rule_id: "01-003".to_string(),
                checkpoint: 1,
                description: format!(
                    "{artifact_in_tagged} Artifact marker(s) found nested inside tagged content"
                ),
                severity: Severity::Error,
                outcome: CheckOutcome::Fail {
                    message: format!(
                        "{artifact_in_tagged} /Artifact BMC/BDC found inside non-Artifact marked content — artifacts must not be nested in tagged content"
                    ),
                    location: None,
                },
            });
        }

        Ok(results)
    }
}

struct PageAnalysis {
    total_text_ops: u32,
    untagged_text_ops: u32,
    untagged_xobject_ops: u32,
    artifact_inside_tagged: u32,
    tagged_inside_artifact: u32,
}

/// Effective state of an open marked-content sequence.
#[derive(Clone, Copy, PartialEq, Eq)]
enum McKind {
    /// `BDC` with an `/MCID` — real content that belongs to the structure tree.
    Tagged,
    /// `/Artifact BMC` or `/Artifact BDC`.
    Artifact,
    /// Any other sequence (e.g. `/Span BMC`, `/Span <</Lang ..>> BDC`): it does
    /// not by itself associate content with the structure tree.
    Other,
}

/// Analyze a page's content stream operations for marked content coverage.
///
/// `properties` is the page's `/Resources/Properties` dictionary, used to
/// resolve `BDC` operands given as a name (`/P /MC0 BDC`).
fn analyze_page_content(
    doc: &lopdf::Document,
    ops: &[lopdf::content::Operation],
    properties: Option<&lopdf::Dictionary>,
) -> PageAnalysis {
    let mut result = PageAnalysis {
        total_text_ops: 0,
        untagged_text_ops: 0,
        untagged_xobject_ops: 0,
        artifact_inside_tagged: 0,
        tagged_inside_artifact: 0,
    };

    let mut mc_stack: Vec<McKind> = Vec::new();
    let inside = |stack: &[McKind], kind: McKind| stack.contains(&kind);

    for op in ops {
        match op.operator.as_str() {
            "BMC" => {
                let tag = op.operands.first().and_then(|o| o.as_name().ok());
                mc_stack.push(if tag == Some(b"Artifact") {
                    McKind::Artifact
                } else {
                    McKind::Other
                });
            }
            "BDC" => {
                let tag = op
                    .operands
                    .first()
                    .and_then(|o| o.as_name().ok())
                    .unwrap_or(b"");
                let is_artifact = tag == b"Artifact";
                let has_mcid = bdc_has_mcid(doc, op.operands.get(1), properties);

                // 01-003: artifact nested inside MCID-bearing content
                if is_artifact && inside(&mc_stack, McKind::Tagged) {
                    result.artifact_inside_tagged += 1;
                }
                // 01-004: tagged (MCID-bearing) content nested inside an Artifact
                if has_mcid && !is_artifact && inside(&mc_stack, McKind::Artifact) {
                    result.tagged_inside_artifact += 1;
                }

                mc_stack.push(if is_artifact {
                    McKind::Artifact
                } else if has_mcid {
                    McKind::Tagged
                } else {
                    McKind::Other
                });
            }
            "EMC" => {
                mc_stack.pop();
            }

            // Text showing operators: real content must be tagged or an artifact
            "Tj" | "TJ" | "'" | "\"" => {
                result.total_text_ops += 1;
                if !inside(&mc_stack, McKind::Tagged) && !inside(&mc_stack, McKind::Artifact) {
                    result.untagged_text_ops += 1;
                }
            }

            // XObject invocation — only flag image XObjects outside marked content.
            // Form XObjects contain their own content stream with their own marked
            // content structure, so they don't need to be inside page-level BDC/EMC.
            // Image XObjects are leaf content and must be tagged or artifacts.
            "Do" if !inside(&mc_stack, McKind::Tagged) && !inside(&mc_stack, McKind::Artifact) => {
                let is_image = op
                    .operands
                    .first()
                    .and_then(|o| o.as_name().ok())
                    .is_some_and(|name| name.starts_with(b"Im"));
                if is_image {
                    result.untagged_xobject_ops += 1;
                }
            }

            _ => {}
        }
    }

    result
}

/// Whether a `BDC` property operand (inline dictionary or a name resolved
/// through `/Resources/Properties`) carries an `/MCID`.
pub fn bdc_has_mcid(
    doc: &lopdf::Document,
    operand: Option<&lopdf::Object>,
    properties: Option<&lopdf::Dictionary>,
) -> bool {
    let Some(operand) = operand else {
        return false;
    };
    let dict = match operand {
        lopdf::Object::Dictionary(d) => Some(d),
        lopdf::Object::Name(name) => properties
            .and_then(|p| p.get(name).ok())
            .and_then(|o| match o {
                lopdf::Object::Reference(id) => doc.get_object(*id).ok(),
                other => Some(other),
            })
            .and_then(|o| o.as_dict().ok()),
        _ => None,
    };
    dict.is_some_and(|d| d.get(b"MCID").is_ok())
}

/// The `/Properties` sub-dictionary of a resources dictionary.
pub fn properties_of<'a>(
    doc: &'a lopdf::Document,
    resources: Option<&'a lopdf::Dictionary>,
) -> Option<&'a lopdf::Dictionary> {
    resources?
        .get_deref(b"Properties", doc)
        .ok()
        .and_then(|o| o.as_dict().ok())
}
