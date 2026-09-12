use crate::checks::Check;
use crate::checks::namespaces;
use crate::document::HornDocument;
use crate::model::{CheckOutcome, CheckResult, Location, Severity, Standard};
use anyhow::Result;
use std::collections::HashMap;

/// Information about an OBJR entry found in the structure tree.
#[allow(clippy::struct_field_names)]
struct ObjrInfo {
    /// The struct elem type (/S) of the parent element containing this OBJR
    parent_type: Vec<u8>,
    /// The parent's type after role mapping (document `/RoleMap` and PDF 2.0 `/RoleMapNS`)
    parent_std_type: Vec<u8>,
    /// Whether the parent struct elem has a non-empty /Alt
    parent_has_alt: bool,
    /// Whether the parent struct elem has content of its own (marked content or child elements)
    parent_has_content: bool,
    /// Whether the parent struct elem has a non-empty /TU (for Form fields)
    #[allow(dead_code)]
    parent_has_tu: bool,
}

/// Facts about the structure element enclosing an OBJR, passed down the walk.
#[derive(Clone, Default)]
struct ParentCtx {
    /// The struct elem type (/S) as written
    type_name: Vec<u8>,
    /// The type after role mapping (document `/RoleMap` and PDF 2.0 `/RoleMapNS`)
    std_type: Vec<u8>,
    has_alt: bool,
    has_content: bool,
    has_tu: bool,
}

/// Annotation-to-structure tree cross-reference validation.
///
/// PDF/UA-1 requires that all annotations (except Popup and `PrinterMark`) are
/// represented in the structure tree via OBJR (object reference) entries. This
/// check walks the structure tree to collect all OBJR references with metadata
/// about their parent elements, then validates annotation-structure associations.
pub struct AnnotStructChecks;

