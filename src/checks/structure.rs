use crate::checks::Check;
use crate::document::HornDocument;
use crate::model::{CheckOutcome, CheckResult, Severity};
use anyhow::Result;

/// Checkpoints 01/09: Document structure checks.
///
/// Validates `StructTreeRoot` presence, MarkInfo/Marked, and role mapping.
pub struct StructureChecks;

impl Check for StructureChecks {
    fn id(&self) -> &'static str {
        "01-structure"
    }

    fn checkpoint(&self) -> u8 {
        1
    }

    fn description(&self) -> &'static str {
        "Structure: tagged PDF, StructTreeRoot, MarkInfo, role mapping"
    }

    fn run(&self, doc: &mut HornDocument) -> Result<Vec<CheckResult>> {
        let mut results = Vec::new();

        check_mark_info(doc, &mut results);
        check_struct_tree_root(doc, &mut results);
        check_role_mapping(doc, &mut results);

        Ok(results)
    }
}

/// 01-003: `MarkInfo` must exist with /Marked = true.
fn check_mark_info(doc: &mut HornDocument, results: &mut Vec<CheckResult>) {
    let Ok(catalog) = doc.raw_catalog() else {
        results.push(fail("01-003", 1, "Cannot read document catalog"));
        return;
    };

    let lopdf_doc = doc.lopdf();

    match catalog.get_deref(b"MarkInfo", lopdf_doc) {
        Ok(obj) => {
            if let Ok(mark_info) = obj.as_dict() {
                match mark_info.get_deref(b"Marked", lopdf_doc) {
                    Ok(val) => {
                        // Marked can be a boolean or integer (0/1)
                        let marked_value = val.as_bool().or_else(|_| val.as_i64().map(|i| i != 0));
                        if let Ok(marked) = marked_value {
                            if marked {
                                results.push(pass("01-003", 1, "MarkInfo/Marked is true"));

                                // Additional: check for /Suspects = true (indicates auto-tagged)
                                if let Ok(suspects) =
                                    mark_info.get(b"Suspects").and_then(lopdf::Object::as_bool)
                                {
                                    if suspects {
                                        results.push(CheckResult {
                                            rule_id: "01-003".to_string(),
                                            checkpoint: 1,
                                            description: "MarkInfo/Suspects is true — structure may be unreliable".to_string(),
                                            severity: Severity::Warning,
                                            outcome: CheckOutcome::NeedsReview {
                                                reason: "Suspects flag indicates the structure tree may have been auto-generated and needs manual review".to_string(),
                                            },
                                        });
                                    }
                                }
                            } else {
                                results.push(fail(
                                    "01-003",
                                    1,
                                    "MarkInfo/Marked is false — document is not tagged",
                                ));
                            }
                        } else {
                            results.push(fail("01-003", 1, "MarkInfo/Marked is not a boolean"));
                        }
                    }
                    Err(_) => {
                        results.push(fail("01-003", 1, "MarkInfo missing /Marked entry"));
                    }
                }
            } else {
                results.push(fail("01-003", 1, "MarkInfo is not a dictionary"));
            }
        }
        Err(_) => {
            results.push(fail("01-003", 1, "Document catalog missing /MarkInfo"));
        }
    }
}

/// 01-004: `StructTreeRoot` must exist in the catalog.
fn check_struct_tree_root(doc: &mut HornDocument, results: &mut Vec<CheckResult>) {
    let Ok(catalog) = doc.raw_catalog() else {
        return;
    };

    let lopdf_doc = doc.lopdf();

    match catalog.get(b"StructTreeRoot") {
        Ok(obj) => {
            if let Ok(ref_id) = obj.as_reference() {
                match lopdf_doc.get_object(ref_id) {
                    Ok(tree_obj) => {
                        if tree_obj.as_dict().is_ok() {
                            results.push(pass("01-004", 1, "StructTreeRoot exists"));

                            // Check that it has /K (kids) with at least one child
                            if let Ok(dict) = tree_obj.as_dict() {
                                if dict.get(b"K").is_err() {
                                    results.push(fail(
                                        "01-004",
                                        1,
                                        "StructTreeRoot has no /K (children) entry",
                                    ));
                                }
                            }
                        } else {
                            results.push(fail(
                                "01-004",
                                1,
                                "StructTreeRoot reference does not point to a dictionary",
                            ));
                        }
                    }
                    Err(_) => {
                        results.push(fail("01-004", 1, "Cannot resolve StructTreeRoot reference"));
                    }
                }
            } else {
                results.push(fail("01-004", 1, "StructTreeRoot is not a reference"));
            }
        }
        Err(_) => {
            results.push(fail(
                "01-004",
                1,
                "Document catalog missing /StructTreeRoot — document is not tagged",
            ));
        }
    }
}

