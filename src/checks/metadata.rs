use crate::checks::Check;
use crate::document::HornDocument;
use crate::model::{CheckOutcome, CheckResult, Severity};
use anyhow::Result;

/// Checkpoint 06: Metadata checks.
///
/// Validates XMP metadata, document title display, language, and PDF/UA identifier.
/// These go beyond the baseline by parsing the actual XMP stream content.
pub struct MetadataChecks;

impl Check for MetadataChecks {
    fn id(&self) -> &'static str {
        "06-metadata"
    }

    fn checkpoint(&self) -> u8 {
        6
    }

    fn description(&self) -> &'static str {
        "Metadata: XMP, title, language, PDF/UA identifier"
    }

    fn run(&self, doc: &mut HornDocument) -> Result<Vec<CheckResult>> {
        let mut results = Vec::new();

        check_document_language(doc, &mut results);
        check_title_display(doc, &mut results);
        check_pdfua_identifier(doc, &mut results);
        check_xmp_metadata(doc, &mut results);

        Ok(results)
    }
}

/// 06-001: Document catalog must contain a Lang entry.
///
/// Per PDF/UA-1, the catalog should have `/Lang`. However, if the catalog
/// lacks `/Lang` but the StructTreeRoot's direct children all carry `/Lang`,
/// the document still provides language identification — we downgrade to a
/// warning rather than a hard fail.
fn check_document_language(doc: &mut HornDocument, results: &mut Vec<CheckResult>) {
    let Ok(catalog) = doc.raw_catalog() else {
        results.push(fail(
            "06-001",
            6,
            "Document language not set: cannot read catalog",
        ));
        return;
    };

    let lopdf_doc = doc.lopdf();
    match catalog.get_deref(b"Lang", lopdf_doc) {
        Ok(obj) => {
            let lang_bytes = obj.as_str().or_else(|_| obj.as_name());
            if let Ok(lang) = lang_bytes {
                let lang_str = String::from_utf8_lossy(lang);
                if lang_str.is_empty() {
                    results.push(fail("06-001", 6, "Document language is empty"));
                } else {
                    results.push(pass("06-001", 6, "Document language is set"));
                }
            } else {
                results.push(fail("06-001", 6, "Document /Lang entry is not a string"));
            }
        }
        Err(_) => {
            // No catalog /Lang — check if structure tree elements provide it
            if has_struct_level_lang(catalog, lopdf_doc) {
                results.push(CheckResult {
                    rule_id: "06-001".to_string(),
                    checkpoint: 6,
                    description: "Document catalog missing /Lang but structure elements provide language identification".to_string(),
                    severity: Severity::Warning,
                    outcome: CheckOutcome::NeedsReview {
                        reason: "Catalog /Lang is missing; language is specified on structure elements instead".to_string(),
                    },
                });
            } else {
                results.push(fail("06-001", 6, "Document catalog missing /Lang entry"));
            }
        }
    }
}

/// Check if any structure element in the tree has a valid /Lang attribute.
fn has_struct_level_lang(catalog: &lopdf::Dictionary, doc: &lopdf::Document) -> bool {
    let struct_tree = match catalog.get(b"StructTreeRoot") {
        Ok(obj) => {
            let ref_id = match obj.as_reference() {
                Ok(r) => r,
                Err(_) => return false,
            };
            match doc.get_object(ref_id) {
                Ok(o) => match o.as_dict() {
                    Ok(d) => d,
                    Err(_) => return false,
                },
                Err(_) => return false,
            }
        }
        Err(_) => return false,
    };

    // Check StructTreeRoot itself for /Lang
    if let Ok(lang) = struct_tree.get(b"Lang") {
        if lang.as_str().is_ok_and(|s| !s.is_empty()) {
            return true;
        }
    }

    // Check direct children
    let Ok(kids) = struct_tree.get(b"K") else {
        return false;
    };
    check_kids_for_lang(kids, doc, 0)
}

fn check_kids_for_lang(obj: &lopdf::Object, doc: &lopdf::Document, depth: usize) -> bool {
    if depth > 5 {
        return false;
    } // Don't go too deep

    match obj {
        lopdf::Object::Reference(ref_id) => {
            if let Ok(resolved) = doc.get_object(*ref_id) {
                if let Ok(dict) = resolved.as_dict() {
                    if let Ok(lang) = dict.get(b"Lang") {
                        if lang.as_str().is_ok_and(|s| !s.is_empty()) {
                            return true;
                        }
                    }
                    // Check this element's children
                    if let Ok(kids) = dict.get(b"K") {
                        return check_kids_for_lang(kids, doc, depth + 1);
                    }
                }
            }
            false
        }
        lopdf::Object::Array(arr) => arr.iter().any(|item| check_kids_for_lang(item, doc, depth)),
        lopdf::Object::Dictionary(dict) => {
            if let Ok(lang) = dict.get(b"Lang") {
                if lang.as_str().is_ok_and(|s| !s.is_empty()) {
                    return true;
                }
            }
            false
        }
        _ => false,
    }
}