impl Check for AnnotStructChecks {
    fn id(&self) -> &'static str {
        "28-annot-struct"
    }

    fn checkpoint(&self) -> u8 {
        28
    }

    fn rules(&self) -> &'static [&'static str] {
        &[
            "28-002", "28-004", "28-005", "28-006", "28-007", "28-010", "28-011", "28-012",
            "28-014", "28-015", "28-016", "28-017", "28-018",
        ]
    }

    fn description(&self) -> &'static str {
        "Annotations: structure tree association for all annotations"
    }

    #[allow(clippy::too_many_lines)]
    fn run(&self, doc: &mut HornDocument) -> Result<Vec<CheckResult>> {
        let mut results = Vec::new();
        let standard = doc.standard();
        let Ok(catalog) = doc.raw_catalog() else {
            return Ok(results);
        };
        let lopdf_doc = doc.lopdf();

        // Step 1: Walk the structure tree, collecting OBJR entries with parent info
        let struct_tree = match catalog.get(b"StructTreeRoot") {
            Ok(obj) => {
                let ref_id = obj.as_reference().ok();
                ref_id
                    .and_then(|r| lopdf_doc.get_object(r).ok())
                    .and_then(|o| o.as_dict().ok())
            }
            Err(_) => None,
        };

        let Some(tree) = struct_tree else {
            return Ok(results);
        };

        // Role maps (document /RoleMap and PDF 2.0 /RoleMapNS) resolve custom parent types
        let role_map = namespaces::role_map(lopdf_doc, tree);
        let mut objr_map: HashMap<lopdf::ObjectId, ObjrInfo> = HashMap::new();
        let root_ctx = ParentCtx::default();
        collect_objr_with_info(lopdf_doc, tree, &root_ctx, role_map, &mut objr_map, 0);

        // Step 2: Process all annotations from all pages
        let pages = lopdf_doc.get_pages();
        let mut total_annots = 0u32;
        let mut unlinked_annots = 0u32;

        for (page_num, page_id) in &pages {
            let Ok(page_dict) = lopdf_doc.get_dictionary(*page_id) else {
                continue;
            };

            let annots_array = match page_dict.get(b"Annots") {
                Ok(obj) => {
                    if let Ok(arr) = obj.as_array() {
                        arr.clone()
                    } else if let Ok(ref_id) = obj.as_reference() {
                        if let Ok(resolved) = lopdf_doc.get_object(ref_id) {
                            resolved.as_array().cloned().unwrap_or_default()
                        } else {
                            continue;
                        }
                    } else {
                        continue;
                    }
                }
                Err(_) => continue,
            };

            for annot_ref in &annots_array {
                let Ok(annot_id) = annot_ref.as_reference() else {
                    continue;
                };

                let annot_dict = match lopdf_doc.get_object(annot_id) {
                    Ok(obj) => match obj.as_dict() {
                        Ok(d) => d,
                        Err(_) => continue,
                    },
                    Err(_) => continue,
                };

                let subtype = annot_dict
                    .get(b"Subtype")
                    .ok()
                    .and_then(|o| o.as_name().ok())
                    .unwrap_or(b"");

                // 28-007: TrapNet annotations are forbidden in PDF/UA
                if subtype == b"TrapNet" {
                    results.push(annot_fail(
                        "28-007",
                        *page_num,
                        &format!(
                            "TrapNet annotation (obj {}.{}) is not permitted in PDF/UA-1",
                            annot_id.0, annot_id.1
                        ),
                        "/TrapNet",
                    ));
                    continue;
                }

                // PrinterMark annotations must NOT be in the structure tree
                if subtype == b"PrinterMark" {
                    if objr_map.contains_key(&annot_id) {
                        results.push(annot_fail(
                            "28-017", *page_num,
                            &format!("PrinterMark annotation (obj {}.{}) has OBJR in structure tree — must be artifact only", annot_id.0, annot_id.1),
                            "/PrinterMark",
                        ));
                    }
                    // 28-018: the appearance stream must be marked as an artifact
                    if let Some(unmarked) = printer_mark_unmarked_content(lopdf_doc, annot_dict) {
                        results.push(annot_fail(
                            "28-018", *page_num,
                            &format!("PrinterMark annotation (obj {}.{}) appearance stream has {unmarked} content operation(s) outside /Artifact marked content", annot_id.0, annot_id.1),
                            "/PrinterMark",
                        ));
                    }
                    continue;
                }

                // Popup annotations don't need struct association
                if subtype == b"Popup" {
                    continue;
                }

                // Screen annotations: check media clip CT and Alt
                if subtype == b"Screen" {
                    check_screen_annotation(lopdf_doc, annot_dict, *page_num, &mut results);
                }

                // FileAttachment: check FileSpec /F and /UF
                if subtype == b"FileAttachment" {
                    check_file_attachment(lopdf_doc, annot_dict, *page_num, &mut results);
                }

                total_annots += 1;
                let results_before = results.len();

                // 28-002: Check if this annotation is referenced in the structure tree
                if let Some(info) = objr_map.get(&annot_id) {
                    // Validate parent struct elem type
                    check_objr_parent_type(
                        lopdf_doc,
                        annot_dict,
                        subtype,
                        info,
                        standard,
                        *page_num,
                        &mut results,
                    );

                    // Check accessible text for Annot struct elems
                    check_annot_accessible_text(
                        lopdf_doc,
                        annot_dict,
                        subtype,
                        info,
                        standard,
                        *page_num,
                        &mut results,
                    );
                } else {
                    unlinked_annots += 1;
                    let type_name = String::from_utf8_lossy(subtype);
                    let (rule, expected) = match subtype {
                        b"Widget" => ("28-010", "Form"),
                        b"Link" => ("28-011", "Link"),
                        _ => ("28-002", "Annot"),
                    };
                    results.push(annot_fail(
                        rule,
                        *page_num,
                        &format!(
                            "/{type_name} annotation (obj {}.{}) has no OBJR in the structure tree — must be nested in a /{expected} structure element",
                            annot_id.0, annot_id.1
                        ),
                        &format!("/{type_name}"),
                    ));
                }

                // 28-006: an annotation whose subtype is not defined in ISO 32000 must
                // still satisfy 7.18.1 (28-002 / 28-004). Report it under its own index
                // so the failure is attributable to the non-standard subtype.
                if !is_iso32000_annotation_subtype(subtype) && results.len() > results_before {
                    let type_name = String::from_utf8_lossy(subtype);
                    results.push(annot_fail(
                        "28-006",
                        *page_num,
                        &format!(
                            "/{type_name} annotation (obj {}.{}) uses a subtype not defined in ISO 32000 and does not meet 7.18.1",
                            annot_id.0, annot_id.1
                        ),
                        &format!("/{type_name}"),
                    ));
                }
            }
        }

        if total_annots > 0 && unlinked_annots == 0 {
            results.push(CheckResult {
                rule_id: "28-002".to_string(),
                checkpoint: 28,
                description: format!(
                    "All {total_annots} annotation(s) are linked to the structure tree"
                ),
                severity: Severity::Info,
                outcome: CheckOutcome::Pass,
            });
        }

        Ok(results)
    }
}

