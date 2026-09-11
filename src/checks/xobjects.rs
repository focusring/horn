use crate::checks::Check;
use crate::document::HornDocument;
use crate::model::{CheckOutcome, CheckResult, Severity};
use anyhow::Result;
use lopdf::content::Content;

/// Checkpoint 30: `XObjects`.
///
/// - 30-002: a Form `XObject` whose content carries MCIDs (i.e. is part of the
///   logical structure) must not be painted more than once, otherwise the same
///   structure content would appear in several places.
///
/// 30-001 (reference `XObjects`) is checked in `dict_entries.rs`.
pub struct XObjectChecks;

impl Check for XObjectChecks {
    fn id(&self) -> &'static str {
        "30-xobjects"
    }

    fn checkpoint(&self) -> u8 {
        30
    }

    fn rules(&self) -> &'static [&'static str] {
        &["30-002"]
    }

    fn description(&self) -> &'static str {
        "XObjects: structured Form XObjects painted only once"
    }

    fn run(&self, doc: &mut HornDocument) -> Result<Vec<CheckResult>> {
        let mut results = Vec::new();
        let usage = doc.content_usage();
        let lopdf = doc.lopdf();

        let mut multi: Vec<(lopdf::ObjectId, u32)> = usage
            .form_xobject_uses
            .iter()
            .filter(|(_, count)| **count > 1)
            .map(|(id, count)| (*id, *count))
            .collect();
        multi.sort_unstable();

        let mut structured_multi = 0usize;
        for (id, count) in multi {
            let Ok(obj) = lopdf.get_object(id) else {
                continue;
            };
            let Ok(stream) = obj.as_stream() else {
                continue;
            };
            let Ok(data) = stream.decompressed_content() else {
                continue;
            };
            let resources = stream
                .dict
                .get_deref(b"Resources", lopdf)
                .ok()
                .and_then(|o| o.as_dict().ok());
            let properties = crate::checks::content_stream::properties_of(lopdf, resources);
            if !content_has_mcid(lopdf, &data, properties) {
                continue;
            }
            structured_multi += 1;
            results.push(CheckResult {
                rule_id: "30-002".to_string(),
                checkpoint: 30,
                description: format!(
                    "Form XObject {}.{} contains MCIDs but is painted {count} times",
                    id.0, id.1
                ),
                severity: Severity::Error,
                outcome: CheckOutcome::Fail {
                    message: format!(
                        "Form XObject {}.{} contains marked content with MCIDs and is referenced {count} times — structured content must be painted only once",
                        id.0, id.1
                    ),
                    location: None,
                },
            });
        }

        if structured_multi == 0 && !usage.form_xobject_uses.is_empty() {
            results.push(CheckResult {
                rule_id: "30-002".to_string(),
                checkpoint: 30,
                description: "No structured Form XObject is painted more than once".to_string(),
                severity: Severity::Info,
                outcome: CheckOutcome::Pass,
            });
        }

        Ok(results)
    }
}

/// True if a content stream contains a `BDC` with an `/MCID` property, given
/// either inline or through a named property list in `properties`.
pub fn content_has_mcid(
    doc: &lopdf::Document,
    data: &[u8],
    properties: Option<&lopdf::Dictionary>,
) -> bool {
    let Ok(content) = Content::decode(data) else {
        return false;
    };
    content.operations.iter().any(|op| {
        op.operator == "BDC"
            && crate::checks::content_stream::bdc_has_mcid(doc, op.operands.get(1), properties)
    })
}
