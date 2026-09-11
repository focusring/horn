use crate::checks::Check;
use crate::document::HornDocument;
use crate::model::{CheckOutcome, CheckResult, Severity};
use anyhow::Result;

/// Checkpoint 20: Optional content (layers).
///
/// Optional content configuration dictionaries (ISO 32000-1, 8.11.4.3) drive
/// which layers are visible. PDF/UA-1 clause 7.10 requires every configuration
/// dictionary to carry a human-readable `/Name` and forbids `/AS` (usage
/// application) arrays, which can switch content on and off without user
/// interaction.
///
/// - 20-001: `/Name` missing or empty in a configuration in `/Configs`
/// - 20-002: `/Name` missing or empty in the default configuration `/D`
/// - 20-003: `/AS` present in any configuration dictionary
/// - 20-x01 (Horn extension): optional content groups (`/OCGs`) without `/Name`
pub struct OptionalContentChecks;

impl Check for OptionalContentChecks {
    fn id(&self) -> &'static str {
        "20-optional-content"
    }

    fn checkpoint(&self) -> u8 {
        20
    }

    fn rules(&self) -> &'static [&'static str] {
        &["20-001", "20-002", "20-003", "20-x01"]
    }

    fn description(&self) -> &'static str {
        "Optional content: configuration names, no auto-state"
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

        check_default_config(oc_props, lopdf_doc, &mut results);
        check_configs_array(oc_props, lopdf_doc, &mut results);
        check_ocg_names(oc_props, lopdf_doc, &mut results);

        Ok(results)
    }
}

/// 20-002 / 20-003: the default configuration dictionary `/D`.
fn check_default_config(
    oc_props: &lopdf::Dictionary,
    doc: &lopdf::Document,
    results: &mut Vec<CheckResult>,
) {
    let d_dict = if let Ok(obj) = oc_props.get_deref(b"D", doc) {
        if let Ok(d) = obj.as_dict() {
            d
        } else {
            results.push(fail("20-002", "/OCProperties/D is not a dictionary"));
            return;
        }
    } else {
        results.push(fail(
            "20-002",
            "/OCProperties missing default configuration /D",
        ));
        return;
    };

    check_config_dict(
        d_dict,
        doc,
        "Default OC configuration /D",
        "20-002",
        results,
    );
}

/// 20-001 / 20-003: every configuration dictionary in the `/Configs` array.
fn check_configs_array(
    oc_props: &lopdf::Dictionary,
    doc: &lopdf::Document,
    results: &mut Vec<CheckResult>,
) {
    let Ok(configs_obj) = oc_props.get_deref(b"Configs", doc) else {
        return;
    };
    let Ok(configs) = configs_obj.as_array() else {
        results.push(fail("20-001", "/OCProperties/Configs is not an array"));
        return;
    };

    for (i, item) in configs.iter().enumerate() {
        let dict = match item {
            lopdf::Object::Reference(r) => doc.get_object(*r).ok().and_then(|o| o.as_dict().ok()),
            lopdf::Object::Dictionary(d) => Some(d),
            _ => None,
        };
        let label = format!("OC configuration /Configs[{i}]");
        match dict {
            Some(d) => check_config_dict(d, doc, &label, "20-001", results),
            None => results.push(fail("20-001", &format!("{label} is not a dictionary"))),
        }
    }
}

/// Shared validation of one optional content configuration dictionary.
fn check_config_dict(
    dict: &lopdf::Dictionary,
    doc: &lopdf::Document,
    label: &str,
    name_rule: &str,
    results: &mut Vec<CheckResult>,
) {
    match dict.get_deref(b"Name", doc) {
        Ok(name_obj) => {
            // /Name is a text string (ISO 32000-1 Table 101); a name object is invalid
            let is_empty = match name_obj {
                lopdf::Object::String(bytes, _) => bytes.is_empty(),
                _ => true,
            };
            if is_empty {
                results.push(fail(
                    name_rule,
                    &format!("{label}: /Name is empty or not a string"),
                ));
            } else {
                results.push(pass(name_rule, &format!("{label} has a valid /Name")));
            }
        }
        Err(_) => {
            results.push(fail(name_rule, &format!("{label}: missing /Name entry")));
        }
    }

    // 20-003: /AS (usage application) arrays are forbidden in every configuration.
    if dict.get(b"AS").is_ok() {
        results.push(fail(
            "20-003",
            &format!("{label} has an /AS (usage application) entry — not permitted in PDF/UA"),
        ));
    }
}

/// 20-x01 (extension): each optional content group should have a /Name so that
/// assistive technology can announce the layer.
fn check_ocg_names(
    oc_props: &lopdf::Dictionary,
    doc: &lopdf::Document,
    results: &mut Vec<CheckResult>,
) {
    let Ok(ocgs_obj) = oc_props.get_deref(b"OCGs", doc) else {
        return;
    };
    let Ok(ocgs) = ocgs_obj.as_array() else {
        return;
    };

    let mut missing = 0usize;
    for item in ocgs {
        let dict = match item {
            lopdf::Object::Reference(r) => doc.get_object(*r).ok().and_then(|o| o.as_dict().ok()),
            lopdf::Object::Dictionary(d) => Some(d),
            _ => None,
        };
        let Some(ocg) = dict else { continue };
        let has_name = match ocg.get_deref(b"Name", doc) {
            Ok(lopdf::Object::String(bytes, _)) => !bytes.is_empty(),
            _ => false,
        };
        if !has_name {
            missing += 1;
        }
    }

    if missing > 0 {
        results.push(fail(
            "20-x01",
            &format!("{missing} optional content group(s) have no /Name"),
        ));
    } else if !ocgs.is_empty() {
        results.push(pass("20-x01", "All optional content groups have a /Name"));
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
