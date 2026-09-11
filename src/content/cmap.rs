//! Parsers for the two flavours of `CMap` that appear in PDF files:
//!
//! - **Encoding `CMaps`** embedded as the `/Encoding` of a Type 0 font
//!   (`begincodespacerange`, `begincidrange`, `begincidchar`, `usecmap`), used
//!   to split byte strings into character codes and map codes to CIDs.
//! - **`ToUnicode` `CMaps`** (`beginbfchar`, `beginbfrange`), used to map codes to
//!   Unicode.
//!
//! Both share the small PostScript-like token syntax handled by [`tokenize`].

// CMap numbers are small non-negative integers by construction (codes, CIDs, byte lengths).
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::too_many_lines
)]

use std::collections::BTreeMap;

/// A token of `CMap` syntax.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// `<hex>` string with its byte length.
    Hex(Vec<u8>),
    /// Integer or real number.
    Number(f64),
    /// `/Name`.
    Name(String),
    /// Bare operator / keyword.
    Op(String),
    /// `[`, `]`, `<<`, `>>`, `{`, `}`.
    Delim(&'static str),
}

/// Tokenize `CMap` / PostScript syntax (strings in parentheses are skipped).
pub fn tokenize(data: &[u8]) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < data.len() {
        let c = data[i];
        match c {
            b'%' => {
                while i < data.len() && data[i] != b'\n' && data[i] != b'\r' {
                    i += 1;
                }
            }
            b'<' => {
                if data.get(i + 1) == Some(&b'<') {
                    tokens.push(Token::Delim("<<"));
                    i += 2;
                } else {
                    let mut j = i + 1;
                    let mut bytes = Vec::new();
                    let mut hi: Option<u8> = None;
                    while j < data.len() && data[j] != b'>' {
                        let v = match data[j] {
                            b'0'..=b'9' => Some(data[j] - b'0'),
                            b'a'..=b'f' => Some(data[j] - b'a' + 10),
                            b'A'..=b'F' => Some(data[j] - b'A' + 10),
                            _ => None,
                        };
                        if let Some(v) = v {
                            if let Some(h) = hi {
                                bytes.push(h << 4 | v);
                                hi = None;
                            } else {
                                hi = Some(v);
                            }
                        }
                        j += 1;
                    }
                    if let Some(h) = hi {
                        bytes.push(h << 4);
                    }
                    tokens.push(Token::Hex(bytes));
                    i = j + 1;
                }
            }
            b'>' => {
                if data.get(i + 1) == Some(&b'>') {
                    tokens.push(Token::Delim(">>"));
                    i += 2;
                } else {
                    i += 1;
                }
            }
            b'[' => {
                tokens.push(Token::Delim("["));
                i += 1;
            }
            b']' => {
                tokens.push(Token::Delim("]"));
                i += 1;
            }
            b'{' => {
                tokens.push(Token::Delim("{"));
                i += 1;
            }
            b'}' => {
                tokens.push(Token::Delim("}"));
                i += 1;
            }
            b'(' => {
                // literal string: skip with nesting
                let mut depth = 0;
                while i < data.len() {
                    match data[i] {
                        b'\\' => i += 1,
                        b'(' => depth += 1,
                        b')' => {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                        _ => {}
                    }
                    i += 1;
                }
                i += 1;
            }
            b'/' => {
                let mut j = i + 1;
                while j < data.len() && !is_delim_or_ws(data[j]) {
                    j += 1;
                }
                tokens.push(Token::Name(
                    String::from_utf8_lossy(&data[i + 1..j]).into_owned(),
                ));
                i = j;
            }
            c if c.is_ascii_whitespace() => i += 1,
            _ => {
                let mut j = i;
                while j < data.len() && !is_delim_or_ws(data[j]) {
                    j += 1;
                }
                let word = String::from_utf8_lossy(&data[i..j]).into_owned();
                if let Ok(n) = word.parse::<f64>() {
                    tokens.push(Token::Number(n));
                } else {
                    tokens.push(Token::Op(word));
                }
                i = j.max(i + 1);
            }
        }
    }
    tokens
}

fn is_delim_or_ws(b: u8) -> bool {
    b.is_ascii_whitespace()
        || matches!(
            b,
            b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'(' | b')' | b'%'
        )
}

