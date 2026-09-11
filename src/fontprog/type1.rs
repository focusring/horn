//! Minimal parser for Type 1 font programs (`FontFile`), see *Adobe Type 1
//! Font Format* (the "black book"). Handles PFA and PFB containers, eexec
//! decryption, the built-in `/Encoding`, glyph names from `/CharStrings`,
//! and the advance width from each charstring's `hsbw` / `sbw` operator.

use std::collections::HashMap;

/// A parsed Type 1 font program.
#[derive(Debug, Default)]
pub struct Type1Font {
    /// Glyph names in `/CharStrings` order, with their decrypted charstrings.
    charstrings: Vec<(String, Vec<u8>)>,
    /// Built-in encoding: code → glyph name (None for StandardEncoding).
    builtin_encoding: Option<HashMap<u8, String>>,
    /// True when the cleartext declares `/Encoding StandardEncoding def`.
    pub uses_standard_encoding: bool,
    /// FontMatrix scale (1/1000 for almost every Type 1 font).
    font_matrix_scale: f64,
}

const EEXEC_R: u16 = 55665;
const CHARSTRING_R: u16 = 4330;
const C1: u16 = 52845;
const C2: u16 = 22719;

impl Type1Font {
    /// Parse a Type 1 font program (PFA or PFB). Returns `None` if no
    /// `eexec` section or `/CharStrings` dictionary can be found.
    pub fn parse(data: &[u8]) -> Option<Self> {
        let data = unwrap_pfb(data);
        let eexec_pos = find(&data, b"eexec")?;
        let clear = &data[..eexec_pos];
        let mut enc_start = eexec_pos + 5;
        while enc_start < data.len() && matches!(data[enc_start], b'\r' | b'\n' | b' ' | b'\t') {
            enc_start += 1;
        }
        let encrypted = &data[enc_start..];
        let bin = if encrypted.iter().take(4).all(u8::is_ascii_hexdigit) {
            hex_decode(encrypted)
        } else {
            encrypted.to_vec()
        };
        let private = decrypt(&bin, EEXEC_R, 4);

        let len_iv = find(&private, b"/lenIV")
            .and_then(|p| parse_int_after(&private, p + 6))
            .unwrap_or(4);

        let mut font = Self {
            charstrings: Vec::new(),
            builtin_encoding: None,
            uses_standard_encoding: false,
            font_matrix_scale: 0.001,
        };
        font.parse_cleartext(clear);
        font.parse_charstrings(&private, len_iv)?;
        Some(font)
    }

    /// All glyph names defined in `/CharStrings`.
    pub fn glyph_names(&self) -> Vec<&str> {
        self.charstrings.iter().map(|(n, _)| n.as_str()).collect()
    }

    /// True if the font program defines a glyph with this name.
    pub fn has_glyph(&self, name: &[u8]) -> bool {
        self.charstrings.iter().any(|(n, _)| n.as_bytes() == name)
    }

    /// Glyph name for a character code in the font's built-in encoding.
    pub fn builtin_glyph_name(&self, code: u8) -> Option<&str> {
        match &self.builtin_encoding {
            Some(map) => map.get(&code).map(String::as_str),
            None => super::encodings::STANDARD[usize::from(code)],
        }
    }

    /// Advance width of a glyph in 1/1000 text space units, from `hsbw`/`sbw`.
    pub fn advance_width(&self, name: &[u8]) -> Option<f64> {
        let (_, cs) = self.charstrings.iter().find(|(n, _)| n.as_bytes() == name)?;
        let wx = charstring_width(cs)?;
        Some(wx * self.font_matrix_scale * 1000.0)
    }

