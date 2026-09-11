//! Minimal parser for bare CFF font programs (`FontFile3` with subtype
//! `Type1C` or `CIDFontType0C`), see *The Compact Font Format Specification*
//! (Adobe Technical Note #5176) and *Type 2 Charstring Format* (TN #5177).
//!
//! Only what the PDF/UA font checks need is parsed: the glyph count, the
//! charset (glyph names or CIDs), and the advance width encoded at the start
//! of each Type 2 charstring.

use std::collections::HashMap;

/// A parsed CFF font program.
#[derive(Debug)]
pub struct CffFont {
    /// Number of glyphs (entries in the CharStrings INDEX).
    pub glyph_count: usize,
    /// Charset: for name-keyed fonts the SID of every glyph, for CID-keyed
    /// fonts the CID of every glyph (index = GID). Empty if the charset is a
    /// predefined Expert charset we do not model.
    charset: Vec<u16>,
    /// True for CID-keyed fonts (Top DICT has a ROS operator).
    pub is_cid: bool,
    /// Custom strings from the String INDEX (SID 391+).
    strings: Vec<Vec<u8>>,
    /// Raw charstrings, index = GID.
    charstrings: Vec<Vec<u8>>,
    /// Global subroutines.
    gsubrs: Vec<Vec<u8>>,
    /// Private DICT data per font DICT (one entry for name-keyed fonts).
    privates: Vec<PrivateDict>,
    /// GID -> font DICT index (CID fonts only).
    fd_select: Vec<u8>,
    /// CharstringType from the Top DICT (2 is the only one interpreted).
    charstring_type: i32,
    /// Horizontal scale of the FontMatrix (default 0.001).
    font_matrix_scale: f64,
    /// Built-in encoding: code → GID (None = StandardEncoding by name).
    encoding: Option<HashMap<u8, u16>>,
}

#[derive(Debug, Clone, Default)]
struct PrivateDict {
    default_width_x: f64,
    nominal_width_x: f64,
    subrs: Vec<Vec<u8>>,
}

impl CffFont {
    /// Parse a bare CFF font. Returns `None` on any structural error.
    pub fn parse(data: &[u8]) -> Option<Self> {
        let hdr_size = *data.get(2)? as usize;
        let mut pos = hdr_size;
        let (_names, next) = read_index(data, pos)?;
        pos = next;
        let (top_dicts, next) = read_index(data, pos)?;
        pos = next;
        let (strings, next) = read_index(data, pos)?;
        pos = next;
        let (gsubrs, _) = read_index(data, pos)?;

        let top = parse_dict(top_dicts.first()?);
        let is_cid = top.contains_key(&0x0c1e);
        let charstring_type = top
            .get(&0x0c06)
            .and_then(|v| v.first())
            .map_or(2, |v| *v as i32);

        let charstrings_off = usize_op(&top, 17)?;
        let (charstrings, _) = read_index(data, charstrings_off)?;
        let glyph_count = charstrings.len();

        let charset_off = usize_op(&top, 15).unwrap_or(0);
        let charset = parse_charset(data, charset_off, glyph_count);
        let font_matrix_scale = top
            .get(&0x0c07)
            .and_then(|v| v.first())
            .copied()
            .filter(|a| *a > 0.0)
            .unwrap_or(0.001);
        let encoding_off = usize_op(&top, 16).unwrap_or(0);
        let encoding = if is_cid {
            None
        } else {
            parse_encoding(data, encoding_off, &charset)
        };

        let mut privates = Vec::new();
        let mut fd_select = Vec::new();
        if is_cid {
            let fdarray_off = usize_op(&top, 0x0c24)?;
            let (fdicts, _) = read_index(data, fdarray_off)?;
            for fd in &fdicts {
                let fd_dict = parse_dict(fd);
                privates.push(read_private(data, &fd_dict));
            }
            if let Some(fdselect_off) = usize_op(&top, 0x0c25) {
                fd_select = parse_fd_select(data, fdselect_off, glyph_count);
            }
        } else {
            privates.push(read_private(data, &top));
        }

        Some(Self {
            glyph_count,
            charset,
            is_cid,
            strings,
            charstrings,
            gsubrs,
            privates,
            fd_select,
            charstring_type,
            font_matrix_scale,
            encoding,
        })
    }

