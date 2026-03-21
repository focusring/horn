use crate::checks::Check;
use crate::document::HornDocument;
use crate::model::{CheckOutcome, CheckResult, Severity};
use anyhow::Result;

/// Checkpoint 11: Natural Language.
///
/// PDF/UA-1 requires proper language identification at multiple levels:
/// - 11-001: Document-level `/Lang` must be present (handled by metadata.rs)
/// - 11-002: All `/Lang` values must be valid BCP 47 tags
/// - 11-005: Elements with `/Alt` should have language context
/// - 11-006: Elements with `/ActualText` should have language context
/// - 11-007: Elements with `/E` (expansion) should have language context
pub struct LanguageChecks;

impl Check for LanguageChecks {
    fn id(&self) -> &'static str {
        "11-language"
    }

    fn checkpoint(&self) -> u8 {
        11
    }

    fn description(&self) -> &'static str {
        "Natural language: BCP 47 validation, language identification"
    }

    fn run(&self, doc: &mut HornDocument) -> Result<Vec<CheckResult>> {
        let mut results = Vec::new();
        let Ok(catalog) = doc.raw_catalog() else {
            return Ok(results);
        };
        let lopdf_doc = doc.lopdf();

        // 11-002: Validate document-level Lang
        // Use get_deref to follow indirect references, and accept both String and Name types.
        let doc_lang = catalog
            .get_deref(b"Lang", lopdf_doc)
            .ok()
            .and_then(|o| o.as_str().or_else(|_| o.as_name()).ok())
            .map(<[u8]>::to_vec);

        if let Some(ref lang) = doc_lang {
            if !lang.is_empty() && !is_valid_bcp47(lang) {
                let display = String::from_utf8_lossy(lang);
                results.push(fail(
                    "11-002",
                    &format!("Document /Lang \"{display}\" is not a valid BCP 47 language tag"),
                ));
            }
        }

        // Check text strings that need language context (outside struct tree).
        // Only flag when there's no catalog /Lang AND no struct-level /Lang.
        // When struct elements provide /Lang, text strings in the document
        // are considered to have language context by veraPDF's interpretation.
        let has_catalog_lang = doc_lang.as_ref().is_some_and(|l| !l.is_empty());
        let has_any_struct_lang = has_catalog_lang
            || get_struct_tree(catalog, lopdf_doc)
                .is_some_and(|st| has_struct_level_lang(lopdf_doc, st, 0));

        if !has_catalog_lang && !has_any_struct_lang {
            check_outline_language(lopdf_doc, catalog, &mut results);
            check_annotation_text_language(lopdf_doc, &mut results);
            check_dc_title_language(lopdf_doc, catalog, &mut results);
        }

        // Walk structure tree for language checks
        let Some(struct_tree) = get_struct_tree(catalog, lopdf_doc) else {
            return Ok(results);
        };

        let mut invalid_langs: Vec<String> = Vec::new();
        let mut empty_langs = 0;
        let mut missing_lang_on_alt = 0;
        let mut missing_lang_on_actual = 0;
        let mut missing_lang_on_expansion = 0;

        // First pass: collect all valid /Lang tags from the structure tree.
        // In PDF, /Lang on a parent element is inherited by all descendants,
        // so if any ancestor has /Lang, children have language context.
        // We track this with a stack of inherited lang values.
        walk_struct_tree_with_lang(
            lopdf_doc,
            struct_tree,
            doc_lang.as_ref(),
            &mut invalid_langs,
            &mut empty_langs,
            &mut missing_lang_on_alt,
            &mut missing_lang_on_actual,
            &mut missing_lang_on_expansion,
            0,
        );

        // Emit results for 11-002
        if !invalid_langs.is_empty() {
            for lang in &invalid_langs {
                results.push(fail(
                    "11-002",
                    &format!("/Lang \"{lang}\" is not a valid BCP 47 language tag"),
                ));
            }
        }

        if empty_langs > 0 {
            results.push(fail(
                "11-002",
                &format!("{empty_langs} structure element(s) have empty /Lang values"),
            ));
        }

        if invalid_langs.is_empty() && empty_langs == 0 {
            // Check if there were any langs to validate
            let has_any_lang = doc_lang.is_some();
            if has_any_lang {
                results.push(pass("11-002", "All /Lang values are valid BCP 47 tags"));
            }
        }

        // Emit results for 11-005, 11-006, 11-007
        if missing_lang_on_alt > 0 {
            results.push(fail(
                "11-005",
                &format!(
                    "{missing_lang_on_alt} element(s) with /Alt text have no language context"
                ),
            ));
        }

        if missing_lang_on_actual > 0 {
            results.push(fail(
                "11-006",
                &format!(
                    "{missing_lang_on_actual} element(s) with /ActualText have no language context"
                ),
            ));
        }

        if missing_lang_on_expansion > 0 {
            results.push(fail(
                "11-007",
                &format!(
                    "{missing_lang_on_expansion} element(s) with /E (expansion text) have no language context"
                ),
            ));
        }

        Ok(results)
    }
}