/// A byte-length-aware codespace range.
#[derive(Debug, Clone)]
pub struct CodespaceRange {
    pub num_bytes: usize,
    pub low: u32,
    pub high: u32,
}

/// A parsed encoding `CMap` (code → CID).
#[derive(Debug, Clone, Default)]
pub struct EncodingCMap {
    pub codespaces: Vec<CodespaceRange>,
    /// Single-code mappings.
    pub cid_chars: BTreeMap<(usize, u32), u32>,
    /// Range mappings: (`num_bytes`, low, high, first cid).
    pub cid_ranges: Vec<(usize, u32, u32, u32)>,
    /// `usecmap` target, if any (only predefined names are recorded).
    pub use_cmap: Option<String>,
    /// `/WMode` declared in the `CMap` program.
    pub wmode: Option<i64>,
}

impl EncodingCMap {
    /// The Identity-H / Identity-V `CMap`: two-byte codes, CID = code.
    pub fn identity() -> Self {
        Self {
            codespaces: vec![CodespaceRange {
                num_bytes: 2,
                low: 0,
                high: 0xFFFF,
            }],
            cid_chars: BTreeMap::new(),
            cid_ranges: vec![(2, 0, 0xFFFF, 0)],
            use_cmap: None,
            wmode: None,
        }
    }

    /// Parse an embedded `CMap` program.
    pub fn parse(data: &[u8]) -> Self {
        let tokens = tokenize(data);
        let mut cmap = Self::default();
        let mut i = 0;
        while i < tokens.len() {
            match &tokens[i] {
                Token::Op(op) if op == "begincodespacerange" => {
                    i += 1;
                    while i + 1 < tokens.len() {
                        match (&tokens[i], &tokens[i + 1]) {
                            (Token::Hex(lo), Token::Hex(hi)) => {
                                cmap.codespaces.push(CodespaceRange {
                                    num_bytes: lo.len().clamp(1, 4),
                                    low: be_u32(lo),
                                    high: be_u32(hi),
                                });
                                i += 2;
                            }
                            _ => break,
                        }
                    }
                }
                Token::Op(op) if op == "begincidrange" => {
                    i += 1;
                    while i + 2 < tokens.len() {
                        match (&tokens[i], &tokens[i + 1], &tokens[i + 2]) {
                            (Token::Hex(lo), Token::Hex(hi), Token::Number(cid)) => {
                                cmap.cid_ranges.push((
                                    lo.len().clamp(1, 4),
                                    be_u32(lo),
                                    be_u32(hi),
                                    *cid as u32,
                                ));
                                i += 3;
                            }
                            _ => break,
                        }
                    }
                }
                Token::Op(op) if op == "begincidchar" => {
                    i += 1;
                    while i + 1 < tokens.len() {
                        match (&tokens[i], &tokens[i + 1]) {
                            (Token::Hex(code), Token::Number(cid)) => {
                                cmap.cid_chars
                                    .insert((code.len().clamp(1, 4), be_u32(code)), *cid as u32);
                                i += 2;
                            }
                            _ => break,
                        }
                    }
                }
                Token::Op(op) if op == "usecmap" => {
                    if let Some(Token::Name(n)) = i.checked_sub(1).and_then(|p| tokens.get(p)) {
                        cmap.use_cmap = Some(n.clone());
                    }
                    i += 1;
                }
                Token::Name(n) if n == "WMode" => {
                    if let Some(Token::Number(v)) = tokens.get(i + 1) {
                        cmap.wmode = Some(*v as i64);
                    }
                    i += 1;
                }
                _ => i += 1,
            }
        }
        if cmap.codespaces.is_empty() {
            // Infer from the mappings' byte lengths
            let mut lens: Vec<usize> = cmap.cid_ranges.iter().map(|r| r.0).collect();
            lens.extend(cmap.cid_chars.keys().map(|k| k.0));
            lens.sort_unstable();
            lens.dedup();
            for n in lens {
                cmap.codespaces.push(CodespaceRange {
                    num_bytes: n,
                    low: 0,
                    high: if n >= 4 {
                        u32::MAX
                    } else {
                        (1u32 << (8 * n as u32)) - 1
                    },
                });
            }
        }
        cmap
    }