    /// Glyph name for a code in the font's built-in encoding (name-keyed
    /// fonts). With the standard (or expert) predefined encoding, the code is
    /// looked up in StandardEncoding and resolved by name.
    pub fn builtin_glyph_name(&self, code: u8) -> Option<String> {
        if self.is_cid {
            return None;
        }
        match &self.encoding {
            Some(map) => self.glyph_name(usize::from(*map.get(&code)?)),
            None => {
                let name = super::encodings::STANDARD[usize::from(code)]?;
                self.gid_by_name(name.as_bytes()).map(|_| name.to_string())
            }
        }
    }

    /// Glyph name of a GID (name-keyed fonts only).
    pub fn glyph_name(&self, gid: usize) -> Option<String> {
        if self.is_cid {
            return None;
        }
        let sid = *self.charset.get(gid)? as usize;
        self.sid_to_string(sid)
    }

    /// All glyph names present in the font program (name-keyed fonts only).
    pub fn glyph_names(&self) -> Option<Vec<String>> {
        if self.is_cid || self.charset.is_empty() {
            return None;
        }
        Some(
            (0..self.glyph_count)
                .filter_map(|gid| self.glyph_name(gid))
                .collect(),
        )
    }

    /// GID of a glyph name (name-keyed fonts only).
    pub fn gid_by_name(&self, name: &[u8]) -> Option<usize> {
        if self.is_cid {
            return None;
        }
        (0..self.glyph_count).find(|&gid| {
            self.glyph_name(gid)
                .is_some_and(|n| n.as_bytes() == name)
        })
    }

    /// CID of a GID (CID-keyed fonts). For name-keyed fonts the GID is returned.
    pub fn cid_of_gid(&self, gid: usize) -> Option<u16> {
        if !self.is_cid {
            return (gid < self.glyph_count).then_some(u16::try_from(gid).ok()?);
        }
        self.charset.get(gid).copied()
    }

    /// GID of a CID (CID-keyed fonts). For name-keyed fonts the CID is the GID.
    pub fn gid_of_cid(&self, cid: u16) -> Option<usize> {
        if !self.is_cid {
            return (usize::from(cid) < self.glyph_count).then_some(usize::from(cid));
        }
        self.charset.iter().position(|c| *c == cid)
    }

    /// All CIDs present in the font program (CID-keyed fonts).
    pub fn cids(&self) -> Vec<u16> {
        if self.is_cid {
            self.charset.clone()
        } else {
            (0..self.glyph_count)
                .filter_map(|g| u16::try_from(g).ok())
                .collect()
        }
    }

    /// Advance width of a glyph in font units (1/1000 em for CFF fonts with the
    /// default FontMatrix), read from the start of its Type 2 charstring.
    pub fn advance_width(&self, gid: usize) -> Option<f64> {
        if self.charstring_type != 2 {
            return None;
        }
        let cs = self.charstrings.get(gid)?;
        let fd_index = if self.is_cid {
            self.fd_select.get(gid).copied().unwrap_or(0) as usize
        } else {
            0
        };
        let private = self.privates.get(fd_index)?;
        let mut interp = WidthInterp {
            stack: Vec::new(),
            width: None,
            gsubrs: &self.gsubrs,
            subrs: &private.subrs,
            done: false,
            depth: 0,
        };
        interp.run(cs);
        let w = match interp.width {
            Some(w) => private.nominal_width_x + w,
            None => private.default_width_x,
        };
        Some(w * self.font_matrix_scale * 1000.0)
    }

    fn sid_to_string(&self, sid: usize) -> Option<String> {
        if sid < STANDARD_STRINGS.len() {
            Some(STANDARD_STRINGS[sid].to_string())
        } else {
            self.strings
                .get(sid - STANDARD_STRINGS.len())
                .map(|s| String::from_utf8_lossy(s).into_owned())
        }
    }
}

