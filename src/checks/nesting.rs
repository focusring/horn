use crate::checks::Check;
use crate::document::HornDocument;
use crate::model::{CheckOutcome, CheckResult, Severity};
use anyhow::Result;

/// Structure element nesting validation.
///
/// PDF/UA-1 (via ISO 32000-1 §14.8.4) requires that structure elements follow
/// strict parent-child rules. This check validates:
/// - Table children: only `THead`, `TBody`, `TFoot`, TR, Caption allowed
/// - TR children: only TH, TD allowed
/// - THead/TBody/TFoot children: only TR allowed
/// - List (L) children: only LI, Caption allowed
/// - LI children: only Lbl, `LBody` allowed
/// - TOC children: only TOCI, TOC, Caption allowed
/// - Cardinality: at most one `THead`, one `TFoot` per Table; THead/TFoot require `TBody`
/// - Caption position: first or last child only
pub struct NestingChecks;

impl Check for NestingChecks {
    fn id(&self) -> &'static str {
        "09-nesting"
    }

    fn checkpoint(&self) -> u8 {
        9
    }

    fn description(&self) -> &'static str {
        "Structure nesting: Table/List/TOC child type rules"
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

        walk_and_validate(lopdf_doc, struct_tree, &mut results, 0);

        Ok(results)
    }
}

fn walk_and_validate(
    doc: &lopdf::Document,
    dict: &lopdf::Dictionary,
    results: &mut Vec<CheckResult>,
    depth: usize,
) {
    if depth > 100 {
        return;
    }

    let parent_type = dict
        .get(b"S")
        .ok()
        .and_then(|o| o.as_name().ok())
        .unwrap_or(b"");

    // Collect children types
    let children = collect_children(doc, dict);

    // Validate nesting rules based on parent type
    match parent_type {
        b"Table" => validate_table_children(&children, results),
        b"TR" => validate_tr_children(&children, results),
        b"THead" | b"TBody" | b"TFoot" => {
            validate_table_section_children(parent_type, &children, results);
        }
        b"L" => validate_list_children(&children, results),
        b"LI" => validate_li_children(&children, results),
        b"TOC" => validate_toc_children(&children, results),
        _ => {}
    }

    // Validate "must be child of" rules — certain types require specific parents
    for child in &children {
        validate_required_parent(parent_type, &child.elem_type, results);
    }

    // Recurse into children
    let Ok(kids) = dict.get(b"K") else { return };
    recurse_kids(doc, kids, results, depth);
}

struct ChildInfo {
    elem_type: Vec<u8>,
}

fn collect_children(doc: &lopdf::Document, dict: &lopdf::Dictionary) -> Vec<ChildInfo> {
    let mut children = Vec::new();
    let Ok(kids) = dict.get(b"K") else {
        return children;
    };

    match kids {
        lopdf::Object::Array(arr) => {
            for kid in arr {
                if let Some(child_type) = resolve_child_type(doc, kid) {
                    children.push(ChildInfo {
                        elem_type: child_type,
                    });
                }
            }
        }
        lopdf::Object::Reference(_) | lopdf::Object::Dictionary(_) => {
            if let Some(child_type) = resolve_child_type(doc, kids) {
                children.push(ChildInfo {
                    elem_type: child_type,
                });
            }
        }
        _ => {}
    }

    children
}

fn resolve_child_type(doc: &lopdf::Document, obj: &lopdf::Object) -> Option<Vec<u8>> {
    let dict = match obj {
        lopdf::Object::Reference(ref_id) => doc.get_object(*ref_id).ok()?.as_dict().ok()?,
        lopdf::Object::Dictionary(d) => d,
        _ => return None, // Integer MCIDs, etc.
    };

    dict.get(b"S")
        .ok()
        .and_then(|o| o.as_name().ok())
        .map(<[u8]>::to_vec)
}

