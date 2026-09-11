use crate::checks::Check;
use crate::document::HornDocument;
use crate::model::{CheckOutcome, CheckResult, Severity};
use anyhow::Result;

/// Checkpoint 25: XFA forms.
///
/// PDF/UA-1 clause 7.15 forbids *dynamic* XFA forms: the XFA `config` packet
/// must not contain a `dynamicRender` element with the value `required`
/// (Matterhorn 25-001). Static XFA (`dynamicRender` absent or `forbidden`) is
/// permitted, although the AcroForm fields still have to satisfy checkpoint 28.
pub struct XfaCheck;

impl Check for XfaCheck {
    fn id(&self) -> &'static str {
        "25-xfa"
    }

    fn checkpoint(&self) -> u8 {
        25
    }

    fn description(&self) -> &'static str {
        "XFA: dynamicRender must not be required"
    }

    fn run(&self, doc: &mut HornDocument) -> Result<Vec<CheckResult>> {
        let mut results = Vec::new();
        check_xfa_presence(doc, &mut results);
        Ok(results)
    }
}

/// 25-001: Document must not contain XFA form data.
///
/// XFA (XML Forms Architecture) is an Adobe proprietary format that is not
/// accessible to assistive technologies. PDF/UA requires `AcroForm` instead.
fn check_xfa_presence(doc: &mut HornDocument, results: &mut Vec<CheckResult>) {
    let Ok(catalog) = doc.raw_catalog() else {
        // No catalog — can't have XFA either; skip silently
        return;
    };

    let lopdf_doc = doc.lopdf();

    // Check for /AcroForm in catalog
    let acro_form = if let Ok(obj) = catalog.get_deref(b"AcroForm", lopdf_doc) {
        if let Ok(dict) = obj.as_dict() {
            dict
        } else {
            // AcroForm exists but isn't a dictionary — no XFA possible
            results.push(pass(
                "25-001",
                "No XFA form data (AcroForm is not a dictionary)",
            ));
            return;
        }
    } else {
        // No AcroForm at all — no XFA possible
        results.push(pass("25-001", "No XFA form data (no AcroForm present)"));
        return;
    };

    // Check for /XFA key within AcroForm
    let Ok(xfa_obj) = acro_form.get(b"XFA") else {
        results.push(pass("25-001", "No XFA form data in AcroForm"));
        return;
    };

    let xml = collect_xfa_xml(lopdf_doc, xfa_obj);
    if xfa_requires_dynamic_render(&xml) {
        results.push(fail(
            "25-001",
            "XFA config packet sets <dynamicRender>required</dynamicRender> — dynamic XFA forms are not permitted in PDF/UA",
        ));
    } else {
        results.push(CheckResult {
            rule_id: "25-001".to_string(),
            checkpoint: 25,
            description: "XFA form data present but dynamicRender is not required (static XFA)"
                .to_string(),
            severity: Severity::Warning,
            outcome: CheckOutcome::NeedsReview {
                reason: "Document contains static XFA form data; verify the AcroForm representation is complete and accessible".to_string(),
            },
        });
    }
}

/// Concatenate the XML of an /XFA entry, which is either a single stream or an
/// array of `(packet name, stream)` pairs (ISO 32000-1, 12.7.8).
fn collect_xfa_xml(doc: &lopdf::Document, xfa: &lopdf::Object) -> String {
    let mut out = String::new();
    let mut append_stream = |obj: &lopdf::Object| {
        let resolved = match obj {
            lopdf::Object::Reference(r) => doc.get_object(*r).ok(),
            other => Some(other),
        };
        if let Some(stream) = resolved.and_then(|o| o.as_stream().ok()) {
            if let Ok(data) = stream.decompressed_content() {
                out.push_str(&String::from_utf8_lossy(&data));
            }
        }
    };
    match xfa {
        lopdf::Object::Array(items) => {
            for item in items {
                append_stream(item);
            }
        }
        other => append_stream(other),
    }
    out
}

/// True if the XFA XML contains `<dynamicRender>required</dynamicRender>`
/// (whitespace-insensitive, ignoring a namespace prefix on the element name).
fn xfa_requires_dynamic_render(xml: &str) -> bool {
    let lower = xml.to_ascii_lowercase();
    let mut pos = 0;
    while let Some(idx) = lower[pos..].find("dynamicrender") {
        let start = pos + idx;
        let Some(gt) = lower[start..].find('>') else { break };
        let value_start = start + gt + 1;
        let Some(lt) = lower[value_start..].find('<') else { break };
        let value = lower[value_start..value_start + lt].trim();
        if value == "required" {
            return true;
        }
        pos = value_start + lt;
    }
    false
}

fn pass(rule_id: &str, description: &str) -> CheckResult {
    CheckResult {
        rule_id: rule_id.to_string(),
        checkpoint: 25,
        description: description.to_string(),
        severity: Severity::Info,
        outcome: CheckOutcome::Pass,
    }
}

fn fail(rule_id: &str, message: &str) -> CheckResult {
    CheckResult {
        rule_id: rule_id.to_string(),
        checkpoint: 25,
        description: message.to_string(),
        severity: Severity::Error,
        outcome: CheckOutcome::Fail {
            message: message.to_string(),
            location: None,
        },
    }
}