/// Walk the structure tree collecting OBJR entries with parent struct elem info.
fn collect_objr_with_info(
    doc: &lopdf::Document,
    dict: &lopdf::Dictionary,
    parent: &ParentCtx,
    role_map: Option<&lopdf::Dictionary>,
    map: &mut HashMap<lopdf::ObjectId, ObjrInfo>,
    depth: usize,
) {
    if depth > 100 {
        return;
    }

    // Check if this dict IS an OBJR entry
    let is_objr = dict.get(b"Type").ok().and_then(|o| o.as_name().ok()) == Some(b"OBJR");

    if is_objr {
        if let Ok(obj_ref) = dict.get(b"Obj") {
            if let Ok(ref_id) = obj_ref.as_reference() {
                map.insert(
                    ref_id,
                    ObjrInfo {
                        parent_type: parent.type_name.clone(),
                        parent_std_type: parent.std_type.clone(),
                        parent_has_alt: parent.has_alt,
                        parent_has_content: parent.has_content,
                        parent_has_tu: parent.has_tu,
                    },
                );
            }
        }
        return;
    }

    // Determine this element's type and attributes for passing to children.
    // The structure tree root has no /S and passes its (empty) context through.
    let current = if dict.get(b"S").is_ok() {
        let qt = namespaces::resolve_qualified_type(doc, role_map, dict);
        ParentCtx {
            type_name: dict
                .get(b"S")
                .ok()
                .and_then(|o| o.as_name().ok())
                .unwrap_or(b"")
                .to_vec(),
            std_type: qt.name,
            has_alt: has_nonempty_string(dict, b"Alt"),
            has_content: element_has_content(doc, dict),
            has_tu: has_nonempty_string(dict, b"TU"),
        }
    } else {
        parent.clone()
    };

    // Walk /K children
    let Ok(kids) = dict.get(b"K") else { return };

    let mut visit = |child_dict: &lopdf::Dictionary| {
        collect_objr_with_info(doc, child_dict, &current, role_map, map, depth + 1);
    };

    match kids {
        lopdf::Object::Array(arr) => {
            for kid in arr {
                match kid {
                    lopdf::Object::Reference(ref_id) => {
                        if let Ok(obj) = doc.get_object(*ref_id) {
                            if let Ok(d) = obj.as_dict() {
                                visit(d);
                            }
                        }
                    }
                    lopdf::Object::Dictionary(d) => visit(d),
                    _ => {}
                }
            }
        }
        lopdf::Object::Reference(ref_id) => {
            if let Ok(obj) = doc.get_object(*ref_id) {
                if let Ok(d) = obj.as_dict() {
                    visit(d);
                }
            }
        }
        lopdf::Object::Dictionary(d) => visit(d),
        _ => {}
    }
}

