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
        let doc_lang = catalog
            .get(b"Lang")
            .ok()
            .and_then(|o| o.as_str().ok())
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
