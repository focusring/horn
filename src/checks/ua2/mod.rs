//! Checks for requirements that exist only in PDF/UA-2 (ISO 14289-2).
//!
//! These checks run only for documents that identify as PDF/UA-2
//! (`pdfuaid:part = 2`). Rule ids and descriptions come from
//! [`crate::pdfua2`]; requirements shared with PDF/UA-1 are covered by the
//! regular Matterhorn checks, which are namespace-aware where PDF 2.0 needs it.

pub mod annotations;
pub mod destinations;
pub mod identification;
pub mod structure;

use crate::model::{CheckOutcome, CheckResult, Location, Severity};
use crate::pdfua2;
use lopdf::{Dictionary, Document, Object};

fn checkpoint(rule: &str) -> u8 {
    pdfua2::checkpoint(rule).unwrap_or(0)
}

/// A failing result for a PDF/UA-2 rule.
pub(crate) fn fail(
    rule: &str,
    message: impl Into<String>,
    location: Option<Location>,
) -> CheckResult {
    let message = message.into();
    CheckResult {
        rule_id: rule.to_string(),
        checkpoint: checkpoint(rule),
        description: message.clone(),
        severity: Severity::Error,
        outcome: CheckOutcome::Fail { message, location },
    }
}

/// A passing result for a PDF/UA-2 rule.
pub(crate) fn pass(rule: &str, description: impl Into<String>) -> CheckResult {
    CheckResult {
        rule_id: rule.to_string(),
        checkpoint: checkpoint(rule),
        description: description.into(),
        severity: Severity::Info,
        outcome: CheckOutcome::Pass,
    }
}

/// Location on a page.
pub(crate) fn page_location(page: u32, element: &str) -> Location {
    Location {
        page: Some(page),
        element: Some(element.to_string()),
    }
}

/// Follow one level of indirection.
pub(crate) fn resolve<'a>(doc: &'a Document, obj: &'a Object) -> Option<&'a Object> {
    match obj {
        Object::Reference(id) => doc.get_object(*id).ok(),
        other => Some(other),
    }
}

/// Resolve an object to a dictionary (streams excluded).
pub(crate) fn resolve_dict<'a>(doc: &'a Document, obj: &'a Object) -> Option<&'a Dictionary> {
    resolve(doc, obj)?.as_dict().ok()
}

/// The structure tree root dictionary, if the document has one.
pub(crate) fn struct_tree_root<'a>(
    doc: &'a Document,
    catalog: &'a Dictionary,
) -> Option<&'a Dictionary> {
    catalog
        .get(b"StructTreeRoot")
        .ok()
        .and_then(|o| resolve_dict(doc, o))
}

/// The `/K` entry of a structure element as a list of (unresolved) kid objects.
pub(crate) fn kids<'a>(doc: &'a Document, dict: &'a Dictionary) -> Vec<&'a Object> {
    let Ok(k) = dict.get(b"K") else {
        return Vec::new();
    };
    let k = match resolve(doc, k) {
        Some(arr @ Object::Array(_)) => arr,
        _ => k,
    };
    match k {
        Object::Array(arr) => arr.iter().collect(),
        other => vec![other],
    }
}

/// `/Type` name of a dictionary, if any.
pub(crate) fn type_name(dict: &Dictionary) -> Option<&[u8]> {
    dict.get(b"Type").ok().and_then(|o| o.as_name().ok())
}

/// Look up an attribute in a structure element's `/A` entry (a dictionary,
/// an array of dictionaries, or references to either).
pub(crate) fn attribute<'a>(
    doc: &'a Document,
    dict: &'a Dictionary,
    key: &[u8],
) -> Option<&'a Object> {
    let attrs = resolve(doc, dict.get(b"A").ok()?)?;
    match attrs {
        Object::Dictionary(d) => d.get(key).ok(),
        Object::Array(arr) => arr
            .iter()
            .filter_map(|o| resolve_dict(doc, o))
            .find_map(|d| d.get(key).ok()),
        _ => None,
    }
}

/// A non-empty text string entry.
pub(crate) fn nonempty_text<'a>(
    doc: &'a Document,
    dict: &'a Dictionary,
    key: &[u8],
) -> Option<&'a [u8]> {
    let bytes = resolve(doc, dict.get(key).ok()?)?.as_str().ok()?;
    (!bytes.is_empty()).then_some(bytes)
}

/// Decode a PDF text string: UTF-16BE (with BOM), UTF-8 (with BOM, PDF 2.0)
/// or `PDFDocEncoding` (approximated as Latin-1).
pub(crate) fn decode_text_string(bytes: &[u8]) -> String {
    if let Some(rest) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        let units: Vec<u16> = rest
            .chunks(2)
            .map(|c| u16::from_be_bytes([c[0], *c.get(1).unwrap_or(&0)]))
            .collect();
        return String::from_utf16_lossy(&units);
    }
    if let Some(rest) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        return String::from_utf8_lossy(rest).into_owned();
    }
    bytes.iter().map(|b| char::from(*b)).collect()
}

/// Whether a string contains Unicode private-use-area code points
/// (U+E000–U+F8FF, U+F0000–U+FFFFD, U+100000–U+10FFFD).
pub(crate) fn contains_private_use(s: &str) -> bool {
    s.chars().any(|c| {
        matches!(
            u32::from(c),
            0xE000..=0xF8FF | 0xF0000..=0xFFFFD | 0x10_0000..=0x10_FFFD
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_string_decoding() {
        assert_eq!(decode_text_string(b"Hello"), "Hello");
        assert_eq!(
            decode_text_string(&[0xFE, 0xFF, 0x00, 0x48, 0x00, 0x69]),
            "Hi"
        );
        assert_eq!(decode_text_string(&[0xEF, 0xBB, 0xBF, 0xC3, 0xA9]), "é");
    }

    #[test]
    fn private_use_detection() {
        assert!(!contains_private_use("plain text"));
        assert!(contains_private_use("\u{E001}"));
        assert!(contains_private_use("a\u{F0000}b"));
        assert!(!contains_private_use("\u{FFFD}"));
    }
}