/// Marked content (MCID / MCR) or child structure elements directly inside an element.
fn element_has_content(doc: &lopdf::Document, dict: &lopdf::Dictionary) -> bool {
    let Ok(kids) = dict.get(b"K") else {
        return false;
    };
    let items: Vec<&lopdf::Object> = match kids {
        lopdf::Object::Array(arr) => arr.iter().collect(),
        other => vec![other],
    };
    let is_content = |d: &lopdf::Dictionary| {
        let ty = d.get(b"Type").ok().and_then(|o| o.as_name().ok());
        ty == Some(b"MCR") || (ty != Some(b"OBJR") && d.get(b"S").is_ok())
    };
    items.iter().any(|kid| match kid {
        lopdf::Object::Integer(_) => true,
        lopdf::Object::Dictionary(d) => is_content(d),
        lopdf::Object::Reference(id) => doc
            .get_object(*id)
            .ok()
            .and_then(|o| o.as_dict().ok())
            .is_some_and(is_content),
        _ => false,
    })
}

/// Check that the OBJR parent struct elem type matches the annotation subtype.
fn check_objr_parent_type(
    _doc: &lopdf::Document,
    _annot_dict: &lopdf::Dictionary,
    subtype: &[u8],
    info: &ObjrInfo,
    standard: Standard,
    page_num: u32,
    results: &mut Vec<CheckResult>,
) {
    let resolved_type = info.parent_std_type.as_slice();

    // PDF/UA-2: an annotation enclosed in an Artifact structure element is an
    // artifact (ISO 14289-2, 8.9.2.2) and is exempt from the parent-type rules.
    if standard == Standard::Ua2 && resolved_type == b"Artifact" {
        return;
    }

    let (expected, rule) = match subtype {
        b"Link" => (b"Link" as &[u8], "28-011"),
        b"Widget" => (b"Form" as &[u8], "28-010"),
        _ => (b"Annot" as &[u8], "28-002"),
    };
    // PDF/UA-2 also allows links inside a Reference element (ISO 14289-2, 8.2.5.20)
    let ua2_reference =
        standard == Standard::Ua2 && subtype == b"Link" && resolved_type == b"Reference";

    if resolved_type != expected && !ua2_reference {
        let parent_str = String::from_utf8_lossy(&info.parent_type);
        let subtype_str = String::from_utf8_lossy(subtype);
        let expected_str = String::from_utf8_lossy(expected);
        results.push(annot_fail(
            rule, page_num,
            &format!(
                "/{subtype_str} annotation is under /{parent_str} struct elem — should be under /{expected_str}"
            ),
            &format!("/{subtype_str}"),
        ));
    }
}

