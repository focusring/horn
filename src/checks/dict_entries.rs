use crate::checks::Check;
use crate::document::HornDocument;
use crate::model::{CheckOutcome, CheckResult, Severity};
use anyhow::Result;

/// Checkpoint 07: Dictionary entry validation + structure integrity.
///
/// Validates structural requirements from ISO 14289-1 section 7.1:
/// - 07-001: /`ParentTree` must exist in `StructTreeRoot`
/// - 07-002: MarkInfo/Suspects must not be true
/// - 07-003: Structure elements with non-standard types must have role map entries
pub struct DictEntryChecks;

impl Check for DictEntryChecks {
    fn id(&self) -> &'static str {
        "07-dict"
    }

    fn checkpoint(&self) -> u8 {
        7
    }

    fn description(&self) -> &'static str {
        "Dictionary entries: ParentTree, Suspects flag, structure type coverage"
    }

    fn run(&self, doc: &mut HornDocument) -> Result<Vec<CheckResult>> {
        let mut results = Vec::new();

        check_parent_tree(doc, &mut results);
        check_suspects_flag(doc, &mut results);
        check_unmapped_types(doc, &mut results);
        check_reference_xobjects(doc, &mut results);

        Ok(results)
    }
}

/// 07-001: `StructTreeRoot` must contain a /`ParentTree` entry, and the `ParentTree`
/// must cover all pages that have structured content.
///
/// The `ParentTree` is a number tree that maps marked-content identifiers (MCIDs)
/// to their parent structure elements. Without it, assistive technologies cannot
/// navigate from page content to the structure tree.
///
/// We also validate completeness: every page with /`StructParents` must have a
/// corresponding entry in the `ParentTree` /Nums array.
fn check_parent_tree(doc: &mut HornDocument, results: &mut Vec<CheckResult>) {
    let Ok(catalog) = doc.raw_catalog() else {
        return;
    };
    let lopdf_doc = doc.lopdf();

    let Some(struct_tree) = get_struct_tree_dict(catalog, lopdf_doc) else {
        // No StructTreeRoot — other checks will catch this
        return;
    };

    let parent_tree_dict = if let Ok(obj) = struct_tree.get(b"ParentTree") {
        // Resolve the reference
        let resolved = if let Ok(ref_id) = obj.as_reference() {
            lopdf_doc.get_object(ref_id).ok()
        } else {
            Some(obj)
        };

        if let Some(resolved_obj) = resolved {
            if let Ok(d) = resolved_obj.as_dict() {
                results.push(pass("07-001", "StructTreeRoot contains /ParentTree"));
                Some(d)
            } else {
                results.push(fail(
                    "07-001",
                    "StructTreeRoot /ParentTree is not a valid dictionary (number tree)",
                ));
                None
            }
        } else {
            results.push(fail(
                "07-001",
                "StructTreeRoot /ParentTree reference cannot be resolved",
            ));
            None
        }
    } else {
        results.push(fail(
            "07-001",
            "StructTreeRoot missing /ParentTree — MCIDs cannot be mapped to structure",
        ));
        None
    };

    // Validate ParentTree completeness: every page with /StructParents
    // must have a matching entry in the ParentTree /Nums array.
    if let Some(pt_dict) = parent_tree_dict {
        check_parent_tree_completeness(lopdf_doc, pt_dict, results);
    }
}

/// Validate that every page's /`StructParents` index has a corresponding entry
/// in the `ParentTree` number tree.
fn check_parent_tree_completeness(
    doc: &lopdf::Document,
    parent_tree: &lopdf::Dictionary,
    results: &mut Vec<CheckResult>,
) {
    // Collect all /StructParents values from pages
    let mut struct_parents_indices: Vec<i64> = Vec::new();
    for page_id in doc.page_iter() {
        if let Ok(page_obj) = doc.get_object(page_id) {
            if let Ok(page_dict) = page_obj.as_dict() {
                if let Ok(sp) = page_dict.get(b"StructParents") {
                    if let Ok(idx) = sp.as_i64() {
                        struct_parents_indices.push(idx);
                    }
                }
            }
        }
    }

    if struct_parents_indices.is_empty() {
        // No pages have /StructParents — nothing to validate
        return;
    }

    // Collect all keys from the number tree (handles both /Nums and /Kids)
    let mut nums_indices = Vec::new();
    collect_number_tree_keys(doc, parent_tree, &mut nums_indices, 0);

    // Check that every StructParents index has a ParentTree entry
    let mut missing = Vec::new();
    for sp_idx in &struct_parents_indices {
        if !nums_indices.contains(sp_idx) {
            missing.push(*sp_idx);
        }
    }

    if !missing.is_empty() {
        for idx in &missing {
            results.push(fail(
                "07-001",
                &format!("Page with /StructParents {idx} has no corresponding ParentTree entry"),
            ));
        }
    }
}