/// 02-001: Role map entries must map to valid standard structure types.
fn check_role_mapping(doc: &mut HornDocument, results: &mut Vec<CheckResult>) {
    let Ok(catalog) = doc.raw_catalog() else {
        return;
    };

    let lopdf_doc = doc.lopdf();

    let Some(struct_tree) = get_struct_tree_dict(catalog, lopdf_doc) else {
        return;
    };

    match struct_tree.get_deref(b"RoleMap", lopdf_doc) {
        Ok(obj) => {
            if let Ok(role_map) = obj.as_dict() {
                let mut invalid_count = 0;
                for (user_role, std_role_obj) in role_map {
                    if let Ok(std_role) = std_role_obj.as_name() {
                        // Check for identity mapping (LI -> LI)
                        if user_role == std_role {
                            let role_str = String::from_utf8_lossy(user_role);
                            results.push(fail(
                                "02-001",
                                2,
                                &format!(
                                    "Role map entry /{role_str} -> /{role_str} is an identity mapping (circular)"
                                ),
                            ));
                            invalid_count += 1;
                            continue;
                        }

                        // Check for circular chains: follow the mapping chain
                        // and detect if it loops back without reaching a standard type
                        if !is_standard_structure_type(std_role) {
                            if let Some(cycle) = detect_role_cycle(role_map, user_role) {
                                let user_role_str = String::from_utf8_lossy(user_role);
                                results.push(fail(
                                    "02-001",
                                    2,
                                    &format!(
                                        "Role map entry /{user_role_str} creates a circular mapping: {cycle}"
                                    ),
                                ));
                                invalid_count += 1;
                            } else {
                                // Not circular, but check if chain eventually reaches standard type
                                let std_role_str = String::from_utf8_lossy(std_role);
                                if role_map.get(std_role).is_err() {
                                    let user_role_str = String::from_utf8_lossy(user_role);
                                    results.push(fail(
                                        "02-001",
                                        2,
                                        &format!(
                                            "Role map entry /{user_role_str} -> /{std_role_str} does not resolve to a standard type"
                                        ),
                                    ));
                                    invalid_count += 1;
                                }
                            }
                        }
                    }
                }
                if invalid_count == 0 {
                    results.push(pass(
                        "02-001",
                        2,
                        "All role map entries resolve to standard types",
                    ));
                }
            }
        }
        Err(_) => {
            // RoleMap is optional — no custom roles means nothing to validate
            results.push(pass("02-001", 2, "No custom role mapping (RoleMap absent)"));
        }
    }
}

/// Detect circular chains in a `RoleMap`. Returns a description of the cycle if found.
fn detect_role_cycle(role_map: &lopdf::Dictionary, start: &[u8]) -> Option<String> {
    let mut visited = Vec::new();
    let mut current = start;
    visited.push(String::from_utf8_lossy(current).into_owned());

    loop {
        let target = role_map.get(current).ok()?.as_name().ok()?;
        let target_str = String::from_utf8_lossy(target).into_owned();

        if target == start || visited.contains(&target_str) {
            visited.push(target_str);
            return Some(visited.join(" -> "));
        }

        if is_standard_structure_type(target) {
            return None; // Chain resolves to a standard type
        }

        visited.push(target_str);
        current = target;

        if visited.len() > 50 {
            return Some(format!("{} (chain too long)", visited.join(" -> ")));
        }
    }
}

fn get_struct_tree_dict<'a>(
    catalog: &'a lopdf::Dictionary,
    doc: &'a lopdf::Document,
) -> Option<&'a lopdf::Dictionary> {
    let ref_id = catalog.get(b"StructTreeRoot").ok()?.as_reference().ok()?;
    doc.get_object(ref_id).ok()?.as_dict().ok()
}

/// Standard PDF structure types from ISO 32000-1 Table 333-338 and PDF/UA.
fn is_standard_structure_type(name: &[u8]) -> bool {
    matches!(
        name,
        // Grouping elements
        b"Document" | b"DocumentFragment" | b"Part" | b"Art" | b"Sect" | b"Div"
        | b"BlockQuote" | b"Caption" | b"TOC" | b"TOCI" | b"Index"
        | b"NonStruct" | b"Private"
        // Block-level structure
        | b"H" | b"H1" | b"H2" | b"H3" | b"H4" | b"H5" | b"H6"
        | b"P" | b"L" | b"LI" | b"Lbl" | b"LBody"
        // Table elements
        | b"Table" | b"TR" | b"TH" | b"TD" | b"THead" | b"TBody" | b"TFoot"
        // Inline elements
        | b"Span" | b"Quote" | b"Note" | b"Reference" | b"BibEntry"
        | b"Code" | b"Link" | b"Annot"
        // Illustration elements
        | b"Figure" | b"Formula" | b"Form"
        // Ruby/Warichu
        | b"Ruby" | b"RB" | b"RT" | b"RP"
        | b"Warichu" | b"WT" | b"WP"
        // PDF 2.0 additions
        | b"Aside" | b"Title" | b"FENote" | b"Sub"
        | b"Em" | b"Strong"
    )
}

fn pass(rule_id: &str, checkpoint: u8, description: &str) -> CheckResult {
    CheckResult {
        rule_id: rule_id.to_string(),
        checkpoint,
        description: description.to_string(),
        severity: Severity::Info,
        outcome: CheckOutcome::Pass,
    }
}

fn fail(rule_id: &str, checkpoint: u8, message: &str) -> CheckResult {
    CheckResult {
        rule_id: rule_id.to_string(),
        checkpoint,
        description: message.to_string(),
        severity: Severity::Error,
        outcome: CheckOutcome::Fail {
            message: message.to_string(),
            location: None,
        },
    }
}
