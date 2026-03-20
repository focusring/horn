use crate::checks::Check;
use crate::document::HornDocument;
use crate::model::{CheckOutcome, CheckResult, Severity};
use anyhow::Result;

/// Checkpoint 25: XFA forms.
///
/// PDF/UA-1 forbids XFA form data. Documents must use AcroForm-only interactive forms.
/// If `/AcroForm/XFA` exists in the catalog, the document fails.
pub struct XfaCheck;

impl Check for XfaCheck {
    fn id(&self) -> &'static str {
        "25-xfa"
    }

    fn checkpoint(&self) -> u8 {
        25
    }

    fn description(&self) -> &'static str {
        "XFA: document must not contain XFA form data"
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
    match acro_form.get(b"XFA") {
        Ok(_) => {
            results.push(fail(
                "25-001",
                "Document contains XFA form data — PDF/UA requires AcroForm-only interactive forms",
            ));
        }
        Err(_) => {
            results.push(pass("25-001", "No XFA form data in AcroForm"));
        }
    }
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