    fn parse_cleartext(&mut self, clear: &[u8]) {
        if let Some(p) = find(clear, b"/FontMatrix") {
            let text = String::from_utf8_lossy(&clear[p + 11..(p + 120).min(clear.len())]);
            if let Some(start) = text.find('[') {
                if let Some(end) = text[start..].find(']') {
                    let nums: Vec<f64> = text[start + 1..start + end]
                        .split_whitespace()
                        .filter_map(|t| t.parse().ok())
                        .collect();
                    if let Some(a) = nums.first() {
                        if *a > 0.0 {
                            self.font_matrix_scale = *a;
                        }
                    }
                }
            }
        }
        let Some(p) = find(clear, b"/Encoding") else {
            return;
        };
        let rest = &clear[p + 9..];
        let head = String::from_utf8_lossy(&rest[..rest.len().min(40)]);
        if head.trim_start().starts_with("StandardEncoding") {
            self.uses_standard_encoding = true;
            return;
        }
        // Custom encoding: `dup <code> /<name> put` entries until `readonly def` / ` def`
        let mut map = HashMap::new();
        let end = find(rest, b" def").unwrap_or(rest.len());
        let text = String::from_utf8_lossy(&rest[..end]);
        let mut tokens = text.split_whitespace().peekable();
        while let Some(tok) = tokens.next() {
            if tok == "dup" {
                let code = tokens.next().and_then(|c| c.parse::<u32>().ok());
                let name = tokens.next();
                if let (Some(code), Some(name)) = (code, name) {
                    if let (Ok(code), Some(name)) = (u8::try_from(code), name.strip_prefix('/')) {
                        map.insert(code, name.to_string());
                    }
                }
            }
        }
        self.builtin_encoding = Some(map);
    }

    fn parse_charstrings(&mut self, private: &[u8], len_iv: usize) -> Option<()> {
        // Skip past /Subrs binary data safely: scan for `/CharStrings` while honouring
        // `<len> RD ` / `<len> -| ` binary runs so binary bytes cannot be mistaken
        // for tokens.
        let cs_pos = find_token_outside_binary(private, b"/CharStrings")?;
        let mut pos = cs_pos + 12;
        // Entries: /name len RD <bin> ND  (until `end`)
        loop {
            let Some(slash) = find(&private[pos..], b"/") else { break };
            let name_start = pos + slash + 1;
            let mut name_end = name_start;
            while name_end < private.len() && !private[name_end].is_ascii_whitespace() && private[name_end] != b'{' {
                name_end += 1;
            }
            let name = String::from_utf8_lossy(&private[name_start..name_end]).into_owned();
            // length integer
            let Some((len, after_len)) = parse_int_from(private, name_end) else {
                break;
            };
            // RD token: skip whitespace, token, one space
            let mut p = after_len;
            while p < private.len() && private[p].is_ascii_whitespace() {
                p += 1;
            }
            while p < private.len() && !private[p].is_ascii_whitespace() {
                p += 1;
            }
            p += 1; // single space after RD
            let Some(bin) = private.get(p..p + len) else { break };
            let cs = decrypt(bin, CHARSTRING_R, len_iv);
            self.charstrings.push((name, cs));
            pos = p + len;
            // Stop at `end`
            let lookahead = &private[pos..(pos + 64).min(private.len())];
            let la = String::from_utf8_lossy(lookahead);
            let la = la.trim_start();
            if la.starts_with("end") || !la.contains('/') && la.contains("end") {
                // `ND end` after the last entry
                if find(lookahead, b"/").is_none() {
                    break;
                }
            }
        }
        if self.charstrings.is_empty() {
            None
        } else {
            Some(())
        }
    }
}

/// Width from a decrypted Type 1 charstring: `sbx wx hsbw` or `sbx sby wx wy sbw`.
fn charstring_width(cs: &[u8]) -> Option<f64> {
    let mut stack: Vec<f64> = Vec::new();
    let mut i = 0;
    while i < cs.len() {
        let b0 = cs[i];
        match b0 {
            13 => return stack.get(1).copied(), // hsbw
            12 => {
                let b1 = *cs.get(i + 1)?;
                if b1 == 7 {
                    return stack.get(2).copied(); // sbw
                }
                if b1 == 12 {
                    // div
                    let b = stack.pop()?;
                    let a = stack.pop()?;
                    stack.push(a / b);
                    i += 2;
                    continue;
                }
                return None;
            }
            32..=246 => {
                stack.push(f64::from(i32::from(b0) - 139));
                i += 1;
            }
            247..=250 => {
                let b1 = i32::from(*cs.get(i + 1)?);
                stack.push(f64::from((i32::from(b0) - 247) * 256 + b1 + 108));
                i += 2;
            }
            251..=254 => {
                let b1 = i32::from(*cs.get(i + 1)?);
                stack.push(f64::from(-(i32::from(b0) - 251) * 256 - b1 - 108));
                i += 2;
            }
            255 => {
                let v = i32::from_be_bytes([*cs.get(i + 1)?, *cs.get(i + 2)?, *cs.get(i + 3)?, *cs.get(i + 4)?]);
                stack.push(f64::from(v));
                i += 5;
            }
            _ => return None, // any other operator before hsbw: give up
        }
    }
    None
}