fn recurse_kids(
    doc: &lopdf::Document,
    kids: &lopdf::Object,
    results: &mut Vec<CheckResult>,
    depth: usize,
) {
    match kids {
        lopdf::Object::Array(arr) => {
            for kid in arr {
                if let Ok(ref_id) = kid.as_reference() {
                    if let Ok(obj) = doc.get_object(ref_id) {
                        if let Ok(d) = obj.as_dict() {
                            walk_and_validate(doc, d, results, depth + 1);
                        }
                    }
                } else if let Ok(d) = kid.as_dict() {
                    walk_and_validate(doc, d, results, depth + 1);
                }
            }
        }
        lopdf::Object::Reference(ref_id) => {
            if let Ok(obj) = doc.get_object(*ref_id) {
                if let Ok(d) = obj.as_dict() {
                    walk_and_validate(doc, d, results, depth + 1);
                }
            }
        }
        lopdf::Object::Dictionary(d) => {
            walk_and_validate(doc, d, results, depth + 1);
        }
        _ => {}
    }
}

/// Table may only contain: `THead`, `TBody`, `TFoot`, TR, Caption
fn validate_table_children(children: &[ChildInfo], results: &mut Vec<CheckResult>) {
    let allowed = &[b"THead" as &[u8], b"TBody", b"TFoot", b"TR", b"Caption"];

    let mut thead_count = 0u32;
    let mut tfoot_count = 0u32;
    let mut tbody_count = 0u32;
    let mut caption_positions = Vec::new();

    for (i, child) in children.iter().enumerate() {
        let ct = &child.elem_type;

        if !allowed.contains(&ct.as_slice()) {
            let type_name = String::from_utf8_lossy(ct);
            results.push(fail(
                "09-006",
                &format!("{type_name} is not allowed as a child of Table (only THead/TBody/TFoot/TR/Caption)"),
            ));
        }

        if ct == b"THead" {
            thead_count += 1;
        }
        if ct == b"TFoot" {
            tfoot_count += 1;
        }
        if ct == b"TBody" {
            tbody_count += 1;
        }
        if ct == b"Caption" {
            caption_positions.push(i);
        }
    }

    // At most one THead
    if thead_count > 1 {
        results.push(fail("09-006", "Table has more than one THead"));
    }

    // At most one TFoot
    if tfoot_count > 1 {
        results.push(fail("09-006", "Table has more than one TFoot"));
    }

    // THead requires TBody
    if thead_count > 0 && tbody_count == 0 {
        results.push(fail("09-006", "Table has THead but no TBody"));
    }

    // TFoot requires TBody
    if tfoot_count > 0 && tbody_count == 0 {
        results.push(fail("09-006", "Table has TFoot but no TBody"));
    }

    // Caption must be first or last child
    for &pos in &caption_positions {
        if pos != 0 && pos != children.len() - 1 {
            results.push(fail(
                "09-006",
                "Caption in Table is not the first or last child",
            ));
        }
    }

    // Multiple Captions
    if caption_positions.len() > 1 {
        results.push(fail("09-006", "Table has more than one Caption"));
    }
}

/// TR may only contain: TH, TD
fn validate_tr_children(children: &[ChildInfo], results: &mut Vec<CheckResult>) {
    for child in children {
        let ct = &child.elem_type;
        if ct != b"TH" && ct != b"TD" {
            let type_name = String::from_utf8_lossy(ct);
            results.push(fail(
                "09-006",
                &format!("{type_name} is not allowed as a child of TR (only TH/TD)"),
            ));
        }
    }
}

/// THead/TBody/TFoot may only contain: TR
fn validate_table_section_children(
    parent: &[u8],
    children: &[ChildInfo],
    results: &mut Vec<CheckResult>,
) {
    let parent_name = String::from_utf8_lossy(parent);
    for child in children {
        let ct = &child.elem_type;
        if ct != b"TR" {
            let type_name = String::from_utf8_lossy(ct);
            results.push(fail(
                "09-006",
                &format!("{type_name} is not allowed as a child of {parent_name} (only TR)"),
            ));
        }
    }
}