/// Validate a BCP 47 language tag (RFC 5646).
///
/// A simplified validation that covers the cases encountered in PDF/UA:
/// - Must not be empty
/// - Must not start with `-`
/// - Primary subtag: 1-8 ASCII letters (single char reserved but valid)
/// - Subsequent subtags: 1-8 ASCII alphanumeric characters
/// - Subtags separated by `-`
///
/// We also accept UTF-16BE encoded strings (BOM prefix `\xFE\xFF`) by
/// decoding them first.
fn is_valid_bcp47(tag: &[u8]) -> bool {
    // Handle UTF-16BE encoded strings (PDF BOM: FE FF)
    let decoded: Vec<u8>;
    let tag_bytes = if tag.len() >= 2 && tag[0] == 0xFE && tag[1] == 0xFF {
        // Decode UTF-16BE to ASCII-ish bytes
        decoded = decode_utf16be(tag);
        &decoded
    } else {
        tag
    };

    if tag_bytes.is_empty() {
        return false;
    }

    // Must not start with hyphen
    if tag_bytes[0] == b'-' {
        return false;
    }

    let subtags: Vec<&[u8]> = tag_bytes.split(|&b| b == b'-').collect();
    if subtags.is_empty() {
        return false;
    }

    // Primary subtag: 1-8 ASCII letters
    let primary = subtags[0];
    if primary.is_empty() || primary.len() > 8 {
        return false;
    }
    if !primary.iter().all(u8::is_ascii_alphabetic) {
        return false;
    }

    // Subsequent subtags: 1-8 ASCII alphanumeric
    for subtag in &subtags[1..] {
        if subtag.is_empty() || subtag.len() > 8 {
            return false;
        }
        if !subtag.iter().all(u8::is_ascii_alphanumeric) {
            return false;
        }
    }

    true
}

/// Decode a UTF-16BE byte string to ASCII bytes.
/// Skips the BOM (FE FF) and converts each 16-bit code unit to a byte
/// if it's in the ASCII range.
fn decode_utf16be(data: &[u8]) -> Vec<u8> {
    let mut result = Vec::new();
    let start = if data.len() >= 2 && data[0] == 0xFE && data[1] == 0xFF {
        2
    } else {
        0
    };
    let mut i = start;
    while i + 1 < data.len() {
        let hi = data[i];
        let lo = data[i + 1];
        if hi == 0 && lo > 0 {
            result.push(lo);
        } else if hi > 0 {
            // Non-ASCII character — include as-is for length check
            result.push(lo);
        }
        i += 2;
    }
    result
}

/// Walk the structure tree tracking inherited /Lang for proper context detection.
///
/// PDF /Lang is inherited: if a parent structure element has /Lang, all descendants
/// inherit it. We pass the inherited lang down the tree so that elements with
/// /Alt, /`ActualText`, or /E are correctly evaluated against their ancestor context.
#[allow(clippy::too_many_arguments)]
fn walk_struct_tree_with_lang(
    doc: &lopdf::Document,
    dict: &lopdf::Dictionary,
    inherited_lang: Option<&Vec<u8>>,
    invalid_langs: &mut Vec<String>,
    empty_langs: &mut usize,
    missing_lang_on_alt: &mut usize,
    missing_lang_on_actual: &mut usize,
    missing_lang_on_expansion: &mut usize,
    depth: usize,
) {
    if depth > 100 {
        return;
    }

    // Determine effective lang: this element's /Lang overrides inherited
    let elem_lang = dict
        .get(b"Lang")
        .ok()
        .and_then(|o| o.as_str().ok())
        .map(<[u8]>::to_vec);

    // Validate BCP 47 on every /Lang in the struct tree
    if let Some(ref lang) = elem_lang {
        if lang.is_empty() {
            *empty_langs += 1;
        } else if !is_valid_bcp47(lang) {
            let display = String::from_utf8_lossy(lang).into_owned();
            invalid_langs.push(display);
        }
    }

    // Effective lang: element's own /Lang, or inherited from parent/doc
    let effective_lang = if elem_lang.as_ref().is_some_and(|l| !l.is_empty()) {
        elem_lang.as_ref()
    } else {
        inherited_lang
    };

    let has_lang_context = effective_lang.is_some_and(|l| !l.is_empty() && is_valid_bcp47(l));

    // Only check structure elements (have /S key)
    if dict.get(b"S").is_ok() {
        if dict.get(b"Alt").is_ok() && !has_lang_context {
            *missing_lang_on_alt += 1;
        }
        if dict.get(b"ActualText").is_ok() && !has_lang_context {
            *missing_lang_on_actual += 1;
        }
        if dict.get(b"E").is_ok() && !has_lang_context {
            *missing_lang_on_expansion += 1;
        }
    }

    // Recurse into children, passing effective lang down
    let Ok(kids) = dict.get(b"K") else { return };

    let mut visit_child = |child_dict: &lopdf::Dictionary| {
        walk_struct_tree_with_lang(
            doc,
            child_dict,
            effective_lang,
            invalid_langs,
            empty_langs,
            missing_lang_on_alt,
            missing_lang_on_actual,
            missing_lang_on_expansion,
            depth + 1,
        );
    };

    match kids {
        lopdf::Object::Array(arr) => {
            for kid in arr {
                if let Ok(kid_ref) = kid.as_reference() {
                    if let Ok(kid_obj) = doc.get_object(kid_ref) {
                        if let Ok(kid_dict) = kid_obj.as_dict() {
                            visit_child(kid_dict);
                        }
                    }
                } else if let Ok(kid_dict) = kid.as_dict() {
                    visit_child(kid_dict);
                }
            }
        }
        lopdf::Object::Reference(ref_id) => {
            if let Ok(obj) = doc.get_object(*ref_id) {
                if let Ok(d) = obj.as_dict() {
                    visit_child(d);
                }
            }
        }
        lopdf::Object::Dictionary(d) => {
            visit_child(d);
        }
        _ => {}
    }
}