/// Reads a CFF INDEX at `pos`; returns its items and the position after it.
fn read_index(data: &[u8], pos: usize) -> Option<(Vec<Vec<u8>>, usize)> {
    let count = u16::from_be_bytes([*data.get(pos)?, *data.get(pos + 1)?]) as usize;
    if count == 0 {
        return Some((Vec::new(), pos + 2));
    }
    let off_size = *data.get(pos + 2)? as usize;
    if !(1..=4).contains(&off_size) {
        return None;
    }
    let offsets_start = pos + 3;
    let read_off = |i: usize| -> Option<usize> {
        let p = offsets_start + i * off_size;
        let bytes = data.get(p..p + off_size)?;
        Some(bytes.iter().fold(0usize, |acc, b| (acc << 8) | *b as usize))
    };
    let data_start = offsets_start + (count + 1) * off_size - 1;
    let mut items = Vec::with_capacity(count);
    for i in 0..count {
        let a = data_start + read_off(i)?;
        let b = data_start + read_off(i + 1)?;
        if a > b {
            return None;
        }
        items.push(data.get(a..b)?.to_vec());
    }
    let end = data_start + read_off(count)?;
    Some((items, end))
}

/// Parses a CFF DICT into operator -> operands. Two-byte operators are keyed
/// as `0x0c00 | second byte`.
fn parse_dict(data: &[u8]) -> HashMap<u16, Vec<f64>> {
    let mut dict = HashMap::new();
    let mut operands: Vec<f64> = Vec::new();
    let mut i = 0;
    while i < data.len() {
        let b0 = data[i];
        match b0 {
            0..=21 => {
                let op = if b0 == 12 {
                    i += 1;
                    0x0c00 | u16::from(*data.get(i).unwrap_or(&0))
                } else {
                    u16::from(b0)
                };
                dict.insert(op, std::mem::take(&mut operands));
                i += 1;
            }
            28 => {
                let v = i16::from_be_bytes([*data.get(i + 1).unwrap_or(&0), *data.get(i + 2).unwrap_or(&0)]);
                operands.push(f64::from(v));
                i += 3;
            }
            29 => {
                let v = i32::from_be_bytes([
                    *data.get(i + 1).unwrap_or(&0),
                    *data.get(i + 2).unwrap_or(&0),
                    *data.get(i + 3).unwrap_or(&0),
                    *data.get(i + 4).unwrap_or(&0),
                ]);
                operands.push(f64::from(v));
                i += 5;
            }
            30 => {
                // Real number: nibble-encoded, terminated by 0xf
                let mut s = String::new();
                i += 1;
                'outer: while i < data.len() {
                    for nib in [data[i] >> 4, data[i] & 0x0f] {
                        match nib {
                            0..=9 => s.push((b'0' + nib) as char),
                            0xa => s.push('.'),
                            0xb => s.push('E'),
                            0xc => s.push_str("E-"),
                            0xe => s.push('-'),
                            0xf => {
                                i += 1;
                                break 'outer;
                            }
                            _ => {}
                        }
                    }
                    i += 1;
                }
                operands.push(s.parse().unwrap_or(0.0));
            }
            32..=246 => {
                operands.push(f64::from(i32::from(b0) - 139));
                i += 1;
            }
            247..=250 => {
                let b1 = i32::from(*data.get(i + 1).unwrap_or(&0));
                operands.push(f64::from((i32::from(b0) - 247) * 256 + b1 + 108));
                i += 2;
            }
            251..=254 => {
                let b1 = i32::from(*data.get(i + 1).unwrap_or(&0));
                operands.push(f64::from(-(i32::from(b0) - 251) * 256 - b1 - 108));
                i += 2;
            }
            _ => i += 1,
        }
    }
    dict
}

fn usize_op(dict: &HashMap<u16, Vec<f64>>, op: u16) -> Option<usize> {
    let v = *dict.get(&op)?.first()?;
    (v >= 0.0).then_some(v as usize)
}

