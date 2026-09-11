use crate::checks::Check;
use crate::document::HornDocument;
use crate::model::{CheckOutcome, CheckResult, Severity};
use anyhow::Result;

/// File-level syntax checks (Horn extension, no Matterhorn checkpoint).
///
/// A PDF/UA-1 file must first be a conforming ISO 32000-1 file (clause 5). Two
/// cheap byte-level checks catch files that lenient parsers still open but that
/// no conforming reader is required to accept:
///
/// - 00-x01: the header is not `%PDF-1.n` (n in 0..=7) or `%PDF-2.0`
/// - 00-x02: bytes other than whitespace follow the final `%%EOF` marker
pub struct FileSyntaxChecks;

impl Check for FileSyntaxChecks {
    fn id(&self) -> &'static str {
        "00-file-syntax"
    }

    fn checkpoint(&self) -> u8 {
        0
    }

    fn description(&self) -> &'static str {
        "File syntax: PDF header and end-of-file marker"
    }

    fn run(&self, doc: &mut HornDocument) -> Result<Vec<CheckResult>> {
        let mut results = Vec::new();
        let Some(bytes) = doc.raw_bytes() else {
            return Ok(results);
        };

        check_header(bytes, &mut results);
        check_trailing_data(bytes, &mut results);

        Ok(results)
    }
}

/// 00-x01: `%PDF-1.[0-7]` or `%PDF-2.0` header.
fn check_header(bytes: &[u8], results: &mut Vec<CheckResult>) {
    let header: &[u8] = bytes.get(..8).unwrap_or(bytes);
    let valid = header.len() == 8
        && header.starts_with(b"%PDF-")
        && ((header[5] == b'1' && header[6] == b'.' && (b'0'..=b'7').contains(&header[7]))
            || &header[5..8] == b"2.0");

    if valid {
        results.push(pass("00-x01", "File header is a valid PDF version marker"));
    } else {
        let shown = String::from_utf8_lossy(header).into_owned();
        results.push(fail(
            "00-x01",
            &format!(
                "File header {shown:?} is not a valid PDF version marker (%PDF-1.0 … %PDF-1.7 or %PDF-2.0)"
            ),
        ));
    }
}

/// 00-x02: nothing but whitespace may follow the last `%%EOF`.
fn check_trailing_data(bytes: &[u8], results: &mut Vec<CheckResult>) {
    let Some(eof) = rfind(bytes, b"%%EOF") else {
        results.push(fail("00-x02", "File has no %%EOF end-of-file marker"));
        return;
    };
    let trailing = &bytes[eof + 5..];
    let junk = trailing
        .iter()
        .filter(|b| !matches!(b, b'\r' | b'\n' | b' ' | b'\t' | b'\x0c' | b'\0'))
        .count();
    if junk > 0 {
        results.push(fail(
            "00-x02",
            &format!(
                "{junk} byte(s) of non-whitespace data follow the final %%EOF marker — the file is not a well-formed PDF"
            ),
        ));
    } else {
        results.push(pass("00-x02", "Nothing follows the final %%EOF marker"));
    }
}

fn rfind(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .rposition(|window| window == needle)
}

fn pass(rule_id: &str, description: &str) -> CheckResult {
    CheckResult {
        rule_id: rule_id.to_string(),
        checkpoint: 0,
        description: description.to_string(),
        severity: Severity::Info,
        outcome: CheckOutcome::Pass,
    }
}

fn fail(rule_id: &str, message: &str) -> CheckResult {
    CheckResult {
        rule_id: rule_id.to_string(),
        checkpoint: 0,
        description: message.to_string(),
        severity: Severity::Error,
        outcome: CheckOutcome::Fail {
            message: message.to_string(),
            location: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_and_eof_checks() {
        let mut r = Vec::new();
        check_header(b"%PDF-1.7\n%...", &mut r);
        check_header(b"%PDF-2.0\n", &mut r);
        assert!(r.iter().all(|x| !x.is_failure()));
        check_header(b"%PDF-9.9\n", &mut r);
        check_header(b"<html>", &mut r);
        assert_eq!(r.iter().filter(|x| x.is_failure()).count(), 2);

        let mut r = Vec::new();
        check_trailing_data(b"...%%EOF\n", &mut r);
        check_trailing_data(b"...%%EOF\r\n\n", &mut r);
        assert!(r.iter().all(|x| !x.is_failure()));
        check_trailing_data(b"...%%EOF\nGARBAGE", &mut r);
        check_trailing_data(b"no marker", &mut r);
        assert_eq!(r.iter().filter(|x| x.is_failure()).count(), 2);
    }
}