/// Check accessible text for annotations in the structure tree.
fn check_annot_accessible_text(
    doc: &lopdf::Document,
    annot_dict: &lopdf::Dictionary,
    subtype: &[u8],
    info: &ObjrInfo,
    standard: Standard,
    page_num: u32,
    results: &mut Vec<CheckResult>,
) {
    // PDF/UA-2: annotations enclosed in an Artifact element are artifacts
    if standard == Standard::Ua2 && info.parent_std_type == b"Artifact" {
        return;
    }

    // Skip hidden annotations (F bit 2)
    if let Ok(flags) = annot_dict.get(b"F").and_then(lopdf::Object::as_i64) {
        if flags & 0x02 != 0 {
            return;
        }
    }

    match subtype {
        b"Widget" => {
            // Form fields need an accessible name: /TU on widget (or parent), or /Alt.
            // Skip hidden (F bit 2) or non-visible (no /AP) widgets.
            // Hidden (0x02), NoView (0x20), or Invisible (0x01)
            let is_hidden = annot_dict
                .get(b"F")
                .and_then(lopdf::Object::as_i64)
                .is_ok_and(|f| f & 0x23 != 0)
                || has_inherited_flag_hidden(doc, annot_dict, 10);
            let has_appearance = annot_dict.get(b"AP").is_ok();
            if !is_hidden && has_appearance && !is_zero_size_rect(annot_dict) {
                // /TU is a *field* dictionary entry (ISO 32000-1 Table 220). When the
                // widget is a pure annotation kid (no /T, /FT or /Kids of its own) the
                // field is its /Parent, so a /TU on the widget itself does not count.
                let is_pure_widget = annot_dict.get(b"Parent").is_ok()
                    && annot_dict.get(b"T").is_err()
                    && annot_dict.get(b"FT").is_err()
                    && annot_dict.get(b"Kids").is_err();
                let field_dict = if is_pure_widget {
                    resolve_dict(doc, annot_dict.get(b"Parent").ok()).unwrap_or(annot_dict)
                } else {
                    annot_dict
                };
                let has_tu = has_inherited_key(doc, field_dict, b"TU", 10);
                if !has_tu && !info.parent_has_alt {
                    results.push(annot_fail(
                        "28-005",
                        page_num,
                        "Form field has no /TU (tooltip) and Form struct elem has no /Alt",
                        "/Widget",
                    ));
                }
            }
        }
        b"Link" => {
            // Links need /Contents on the annotation. PDF/UA-2 (ISO 14289-2,
            // 8.2.5.20) takes the accessible text from the content of the
            // enclosing Link/Reference element instead, so /Contents is only
            // required when that element has no content or /Alt of its own.
            let has_contents = annot_dict
                .get(b"Contents")
                .ok()
                .and_then(|o| o.as_str().ok())
                .is_some_and(|s| !s.is_empty());
            let ua2_text_in_structure =
                standard == Standard::Ua2 && (info.parent_has_content || info.parent_has_alt);
            if !has_contents && !ua2_text_in_structure {
                results.push(annot_fail(
                    "28-012",
                    page_num,
                    "Link annotation missing non-empty /Contents for accessible link text",
                    "/Link",
                ));
            }
        }
        _ => {
            // Annotations under /Annot struct elems need accessible text:
            // either /Alt on the struct elem or /Contents on the annotation.
            // Skip if annotation is hidden or has no appearance stream.
            let resolved = info.parent_std_type.as_slice();
            // Hidden (0x02), Invisible (0x01), NoView (0x20), or non-printing without Print flag (0x04)
            let annot_flags = annot_dict
                .get(b"F")
                .and_then(lopdf::Object::as_i64)
                .ok()
                .unwrap_or(0);
            let is_annot_hidden =
                annot_flags & 0x23 != 0 || (annot_flags != 0 && annot_flags & 0x04 == 0); // Has F but no Print flag
            if resolved == b"Annot" && !is_annot_hidden {
                let has_contents = annot_dict
                    .get(b"Contents")
                    .ok()
                    .and_then(|o| o.as_str().ok())
                    .is_some_and(|s| !s.is_empty());
                if !has_contents
                    && !info.parent_has_alt
                    && !matches!(subtype, b"Popup" | b"PrinterMark" | b"TrapNet")
                    && !is_zero_size_rect(annot_dict)
                {
                    let type_str = String::from_utf8_lossy(subtype);
                    results.push(annot_fail(
                            "28-004", page_num,
                            &format!(
                                "/{type_str} annotation has no /Contents and Annot struct elem has no /Alt"
                            ),
                            &format!("/{type_str}"),
                        ));
                }
            }
        }
    }
}

