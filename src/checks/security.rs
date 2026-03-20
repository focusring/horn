use crate::checks::Check;
use crate::document::HornDocument;
use crate::model::{CheckOutcome, CheckResult, Severity};
use anyhow::Result;

/// Checkpoint 26: Security.
///
/// PDF/UA-1 requires that encryption settings do not prevent assistive technology
/// access. Specifically, the /P permissions flag in the /Encrypt dictionary must
/// allow content extraction for accessibility (bit 10) and content copying (bit 5).
pub struct SecurityChecks;

impl Check for SecurityChecks {
    fn id(&self) -> &'static str {
        "26-security"
    }

    fn checkpoint(&self) -> u8 {
        26
    }

    fn description(&self) -> &'static str {
        "Security: encryption must not block assistive technology access"
    }

    fn run(&self, doc: &mut HornDocument) -> Result<Vec<CheckResult>> {
        let mut results = Vec::new();
        check_encryption_permissions(doc, &mut results);
        Ok(results)
    }
}

/// 26-001 / 26-002: Encryption permissions must allow accessibility.
///
/// The /Encrypt dictionary in the trailer contains a /P integer whose bits
/// control permissions. Per PDF spec (ISO 32000-1 Table 22):
/// - Bit 5 (value 16):   Copy or extract text and graphics
/// - Bit 10 (value 512):  Extract text and graphics for accessibility
///
/// PDF/UA-1 requires bit 10 to be set. We also check bit 5 as a secondary concern.
fn check_encryption_permissions(doc: &mut HornDocument, results: &mut Vec<CheckResult>) {
    let lopdf_doc = doc.lopdf();

    // Try to find /Encrypt in the trailer dictionary.
    // lopdf may also expose it after parsing xref streams.
    let encrypt_dict = find_encrypt_dict(lopdf_doc);

    // If lopdf didn't give us an Encrypt dict (it may strip it after decryption),
    // fall back to scanning raw PDF objects for an Encrypt-like dictionary with /P.
    let encrypt_dict = match encrypt_dict {
        Some(d) => Some(d),
        None => find_encrypt_in_objects(lopdf_doc),
    };

    // If lopdf stripped the Encrypt dict entirely (common after decryption),
    // fall back to parsing the raw file bytes to extract /P value.
    // Prefer in-memory bytes (available when loaded via from_bytes / WASM),
    // falling back to reading from disk for file-path mode.
    if encrypt_dict.is_none() {
        let raw = doc.raw_bytes().map(|b| b.to_vec());
        #[cfg(not(target_arch = "wasm32"))]
        let raw = raw.or_else(|| std::fs::read(doc.path()).ok());
        if let Some(ref raw_bytes) = raw {
            if let Some(p_value) = extract_p_from_raw(raw_bytes) {
                emit_permission_results(p_value, results);
                return;
            }
        }
        // Truly no encryption found
        results.push(pass(
            "26-001",
            "No encryption — assistive technology access unrestricted",
        ));
        return;
    }

    let encrypt_dict = encrypt_dict.unwrap();

    match encrypt_dict.get(b"P") {
        Ok(p_obj) => {
            let p_value = match p_obj.as_i64() {
                Ok(v) => v,
                Err(_) => {
                    results.push(fail("26-001", "/Encrypt/P is not an integer"));
                    return;
                }
            };
            emit_permission_results(p_value, results);
        }
        Err(_) => {
            results.push(fail(
                "26-001",
                "/Encrypt dictionary missing /P permissions entry",
            ));
        }
    }
}

/// Emit pass/fail results based on the /P permissions integer.
fn emit_permission_results(p_value: i64, results: &mut Vec<CheckResult>) {
    // Bit 10 (0-indexed bit 9): Extract for accessibility
    // In PDF spec, bits are 1-indexed, so bit 10 = (1 << 9) = 512
    let accessibility_bit = (p_value >> 9) & 1 == 1;

    // Bit 5 (0-indexed bit 4): Copy or extract
    // Bit 5 = (1 << 4) = 16
    let copy_bit = (p_value >> 4) & 1 == 1;

    if accessibility_bit {
        results.push(pass(
            "26-001",
            "Encryption allows assistive technology access (bit 10 set)",
        ));
    } else {
        results.push(fail(
            "26-001",
            "Encryption blocks assistive technology access — /P bit 10 (accessibility extraction) is not set",
        ));
    }

    if copy_bit {
        results.push(pass(
            "26-002",
            "Encryption allows content extraction (bit 5 set)",
        ));
    } else {
        results.push(fail(
            "26-002",
            "Encryption blocks content extraction — /P bit 5 (copy/extract) is not set",
        ));
    }
}