fn read_private(data: &[u8], dict: &HashMap<u16, Vec<f64>>) -> PrivateDict {
    let mut private = PrivateDict::default();
    let Some(entry) = dict.get(&18) else {
        return private;
    };
    if entry.len() < 2 || entry[0] < 0.0 || entry[1] < 0.0 {
        return private;
    }
    let (size, offset) = (entry[0] as usize, entry[1] as usize);
    let Some(bytes) = data.get(offset..offset + size) else {
        return private;
    };
    let pd = parse_dict(bytes);
    private.default_width_x = pd.get(&20).and_then(|v| v.first()).copied().unwrap_or(0.0);
    private.nominal_width_x = pd.get(&21).and_then(|v| v.first()).copied().unwrap_or(0.0);
    if let Some(subrs_off) = usize_op(&pd, 19) {
        if let Some((subrs, _)) = read_index(data, offset + subrs_off) {
            private.subrs = subrs;
        }
    }
    private
}

/// Parses the charset table into a per-GID SID/CID vector (GID 0 = .notdef).
fn parse_charset(data: &[u8], offset: usize, glyph_count: usize) -> Vec<u16> {
    match offset {
        0 => {
            // ISOAdobe: SID i for GID i
            return (0..glyph_count).filter_map(|g| u16::try_from(g).ok()).collect();
        }
        1 | 2 => return Vec::new(), // Expert charsets: not modelled
        _ => {}
    }
    let mut out = vec![0u16];
    let Some(&format) = data.get(offset) else {
        return Vec::new();
    };
    let mut pos = offset + 1;
    match format {
        0 => {
            while out.len() < glyph_count {
                let Some(b) = data.get(pos..pos + 2) else { break };
                out.push(u16::from_be_bytes([b[0], b[1]]));
                pos += 2;
            }
        }
        1 | 2 => {
            while out.len() < glyph_count {
                let Some(b) = data.get(pos..pos + 2) else { break };
                let first = u16::from_be_bytes([b[0], b[1]]);
                let n_left = if format == 1 {
                    let v = u16::from(*data.get(pos + 2).unwrap_or(&0));
                    pos += 3;
                    v
                } else {
                    let Some(b2) = data.get(pos + 2..pos + 4) else { break };
                    pos += 4;
                    u16::from_be_bytes([b2[0], b2[1]])
                };
                for k in 0..=n_left {
                    if out.len() >= glyph_count {
                        break;
                    }
                    out.push(first.wrapping_add(k));
                }
            }
        }
        _ => return Vec::new(),
    }
    out
}

/// Parses the built-in Encoding table (code → GID). Returns `None` for the
/// predefined standard/expert encodings (offset 0 / 1).
fn parse_encoding(data: &[u8], offset: usize, charset: &[u16]) -> Option<HashMap<u8, u16>> {
    if offset == 0 || offset == 1 {
        return None;
    }
    let format = *data.get(offset)?;
    let mut map = HashMap::new();
    let mut pos = offset + 1;
    match format & 0x7f {
        0 => {
            let n_codes = usize::from(*data.get(pos)?);
            pos += 1;
            for gid in 1..=n_codes {
                let code = *data.get(pos)?;
                map.insert(code, u16::try_from(gid).ok()?);
                pos += 1;
            }
        }
        1 => {
            let n_ranges = usize::from(*data.get(pos)?);
            pos += 1;
            let mut gid: u16 = 1;
            for _ in 0..n_ranges {
                let first = *data.get(pos)?;
                let n_left = *data.get(pos + 1)?;
                pos += 2;
                for k in 0..=n_left {
                    if let Some(code) = first.checked_add(k) {
                        map.insert(code, gid);
                    }
                    gid = gid.wrapping_add(1);
                }
            }
        }
        _ => return None,
    }
    if format & 0x80 != 0 {
        // Supplements: code → SID
        let n_sups = usize::from(*data.get(pos)?);
        pos += 1;
        for _ in 0..n_sups {
            let code = *data.get(pos)?;
            let sid = u16::from_be_bytes([*data.get(pos + 1)?, *data.get(pos + 2)?]);
            pos += 3;
            if let Some(gid) = charset.iter().position(|s| *s == sid) {
                map.insert(code, u16::try_from(gid).ok()?);
            }
        }
    }
    Some(map)
}

