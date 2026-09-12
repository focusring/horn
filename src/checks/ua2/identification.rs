//! PDF/UA-2 identification and file-level requirements.
//!
//! - `ua2:5-3` / `ua2:5-4` / `ua2:5-5`: the `pdfuaid:part` and `pdfuaid:rev`
//!   properties of the PDF/UA identification schema (ISO 14289-2, clause 5)
//! - `ua2:8.11.1-2`: the catalog `/Metadata` stream carries `/Type /Metadata`
//!   and `/Subtype /XML`
//! - `ua2:8.14.1-1`: every file specification in the `EmbeddedFiles` name tree
//!   has a `/Desc` entry

use super::{fail, pass, resolve, resolve_dict};
use crate::checks::Check;
use crate::document::HornDocument;
use crate::model::{CheckResult, Standard};
use anyhow::Result;
use lopdf::{Dictionary, Document};

/// Namespace name of the PDF/UA identification schema.
const PDFUA_NS: &str = "http://www.aiim.org/pdfua/ns/id/";

pub struct Ua2IdentificationChecks;

impl Check for Ua2IdentificationChecks {
    fn id(&self) -> &'static str {
        "ua2-identification"
    }

    fn checkpoint(&self) -> u8 {
        6
    }

    fn rules(&self) -> &'static [&'static str] {
        &[
            "ua2:5-3",
            "ua2:5-4",
            "ua2:5-5",
            "ua2:8.11.1-2",
            "ua2:8.14.1-1",
        ]
    }

    fn description(&self) -> &'static str {
        "PDF/UA-2: identification schema (part/rev), metadata stream, embedded file descriptions"
    }

    fn supports(&self, standard: Standard) -> bool {
        standard == Standard::Ua2
    }

    fn run(&self, doc: &mut HornDocument) -> Result<Vec<CheckResult>> {
        let mut results = Vec::new();
        let lopdf = doc.lopdf();
        let Ok(catalog) = lopdf.catalog() else {
            return Ok(results);
        };

        check_metadata_stream(lopdf, catalog, &mut results);
        check_embedded_file_descriptions(lopdf, catalog, &mut results);

        Ok(results)
    }
}

/// `ua2:8.11.1-2` plus the identification-schema checks on the XMP packet.
fn check_metadata_stream(doc: &Document, catalog: &Dictionary, results: &mut Vec<CheckResult>) {
    let Some(stream) = catalog
        .get(b"Metadata")
        .ok()
        .and_then(|o| resolve(doc, o))
        .and_then(|o| o.as_stream().ok())
    else {
        // Absence of the stream is reported by 06-001.
        return;
    };

    let name_of = |key: &[u8]| {
        stream
            .dict
            .get(key)
            .ok()
            .and_then(|o| o.as_name().ok())
            .map(|n| String::from_utf8_lossy(n).into_owned())
    };
    let ty = name_of(b"Type");
    let subtype = name_of(b"Subtype");
    if ty.as_deref() == Some("Metadata") && subtype.as_deref() == Some("XML") {
        results.push(pass(
            "ua2:8.11.1-2",
            "Metadata stream has /Type /Metadata and /Subtype /XML",
        ));
    } else {
        results.push(fail(
            "ua2:8.11.1-2",
            format!(
                "Metadata stream must have /Type /Metadata and /Subtype /XML (found /Type {} and /Subtype {})",
                ty.map_or("missing".to_string(), |t| format!("/{t}")),
                subtype.map_or("missing".to_string(), |t| format!("/{t}")),
            ),
            None,
        ));
    }

    if let Ok(content) = stream.get_plain_content() {
        let xmp = String::from_utf8_lossy(&content);
        check_identification_schema(&xmp, results);
    }
}