/// 28-018: count painting operations in a `PrinterMark`'s normal appearance
/// stream that are not enclosed in `/Artifact` marked content. Returns `None`
/// when there is no appearance stream to inspect.
fn printer_mark_unmarked_content(doc: &lopdf::Document, annot: &lopdf::Dictionary) -> Option<u32> {
    let ap = annot
        .get(b"AP")
        .ok()
        .and_then(|o| resolve_obj(doc, o))?
        .as_dict()
        .ok()?;
    let normal = ap.get(b"N").ok().and_then(|o| resolve_obj(doc, o))?;
    let stream = if let Ok(s) = normal.as_stream() {
        s
    } else {
        // Appearance sub-dictionary: take the first state
        normal
            .as_dict()
            .ok()?
            .iter()
            .find_map(|(_, v)| resolve_obj(doc, v).and_then(|o| o.as_stream().ok()))?
    };
    let data = stream.decompressed_content().ok()?;
    let content = lopdf::content::Content::decode(&data).ok()?;
    let mut artifact_depth = 0u32;
    let mut mc_depth = 0u32;
    let mut unmarked = 0u32;
    for op in &content.operations {
        match op.operator.as_str() {
            "BMC" | "BDC" => {
                mc_depth += 1;
                if op.operands.first().and_then(|o| o.as_name().ok()) == Some(b"Artifact")
                    || artifact_depth > 0
                {
                    artifact_depth += 1;
                }
            }
            "EMC" => {
                mc_depth = mc_depth.saturating_sub(1);
                artifact_depth = artifact_depth.saturating_sub(1);
            }
            "Tj" | "TJ" | "'" | "\"" | "Do" | "f" | "F" | "f*" | "B" | "B*" | "b" | "b*" | "S"
            | "s" | "sh" | "BI" | "EI"
                if artifact_depth == 0 =>
            {
                unmarked += 1;
            }
            _ => {}
        }
    }
    let _ = mc_depth;
    (unmarked > 0).then_some(unmarked)
}

fn resolve_obj<'a>(doc: &'a lopdf::Document, obj: &'a lopdf::Object) -> Option<&'a lopdf::Object> {
    match obj {
        lopdf::Object::Reference(id) => doc.get_object(*id).ok(),
        other => Some(other),
    }
}

/// Check Screen annotations for required media clip properties.
fn check_screen_annotation(
    doc: &lopdf::Document,
    annot_dict: &lopdf::Dictionary,
    page_num: u32,
    results: &mut Vec<CheckResult>,
) {
    // Follow the rendition action chain: /A -> action dict -> /R -> rendition -> /C -> media clip
    let action = annot_dict
        .get(b"A")
        .ok()
        .and_then(|o| {
            if let Ok(r) = o.as_reference() {
                doc.get_object(r).ok()
            } else {
                Some(o)
            }
        })
        .and_then(|o| o.as_dict().ok());

    let Some(action_dict) = action else { return };

    let rendition = action_dict
        .get(b"R")
        .ok()
        .and_then(|o| {
            if let Ok(r) = o.as_reference() {
                doc.get_object(r).ok()
            } else {
                Some(o)
            }
        })
        .and_then(|o| o.as_dict().ok());

    let Some(rendition_dict) = rendition else {
        return;
    };

    let media_clip = rendition_dict
        .get(b"C")
        .ok()
        .and_then(|o| {
            if let Ok(r) = o.as_reference() {
                doc.get_object(r).ok()
            } else {
                Some(o)
            }
        })
        .and_then(|o| o.as_dict().ok());

    let Some(clip_dict) = media_clip else { return };

    // Check /CT (content type) on media clip
    if clip_dict.get(b"CT").is_err() {
        results.push(annot_fail(
            "28-014",
            page_num,
            "Screen annotation media clip missing /CT (content type)",
            "/Screen",
        ));
    }

    // Check /Alt on media clip — must be an array with non-empty text entries
    match clip_dict.get_deref(b"Alt", doc) {
        Ok(alt_obj) => {
            if let Ok(arr) = alt_obj.as_array() {
                // Alt array format: [lang1, text1, lang2, text2, ...]
                // Text entries are at odd indices (1, 3, 5, ...)
                let has_nonempty_text = arr
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| i % 2 == 1)
                    .any(|(_, item)| item.as_str().is_ok_and(|s| !s.is_empty()));
                if !has_nonempty_text {
                    results.push(annot_fail(
                        "28-015",
                        page_num,
                        "Screen annotation media clip /Alt has no non-empty text entries",
                        "/Screen",
                    ));
                }
            } else {
                results.push(annot_fail(
                    "28-015",
                    page_num,
                    "Screen annotation media clip /Alt is not an array of language/text pairs",
                    "/Screen",
                ));
            }
        }
        Err(_) => {
            results.push(annot_fail(
                "28-015",
                page_num,
                "Screen annotation media clip missing /Alt array",
                "/Screen",
            ));
        }
    }
}