/// Recursively collect all integer keys from a PDF number tree.
///
/// PDF number trees (ISO 32000-1 §7.9.7) can be structured as:
/// - Leaf nodes with a /Nums array: `[key1, val1, key2, val2, ...]`
/// - Intermediate nodes with a /Kids array pointing to child number tree nodes
/// - Or both (though typically only one form per node)
///
/// Large documents (like the PDF/UA Reference Suite's 127-page magazine) use
/// /Kids-based trees, so we must traverse the full tree to find all entries.
fn collect_number_tree_keys(
    doc: &lopdf::Document,
    node: &lopdf::Dictionary,
    keys: &mut Vec<i64>,
    depth: usize,
) {
    if depth > 50 {
        return; // Prevent infinite recursion on malformed trees
    }

    // Collect keys from /Nums array if present
    if let Ok(nums_obj) = node.get(b"Nums") {
        if let Ok(arr) = nums_obj.as_array() {
            let mut i = 0;
            while i + 1 < arr.len() {
                if let Ok(key) = arr[i].as_i64() {
                    keys.push(key);
                }
                i += 2;
            }
        }
    }

    // Recurse into /Kids if present (intermediate nodes)
    if let Ok(kids_obj) = node.get(b"Kids") {
        if let Ok(kids_arr) = kids_obj.as_array() {
            for kid in kids_arr {
                let child_dict = match kid {
                    lopdf::Object::Reference(ref_id) => {
                        doc.get_object(*ref_id).ok().and_then(|o| o.as_dict().ok())
                    }
                    lopdf::Object::Dictionary(d) => Some(d),
                    _ => None,
                };
                if let Some(child) = child_dict {
                    collect_number_tree_keys(doc, child, keys, depth + 1);
                }
            }
        }
    }
}

/// 07-002: MarkInfo/Suspects must not be true.
///
/// When /Suspects is true, it indicates the document's tag structure may be
/// unreliable (e.g., auto-generated by OCR). PDF/UA-1 requires this to be false
/// or absent.
fn check_suspects_flag(doc: &mut HornDocument, results: &mut Vec<CheckResult>) {
    let Ok(catalog) = doc.raw_catalog() else {
        return;
    };
    let lopdf_doc = doc.lopdf();

    let mark_info = match catalog.get_deref(b"MarkInfo", lopdf_doc) {
        Ok(obj) => match obj.as_dict() {
            Ok(d) => d,
            Err(_) => return,
        },
        Err(_) => return, // MarkInfo absence caught by structure.rs
    };

    match mark_info.get(b"Suspects") {
        Ok(val) => {
            let suspects = val.as_bool().or_else(|_| val.as_i64().map(|i| i != 0));
            if let Ok(true) = suspects {
                results.push(fail(
                    "07-002",
                    "MarkInfo/Suspects is true — tag structure is flagged as unreliable",
                ));
            } else {
                results.push(pass("07-002", "MarkInfo/Suspects is false or not set"));
            }
        }
        Err(_) => {
            // /Suspects absent — this is fine
            results.push(pass("07-002", "MarkInfo/Suspects is not set (acceptable)"));
        }
    }
}

/// 07-003: All structure elements with non-standard types must be covered by `RoleMap`.
///
/// Walks the structure tree. Any element with a /S value that is neither a
/// standard type nor present in the `RoleMap` is a failure.
fn check_unmapped_types(doc: &mut HornDocument, results: &mut Vec<CheckResult>) {
    let Ok(catalog) = doc.raw_catalog() else {
        return;
    };
    let lopdf_doc = doc.lopdf();

    let Some(struct_tree) = get_struct_tree_dict(catalog, lopdf_doc) else {
        return;
    };

    // Get RoleMap (may be absent if no custom types are used)
    let role_map = struct_tree
        .get_deref(b"RoleMap", lopdf_doc)
        .ok()
        .and_then(|obj| obj.as_dict().ok());

    // Walk the structure tree and collect all non-standard unmapped types
    let mut unmapped_types = Vec::new();
    walk_struct_elements(lopdf_doc, struct_tree, role_map, &mut unmapped_types, 0);

    if unmapped_types.is_empty() {
        results.push(pass(
            "07-003",
            "All structure element types are standard or have role map entries",
        ));
    } else {
        // Deduplicate
        unmapped_types.sort();
        unmapped_types.dedup();
        for type_name in &unmapped_types {
            results.push(fail(
                "07-003",
                &format!(
                    "Structure element type /{type_name} is non-standard and has no RoleMap entry"
                ),
            ));
        }
    }
}