/// Strip PFB segment headers (0x80 0x01/0x02 <len32 LE>) if present.
fn unwrap_pfb(data: &[u8]) -> Vec<u8> {
    if data.len() < 6 || data[0] != 0x80 {
        return data.to_vec();
    }
    let mut out = Vec::with_capacity(data.len());
    let mut pos = 0;
    while pos + 6 <= data.len() && data[pos] == 0x80 {
        let kind = data[pos + 1];
        if kind == 3 {
            break;
        }
        let len = u32::from_le_bytes([data[pos + 2], data[pos + 3], data[pos + 4], data[pos + 5]]) as usize;
        pos += 6;
        let end = (pos + len).min(data.len());
        out.extend_from_slice(&data[pos..end]);
        pos = end;
    }
    out
}

fn decrypt(data: &[u8], mut r: u16, skip: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len());
    for &c in data {
        let p = c ^ (r >> 8) as u8;
        r = (u16::from(c).wrapping_add(r)).wrapping_mul(C1).wrapping_add(C2);
        out.push(p);
    }
    if out.len() > skip {
        out.drain(..skip);
        out
    } else {
        Vec::new()
    }
}

fn hex_decode(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() / 2);
    let mut hi: Option<u8> = None;
    for &b in data {
        let v = match b {
            b'0'..=b'9' => b - b'0',
            b'a'..=b'f' => b - b'a' + 10,
            b'A'..=b'F' => b - b'A' + 10,
            _ => continue,
        };
        if let Some(h) = hi {
            out.push(h << 4 | v);
            hi = None;
        } else {
            hi = Some(v);
        }
    }
    out
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// Find `needle` while skipping binary runs introduced by `<int> RD ` / `<int> -| `.
fn find_token_outside_binary(data: &[u8], needle: &[u8]) -> Option<usize> {
    let mut pos = 0;
    while pos < data.len() {
        let rel = find(&data[pos..], needle)?;
        let candidate = pos + rel;
        // Look for a binary run starting between pos and candidate
        let mut scan = pos;
        let mut skipped = false;
        while scan < candidate {
            // pattern: digits, space, (RD|-|), space
            if data[scan].is_ascii_digit() {
                if let Some((len, after)) = parse_int_from(data, scan) {
                    let tail = &data[after..(after + 4).min(data.len())];
                    if tail.starts_with(b" RD ") || tail.starts_with(b" -| ") {
                        let bin_start = after + 4;
                        let bin_end = bin_start + len;
                        if bin_start <= candidate && candidate < bin_end {
                            pos = bin_end;
                            skipped = true;
                            break;
                        }
                        scan = bin_end.max(scan + 1);
                        continue;
                    }
                }
            }
            scan += 1;
        }
        if !skipped {
            return Some(candidate);
        }
    }
    None
}

/// Parse an ASCII integer starting at `pos` (after optional whitespace).
fn parse_int_from(data: &[u8], mut pos: usize) -> Option<(usize, usize)> {
    while pos < data.len() && data[pos].is_ascii_whitespace() {
        pos += 1;
    }
    let start = pos;
    while pos < data.len() && data[pos].is_ascii_digit() {
        pos += 1;
    }
    if start == pos {
        return None;
    }
    let v: usize = std::str::from_utf8(&data[start..pos]).ok()?.parse().ok()?;
    Some((v, pos))
}

fn parse_int_after(data: &[u8], pos: usize) -> Option<usize> {
    parse_int_from(data, pos).map(|(v, _)| v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eexec_roundtrip() {
        // Encrypt then decrypt a small payload with the eexec constants.
        let plain = b"\0\0\0\0hello /CharStrings";
        let mut r = EEXEC_R;
        let mut enc = Vec::new();
        for &p in plain {
            let c = p ^ (r >> 8) as u8;
            r = (u16::from(c).wrapping_add(r)).wrapping_mul(C1).wrapping_add(C2);
            enc.push(c);
        }
        assert_eq!(decrypt(&enc, EEXEC_R, 4), b"hello /CharStrings");
    }

    #[test]
    fn hsbw_width() {
        // 50 600 hsbw  => sbx=50 wx=600 : numbers as (v+139) single bytes need v<=107,
        // so encode 600 with 247..250 form: (b0-247)*256 + b1 + 108 = 600 -> b0=248, b1=236
        let cs = [50 + 139, 248, 236, 13];
        assert_eq!(charstring_width(&cs), Some(600.0));
    }
}
