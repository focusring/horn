use crate::checks::Check;
use crate::document::HornDocument;
use crate::model::{CheckOutcome, CheckResult, Severity};
use anyhow::Result;

/// Checkpoint 20: Optional Content.
///
/// PDF/UA-1 requires that optional content groups (OCGs) are properly configured:
/// - 20-001: The default configuration dictionary (`/D`) must have a non-empty `/Name`.
/// - 20-002: Each OCG referenced in `/OCGs` must have a `/Name` entry.
pub struct OptionalContentChecks;

impl Check for OptionalContentChecks {
    fn id(&self) -> &'static str {
        "20-optional-content"
    }

    fn checkpoint(&self) -> u8 {
        20
    }

    fn description(&self) -> &'static str {
        "Optional content: OCG names and default configuration"
    }

    fn run(&self, doc: &mut HornDocument) -> Result<Vec<CheckResult>> {
        let mut results = Vec::new();
        let Ok(catalog) = doc.raw_catalog() else {
            return Ok(results);
        };
        let lopdf_doc = doc.lopdf();

        // Get /OCProperties from catalog
        let oc_props = match catalog.get_deref(b"OCProperties", lopdf_doc) {
            Ok(obj) => match obj.as_dict() {
                Ok(d) => d,
                Err(_) => return Ok(results),
            },
            Err(_) => {
                // No optional content — checks not applicable
                return Ok(results);
            }
        };

        check_default_config_name(oc_props, lopdf_doc, &mut results);
        check_default_config_as(oc_props, lopdf_doc, &mut results);
        check_ocg_names(oc_props, lopdf_doc, &mut results);

        Ok(results)
    }
}

/// 20-001: The default configuration dictionary must have a non-empty /Name.
fn check_default_config_name(
    oc_props: &lopdf::Dictionary,
    doc: &lopdf::Document,
    results: &mut Vec<CheckResult>,
) {
    let d_dict = match oc_props.get_deref(b"D", doc) {
        Ok(obj) => match obj.as_dict() {
            Ok(d) => d,
            Err(_) => {
                results.push(fail("20-001", "/OCProperties/D is not a dictionary"));
                return;
            }
        },
        Err(_) => {
            results.push(fail(
                "20-001",
                "/OCProperties missing default configuration /D",
            ));
            return;
        }
    };

    match d_dict.get(b"Name") {
        Ok(name_obj) => {
            if let Ok(name) = name_obj.as_str() {
                if name.is_empty() {
                    results.push(fail("20-001", "Default OC configuration /Name is empty"));
                } else {
                    results.push(pass("20-001", "Default OC configuration has a valid /Name"));
                }
            } else if let Ok(name_bytes) = name_obj.as_name() {
                if name_bytes.is_empty() {
                    results.push(fail("20-001", "Default OC configuration /Name is empty"));
                } else {
                    results.push(pass("20-001", "Default OC configuration has a valid /Name"));
                }
            } else {
                results.push(fail(
                    "20-001",
                    "Default OC configuration /Name is not a valid string",
                ));
            }
        }
        Err(_) => {
            results.push(fail(
                "20-001",
                "Default OC configuration /D missing /Name entry",
            ));
        }
    }
}

/// 20-003: The default configuration dictionary must not have an /AS entry.
///
/// The /AS (auto-state) array allows OCGs to change visibility based on events
/// like print, export, or view — this can hide content from assistive technology
/// without user action, violating PDF/UA accessibility requirements.
fn check_default_config_as(
    oc_props: &lopdf::Dictionary,
    doc: &lopdf::Document,
    results: &mut Vec<CheckResult>,
) {
    let Ok(d_obj) = oc_props.get_deref(b"D", doc) else {
        return;
    };
    let Ok(d_dict) = d_obj.as_dict() else { return };

    if d_dict.get(b"AS").is_ok() {
        results.push(fail(
            "20-003",
            "Default OC configuration has /AS (auto-state) entry — not permitted in PDF/UA",
        ));
    }
}

/// 20-002: Each OCG must have a /Name entry.
fn check_ocg_names(
    oc_props: &lopdf::Dictionary,
    doc: &lopdf::Document,
    results: &mut Vec<CheckResult>,
) {
    let ocgs = match oc_props.get(b"OCGs") {
        Ok(obj) => match obj.as_array() {
            Ok(arr) => arr,
            Err(_) => return,
        },
        Err(_) => return,
    };

    let mut all_named = true;
    let mut ocg_count = 0;

    for ocg_ref in ocgs {
        let ocg_obj = if let Ok(ref_id) = ocg_ref.as_reference() {
            match doc.get_object(ref_id) {
                Ok(obj) => obj,
                Err(_) => continue,
            }
        } else {
            ocg_ref
        };

        let Ok(ocg_dict) = ocg_obj.as_dict() else {
            continue;
        };
        ocg_count += 1;

        match ocg_dict.get(b"Name") {
            Ok(name_obj) => {
                let is_valid = if let Ok(s) = name_obj.as_str() {
                    !s.is_empty()
                } else if let Ok(n) = name_obj.as_name() {
                    !n.is_empty()
                } else {
                    false
                };
                if !is_valid {
                    all_named = false;
                    results.push(fail("20-002", "OCG has empty or invalid /Name"));
                }
            }
            Err(_) => {
                all_named = false;
                results.push(fail("20-002", "OCG missing /Name entry"));
            }
        }
    }

    if ocg_count > 0 && all_named {
        results.push(pass(
            "20-002",
            &format!("All {ocg_count} OCG(s) have valid /Name entries"),
        ));
    }
}

fn pass(rule_id: &str, description: &str) -> CheckResult {
    CheckResult {
        rule_id: rule_id.to_string(),
        checkpoint: 20,
        description: description.to_string(),
        severity: Severity::Info,
        outcome: CheckOutcome::Pass,
    }
}

fn fail(rule_id: &str, message: &str) -> CheckResult {
    CheckResult {
        rule_id: rule_id.to_string(),
        checkpoint: 20,
        description: message.to_string(),
        severity: Severity::Error,
        outcome: CheckOutcome::Fail {
            message: message.to_string(),
            location: None,
        },
    }
}
