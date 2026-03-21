use crate::checks::Check;
use crate::document::HornDocument;
use crate::model::{CheckOutcome, CheckResult, Severity};
use anyhow::Result;
use lopdf::content::Content;

/// Content stream analysis checks.
///
/// Parses PDF page content streams to detect:
/// - 01-001: Content not wrapped in marked content sequences (untagged text/images)
/// - 01-005: Artifact content nested inside tagged content
/// - 30-001: Form `XObjects` not properly tagged
pub struct ContentStreamChecks;

impl Check for ContentStreamChecks {
    fn id(&self) -> &'static str {
        "cs-content-stream"
    }

    fn checkpoint(&self) -> u8 {
        1
    }

    fn description(&self) -> &'static str {
        "Content stream: untagged content, artifact nesting, XObject tagging"
    }

    fn run(&self, doc: &mut HornDocument) -> Result<Vec<CheckResult>> {
        let mut results = Vec::new();
        let lopdf_doc = doc.lopdf();
        let pages = lopdf_doc.get_pages();

        let mut total_text_ops = 0u32;
        let mut untagged_text_ops = 0u32;
        let mut untagged_xobject_ops = 0u32;
        let mut artifact_in_tagged = 0u32;
        let mut notdef_usage = 0u32;
        let mut pages_analyzed = 0u32;

        for (page_num, page_id) in &pages {
            let Ok(content_data) = lopdf_doc.get_page_content(*page_id) else {
                continue;
            };

            let Ok(content) = Content::decode(&content_data) else {
                continue;
            };

            pages_analyzed += 1;
            let page_result = analyze_page_content(&content.operations, *page_num);

            total_text_ops += page_result.total_text_ops;
            untagged_text_ops += page_result.untagged_text_ops;
            untagged_xobject_ops += page_result.untagged_xobject_ops;
            artifact_in_tagged += page_result.artifact_inside_tagged;
            notdef_usage += page_result.notdef_glyph_usage;
        }

        // 01-001: Untagged content detection
        if total_text_ops > 0 {
            if untagged_text_ops > 0 {
                results.push(CheckResult {
                    rule_id: "01-001".to_string(),
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
                    rule_id: "01-001".to_string(),
                    checkpoint: 1,
                    description: "All text content is inside marked content sequences".to_string(),
                    severity: Severity::Info,
                    outcome: CheckOutcome::Pass,
                });
            }
        }

        // 01-002: Untagged XObject (image/form) invocations
        if untagged_xobject_ops > 0 {
            results.push(CheckResult {
                rule_id: "01-002".to_string(),
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

        // 31-025: .notdef glyph usage (CID 0 / \x00\x00 in text strings)
        if notdef_usage > 0 {
            results.push(CheckResult {
                rule_id: "31-025".to_string(),
                checkpoint: 31,
                description: format!(
                    "{notdef_usage} text operation(s) reference the .notdef glyph (CID 0)"
                ),
                severity: Severity::Error,
                outcome: CheckOutcome::Fail {
                    message: format!(
                        "{notdef_usage} text operation(s) use CID 0 (.notdef glyph) — all glyphs must map to valid characters"
                    ),
                    location: None,
                },
            });
        }

        // 01-005: Artifact content inside tagged content
        if artifact_in_tagged > 0 {
            results.push(CheckResult {
                rule_id: "01-005".to_string(),
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
    notdef_glyph_usage: u32,
}

/// Analyze a page's content stream operations for marked content coverage.
fn analyze_page_content(ops: &[lopdf::content::Operation], _page_num: u32) -> PageAnalysis {
    let mut result = PageAnalysis {
        total_text_ops: 0,
        untagged_text_ops: 0,
        untagged_xobject_ops: 0,
        artifact_inside_tagged: 0,
        notdef_glyph_usage: 0,
    };

    // Track marked content nesting.
    // mc_stack entries: true = MCID-bearing tagged sequence, false = other
    let mut mc_stack: Vec<bool> = Vec::new();
    for op in ops {
        match op.operator.as_str() {
            "BMC" => {
                // BMC has no properties dict, so no MCID — just push as non-tagged
                mc_stack.push(false);
            }
            "BDC" => {
                let tag = op
                    .operands
                    .first()
                    .and_then(|o| o.as_name().ok())
                    .unwrap_or(b"");

                let is_artifact = tag == b"Artifact";

                // Check if BDC has an MCID (real structure element content)
                let has_mcid = op.operands.get(1).is_some_and(|prop| {
                    if let Ok(dict) = prop.as_dict() {
                        dict.get(b"MCID").is_ok()
                    } else {
                        false
                    }
                });

                // Only flag artifact-in-tagged when nested inside MCID-bearing content
                if is_artifact && mc_stack.iter().any(|m| *m) {
                    result.artifact_inside_tagged += 1;
                }

                mc_stack.push(has_mcid && !is_artifact);
            }
            "EMC" => {
                mc_stack.pop();
            }

            // Text showing operators
            "Tj" | "TJ" | "'" | "\"" => {
                result.total_text_ops += 1;
                if mc_stack.is_empty() {
                    result.untagged_text_ops += 1;
                }
                // Check for .notdef glyph (CID 0 = \x00\x00) in string operands
                for operand in &op.operands {
                    if has_notdef_glyph(operand) {
                        result.notdef_glyph_usage += 1;
                    }
                }
            }

            // XObject invocation — only flag image XObjects outside marked content.
            // Form XObjects (/Fm*) contain their own content stream with their own
            // marked content structure, so they don't need to be inside page-level
            // BDC/EMC. Image XObjects (/Im*) are leaf content and must be tagged.
            "Do" => {
                if mc_stack.is_empty() {
                    // Check XObject name — /Im prefix indicates image
                    let is_image = op
                        .operands
                        .first()
                        .and_then(|o| o.as_name().ok())
                        .is_some_and(|name| name.starts_with(b"Im"));
                    if is_image {
                        result.untagged_xobject_ops += 1;
                    }
                }
            }

            _ => {}
        }
    }

    result
}

/// Check if a text operand contains the .notdef glyph (CID 0 = `\x00\x00`).
///
/// In CID fonts, CID 0 is always the .notdef glyph. A 2-byte string starting
/// with `\x00\x00` indicates .notdef usage. For TJ arrays, check each string element.
fn has_notdef_glyph(operand: &lopdf::Object) -> bool {
    match operand {
        lopdf::Object::String(bytes, _) => {
            // Check for \x00\x00 (CID 0) in 2-byte aligned positions
            let data = bytes.as_slice();
            if data.len() >= 2 {
                let mut i = 0;
                while i + 1 < data.len() {
                    if data[i] == 0 && data[i + 1] == 0 {
                        return true;
                    }
                    i += 2;
                }
            }
            false
        }
        lopdf::Object::Array(arr) => {
            // TJ array: mix of strings and numbers
            arr.iter().any(|item| {
                if let lopdf::Object::String(bytes, _) = item {
                    let data = bytes.as_slice();
                    if data.len() >= 2 {
                        let mut i = 0;
                        while i + 1 < data.len() {
                            if data[i] == 0 && data[i + 1] == 0 {
                                return true;
                            }
                            i += 2;
                        }
                    }
                }
                false
            })
        }
        _ => false,
    }
}

/// 30-002: Check for Reference `XObjects` which are forbidden in PDF/UA.
#[allow(dead_code)]
fn check_reference_xobjects(doc: &lopdf::Document, results: &mut Vec<CheckResult>) {
    for obj in doc.objects.values() {
        let Ok(stream) = obj.as_stream() else {
            continue;
        };

        let dict = &stream.dict;

        let is_form = dict.get(b"Subtype").ok().and_then(|o| o.as_name().ok()) == Some(b"Form");

        if is_form && dict.get(b"Ref").is_ok() {
            results.push(CheckResult {
                rule_id: "30-002".to_string(),
                checkpoint: 30,
                description: "Reference XObject found — forbidden in PDF/UA".to_string(),
                severity: Severity::Error,
                outcome: CheckOutcome::Fail {
                    message:
                        "Form XObject with /Ref key (Reference XObject) is not allowed in PDF/UA"
                            .to_string(),
                    location: None,
                },
            });
        }
    }
}