/// Extract the /P value from raw PDF bytes by finding the Encrypt dictionary.
///
/// This is a fallback for when lopdf strips the /Encrypt dict after decryption.
/// We scan for PDF objects containing `/Filter/Standard` and extract the /P value.
fn extract_p_from_raw(data: &[u8]) -> Option<i64> {
    // Find each "N N obj" ... "endobj" block and check for encryption dict
    let mut pos = 0;
    while pos < data.len() {
        // Find next "obj" keyword (part of "N N obj")
        let obj_pos = match find_bytes(data, b" obj", pos) {
            Some(p) => p,
            None => break,
        };

        // Find the matching endobj
        let search_from = obj_pos + 4;
        let endobj_pos = match find_bytes(data, b"endobj", search_from) {
            Some(p) => p,
            None => break,
        };

        // Extract this object's content
        let obj_content = &data[obj_pos..endobj_pos];

        // Check if this object contains /Filter/Standard (encryption dictionary)
        let has_standard = find_bytes(obj_content, b"/Filter/Standard", 0).is_some()
            || find_bytes(obj_content, b"/Filter /Standard", 0).is_some();

        if has_standard {
            // Extract /P value from this object
            if let Some(p_val) = extract_p_value(obj_content) {
                return Some(p_val);
            }
        }

        pos = endobj_pos + 6;
    }
    None
}

/// Extract /P integer value from a byte slice containing a PDF dictionary.
fn extract_p_value(data: &[u8]) -> Option<i64> {
    let mut i = 0;
    while i + 2 < data.len() {
        if data[i] == b'/'
            && data[i + 1] == b'P'
            && (i + 2 >= data.len() || !data[i + 2].is_ascii_alphabetic())
        {
            // Skip "/P" and whitespace
            let mut j = i + 2;
            while j < data.len()
                && (data[j] == b' ' || data[j] == b'\n' || data[j] == b'\r' || data[j] == b'\t')
            {
                j += 1;
            }
            // Parse integer (possibly negative)
            let num_start = j;
            if j < data.len() && (data[j] == b'-' || data[j].is_ascii_digit()) {
                j += 1;
                while j < data.len() && data[j].is_ascii_digit() {
                    j += 1;
                }
                let num_str = std::str::from_utf8(&data[num_start..j]).ok()?;
                if let Ok(val) = num_str.parse::<i64>() {
                    return Some(val);
                }
            }
        }
        i += 1;
    }
    None
}

fn find_bytes(haystack: &[u8], needle: &[u8], start: usize) -> Option<usize> {
    if start >= haystack.len() || needle.len() > haystack.len() - start {
        return None;
    }
    haystack[start..]
        .windows(needle.len())
        .position(|w| w == needle)
        .map(|p| p + start)
}

/// Try to find the Encrypt dictionary from the trailer.
fn find_encrypt_dict(doc: &lopdf::Document) -> Option<&lopdf::Dictionary> {
    let encrypt_ref = doc.trailer.get(b"Encrypt").ok()?;
    let encrypt_obj = if let Ok(ref_id) = encrypt_ref.as_reference() {
        doc.get_object(ref_id).ok()?
    } else {
        encrypt_ref
    };
    encrypt_obj.as_dict().ok()
}

/// Scan all document objects for an encryption dictionary.
///
/// lopdf may strip the /Encrypt entry from the trailer after decrypting the
/// document, but the encryption dictionary object itself usually remains.
/// We identify it by the presence of /Filter (typically /Standard),
/// /P (permissions), and /R (revision) keys.
fn find_encrypt_in_objects(doc: &lopdf::Document) -> Option<&lopdf::Dictionary> {
    for (_id, obj) in &doc.objects {
        if let Ok(dict) = obj.as_dict() {
            // Check for encryption dictionary hallmarks
            let has_filter =
                dict.get(b"Filter").ok().and_then(|f| f.as_name().ok()) == Some(b"Standard");
            let has_p = dict.get(b"P").ok().and_then(|p| p.as_i64().ok()).is_some();
            let has_r = dict.get(b"R").ok().and_then(|r| r.as_i64().ok()).is_some();

            if has_filter && has_p && has_r {
                return Some(dict);
            }
        }
        // Also check inside streams (xref streams have dict entries)
        if let Ok(stream) = obj.as_stream() {
            let dict = &stream.dict;
            let has_filter =
                dict.get(b"Filter").ok().and_then(|f| f.as_name().ok()) == Some(b"Standard");
            let has_p = dict.get(b"P").ok().and_then(|p| p.as_i64().ok()).is_some();
            let has_r = dict.get(b"R").ok().and_then(|r| r.as_i64().ok()).is_some();

            if has_filter && has_p && has_r {
                return Some(dict);
            }
        }
    }
    None
}

fn pass(rule_id: &str, description: &str) -> CheckResult {
    CheckResult {
        rule_id: rule_id.to_string(),
        checkpoint: 26,
        description: description.to_string(),
        severity: Severity::Info,
        outcome: CheckOutcome::Pass,
    }
}

fn fail(rule_id: &str, message: &str) -> CheckResult {
    CheckResult {
        rule_id: rule_id.to_string(),
        checkpoint: 26,
        description: message.to_string(),
        severity: Severity::Error,
        outcome: CheckOutcome::Fail {
            message: message.to_string(),
            location: None,
        },
    }
}