/// Parses FDSelect into a per-GID font DICT index.
fn parse_fd_select(data: &[u8], offset: usize, glyph_count: usize) -> Vec<u8> {
    let mut out = vec![0u8; glyph_count];
    let Some(&format) = data.get(offset) else {
        return out;
    };
    match format {
        0 => {
            for (gid, slot) in out.iter_mut().enumerate() {
                *slot = *data.get(offset + 1 + gid).unwrap_or(&0);
            }
        }
        3 => {
            let Some(b) = data.get(offset + 1..offset + 3) else { return out };
            let n_ranges = u16::from_be_bytes([b[0], b[1]]) as usize;
            let mut pos = offset + 3;
            let sentinel_pos = offset + 3 + n_ranges * 3;
            let sentinel = data
                .get(sentinel_pos..sentinel_pos + 2)
                .map_or(glyph_count, |b| u16::from_be_bytes([b[0], b[1]]) as usize);
            for i in 0..n_ranges {
                let Some(b) = data.get(pos..pos + 3) else { break };
                let first = u16::from_be_bytes([b[0], b[1]]) as usize;
                let fd = b[2];
                let next = if i + 1 < n_ranges {
                    data.get(pos + 3..pos + 5)
                        .map_or(glyph_count, |b| u16::from_be_bytes([b[0], b[1]]) as usize)
                } else {
                    sentinel
                };
                for slot in out.iter_mut().take(next.min(glyph_count)).skip(first) {
                    *slot = fd;
                }
                pos += 3;
            }
        }
        _ => {}
    }
    out
}

/// Interprets the start of a Type 2 charstring far enough to find the optional
/// leading width argument (TN #5177, section 3.1 "Width").
struct WidthInterp<'a> {
    stack: Vec<f64>,
    width: Option<f64>,
    gsubrs: &'a [Vec<u8>],
    subrs: &'a [Vec<u8>],
    done: bool,
    depth: u8,
}

impl WidthInterp<'_> {
    fn run(&mut self, cs: &[u8]) {
        if self.depth > 10 {
            self.done = true;
            return;
        }
        let mut i = 0;
        while i < cs.len() && !self.done {
            let b0 = cs[i];
            match b0 {
                // Stack-clearing operators that may carry a leading width
                1 | 3 | 18 | 23 => {
                    // hstem vstem hstemhm vstemhm: even number of args, odd => width
                    self.take_width(self.stack.len() % 2 == 1);
                    self.done = true;
                }
                19 | 20 => {
                    // hintmask cntrmask (implicit vstem)
                    self.take_width(self.stack.len() % 2 == 1);
                    self.done = true;
                }
                21 => {
                    self.take_width(self.stack.len() > 2);
                    self.done = true;
                }
                4 | 22 => {
                    self.take_width(self.stack.len() > 1);
                    self.done = true;
                }
                14 => {
                    // endchar: 1 or 5 args => width present
                    self.take_width(self.stack.len() == 1 || self.stack.len() == 5);
                    self.done = true;
                }
                10 => {
                    // callsubr
                    if let Some(idx) = self.stack.pop() {
                        let n = idx as i64 + bias(self.subrs.len());
                        if let Some(sub) = usize::try_from(n).ok().and_then(|n| self.subrs.get(n)) {
                            self.depth += 1;
                            let sub = sub.clone();
                            self.run(&sub);
                            self.depth -= 1;
                        }
                    }
                    i += 1;
                }
                29 => {
                    // callgsubr
                    if let Some(idx) = self.stack.pop() {
                        let n = idx as i64 + bias(self.gsubrs.len());
                        if let Some(sub) = usize::try_from(n).ok().and_then(|n| self.gsubrs.get(n)) {
                            self.depth += 1;
                            let sub = sub.clone();
                            self.run(&sub);
                            self.depth -= 1;
                        }
                    }
                    i += 1;
                }
                11 => return, // return
                28 => {
                    let v = i16::from_be_bytes([*cs.get(i + 1).unwrap_or(&0), *cs.get(i + 2).unwrap_or(&0)]);
                    self.stack.push(f64::from(v));
                    i += 3;
                }
                32..=246 => {
                    self.stack.push(f64::from(i32::from(b0) - 139));
                    i += 1;
                }
                247..=250 => {
                    let b1 = i32::from(*cs.get(i + 1).unwrap_or(&0));
                    self.stack.push(f64::from((i32::from(b0) - 247) * 256 + b1 + 108));
                    i += 2;
                }
                251..=254 => {
                    let b1 = i32::from(*cs.get(i + 1).unwrap_or(&0));
                    self.stack.push(f64::from(-(i32::from(b0) - 251) * 256 - b1 - 108));
                    i += 2;
                }
                255 => {
                    let v = i32::from_be_bytes([
                        *cs.get(i + 1).unwrap_or(&0),
                        *cs.get(i + 2).unwrap_or(&0),
                        *cs.get(i + 3).unwrap_or(&0),
                        *cs.get(i + 4).unwrap_or(&0),
                    ]);
                    self.stack.push(f64::from(v) / 65536.0);
                    i += 5;
                }
                _ => {
                    // Any other operator: no width can follow
                    self.done = true;
                }
            }
        }
    }

    fn take_width(&mut self, present: bool) {
        if present {
            self.width = self.stack.first().copied();
        }
    }
}