    /// True when the codespace is well-defined enough to split strings.
    pub fn can_split(&self) -> bool {
        !self.codespaces.is_empty()
    }

    /// Split a byte string into (code, byte length) pairs following the
    /// codespace ranges (ISO 32000-1, 9.7.6.3). Undefined byte sequences use the
    /// shortest codespace length so that scanning can continue.
    pub fn split_codes(&self, bytes: &[u8]) -> Vec<(u32, usize)> {
        let mut out = Vec::new();
        let mut i = 0;
        let min_len = self
            .codespaces
            .iter()
            .map(|c| c.num_bytes)
            .min()
            .unwrap_or(1);
        while i < bytes.len() {
            let mut matched = None;
            for n in 1..=4usize {
                if i + n > bytes.len() {
                    break;
                }
                let code = be_u32(&bytes[i..i + n]);
                if self
                    .codespaces
                    .iter()
                    .any(|c| c.num_bytes == n && code >= c.low && code <= c.high)
                {
                    matched = Some((code, n));
                    break;
                }
            }
            let (code, n) = matched.unwrap_or_else(|| {
                let n = min_len.min(bytes.len() - i).max(1);
                (be_u32(&bytes[i..i + n]), n)
            });
            out.push((code, n));
            i += n;
        }
        out
    }

    /// Map a code (with its byte length) to a CID.
    pub fn to_cid(&self, code: u32, num_bytes: usize) -> Option<u32> {
        if let Some(cid) = self.cid_chars.get(&(num_bytes, code)) {
            return Some(*cid);
        }
        for (n, lo, hi, cid) in &self.cid_ranges {
            if *n == num_bytes && code >= *lo && code <= *hi {
                return Some(cid + (code - lo));
            }
        }
        if matches!(self.use_cmap.as_deref(), Some("Identity-H" | "Identity-V")) {
            return Some(code);
        }
        None
    }
}

/// Unicode mapping entries of a `ToUnicode` `CMap`: for every code, the mapped
/// UTF-16BE code units (empty if the destination string was empty).
#[derive(Debug, Clone, Default)]
pub struct ToUnicodeCMap {
    pub chars: BTreeMap<(usize, u32), Vec<u16>>,
    /// (`num_bytes`, low, high, first destination code units)
    pub ranges: Vec<(usize, u32, u32, Vec<u16>)>,
    pub codespaces: Vec<CodespaceRange>,
}

impl ToUnicodeCMap {
    /// Parse a `ToUnicode` `CMap` stream.
    pub fn parse(data: &[u8]) -> Self {
        let tokens = tokenize(data);
        let mut cmap = Self::default();
        let mut i = 0;
        while i < tokens.len() {
            match &tokens[i] {
                Token::Op(op) if op == "begincodespacerange" => {
                    i += 1;
                    while i + 1 < tokens.len() {
                        match (&tokens[i], &tokens[i + 1]) {
                            (Token::Hex(lo), Token::Hex(hi)) => {
                                cmap.codespaces.push(CodespaceRange {
                                    num_bytes: lo.len().clamp(1, 4),
                                    low: be_u32(lo),
                                    high: be_u32(hi),
                                });
                                i += 2;
                            }
                            _ => break,
                        }
                    }
                }
                Token::Op(op) if op == "beginbfchar" => {
                    i += 1;
                    while i + 1 < tokens.len() {
                        match (&tokens[i], &tokens[i + 1]) {
                            (Token::Hex(src), Token::Hex(dst)) => {
                                cmap.chars
                                    .insert((src.len().clamp(1, 4), be_u32(src)), utf16_units(dst));
                                i += 2;
                            }
                            (Token::Hex(src), Token::Name(n)) => {
                                // dst may be a glyph name (rare); record as empty
                                let _ = n;
                                cmap.chars
                                    .insert((src.len().clamp(1, 4), be_u32(src)), Vec::new());
                                i += 2;
                            }
                            _ => break,
                        }
                    }
                }
                Token::Op(op) if op == "beginbfrange" => {
                    i += 1;
                    while i + 2 < tokens.len() {
                        match (&tokens[i], &tokens[i + 1], &tokens[i + 2]) {
                            (Token::Hex(lo), Token::Hex(hi), Token::Hex(dst)) => {
                                cmap.ranges.push((
                                    lo.len().clamp(1, 4),
                                    be_u32(lo),
                                    be_u32(hi),
                                    utf16_units(dst),
                                ));
                                i += 3;
                            }
                            (Token::Hex(lo), Token::Hex(hi), Token::Delim("[")) => {
                                let n = lo.len().clamp(1, 4);
                                let (lo, hi) = (be_u32(lo), be_u32(hi));
                                i += 3;
                                let mut code = lo;
                                while i < tokens.len() && tokens[i] != Token::Delim("]") {
                                    if let Token::Hex(dst) = &tokens[i] {
                                        if code <= hi {
                                            cmap.chars.insert((n, code), utf16_units(dst));
                                        }
                                        code = code.wrapping_add(1);
                                    }
                                    i += 1;
                                }
                                i += 1;
                            }
                            _ => break,
                        }
                    }
                }
                _ => i += 1,
            }
        }
        cmap
    }