/// Check `FileAttachment` annotations for required `FileSpec` entries.
fn check_file_attachment(
    doc: &lopdf::Document,
    annot_dict: &lopdf::Dictionary,
    page_num: u32,
    results: &mut Vec<CheckResult>,
) {
    let fs = annot_dict
        .get(b"FS")
        .ok()
        .and_then(|o| {
            if let Ok(r) = o.as_reference() {
                doc.get_object(r).ok()
            } else {
                Some(o)
            }
        })
        .and_then(|o| o.as_dict().ok());

    let Some(fs_dict) = fs else { return };

    // /F must exist and be non-empty
    let has_filename = fs_dict
        .get(b"F")
        .ok()
        .and_then(|o| o.as_str().ok())
        .is_some_and(|s| !s.is_empty());

    if !has_filename {
        results.push(annot_fail(
            "28-016",
            page_num,
            "FileAttachment FileSpec missing or has empty /F entry",
            "/FileAttachment",
        ));
    }

    // /UF must exist and be non-empty
    let has_unicode_filename = fs_dict
        .get(b"UF")
        .ok()
        .and_then(|o| o.as_str().ok())
        .is_some_and(|s| !s.is_empty());

    if !has_unicode_filename {
        results.push(annot_fail(
            "28-016",
            page_num,
            "FileAttachment FileSpec missing or has empty /UF entry",
            "/FileAttachment",
        ));
    }
}

/// Annotation subtypes defined in ISO 32000-1 Table 169 (plus the ISO 32000-2
/// additions `RichMedia` and Projection).
fn is_iso32000_annotation_subtype(subtype: &[u8]) -> bool {
    matches!(
        subtype,
        b"Text"
            | b"Link"
            | b"FreeText"
            | b"Line"
            | b"Square"
            | b"Circle"
            | b"Polygon"
            | b"PolyLine"
            | b"Highlight"
            | b"Underline"
            | b"Squiggly"
            | b"StrikeOut"
            | b"Stamp"
            | b"Caret"
            | b"Ink"
            | b"Popup"
            | b"FileAttachment"
            | b"Sound"
            | b"Movie"
            | b"Widget"
            | b"Screen"
            | b"PrinterMark"
            | b"TrapNet"
            | b"Watermark"
            | b"3D"
            | b"Redact"
            | b"RichMedia"
            | b"Projection"
    )
}

/// Resolve an object (direct dictionary or reference) to a dictionary.
fn resolve_dict<'a>(
    doc: &'a lopdf::Document,
    obj: Option<&'a lopdf::Object>,
) -> Option<&'a lopdf::Dictionary> {
    match obj? {
        lopdf::Object::Reference(ref_id) => doc.get_object(*ref_id).ok()?.as_dict().ok(),
        lopdf::Object::Dictionary(d) => Some(d),
        _ => None,
    }
}

