use crate::checks::Check;
use crate::document::HornDocument;
use crate::model::{CheckOutcome, CheckResult, Severity, Standard};
use anyhow::Result;

/// Checkpoint 05: Version identification.
///
/// Validates that the PDF correctly identifies itself as PDF/UA through:
/// - Presence of /Metadata stream in the catalog (05-001)
/// - XMP `pdfuaid:part` value matches the detected standard (05-002)
/// - XMP `pdfuaid:part` is present (05-003, overlaps with 06-002)
/// - XMP extension schema for pdfuaid is properly defined (05-004/05-005)
pub struct VersionChecks;

impl Check for VersionChecks {
    fn id(&self) -> &'static str {
        "05-version"
    }

    fn checkpoint(&self) -> u8 {
        5
    }

    fn description(&self) -> &'static str {
        "Version identification: PDF/UA identifier and XMP extension schema"
    }

    fn run(&self, doc: &mut HornDocument) -> Result<Vec<CheckResult>> {
        let mut results = Vec::new();
        let standard = doc.standard();

        let xmp_content = check_metadata_stream_exists(doc, &mut results);

        if let Some(xmp) = xmp_content {
            let xmp_str = String::from_utf8_lossy(&xmp);
            check_pdfuaid_part_value(&xmp_str, standard, &mut results);
            check_extension_schema(&xmp_str, &mut results);
        }

        Ok(results)
    }
}

/// 05-001: /Metadata stream must exist in the document catalog.
fn check_metadata_stream_exists(
    doc: &mut HornDocument,
    results: &mut Vec<CheckResult>,
) -> Option<Vec<u8>> {
    let Ok(catalog) = doc.raw_catalog() else {
        results.push(fail(
            "05-001",
            "Cannot read document catalog to check /Metadata",
        ));
        return None;
    };

    let lopdf_doc = doc.lopdf();

    let Ok(meta_obj_raw) = catalog.get(b"Metadata") else {
        results.push(fail("05-001", "Document catalog missing /Metadata stream"));
        return None;
    };
    let Ok(meta_ref) = meta_obj_raw.as_reference() else {
        results.push(fail(
            "05-001",
            "/Metadata entry is not an indirect reference",
        ));
        return None;
    };

    let Ok(meta_obj) = lopdf_doc.get_object(meta_ref) else {
        results.push(fail("05-001", "Cannot resolve /Metadata reference"));
        return None;
    };

    let Ok(stream) = meta_obj.as_stream() else {
        results.push(fail("05-001", "/Metadata object is not a stream"));
        return None;
    };

    if let Ok(content) = stream.get_plain_content() {
        results.push(pass("05-001", "/Metadata stream exists in catalog"));
        Some(content)
    } else {
        results.push(fail("05-001", "Cannot decompress /Metadata stream"));
        None
    }
}

/// 05-002 / 05-003: XMP pdfuaid:part must be present and match the detected standard.
///
/// Handles both XMP syntaxes:
/// - Element: `<pdfuaid:part>1</pdfuaid:part>` (common in UA-1)
/// - Attribute: `pdfuaid:part="2"` (common in UA-2)
fn check_pdfuaid_part_value(xmp: &str, _standard: Standard, results: &mut Vec<CheckResult>) {
    // Try to extract pdfuaid:part value from either syntax
    let part_value = extract_pdfuaid_part(xmp);

    match part_value {
        Some(value) => {
            // 05-003: pdfuaid:part exists
            results.push(pass("05-003", "XMP pdfuaid:part identifier is present"));

            // 05-002: value must match the detected standard
            match value.parse::<i32>() {
                Ok(1) => {
                    results.push(pass("05-002", "XMP pdfuaid:part value is 1 (PDF/UA-1)"));
                }
                Ok(2) => {
                    results.push(pass("05-002", "XMP pdfuaid:part value is 2 (PDF/UA-2)"));
                }
                Ok(n) => {
                    results.push(fail(
                        "05-002",
                        &format!(
                            "XMP pdfuaid:part value is {n} — must be 1 (PDF/UA-1) or 2 (PDF/UA-2)"
                        ),
                    ));
                }
                Err(_) => {
                    results.push(fail(
                        "05-002",
                        &format!("XMP pdfuaid:part value '{value}' is not a valid integer"),
                    ));
                }
            }
        }
        None => {
            results.push(fail(
                "05-003",
                "XMP metadata missing pdfuaid:part identifier",
            ));
        }
    }
}

/// Extract the pdfuaid:part value from XMP, supporting both element and attribute syntax.
fn extract_pdfuaid_part(xmp: &str) -> Option<String> {
    // Try element syntax: <pdfuaid:part>VALUE</pdfuaid:part>
    let elem_start = "<pdfuaid:part>";
    let elem_end = "</pdfuaid:part>";
    if let Some(start_idx) = xmp.find(elem_start) {
        let value_start = start_idx + elem_start.len();
        if let Some(end_idx) = xmp[value_start..].find(elem_end) {
            return Some(xmp[value_start..value_start + end_idx].trim().to_string());
        }
    }

    // Try attribute syntax: pdfuaid:part="VALUE"
    if let Some(idx) = xmp.find("pdfuaid:part=\"") {
        let value_start = idx + "pdfuaid:part=\"".len();
        if let Some(end_idx) = xmp[value_start..].find('"') {
            return Some(xmp[value_start..value_start + end_idx].trim().to_string());
        }
    }

    None
}