/// `ua2:5-3` / `ua2:5-4` / `ua2:5-5`: `part` and `rev` must be bound to the
/// `pdfuaid` prefix and `rev` must be `2024`.
fn check_identification_schema(xmp: &str, results: &mut Vec<CheckResult>) {
    let prefixes = prefixes_bound_to(xmp, PDFUA_NS);
    let other_prefixes = prefixes.iter().filter(|p| p.as_str() != "pdfuaid");

    // part
    if property_values(xmp, "pdfuaid", "part").is_empty() {
        let wrong = other_prefixes
            .clone()
            .find(|p| !property_values(xmp, p, "part").is_empty());
        if let Some(prefix) = wrong {
            results.push(fail(
                "ua2:5-3",
                format!(
                    "The 'part' property of the PDF/UA identification schema uses the namespace prefix '{prefix}' — it must be 'pdfuaid:part'"
                ),
                None,
            ));
        }
        // A completely missing part property is reported by 06-002.
    } else {
        results.push(pass("ua2:5-3", "pdfuaid:part uses the 'pdfuaid' prefix"));
    }

    // rev
    let rev = property_values(xmp, "pdfuaid", "rev");
    match rev.first() {
        None => {
            let wrong = other_prefixes
                .clone()
                .find(|p| !property_values(xmp, p, "rev").is_empty());
            if let Some(prefix) = wrong {
                results.push(fail(
                    "ua2:5-4",
                    format!(
                        "The 'rev' property of the PDF/UA identification schema uses the namespace prefix '{prefix}' — it must be 'pdfuaid:rev'"
                    ),
                    None,
                ));
            } else {
                results.push(fail(
                    "ua2:5-5",
                    "XMP metadata has no pdfuaid:rev property — PDF/UA-2 requires pdfuaid:rev=\"2024\"",
                    None,
                ));
            }
        }
        Some(value) => {
            results.push(pass("ua2:5-4", "pdfuaid:rev uses the 'pdfuaid' prefix"));
            if value == "2024" {
                results.push(pass("ua2:5-5", "pdfuaid:rev is 2024"));
            } else {
                let detail = if value.len() == 4 && value.bytes().all(|b| b.is_ascii_digit()) {
                    "must be 2024 for ISO 14289-2:2024"
                } else {
                    "must be the four-digit year 2024"
                };
                results.push(fail(
                    "ua2:5-5",
                    format!("pdfuaid:rev is \"{value}\" — {detail}"),
                    None,
                ));
            }
        }
    }
}

/// All XML namespace prefixes that `xmlns:` declarations bind to `uri`.
fn prefixes_bound_to(xmp: &str, uri: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut pos = 0;
    while let Some(i) = xmp[pos..].find("xmlns:") {
        let start = pos + i + "xmlns:".len();
        pos = start;
        let rest = &xmp[start..];
        let Some(eq) = rest.find('=') else { break };
        let prefix = rest[..eq].trim();
        if prefix.is_empty()
            || !prefix
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-' || b == b'.')
        {
            continue;
        }
        let after = rest[eq + 1..].trim_start();
        let Some(quote) = after.chars().next().filter(|c| matches!(c, '"' | '\'')) else {
            continue;
        };
        let Some(end) = after[1..].find(quote) else {
            continue;
        };
        if &after[1..=end] == uri && !out.iter().any(|p| p == prefix) {
            out.push(prefix.to_string());
        }
    }
    out
}

/// Values of an XMP property given in attribute form (`prefix:local="…"`) or
/// element form (`<prefix:local>…</prefix:local>`).
fn property_values(xmp: &str, prefix: &str, local: &str) -> Vec<String> {
    let mut values = Vec::new();

    // Attribute form
    let attr = format!("{prefix}:{local}=");
    let mut pos = 0;
    while let Some(i) = xmp[pos..].find(&attr) {
        let at = pos + i;
        pos = at + attr.len();
        // The attribute name must start a token (avoid matching e.g. "xpdfuaid:part=")
        let starts_token = at == 0
            || xmp[..at]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_whitespace() || c == '<' || c == '"' || c == '\'');
        if !starts_token {
            continue;
        }
        let after = &xmp[pos..];
        let Some(quote) = after.chars().next().filter(|c| matches!(c, '"' | '\'')) else {
            continue;
        };
        if let Some(end) = after[1..].find(quote) {
            values.push(after[1..=end].trim().to_string());
        }
    }

    // Element form
    let open = format!("<{prefix}:{local}>");
    let close = format!("</{prefix}:{local}>");
    let mut pos = 0;
    while let Some(i) = xmp[pos..].find(&open) {
        let value_start = pos + i + open.len();
        let Some(end) = xmp[value_start..].find(&close) else {
            break;
        };
        values.push(xmp[value_start..value_start + end].trim().to_string());
        pos = value_start + end + close.len();
    }

    values
}

/// `ua2:8.14.1-1`: file specifications in the `EmbeddedFiles` name tree need `/Desc`.
fn check_embedded_file_descriptions(
    doc: &Document,
    catalog: &Dictionary,
    results: &mut Vec<CheckResult>,
) {
    let Some(tree) = catalog
        .get(b"Names")
        .ok()
        .and_then(|o| resolve_dict(doc, o))
        .and_then(|names| names.get(b"EmbeddedFiles").ok())
        .and_then(|o| resolve_dict(doc, o))
    else {
        return;
    };

    let mut specs: Vec<(String, &Dictionary)> = Vec::new();
    collect_name_tree(doc, tree, &mut specs, 0);
    if specs.is_empty() {
        return;
    }

    let mut missing = 0usize;
    for (key, spec) in &specs {
        if spec.get(b"Desc").is_err() {
            missing += 1;
            results.push(fail(
                "ua2:8.14.1-1",
                format!(
                    "Embedded file \"{key}\": file specification dictionary has no /Desc entry describing the file"
                ),
                None,
            ));
        }
    }
    if missing == 0 {
        results.push(pass(
            "ua2:8.14.1-1",
            format!(
                "All {} embedded file specification(s) have a /Desc entry",
                specs.len()
            ),
        ));
    }
}