    /// Unicode code units for a code, if mapped.
    pub fn lookup(&self, code: u32, num_bytes: usize) -> Option<Vec<u16>> {
        if let Some(u) = self.chars.get(&(num_bytes, code)) {
            return Some(u.clone());
        }
        for (n, lo, hi, dst) in &self.ranges {
            if *n == num_bytes && code >= *lo && code <= *hi {
                let mut units = dst.clone();
                if let Some(last) = units.last_mut() {
                    *last = last.wrapping_add((code - lo) as u16);
                }
                return Some(units);
            }
        }
        None
    }

    /// Every destination code-unit sequence in the `CMap` (chars and range starts),
    /// for scanning for forbidden values.
    pub fn all_destinations(&self) -> impl Iterator<Item = &Vec<u16>> {
        self.chars.values().chain(self.ranges.iter().map(|r| &r.3))
    }

    /// True if the `CMap` has no mappings at all.
    pub fn is_empty(&self) -> bool {
        self.chars.is_empty() && self.ranges.is_empty()
    }
}

fn be_u32(bytes: &[u8]) -> u32 {
    bytes
        .iter()
        .take(4)
        .fold(0u32, |acc, b| (acc << 8) | u32::from(*b))
}

fn utf16_units(bytes: &[u8]) -> Vec<u16> {
    if bytes.len() == 1 {
        return vec![u16::from(bytes[0])];
    }
    bytes
        .chunks(2)
        .map(|c| u16::from_be_bytes([c[0], *c.get(1).unwrap_or(&0)]))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_cid_cmap() {
        let src = b"/CIDInit /ProcSet findresource begin begincmap /WMode 1 def
1 begincodespacerange <00> <80> <8140> <9ffc> endcodespacerange
2 begincidrange <20> <7e> 1 <8140> <817e> 633 endcidrange
1 begincidchar <80> 97 endcidchar endcmap";
        let c = EncodingCMap::parse(src);
        assert_eq!(c.wmode, Some(1));
        assert_eq!(c.codespaces.len(), 2);
        assert_eq!(
            c.split_codes(&[0x20, 0x81, 0x40, 0x80]),
            vec![(0x20, 1), (0x8140, 2), (0x80, 1)]
        );
        assert_eq!(c.to_cid(0x21, 1), Some(2));
        assert_eq!(c.to_cid(0x8141, 2), Some(634));
        assert_eq!(c.to_cid(0x80, 1), Some(97));
    }

    #[test]
    fn parses_tounicode() {
        let src = b"1 begincodespacerange <0000> <FFFF> endcodespacerange
2 beginbfchar <0003> <0020> <0024> <D835DC00> endbfchar
2 beginbfrange <0010> <0012> <0041> <0020> <0021> [<0061> <0000>] endbfrange";
        let c = ToUnicodeCMap::parse(src);
        assert_eq!(c.lookup(0x0003, 2), Some(vec![0x20]));
        assert_eq!(c.lookup(0x0024, 2), Some(vec![0xD835, 0xDC00]));
        assert_eq!(c.lookup(0x0011, 2), Some(vec![0x42]));
        assert_eq!(c.lookup(0x0021, 2), Some(vec![0x0000]));
        assert!(c.all_destinations().any(|d| d == &vec![0u16]));
    }
}