/// Check if a dictionary key exists with a non-empty value on the given dict
/// or any ancestor via /Parent chain.
fn has_inherited_key(
    doc: &lopdf::Document,
    dict: &lopdf::Dictionary,
    key: &[u8],
    max_depth: u8,
) -> bool {
    // Check if key exists and has a non-empty value
    if let Ok(obj) = dict.get(key) {
        let is_empty = obj.as_str().is_ok_and(<[u8]>::is_empty);
        if !is_empty {
            return true;
        }
    }
    if max_depth == 0 {
        return false;
    }
    if let Ok(parent_obj) = dict.get(b"Parent") {
        let parent_dict = match parent_obj {
            lopdf::Object::Reference(ref_id) => {
                doc.get_object(*ref_id).ok().and_then(|o| o.as_dict().ok())
            }
            lopdf::Object::Dictionary(d) => Some(d),
            _ => None,
        };
        if let Some(parent) = parent_dict {
            return has_inherited_key(doc, parent, key, max_depth - 1);
        }
    }
    false
}

/// Check if an annotation has a zero-size or degenerate /Rect (effectively invisible).
/// A zero-size rect like [800, 800, 800, 800] means the annotation is off-page or invisible,
/// so it doesn't require accessible text.
fn is_zero_size_rect(dict: &lopdf::Dictionary) -> bool {
    let Ok(rect_obj) = dict.get(b"Rect") else {
        return false;
    };
    let Ok(arr) = rect_obj.as_array() else {
        return false;
    };
    if arr.len() != 4 {
        return false;
    }
    let coords: Vec<f64> = arr
        .iter()
        .filter_map(|o| match o {
            lopdf::Object::Real(f) => Some(f64::from(*f)),
            #[allow(clippy::cast_precision_loss)]
            lopdf::Object::Integer(i) => Some(*i as f64),
            _ => None,
        })
        .collect();
    if coords.len() != 4 {
        return false;
    }
    let width = (coords[2] - coords[0]).abs();
    let height = (coords[3] - coords[1]).abs();
    width < 0.001 || height < 0.001
}

/// Check if a dictionary has a non-empty string value for a key.
/// Handles both byte strings and name objects.
fn has_nonempty_string(dict: &lopdf::Dictionary, key: &[u8]) -> bool {
    let Ok(obj) = dict.get(key) else { return false };
    // Try as string (byte string / text string)
    if let Ok(s) = obj.as_str() {
        return !s.is_empty();
    }
    // Try as name
    if let Ok(n) = obj.as_name() {
        return !n.is_empty();
    }
    // The key exists but isn't a string — still counts as "present"
    // (e.g., could be a reference to a string object)
    true
}

/// Check if an annotation or its parent has the Hidden flag (F bit 2).
fn has_inherited_flag_hidden(
    doc: &lopdf::Document,
    dict: &lopdf::Dictionary,
    max_depth: u8,
) -> bool {
    if let Ok(f) = dict.get(b"F").and_then(lopdf::Object::as_i64) {
        if f & 0x02 != 0 {
            return true;
        }
    }
    if max_depth == 0 {
        return false;
    }
    if let Ok(parent_obj) = dict.get(b"Parent") {
        let parent_dict = match parent_obj {
            lopdf::Object::Reference(ref_id) => {
                doc.get_object(*ref_id).ok().and_then(|o| o.as_dict().ok())
            }
            lopdf::Object::Dictionary(d) => Some(d),
            _ => None,
        };
        if let Some(parent) = parent_dict {
            return has_inherited_flag_hidden(doc, parent, max_depth - 1);
        }
    }
    false
}

fn annot_fail(rule_id: &str, page_num: u32, message: &str, element: &str) -> CheckResult {
    CheckResult {
        rule_id: rule_id.to_string(),
        checkpoint: 28,
        description: format!("Page {page_num}: {message}"),
        severity: Severity::Error,
        outcome: CheckOutcome::Fail {
            message: format!("Page {page_num}: {message}"),
            location: Some(Location {
                page: Some(page_num),
                element: Some(element.to_string()),
            }),
        },
    }
}