fn collect_name_tree<'a>(
    doc: &'a Document,
    node: &'a Dictionary,
    out: &mut Vec<(String, &'a Dictionary)>,
    depth: usize,
) {
    if depth > 32 {
        return;
    }
    if let Some(names) = node
        .get(b"Names")
        .ok()
        .and_then(|o| resolve(doc, o))
        .and_then(|o| o.as_array().ok())
    {
        for pair in names.chunks(2) {
            if pair.len() < 2 {
                break;
            }
            let key = pair[0]
                .as_str()
                .map(super::decode_text_string)
                .unwrap_or_default();
            if let Some(spec) = resolve_dict(doc, &pair[1]) {
                out.push((key, spec));
            }
        }
    }
    if let Some(kids) = node
        .get(b"Kids")
        .ok()
        .and_then(|o| resolve(doc, o))
        .and_then(|o| o.as_array().ok())
    {
        for kid in kids {
            if let Some(child) = resolve_dict(doc, kid) {
                collect_name_tree(doc, child, out, depth + 1);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const XMP_OK: &str = r#"<rdf:Description rdf:about="" xmlns:pdfuaid="http://www.aiim.org/pdfua/ns/id/" pdfuaid:part="2" pdfuaid:rev="2024"/>"#;

    #[test]
    fn prefixes() {
        let xmp = r#"<x xmlns:pdfuaid="http://www.aiim.org/pdfua/ns/id/" xmlns:pdfuadd='http://www.aiim.org/pdfua/ns/id/' xmlns:dc="http://purl.org/dc/elements/1.1/">"#;
        assert_eq!(prefixes_bound_to(xmp, PDFUA_NS), vec!["pdfuaid", "pdfuadd"]);
    }

    #[test]
    fn property_forms() {
        assert_eq!(property_values(XMP_OK, "pdfuaid", "rev"), vec!["2024"]);
        let elem = "<pdfuaid:part>2</pdfuaid:part>\n<pdfuaid:rev> 2024 </pdfuaid:rev>";
        assert_eq!(property_values(elem, "pdfuaid", "part"), vec!["2"]);
        assert_eq!(property_values(elem, "pdfuaid", "rev"), vec!["2024"]);
        assert!(property_values("xpdfuaid:rev=\"2024\"", "pdfuaid", "rev").is_empty());
    }

    fn failures(xmp: &str) -> Vec<String> {
        let mut r = Vec::new();
        check_identification_schema(xmp, &mut r);
        r.iter()
            .filter(|x| x.is_failure())
            .map(|x| x.rule_id.clone())
            .collect()
    }

    #[test]
    fn identification_schema_rules() {
        assert!(failures(XMP_OK).is_empty());
        let wrong_rev_prefix = r#"<x xmlns:pdfuaid="http://www.aiim.org/pdfua/ns/id/" xmlns:pdfuadd="http://www.aiim.org/pdfua/ns/id/" pdfuaid:part="2" pdfuadd:rev="2024"/>"#;
        assert_eq!(failures(wrong_rev_prefix), vec!["ua2:5-4"]);
        let wrong_part_prefix = r#"<x xmlns:pdfuaid="http://www.aiim.org/pdfua/ns/id/" xmlns:pdfuadd="http://www.aiim.org/pdfua/ns/id/" pdfuadd:part="2" pdfuaid:rev="2024"/>"#;
        assert_eq!(failures(wrong_part_prefix), vec!["ua2:5-3"]);
        let bad_year = r#"<x xmlns:pdfuaid="http://www.aiim.org/pdfua/ns/id/" pdfuaid:part="2" pdfuaid:rev="2024a"/>"#;
        assert_eq!(failures(bad_year), vec!["ua2:5-5"]);
        let old_year = r#"<x xmlns:pdfuaid="http://www.aiim.org/pdfua/ns/id/" pdfuaid:part="2" pdfuaid:rev="2018"/>"#;
        assert_eq!(failures(old_year), vec!["ua2:5-5"]);
        let missing_rev =
            r#"<x xmlns:pdfuaid="http://www.aiim.org/pdfua/ns/id/" pdfuaid:part="2"/>"#;
        assert_eq!(failures(missing_rev), vec!["ua2:5-5"]);
    }
}
