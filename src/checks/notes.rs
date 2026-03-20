use crate::checks::Check;
use crate::document::HornDocument;
use crate::model::{CheckOutcome, CheckResult, Severity};
use anyhow::Result;
use std::collections::HashSet;

/// Checkpoint 19: Notes and References.
///
/// PDF/UA-1 requires:
/// - 19-001: Every Note structure element must have a non-empty `/ID` attribute.
/// - 19-002: Note `/ID` values must be unique within the document.
pub struct NoteChecks;

impl Check for NoteChecks {
    fn id(&self) -> &'static str {
        "19-notes"
    }

    fn checkpoint(&self) -> u8 {
        19
    }

    fn description(&self) -> &'static str {
        "Notes: Note elements must have unique IDs"
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

        let mut note_count = 0;
        let mut missing_id = 0;
        let mut seen_ids: HashSet<Vec<u8>> = HashSet::new();
        let mut duplicate_ids = 0;

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

                if elem_type != b"Note" {
                    return;
                }

                note_count += 1;

                if let Ok(id_obj) = dict.get(b"ID") {
                    if let Ok(id_bytes_ref) = id_obj.as_str() {
                        if id_bytes_ref.is_empty() {
                            missing_id += 1;
                            results
                                .push(fail("19-001", &format!("Note {note_count}: /ID is empty")));
                        } else {
                            let id_vec = id_bytes_ref.to_vec();
                            let id_display = String::from_utf8_lossy(&id_vec).into_owned();
                            if !seen_ids.insert(id_vec) {
                                duplicate_ids += 1;
                                results.push(fail(
                                "19-002",
                                &format!(
                                    "Note {note_count}: duplicate /ID \"{id_display}\" — Note IDs must be unique"
                                ),
                            ));
                            }
                        }
                    } else {
                        // ID exists but isn't a string — treat as missing
                        missing_id += 1;
                        results.push(fail(
                            "19-001",
                            &format!("Note {note_count}: /ID is not a valid string"),
                        ));
                    }
                } else {
                    missing_id += 1;
                    results.push(fail(
                        "19-001",
                        &format!("Note {note_count}: missing /ID attribute"),
                    ));
                }
            },
            0,
        );

        if note_count > 0 && missing_id == 0 && duplicate_ids == 0 {
            results.push(pass(
                "19-001",
                &format!("All {note_count} Note element(s) have unique /ID attributes"),
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

    if dict.get(b"S").is_ok() {
        callback(dict, depth);
    }

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
        checkpoint: 19,
        description: description.to_string(),
        severity: Severity::Info,
        outcome: CheckOutcome::Pass,
    }
}

fn fail(rule_id: &str, message: &str) -> CheckResult {
    CheckResult {
        rule_id: rule_id.to_string(),
        checkpoint: 19,
        description: message.to_string(),
        severity: Severity::Error,
        outcome: CheckOutcome::Fail {
            message: message.to_string(),
            location: None,
        },
    }
}