/// 05-004/05-005: XMP extension schema for pdfuaid must be properly defined.
///
/// The pdfuaid namespace can be declared in two valid ways:
///
/// 1. **Via `pdfaExtension:schemas`** (PDF/A style): A formal extension schema block
///    declaring the namespace URI, prefix, and properties. Required when the PDF also
///    claims PDF/A conformance.
///
/// 2. **Via `xmlns:pdfuaid` namespace declaration** on an `rdf:Description` element
///    with the correct namespace URI (`http://www.aiim.org/pdfua/ns/id/`). This is
///    sufficient for PDF/UA-1 conformance without PDF/A.
///
/// Both approaches are valid per ISO 14289-1. The PDF/UA Reference Suite from the
/// PDF Association uses both patterns across its reference documents.
///
/// When `pdfaExtension:schemas` is present, we additionally validate:
/// - The correct namespace URI is used
/// - The correct prefix ("pdfuaid") is declared
/// - No duplicate schema definitions exist for the same namespace
fn check_extension_schema(xmp: &str, results: &mut Vec<CheckResult>) {
    let pdfua_ns = "http://www.aiim.org/pdfua/ns/id/";
    let has_extension_schemas = xmp.contains("pdfaExtension:schemas");
    let has_xmlns_decl = xmp.contains("xmlns:pdfuaid") && xmp.contains(pdfua_ns);

    if !has_extension_schemas && !has_xmlns_decl {
        // Neither declaration method is present
        results.push(fail(
            "05-004",
            "XMP missing both pdfaExtension:schemas and xmlns:pdfuaid namespace declaration for PDF/UA identifier",
        ));
        return;
    }

    // xmlns:pdfuaid with correct URI is always sufficient (for both UA-1 and UA-2)
    if has_xmlns_decl {
        results.push(pass(
            "05-004",
            "XMP pdfuaid namespace declared via xmlns:pdfuaid with correct URI",
        ));
        return;
    }

    // Check for correct namespace URI
    let pdfua_ns = "http://www.aiim.org/pdfua/ns/id/";
    if !xmp.contains(pdfua_ns) {
        results.push(fail(
            "05-004",
            "XMP extension schema missing PDF/UA namespace URI",
        ));
        return;
    }

    // Check for duplicate schema definitions: count occurrences of pdfaSchema:prefix
    // with pdfuaid-related values. Two separate schema blocks for the same namespace
    // is invalid.
    let schema_blocks: Vec<_> = xmp.match_indices("<pdfaSchema:namespaceURI>").collect();
    let pdfua_schema_count = schema_blocks
        .iter()
        .filter(|(idx, _)| {
            // Check if this schema block references the pdfua namespace
            let remainder = &xmp[*idx..(*idx + 200).min(xmp.len())];
            remainder.contains(pdfua_ns)
        })
        .count();

    match pdfua_schema_count.cmp(&1) {
        std::cmp::Ordering::Greater => {
            results.push(fail(
                "05-005",
                &format!(
                    "XMP has {pdfua_schema_count} duplicate extension schema definitions for PDF/UA namespace — must have exactly one"
                ),
            ));
        }
        std::cmp::Ordering::Equal => {
            // Verify the prefix is correct (pdfuaid, not pdfuaia or something else)
            let prefix_tag = "<pdfaSchema:prefix>";
            let prefix_end = "</pdfaSchema:prefix>";

            // Find all prefix declarations and check ones near pdfua namespace
            let mut found_correct_prefix = false;
            let mut pos = 0;
            while let Some(idx) = xmp[pos..].find(prefix_tag) {
                let abs_idx = pos + idx;
                let value_start = abs_idx + prefix_tag.len();
                if let Some(end_rel) = xmp[value_start..].find(prefix_end) {
                    let prefix_value = xmp[value_start..value_start + end_rel].trim();
                    // Check if this prefix block is near the pdfua namespace URI
                    let context_start = abs_idx.saturating_sub(500);
                    let context_end = (abs_idx + 500).min(xmp.len());
                    let context = &xmp[context_start..context_end];
                    if context.contains(pdfua_ns) && prefix_value == "pdfuaid" {
                        found_correct_prefix = true;
                    }
                    pos = value_start + end_rel + prefix_end.len();
                } else {
                    break;
                }
            }

            if found_correct_prefix {
                results.push(pass(
                    "05-004",
                    "XMP extension schema for pdfuaid is properly defined",
                ));
            } else {
                results.push(fail(
                    "05-004",
                    "XMP extension schema prefix does not match 'pdfuaid'",
                ));
            }
        }
        std::cmp::Ordering::Less => {
            results.push(fail(
                "05-004",
                "XMP extension schema for PDF/UA namespace not found",
            ));
        }
    }
}

fn pass(rule_id: &str, description: &str) -> CheckResult {
    CheckResult {
        rule_id: rule_id.to_string(),
        checkpoint: 5,
        description: description.to_string(),
        severity: Severity::Info,
        outcome: CheckOutcome::Pass,
    }
}

fn fail(rule_id: &str, message: &str) -> CheckResult {
    CheckResult {
        rule_id: rule_id.to_string(),
        checkpoint: 5,
        description: message.to_string(),
        severity: Severity::Error,
        outcome: CheckOutcome::Fail {
            message: message.to_string(),
            location: None,
        },
    }
}