/// Check if any element in the structure tree has a /Lang attribute.
fn has_struct_level_lang(doc: &lopdf::Document, dict: &lopdf::Dictionary, depth: usize) -> bool {
    if depth > 10 {
        return false;
    }
    if let Ok(lang) = dict.get(b"Lang") {
        if lang.as_str().is_ok_and(|s| !s.is_empty()) {
            return true;
        }
    }
    let Ok(kids) = dict.get(b"K") else {
        return false;
    };
    match kids {
        lopdf::Object::Array(arr) => arr.iter().any(|kid| {
            if let Ok(ref_id) = kid.as_reference() {
                doc.get_object(ref_id)
                    .ok()
                    .and_then(|o| o.as_dict().ok())
                    .is_some_and(|d| has_struct_level_lang(doc, d, depth + 1))
            } else if let Ok(d) = kid.as_dict() {
                has_struct_level_lang(doc, d, depth + 1)
            } else {
                false
            }
        }),
        lopdf::Object::Reference(ref_id) => doc
            .get_object(*ref_id)
            .ok()
            .and_then(|o| o.as_dict().ok())
            .is_some_and(|d| has_struct_level_lang(doc, d, depth + 1)),
        _ => false,
    }
}

/// 02-001: Outline entries (bookmarks) with /Title text must have language context.
///
/// When there's no catalog /Lang, outline text strings have no language specification,
/// making them inaccessible to assistive technologies that need to know the language
/// for proper text-to-speech rendering.
fn check_outline_language(
    doc: &lopdf::Document,
    catalog: &lopdf::Dictionary,
    results: &mut Vec<CheckResult>,
) {
    let Ok(outlines_obj) = catalog.get(b"Outlines") else {
        return;
    };
    let outlines_ref = outlines_obj.as_reference().ok();
    let outlines = outlines_ref
        .and_then(|r| doc.get_object(r).ok())
        .and_then(|o| o.as_dict().ok());
    let Some(outlines_dict) = outlines else {
        return;
    };

    // Check if any outline item has /Title (they almost always do)
    let has_titles = has_outline_titles(doc, outlines_dict, 0);
    if has_titles {
        results.push(fail(
            "02-001",
            "Outline entries have /Title text but no language context (catalog /Lang is missing)",
        ));
    }
}

fn has_outline_titles(doc: &lopdf::Document, node: &lopdf::Dictionary, depth: usize) -> bool {
    if depth > 50 {
        return false;
    }
    let Ok(first_obj) = node.get(b"First") else {
        return false;
    };
    let first_ref = first_obj.as_reference().ok();
    let first = first_ref
        .and_then(|r| doc.get_object(r).ok())
        .and_then(|o| o.as_dict().ok());
    let Some(item) = first else {
        return false;
    };

    // Check this item for /Title
    if item.get(b"Title").is_ok() {
        return true;
    }

    // Check siblings via /Next chain
    let mut current = item;
    loop {
        let Ok(next_obj) = current.get(b"Next") else {
            break;
        };
        let next_ref = next_obj.as_reference().ok();
        let next = next_ref
            .and_then(|r| doc.get_object(r).ok())
            .and_then(|o| o.as_dict().ok());
        let Some(next_dict) = next else {
            break;
        };
        if next_dict.get(b"Title").is_ok() {
            return true;
        }
        current = next_dict;
    }

    false
}

