use crate::checks::Check;
use crate::document::HornDocument;
use crate::model::{CheckOutcome, CheckResult, Severity, Standard};
use anyhow::Result;

/// Checkpoint 17: Mathematical Expressions.
///
/// PDF/UA-1 requires that Formula structure elements have alternative text
/// (`/Alt`) so assistive technologies can convey mathematical content.
///
/// PDF/UA-2 allows formulas to be accessible through MathML structure trees
/// (Math elements with /NS referencing the MathML namespace), so the /Alt
/// requirement is relaxed — this check only applies to UA-1.
pub struct MathChecks;

impl Check for MathChecks {
    fn id(&self) -> &'static str {
        "17-math"
    }

    fn checkpoint(&self) -> u8 {
        17
    }

    fn description(&self) -> &'static str {
        "Math: Formula elements must have alternative text"
    }

    fn supports(&self, standard: Standard) -> bool {
        // UA-2 allows MathML structure as an alternative to /Alt text
        standard == Standard::Ua1 || standard == Standard::Unknown
    }

    fn run(&self, doc: &mut HornDocument) -> Result<Vec<CheckResult>> {
        let mut results = Vec::new();
        let Ok(catalog) = doc.raw_catalog() else {
            return Ok(results);
        };
        let lopdf_doc = doc.lopdf();

        let Some(struct_tree) = get_struct_tree(catalog, lopdf_doc) else {
            return Ok(results);
        };

        let mut formula_count = 0;
        let mut missing_alt = 0;

        walk_struct_tree(
            lopdf_doc,
            struct_tree,
            &mut |dict, _depth| {
                let elem_type = match dict.get(b"S") {
                    Ok(obj) => match obj.as_name() {
                        Ok(name) => name,
                        Err(_) => return,
                    },
                    Err(_) => return,
                };

                // Match both "Formula" (PDF 1.7 / UA-1) and "Math" (PDF 2.0 / UA-2)
                if elem_type != b"Formula" && elem_type != b"Math" {
                    return;
                }

                formula_count += 1;

                // Check for /Alt
                let has_alt = dict
                    .get(b"Alt")
                    .ok()
                    .and_then(|o| o.as_str().ok())
                    .is_some_and(|s| !s.is_empty());

                // Check for /ActualText as fallback — presence of the key is
                // sufficient per PDF/UA-1 (quality of text is a human judgment).
                let has_actual = dict.get(b"ActualText").is_ok();

                if !has_alt && !has_actual {
                    missing_alt += 1;
                    results.push(fail(
                    "17-001",
                    &format!(
                        "Formula {formula_count} has no /Alt or /ActualText — mathematical expressions must have alternative text"
                    ),
                ));
                }
            },
            0,
        );

        if formula_count > 0 && missing_alt == 0 {
            results.push(pass(
                "17-001",
                &format!("All {formula_count} Formula element(s) have alternative text"),
            ));
        }

        Ok(results)
    }
}

fn walk_struct_tree(
    doc: &lopdf::Document,
    dict: &lopdf::Dictionary,
    callback: &mut impl FnMut(&lopdf::Dictionary, usize),
    depth: usize,
) {
    if depth > 100 {
        return;
    }

    // Call back for this element (skip StructTreeRoot itself)
    if dict.get(b"S").is_ok() {
        callback(dict, depth);
    }

    // Walk children via /K
    let Ok(kids) = dict.get(b"K") else { return };

    match kids {
        lopdf::Object::Array(arr) => {
            for kid in arr {
                if let Ok(kid_ref) = kid.as_reference() {
                    if let Ok(kid_obj) = doc.get_object(kid_ref) {
                        if let Ok(kid_dict) = kid_obj.as_dict() {
                            walk_struct_tree(doc, kid_dict, callback, depth + 1);
                        }
                    }
                } else if let Ok(kid_dict) = kid.as_dict() {
                    walk_struct_tree(doc, kid_dict, callback, depth + 1);
                }
            }
        }
        lopdf::Object::Reference(ref_id) => {
            if let Ok(obj) = doc.get_object(*ref_id) {
                if let Ok(d) = obj.as_dict() {
                    walk_struct_tree(doc, d, callback, depth + 1);
                }
            }
        }
        lopdf::Object::Dictionary(d) => {
            walk_struct_tree(doc, d, callback, depth + 1);
        }
        _ => {}
    }
}

fn get_struct_tree<'a>(
    catalog: &'a lopdf::Dictionary,
    doc: &'a lopdf::Document,
) -> Option<&'a lopdf::Dictionary> {
    let ref_id = catalog.get(b"StructTreeRoot").ok()?.as_reference().ok()?;
    doc.get_object(ref_id).ok()?.as_dict().ok()
}

fn pass(rule_id: &str, description: &str) -> CheckResult {
    CheckResult {
        rule_id: rule_id.to_string(),
        checkpoint: 17,
        description: description.to_string(),
        severity: Severity::Info,
        outcome: CheckOutcome::Pass,
    }
}

fn fail(rule_id: &str, message: &str) -> CheckResult {
    CheckResult {
        rule_id: rule_id.to_string(),
        checkpoint: 17,
        description: message.to_string(),
        severity: Severity::Error,
        outcome: CheckOutcome::Fail {
            message: message.to_string(),
            location: None,
        },
    }
}