fn bias(count: usize) -> i64 {
    if count < 1240 {
        107
    } else if count < 33900 {
        1131
    } else {
        32768
    }
}

/// The 391 CFF standard strings (TN #5176, Appendix A).
#[rustfmt::skip]
pub static STANDARD_STRINGS: [&str; 391] = [
    ".notdef", "space", "exclam", "quotedbl", "numbersign", "dollar", "percent", "ampersand",
    "quoteright", "parenleft", "parenright", "asterisk", "plus", "comma", "hyphen", "period",
    "slash", "zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine",
    "colon", "semicolon", "less", "equal", "greater", "question", "at", "A", "B", "C", "D", "E",
    "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q", "R", "S", "T", "U", "V", "W",
    "X", "Y", "Z", "bracketleft", "backslash", "bracketright", "asciicircum", "underscore",
    "quoteleft", "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "n", "o",
    "p", "q", "r", "s", "t", "u", "v", "w", "x", "y", "z", "braceleft", "bar", "braceright",
    "asciitilde", "exclamdown", "cent", "sterling", "fraction", "yen", "florin", "section",
    "currency", "quotesingle", "quotedblleft", "guillemotleft", "guilsinglleft",
    "guilsinglright", "fi", "fl", "endash", "dagger", "daggerdbl", "periodcentered",
    "paragraph", "bullet", "quotesinglbase", "quotedblbase", "quotedblright",
    "guillemotright", "ellipsis", "perthousand", "questiondown", "grave", "acute",
    "circumflex", "tilde", "macron", "breve", "dotaccent", "dieresis", "ring", "cedilla",
    "hungarumlaut", "ogonek", "caron", "emdash", "AE", "ordfeminine", "Lslash", "Oslash", "OE",
    "ordmasculine", "ae", "dotlessi", "lslash", "oslash", "oe", "germandbls", "onesuperior",
    "logicalnot", "mu", "trademark", "Eth", "onehalf", "plusminus", "Thorn", "onequarter",
    "divide", "brokenbar", "degree", "thorn", "threequarters", "twosuperior", "registered",
    "minus", "eth", "multiply", "threesuperior", "copyright", "Aacute", "Acircumflex",
    "Adieresis", "Agrave", "Aring", "Atilde", "Ccedilla", "Eacute", "Ecircumflex", "Edieresis",
    "Egrave", "Iacute", "Icircumflex", "Idieresis", "Igrave", "Ntilde", "Oacute", "Ocircumflex",
    "Odieresis", "Ograve", "Otilde", "Scaron", "Uacute", "Ucircumflex", "Udieresis", "Ugrave",
    "Yacute", "Ydieresis", "Zcaron", "aacute", "acircumflex", "adieresis", "agrave", "aring",
    "atilde", "ccedilla", "eacute", "ecircumflex", "edieresis", "egrave", "iacute",
    "icircumflex", "idieresis", "igrave", "ntilde", "oacute", "ocircumflex", "odieresis",
    "ograve", "otilde", "scaron", "uacute", "ucircumflex", "udieresis", "ugrave", "yacute",
    "ydieresis", "zcaron", "exclamsmall", "Hungarumlautsmall", "dollaroldstyle",
    "dollarsuperior", "ampersandsmall", "Acutesmall", "parenleftsuperior",
    "parenrightsuperior", "twodotenleader", "onedotenleader", "zerooldstyle", "oneoldstyle",
    "twooldstyle", "threeoldstyle", "fouroldstyle", "fiveoldstyle", "sixoldstyle",
    "sevenoldstyle", "eightoldstyle", "nineoldstyle", "commasuperior",
    "threequartersemdash", "periodsuperior", "questionsmall", "asuperior", "bsuperior",
    "centsuperior", "dsuperior", "esuperior", "isuperior", "lsuperior", "msuperior",
    "nsuperior", "osuperior", "rsuperior", "ssuperior", "tsuperior", "ff", "ffi", "ffl",
    "parenleftinferior", "parenrightinferior", "Circumflexsmall", "hyphensuperior",
    "Gravesmall", "Asmall", "Bsmall", "Csmall", "Dsmall", "Esmall", "Fsmall", "Gsmall", "Hsmall",
    "Ismall", "Jsmall", "Ksmall", "Lsmall", "Msmall", "Nsmall", "Osmall", "Psmall", "Qsmall",
    "Rsmall", "Ssmall", "Tsmall", "Usmall", "Vsmall", "Wsmall", "Xsmall", "Ysmall", "Zsmall",
    "colonmonetary", "onefitted", "rupiah", "Tildesmall", "exclamdownsmall", "centoldstyle",
    "Lslashsmall", "Scaronsmall", "Zcaronsmall", "Dieresissmall", "Brevesmall", "Caronsmall",
    "Dotaccentsmall", "Macronsmall", "figuredash", "hypheninferior", "Ogoneksmall",
    "Ringsmall", "Cedillasmall", "questiondownsmall", "oneeighth", "threeeighths",
    "fiveeighths", "seveneighths", "onethird", "twothirds", "zerosuperior", "foursuperior",
    "fivesuperior", "sixsuperior", "sevensuperior", "eightsuperior", "ninesuperior",
    "zeroinferior", "oneinferior", "twoinferior", "threeinferior", "fourinferior",
    "fiveinferior", "sixinferior", "seveninferior", "eightinferior", "nineinferior",
    "centinferior", "dollarinferior", "periodinferior", "commainferior", "Agravesmall",
    "Aacutesmall", "Acircumflexsmall", "Atildesmall", "Adieresissmall", "Aringsmall",
    "AEsmall", "Ccedillasmall", "Egravesmall", "Eacutesmall", "Ecircumflexsmall",
    "Edieresissmall", "Igravesmall", "Iacutesmall", "Icircumflexsmall", "Idieresissmall",
    "Ethsmall", "Ntildesmall", "Ogravesmall", "Oacutesmall", "Ocircumflexsmall",
    "Otildesmall", "Odieresissmall", "OEsmall", "Oslashsmall", "Ugravesmall", "Uacutesmall",
    "Ucircumflexsmall", "Udieresissmall", "Yacutesmall", "Thornsmall", "Ydieresissmall",
    "001.000", "001.001", "001.002", "001.003", "Black", "Bold", "Book", "Light", "Medium",
    "Regular", "Roman", "Semibold",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_strings_count() {
        assert_eq!(STANDARD_STRINGS.len(), 391);
        assert_eq!(STANDARD_STRINGS[0], ".notdef");
        assert_eq!(STANDARD_STRINGS[1], "space");
        assert_eq!(STANDARD_STRINGS[34], "A");
        assert_eq!(STANDARD_STRINGS[66], "a");
        assert_eq!(STANDARD_STRINGS[390], "Semibold");
    }

    #[test]
    fn dict_number_encodings() {
        // 139 -> 0 ; 247 0 -> 108 ; 28 0x01 0x00 -> 256 ; op 17
        let d = parse_dict(&[139, 247, 0, 28, 1, 0, 17]);
        assert_eq!(d.get(&17).unwrap(), &vec![0.0, 108.0, 256.0]);
        // real 30 [0x12 0xa5 0xff] -> 12.5
        let d = parse_dict(&[30, 0x12, 0xa5, 0xff, 20]);
        assert!((d.get(&20).unwrap()[0] - 12.5).abs() < 1e-9);
    }
}