/// 02-002: Annotation text strings (/Contents, /TU) need language context.
///
/// When there's no catalog /Lang, text strings on annotations have no language,
/// making them inaccessible for text-to-speech.
fn check_annotation_text_language(doc: &lopdf::Document, results: &mut Vec<CheckResult>) {
    let pages = doc.get_pages();
    let mut contents_without_lang = 0;
    let mut tu_without_lang = 0;

    for page_id in pages.values() {
        let Ok(page_obj) = doc.get_object(*page_id) else {
            continue;
        };
        let Ok(page_dict) = page_obj.as_dict() else {
            continue;
        };
        let Ok(annots_obj) = page_dict.get(b"Annots") else {
            continue;
        };

        let annots = if let Ok(arr) = annots_obj.as_array() {
            arr.clone()
        } else if let Ok(ref_id) = annots_obj.as_reference() {
            if let Ok(obj) = doc.get_object(ref_id) {
                obj.as_array().cloned().unwrap_or_default()
            } else {
                continue;
            }
        } else {
            continue;
        };

        for annot_obj in &annots {
            let annot = if let Ok(ref_id) = annot_obj.as_reference() {
                doc.get_object(ref_id).ok().and_then(|o| o.as_dict().ok())
            } else {
                annot_obj.as_dict().ok()
            };
            let Some(annot_dict) = annot else { continue };

            // Check /Contents (non-empty)
            if let Ok(contents) = annot_dict.get(b"Contents") {
                if contents.as_str().is_ok_and(|s| !s.is_empty()) {
                    contents_without_lang += 1;
                }
            }
            // Check /TU (non-empty)
            if let Ok(tu) = annot_dict.get(b"TU") {
                if tu.as_str().is_ok_and(|s| !s.is_empty()) {
                    tu_without_lang += 1;
                }
            }
        }
    }

    if contents_without_lang > 0 {
        results.push(fail(
            "02-002",
            &format!(
                "{contents_without_lang} annotation(s) have /Contents text but no language context (catalog /Lang is missing)"
            ),
        ));
    }
    if tu_without_lang > 0 {
        results.push(fail(
            "02-003",
            &format!(
                "{tu_without_lang} form field(s) have /TU text but no language context (catalog /Lang is missing)"
            ),
        ));
    }
}

/// 02-004: XMP dc:title must have a real language when no catalog /Lang.
///
/// If dc:title only has `xml:lang="x-default"` and no catalog /Lang provides
/// language context, the title has no usable language specification.
fn check_dc_title_language(
    doc: &lopdf::Document,
    catalog: &lopdf::Dictionary,
    results: &mut Vec<CheckResult>,
) {
    let Ok(meta_obj) = catalog.get(b"Metadata") else {
        return;
    };
    let Ok(meta_ref) = meta_obj.as_reference() else {
        return;
    };
    let Ok(meta_resolved) = doc.get_object(meta_ref) else {
        return;
    };
    let Ok(stream) = meta_resolved.as_stream() else {
        return;
    };
    let Ok(content) = stream.get_plain_content() else {
        return;
    };
    let xmp = String::from_utf8_lossy(&content);

    // Check if dc:title exists
    if !xmp.contains("dc:title") {
        return;
    }

    // Extract the dc:title section and check xml:lang attributes
    if let Some(start) = xmp.find("dc:title") {
        if let Some(end) = xmp[start..].find("/dc:title") {
            let title_section = &xmp[start..start + end];
            // Find all xml:lang values
            let lang_pattern = "xml:lang=\"";
            let mut has_real_lang = false;
            let mut pos = 0;
            while let Some(idx) = title_section[pos..].find(lang_pattern) {
                let abs = pos + idx + lang_pattern.len();
                if let Some(end_quote) = title_section[abs..].find('"') {
                    let lang_val = &title_section[abs..abs + end_quote];
                    if lang_val != "x-default" && !lang_val.is_empty() {
                        has_real_lang = true;
                    }
                    pos = abs + end_quote;
                } else {
                    break;
                }
            }
            if !has_real_lang {
                results.push(fail(
                    "02-004",
                    "XMP dc:title has no language specification (only x-default) and catalog /Lang is missing",
                ));
            }
        }
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
        checkpoint: 11,
        description: description.to_string(),
        severity: Severity::Info,
        outcome: CheckOutcome::Pass,
    }
}

fn fail(rule_id: &str, message: &str) -> CheckResult {
    CheckResult {
        rule_id: rule_id.to_string(),
        checkpoint: 11,
        description: message.to_string(),
        severity: Severity::Error,
        outcome: CheckOutcome::Fail {
            message: message.to_string(),
            location: None,
        },
    }
}