/// L (List) may only contain: LI, Caption, L (nested lists)
fn validate_list_children(children: &[ChildInfo], results: &mut Vec<CheckResult>) {
    let mut caption_positions = Vec::new();

    for (i, child) in children.iter().enumerate() {
        let ct = &child.elem_type;
        // Allow LI, Caption, and nested L
        if ct != b"LI" && ct != b"Caption" && ct != b"L" {
            let type_name = String::from_utf8_lossy(ct);
            results.push(fail(
                "09-006",
                &format!("{type_name} is not allowed as a child of L (only LI/Caption/L)"),
            ));
        }
        if ct == b"Caption" {
            caption_positions.push(i);
        }
    }

    // Caption must be first child of List
    for &pos in &caption_positions {
        if pos != 0 {
            results.push(fail("09-006", "Caption in List is not the first child"));
        }
    }

    if caption_positions.len() > 1 {
        results.push(fail("09-006", "List has more than one Caption"));
    }
}

/// LI may only contain: Lbl, `LBody`
fn validate_li_children(children: &[ChildInfo], results: &mut Vec<CheckResult>) {
    for child in children {
        let ct = &child.elem_type;
        if ct != b"Lbl" && ct != b"LBody" {
            let type_name = String::from_utf8_lossy(ct);
            results.push(fail(
                "09-006",
                &format!("{type_name} is not allowed as a child of LI (only Lbl/LBody)"),
            ));
        }
    }
}

/// TOC may only contain: TOCI, TOC, Caption
fn validate_toc_children(children: &[ChildInfo], results: &mut Vec<CheckResult>) {
    let mut caption_positions = Vec::new();

    for (i, child) in children.iter().enumerate() {
        let ct = &child.elem_type;
        if ct != b"TOCI" && ct != b"TOC" && ct != b"Caption" {
            let type_name = String::from_utf8_lossy(ct);
            results.push(fail(
                "09-006",
                &format!("{type_name} is not allowed as a child of TOC (only TOCI/TOC/Caption)"),
            ));
        }
        if ct == b"Caption" {
            caption_positions.push(i);
        }
    }

    if caption_positions.len() > 1 {
        results.push(fail("09-006", "TOC has more than one Caption"));
    }
}

/// Validate that a child type appears inside an appropriate parent.
fn validate_required_parent(parent_type: &[u8], child_type: &[u8], results: &mut Vec<CheckResult>) {
    let child_name = || String::from_utf8_lossy(child_type).into_owned();
    let parent_name = || String::from_utf8_lossy(parent_type).into_owned();

    match child_type {
        // TR must be inside Table, THead, TBody, or TFoot
        b"TR" if !matches!(parent_type, b"Table" | b"THead" | b"TBody" | b"TFoot") => {
            results.push(fail(
                "09-006",
                &format!(
                    "TR is enclosed in {} — must be in Table/THead/TBody/TFoot",
                    parent_name()
                ),
            ));
        }
        // TH/TD must be inside TR
        b"TH" | b"TD" if parent_type != b"TR" => {
            results.push(fail(
                "09-006",
                &format!(
                    "{} is enclosed in {} — must be in TR",
                    child_name(),
                    parent_name()
                ),
            ));
        }
        // THead/TBody/TFoot must be inside Table
        b"THead" | b"TBody" | b"TFoot" if parent_type != b"Table" => {
            results.push(fail(
                "09-006",
                &format!(
                    "{} is enclosed in {} — must be in Table",
                    child_name(),
                    parent_name()
                ),
            ));
        }
        // LI must be inside L
        b"LI" if parent_type != b"L" => {
            results.push(fail(
                "09-006",
                &format!("LI is enclosed in {} — must be in L (List)", parent_name()),
            ));
        }
        // LBody must be inside LI
        b"LBody" if parent_type != b"LI" => {
            results.push(fail(
                "09-006",
                &format!("LBody is enclosed in {} — must be in LI", parent_name()),
            ));
        }
        // TOCI must be inside TOC
        b"TOCI" if parent_type != b"TOC" => {
            results.push(fail(
                "09-006",
                &format!("TOCI is enclosed in {} — must be in TOC", parent_name()),
            ));
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

fn fail(rule_id: &str, message: &str) -> CheckResult {
    CheckResult {
        rule_id: rule_id.to_string(),
        checkpoint: 9,
        description: message.to_string(),
        severity: Severity::Error,
        outcome: CheckOutcome::Fail {
            message: message.to_string(),
            location: None,
        },
    }
}