/// 06-003: ViewerPreferences/DisplayDocTitle must be true.
fn check_title_display(doc: &mut HornDocument, results: &mut Vec<CheckResult>) {
    let Ok(catalog) = doc.raw_catalog() else {
        return;
    };

    let lopdf_doc = doc.lopdf();

    match catalog.get_deref(b"ViewerPreferences", lopdf_doc) {
        Ok(obj) => {
            if let Ok(prefs) = obj.as_dict() {
                match prefs.get(b"DisplayDocTitle") {
                    Ok(val) => {
                        if let Ok(display) = val.as_bool() {
                            if display {
                                results.push(pass("06-003", 6, "DisplayDocTitle is true"));
                            } else {
                                results.push(fail(
                                    "06-003",
                                    6,
                                    "DisplayDocTitle is false — title bar should show document title",
                                ));
                            }
                        } else {
                            results.push(fail(
                                "06-003",
                                6,
                                "DisplayDocTitle is not a boolean value",
                            ));
                        }
                    }
                    Err(_) => {
                        results.push(fail(
                            "06-003",
                            6,
                            "ViewerPreferences missing DisplayDocTitle entry",
                        ));
                    }
                }
            } else {
                results.push(fail("06-003", 6, "ViewerPreferences is not a dictionary"));
            }
        }
        Err(_) => {
            results.push(fail(
                "06-003",
                6,
                "Document catalog missing /ViewerPreferences",
            ));
        }
    }
}

/// 06-002: PDF/UA identifier must be present (pdfuaid:part in XMP or /Metadata).
fn check_pdfua_identifier(doc: &mut HornDocument, results: &mut Vec<CheckResult>) {
    let Ok(catalog) = doc.raw_catalog() else {
        return;
    };

    let lopdf_doc = doc.lopdf();

    // Check for /Metadata stream containing pdfuaid:part
    match catalog.get(b"Metadata") {
        Ok(obj) => {
            if let Ok(ref_id) = obj.as_reference() {
                if let Ok(meta_obj) = lopdf_doc.get_object(ref_id) {
                    if let Ok(stream) = meta_obj.as_stream() {
                        match stream.get_plain_content() {
                            Ok(content) => {
                                let xmp = String::from_utf8_lossy(&content);
                                if xmp.contains("pdfuaid:part") {
                                    results.push(pass(
                                        "06-002",
                                        6,
                                        "PDF/UA identifier (pdfuaid:part) found in XMP",
                                    ));
                                } else {
                                    results.push(fail(
                                        "06-002",
                                        6,
                                        "XMP metadata missing pdfuaid:part identifier",
                                    ));
                                }
                            }
                            Err(_) => {
                                results.push(fail(
                                    "06-002",
                                    6,
                                    "Cannot decompress XMP metadata stream",
                                ));
                            }
                        }
                    } else {
                        results.push(fail("06-002", 6, "Metadata object is not a stream"));
                    }
                } else {
                    results.push(fail("06-002", 6, "Cannot resolve Metadata reference"));
                }
            } else {
                results.push(fail("06-002", 6, "Metadata entry is not a reference"));
            }
        }
        Err(_) => {
            results.push(fail(
                "06-002",
                6,
                "Document catalog missing /Metadata entry",
            ));
        }
    }
}

/// 06-004: XMP metadata must contain dc:title.
fn check_xmp_metadata(doc: &mut HornDocument, results: &mut Vec<CheckResult>) {
    let Ok(catalog) = doc.raw_catalog() else {
        return;
    };

    let lopdf_doc = doc.lopdf();

    let Some(xmp_content) = get_xmp_content(catalog, lopdf_doc) else {
        return;
    };

    let xmp = String::from_utf8_lossy(&xmp_content);

    // Check for dc:title
    if xmp.contains("dc:title") {
        results.push(pass("06-004", 6, "XMP contains dc:title"));
    } else {
        results.push(fail("06-004", 6, "XMP metadata missing dc:title"));
    }
}

fn get_xmp_content(catalog: &lopdf::Dictionary, doc: &lopdf::Document) -> Option<Vec<u8>> {
    let meta_ref = catalog.get(b"Metadata").ok()?.as_reference().ok()?;
    let meta_obj = doc.get_object(meta_ref).ok()?;
    let stream = meta_obj.as_stream().ok()?;
    stream.get_plain_content().ok()
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