fn walk_struct_elements(
    doc: &lopdf::Document,
    dict: &lopdf::Dictionary,
    role_map: Option<&lopdf::Dictionary>,
    unmapped: &mut Vec<String>,
    depth: usize,
) {
    if depth > 100 {
        return; // Prevent infinite recursion
    }

    // Check /S (structure type) on this element
    if let Ok(s_obj) = dict.get(b"S") {
        if let Ok(type_name) = s_obj.as_name() {
            if !is_standard_structure_type(type_name) {
                // Check if it's in the RoleMap
                let is_mapped = role_map.is_some_and(|rm| rm.get(type_name).is_ok());
                if !is_mapped {
                    unmapped.push(String::from_utf8_lossy(type_name).to_string());
                }
            }
        }
    }

    // Walk children via /K
    let Ok(kids) = dict.get(b"K") else { return };

    match kids {
        lopdf::Object::Array(arr) => {
            for item in arr {
                if let Ok(child_dict) = resolve_to_dict(doc, item) {
                    walk_struct_elements(doc, child_dict, role_map, unmapped, depth + 1);
                }
            }
        }
        lopdf::Object::Reference(ref_id) => {
            if let Ok(obj) = doc.get_object(*ref_id) {
                if let Ok(child_dict) = obj.as_dict() {
                    walk_struct_elements(doc, child_dict, role_map, unmapped, depth + 1);
                }
            }
        }
        lopdf::Object::Dictionary(d) => {
            walk_struct_elements(doc, d, role_map, unmapped, depth + 1);
        }
        _ => {} // Integer MCIDs are leaf content, skip
    }
}

fn resolve_to_dict<'a>(
    doc: &'a lopdf::Document,
    obj: &'a lopdf::Object,
) -> Result<&'a lopdf::Dictionary, ()> {
    match obj {
        lopdf::Object::Dictionary(d) => Ok(d),
        lopdf::Object::Reference(ref_id) => doc
            .get_object(*ref_id)
            .map_err(|_| ())?
            .as_dict()
            .map_err(|_| ()),
        _ => Err(()),
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
        // MathML structure element (PDF 2.0 / PDF/UA-2)
        | b"Math"
        // StructTreeRoot itself
        | b"StructTreeRoot"
    )
}

/// 25-001: Reference XObjects (/Ref key on Form XObjects) are not permitted in PDF/UA.
///
/// A Form XObject with a /Ref entry is a reference XObject that points to external
/// content. These are forbidden because assistive technologies cannot access the
/// referenced content.
fn check_reference_xobjects(doc: &mut HornDocument, results: &mut Vec<CheckResult>) {
    let lopdf_doc = doc.lopdf();
    let pages = lopdf_doc.get_pages();

    for (page_num, page_id) in &pages {
        let Ok(page_obj) = lopdf_doc.get_object(*page_id) else {
            continue;
        };
        let Ok(page_dict) = page_obj.as_dict() else {
            continue;
        };

        // Get XObject resources
        let xobjects = page_dict
            .get_deref(b"Resources", lopdf_doc)
            .ok()
            .and_then(|o| o.as_dict().ok())
            .and_then(|res| res.get_deref(b"XObject", lopdf_doc).ok())
            .and_then(|o| o.as_dict().ok());

        let Some(xobj_dict) = xobjects else {
            continue;
        };

        for (name, obj) in xobj_dict.iter() {
            let resolved = if let Ok(ref_id) = obj.as_reference() {
                lopdf_doc.get_object(ref_id).ok()
            } else {
                Some(obj)
            };
            let Some(xobj) = resolved else { continue };

            // Only check Form XObjects (not Image XObjects)
            let is_form = xobj
                .as_stream()
                .ok()
                .and_then(|s| s.dict.get(b"Subtype").ok())
                .and_then(|o| o.as_name().ok())
                .is_some_and(|n| n == b"Form");

            if !is_form {
                continue;
            }

            // Check for /Ref key — reference XObjects are forbidden
            let has_ref = xobj
                .as_stream()
                .ok()
                .is_some_and(|s| s.dict.get(b"Ref").is_ok());

            if has_ref {
                let name_str = String::from_utf8_lossy(name);
                results.push(CheckResult {
                    rule_id: "25-001".to_string(),
                    checkpoint: 25,
                    description: format!(
                        "Page {page_num}: Form XObject /{name_str} is a reference XObject (/Ref)"
                    ),
                    severity: Severity::Error,
                    outcome: CheckOutcome::Fail {
                        message: format!(
                            "Page {page_num}: Form XObject /{name_str} has /Ref entry — reference XObjects are not permitted in PDF/UA"
                        ),
                        location: None,
                    },
                });
            }
        }
    }
}

fn pass(rule_id: &str, description: &str) -> CheckResult {
    CheckResult {
        rule_id: rule_id.to_string(),
        checkpoint: 7,
        description: description.to_string(),
        severity: Severity::Info,
        outcome: CheckOutcome::Pass,
    }
}

fn fail(rule_id: &str, message: &str) -> CheckResult {
    CheckResult {
        rule_id: rule_id.to_string(),
        checkpoint: 7,
        description: message.to_string(),
        severity: Severity::Error,
        outcome: CheckOutcome::Fail {
            message: message.to_string(),
            location: None,
        },
    }
}
