//! Checkpoint 31 (Fonts) — font *program* level checks.
//!
//! These checks look inside the embedded font programs and at how each font
//! is actually used in the content streams (see [`crate::content`]):
//!
//! | Rule   | Condition |
//! |--------|-----------|
//! | 10-001 | a used character code has no Unicode mapping |
//! | 17-003 | Unicode mapping failures in a document with `<Formula>` content |
//! | 31-011 | a rendered glyph is missing from the embedded program |
//! | 31-012 / 31-013 | Type 1 `/CharSet` vs. glyphs in the program |
//! | 31-014 / 31-015 | CID `/CIDSet` vs. glyphs in the program |
//! | 31-016 | dictionary widths vs. program widths (> 1/1000 unit) |
//! | 31-017 / 31-018 | non-symbolic TrueType cmap presence and glyph lookup |
//! | 31-023 | Differences without a (3,1) cmap |
//! | 31-024 / 31-025 / 31-026 | symbolic TrueType Encoding and cmap rules |
//! | 31-027 | missing ToUnicode without a permitted alternative |
//! | 31-028 / 31-029 | ToUnicode maps to U+0000, U+FEFF or U+FFFE |
//! | 31-030 | a text-showing operator references the .notdef glyph |

// Character codes, CIDs and glyph ids are bounded by the PDF/font formats (u16/u32 ranges).
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap
)]

use crate::checks::Check;
use crate::content::FontUsage;
use crate::content::cmap::{EncodingCMap, ToUnicodeCMap};
use crate::document::HornDocument;
use crate::fontprog::{FontFileKind, FontProgram, agl, encodings};
use crate::model::{CheckOutcome, CheckResult, Location, Severity};
use anyhow::Result;
use lopdf::{Dictionary, Document, Object, ObjectId};
use std::collections::{BTreeSet, HashMap};

pub struct FontProgramChecks;

impl Check for FontProgramChecks {
    fn id(&self) -> &'static str {
        "31-font-program"
    }

    fn checkpoint(&self) -> u8 {
        31
    }

    fn rules(&self) -> &'static [&'static str] {
        &[
            "10-001", "17-003", "31-011", "31-012", "31-013", "31-014", "31-015", "31-016",
            "31-017", "31-018", "31-023", "31-024", "31-025", "31-026", "31-027", "31-028",
            "31-029", "31-030",
        ]
    }

    fn description(&self) -> &'static str {
        "Fonts: glyph coverage, CharSet/CIDSet, widths, TrueType cmaps, Unicode mapping"
    }

    fn run(&self, doc: &mut HornDocument) -> Result<Vec<CheckResult>> {
        let mut results = Vec::new();
        let usage_map = doc.font_usage();
        let lopdf = doc.lopdf();

        let mut font_ids: Vec<ObjectId> = lopdf
            .objects
            .iter()
            .filter(|(_, obj)| {
                obj.as_dict().is_ok_and(|d| {
                    d.get(b"Type").ok().and_then(|o| o.as_name().ok()) == Some(b"Font")
                        && !matches!(
                            d.get(b"Subtype").ok().and_then(|o| o.as_name().ok()),
                            Some(b"CIDFontType0" | b"CIDFontType2")
                        )
                })
            })
            .map(|(id, _)| *id)
            .collect();
        font_ids.sort_unstable();

        for id in font_ids {
            let Ok(font) = lopdf.get_dictionary(id) else {
                continue;
            };
            let usage = usage_map.get(&id);
            let ctx = FontContext::new(lopdf, id, font, usage);
            ctx.run(&mut results);
        }

        // 17-003: Unicode mapping requirements for mathematical expressions
        // (ISO 14289-1 7.7-2) are the general 10-001 / 31-027 requirements applied
        // to <Formula> content. When the document contains Formula elements and
        // any font fails to map to Unicode, report the formula condition too.
        let has_formula = lopdf.objects.values().any(|o| {
            o.as_dict()
                .is_ok_and(|d| d.get(b"S").ok().and_then(|o| o.as_name().ok()) == Some(b"Formula"))
        });
        if has_formula {
            let unicode_failures: Vec<String> = results
                .iter()
                .filter(|r| r.is_failure() && matches!(r.rule_id.as_str(), "10-001" | "31-027"))
                .map(|r| r.description.clone())
                .collect();
            if unicode_failures.is_empty() {
                results.push(CheckResult {
                    rule_id: "17-003".to_string(),
                    checkpoint: 17,
                    description:
                        "Fonts used in the document map to Unicode (Formula content included)"
                            .to_string(),
                    severity: Severity::Info,
                    outcome: CheckOutcome::Pass,
                });
            } else {
                results.push(CheckResult {
                    rule_id: "17-003".to_string(),
                    checkpoint: 17,
                    description: format!(
                        "Document contains <Formula> elements and {} font(s) do not meet the Unicode mapping requirements",
                        unicode_failures.len()
                    ),
                    severity: Severity::Error,
                    outcome: CheckOutcome::Fail {
                        message: format!(
                            "<Formula> content cannot be reliably mapped to Unicode: {}",
                            unicode_failures.join("; ")
                        ),
                        location: None,
                    },
                });
            }
        }

        dedup(&mut results);
        Ok(results)
    }
}

/// Everything the checks need to know about one font dictionary.
struct FontContext<'a> {
    doc: &'a Document,
    font: &'a Dictionary,
    label: String,
    subtype: Vec<u8>,
    /// The `CIDFont` dictionary for Type 0 fonts.
    descendant: Option<&'a Dictionary>,
    descriptor: Option<&'a Dictionary>,
    flags: i64,
    usage: Option<&'a FontUsage>,
    /// Decompressed font program bytes and their kind.
    program_data: Option<(FontFileKind, Vec<u8>)>,
}

impl<'a> FontContext<'a> {
    fn new(
        doc: &'a Document,
        id: ObjectId,
        font: &'a Dictionary,
        usage: Option<&'a FontUsage>,
    ) -> Self {
        let subtype = font
            .get(b"Subtype")
            .ok()
            .and_then(|o| o.as_name().ok())
            .unwrap_or(b"")
            .to_vec();
        let base_font = font
            .get_deref(b"BaseFont", doc)
            .ok()
            .and_then(|o| o.as_name().ok())
            .map_or_else(
                || format!("obj {}.{}", id.0, id.1),
                |n| String::from_utf8_lossy(n).into_owned(),
            );
        let descendant = if subtype == b"Type0" {
            font.get_deref(b"DescendantFonts", doc)
                .ok()
                .and_then(|o| o.as_array().ok())
                .and_then(|a| a.first())
                .and_then(|o| resolve_dict(doc, o))
        } else {
            None
        };
        let descriptor = descendant
            .unwrap_or(font)
            .get_deref(b"FontDescriptor", doc)
            .ok()
            .and_then(|o| o.as_dict().ok());
        let flags = descriptor
            .and_then(|d| d.get_deref(b"Flags", doc).ok())
            .and_then(|o| o.as_i64().ok())
            .unwrap_or(0);
        let program_data = descriptor.and_then(|d| {
            for (key, kind) in [
                (b"FontFile2" as &[u8], FontFileKind::TrueType),
                (b"FontFile3", FontFileKind::FontFile3),
                (b"FontFile", FontFileKind::Type1),
            ] {
                if let Some(data) = d
                    .get(key)
                    .ok()
                    .and_then(|o| resolve(doc, o))
                    .and_then(|o| o.as_stream().ok())
                    .and_then(|s| s.decompressed_content().ok())
                {
                    return Some((kind, data));
                }
            }
            None
        });
        Self {
            doc,
            font,
            label: base_font,
            subtype,
            descendant,
            descriptor,
            flags,
            usage,
            program_data,
        }
    }

    fn is_type3(&self) -> bool {
        self.subtype == b"Type3"
    }

    fn is_type0(&self) -> bool {
        self.subtype == b"Type0"
    }

    fn is_truetype(&self) -> bool {
        self.subtype == b"TrueType"
    }

    /// Symbolic flag set (bit 3) and Nonsymbolic (bit 6) clear.
    fn is_symbolic(&self) -> bool {
        self.flags & 0x04 != 0 && self.flags & 0x20 == 0
    }

    fn rendered(&self) -> bool {
        self.usage.is_some_and(|u| u.rendered)
    }

    fn rendered_codes(&self) -> Vec<(u32, usize)> {
        match self.usage {
            Some(u) if !u.codes_unreliable => u.rendered_codes.iter().copied().collect(),
            _ => Vec::new(),
        }
    }

    fn used_codes(&self) -> Vec<(u32, usize)> {
        match self.usage {
            Some(u) if !u.codes_unreliable => u.codes.iter().copied().collect(),
            _ => Vec::new(),
        }
    }

    fn location(&self) -> Location {
        Location {
            page: None,
            element: Some(format!("Font /{}", self.label)),
        }
    }

    fn fail(&self, rule: &str, message: &str) -> CheckResult {
        CheckResult {
            rule_id: rule.to_string(),
            checkpoint: if rule.starts_with("10-") { 10 } else { 31 },
            description: format!("Font /{}: {message}", self.label),
            severity: Severity::Error,
            outcome: CheckOutcome::Fail {
                message: format!("Font /{}: {message}", self.label),
                location: Some(self.location()),
            },
        }
    }

    fn pass(&self, rule: &str, message: &str) -> CheckResult {
        CheckResult {
            rule_id: rule.to_string(),
            checkpoint: if rule.starts_with("10-") { 10 } else { 31 },
            description: format!("Font /{}: {message}", self.label),
            severity: Severity::Info,
            outcome: CheckOutcome::Pass,
        }
    }

    fn run(&self, results: &mut Vec<CheckResult>) {
        // ToUnicode / Unicode mapping applies to every font type, including Type 3,
        // but only to fonts that actually show text (ISO 14289-1 7.21.7 speaks of
        // "used character codes"); fonts that are merely listed in resources
        // (e.g. AcroForm /DR defaults) are exempt.
        if self.usage.is_some_and(|u| u.show_ops > 0) {
            self.check_unicode_mapping(results);
        }

        if self.is_type3() {
            return;
        }

        let program = self
            .program_data
            .as_ref()
            .and_then(|(kind, data)| FontProgram::parse(*kind, data));
        let Some(program) = program else {
            return;
        };

        self.check_charset(&program, results);
        self.check_cidset(&program, results);
        if self.is_truetype() && matches!(program, FontProgram::TrueType(_)) {
            self.check_truetype_cmaps(&program, results);
        }
        self.check_glyph_coverage_and_widths(&program, results);
    }

    // ------------------------------------------------------------------
    // 31-012 / 31-013: Type 1 CharSet
    // ------------------------------------------------------------------
    fn check_charset(&self, program: &FontProgram<'_>, results: &mut Vec<CheckResult>) {
        if self.is_type0() || self.is_truetype() {
            return;
        }
        let Some(charset) = self
            .descriptor
            .and_then(|d| d.get_deref(b"CharSet", self.doc).ok())
            .and_then(|o| o.as_str().ok())
        else {
            return;
        };
        let Some(program_names) = program.glyph_names() else {
            return;
        };
        let listed: BTreeSet<&[u8]> = charset
            .split(|b| *b == b'/')
            .filter(|n| !n.is_empty())
            .collect();
        let present: BTreeSet<&[u8]> = program_names
            .iter()
            .map(std::string::String::as_bytes)
            .filter(|n| *n != b".notdef")
            .collect();

        let not_listed: Vec<String> = present
            .iter()
            .filter(|n| !listed.contains(*n))
            .map(|n| String::from_utf8_lossy(n).into_owned())
            .collect();
        let not_present: Vec<String> = listed
            .iter()
            .filter(|n| **n != b".notdef" && !present.contains(*n))
            .map(|n| String::from_utf8_lossy(n).into_owned())
            .collect();

        if !not_listed.is_empty() {
            results.push(self.fail(
                "31-012",
                &format!(
                    "{} glyph(s) present in the embedded Type 1 program are not listed in /CharSet: /{}",
                    not_listed.len(),
                    not_listed.iter().take(5).cloned().collect::<Vec<_>>().join(", /")
                ),
            ));
        }
        if !not_present.is_empty() {
            results.push(self.fail(
                "31-013",
                &format!(
                    "{} glyph name(s) listed in /CharSet are not present in the embedded Type 1 program: /{}",
                    not_present.len(),
                    not_present.iter().take(5).cloned().collect::<Vec<_>>().join(", /")
                ),
            ));
        }
        if not_listed.is_empty() && not_present.is_empty() {
            results.push(self.pass("31-012", "/CharSet matches the glyphs in the font program"));
        }
    }

    // ------------------------------------------------------------------
    // 31-014 / 31-015: CID font CIDSet
    // ------------------------------------------------------------------
    fn check_cidset(&self, program: &FontProgram<'_>, results: &mut Vec<CheckResult>) {
        let Some(cid_font) = self.descendant else {
            return;
        };
        let Some(cidset) = self
            .descriptor
            .and_then(|d| d.get(b"CIDSet").ok())
            .and_then(|o| resolve(self.doc, o))
            .and_then(|o| o.as_stream().ok())
            .and_then(|s| s.decompressed_content().ok())
        else {
            return;
        };
        let listed: BTreeSet<u32> = cidset
            .iter()
            .enumerate()
            .flat_map(|(i, byte)| {
                (0..8)
                    .filter(move |bit| byte & (0x80 >> bit) != 0)
                    .map(move |bit| (i * 8 + bit) as u32)
            })
            .collect();

        let cid_to_gid = CidToGid::load(self.doc, cid_font);
        let is_cff_cid = program.is_cid_keyed_cff();

        // CIDs present in the program: every glyph with outline data.
        let mut present: BTreeSet<u32> = BTreeSet::new();
        let mut exists: BTreeSet<u32> = BTreeSet::new();
        if is_cff_cid {
            if let FontProgram::Cff(cff) = program {
                for cid in cff.cids() {
                    present.insert(u32::from(cid));
                    exists.insert(u32::from(cid));
                }
            }
        } else {
            let count = program.glyph_count();
            match &cid_to_gid {
                CidToGid::Identity => {
                    for gid in 0..count {
                        exists.insert(gid as u32);
                        if program.gid_is_present(gid) {
                            present.insert(gid as u32);
                        }
                    }
                }
                CidToGid::Map(map) => {
                    for (cid, gid) in map.iter().enumerate() {
                        let gid = usize::from(*gid);
                        if gid < count {
                            exists.insert(cid as u32);
                            if program.gid_is_present(gid) {
                                present.insert(cid as u32);
                            }
                        }
                    }
                }
            }
        }

        let not_listed: Vec<u32> = present
            .iter()
            .filter(|c| **c != 0 && !listed.contains(c))
            .copied()
            .collect();
        let not_present: Vec<u32> = listed
            .iter()
            .filter(|c| **c != 0 && !exists.contains(c))
            .copied()
            .collect();

        if !not_listed.is_empty() {
            results.push(self.fail(
                "31-014",
                &format!(
                    "{} glyph(s) present in the embedded CID font program are not listed in /CIDSet (CIDs {})",
                    not_listed.len(),
                    join_ids(&not_listed)
                ),
            ));
        }
        if !not_present.is_empty() {
            results.push(self.fail(
                "31-015",
                &format!(
                    "{} CID(s) listed in /CIDSet are not present in the embedded font program (CIDs {})",
                    not_present.len(),
                    join_ids(&not_present)
                ),
            ));
        }
        if not_listed.is_empty() && not_present.is_empty() {
            results.push(self.pass("31-014", "/CIDSet matches the glyphs in the font program"));
        }
    }

    // ------------------------------------------------------------------
    // 31-017 … 31-026: TrueType cmap rules
    // ------------------------------------------------------------------
    fn check_truetype_cmaps(&self, program: &FontProgram<'_>, results: &mut Vec<CheckResult>) {
        let subtables = program.cmap_subtables();
        let has_30 = subtables
            .iter()
            .any(|s| s.platform_id == 3 && s.encoding_id == 0);
        let has_31 = subtables
            .iter()
            .any(|s| s.platform_id == 3 && s.encoding_id == 1);
        let encoding = self.font.get(b"Encoding").ok();

        if self.is_symbolic() {
            // 31-024: symbolic TrueType fonts must not have an /Encoding entry
            if encoding.is_some() {
                results.push(self.fail(
                    "31-024",
                    "symbolic TrueType font has an /Encoding entry in the font dictionary",
                ));
            }
            // 31-025 / 31-026
            if subtables.is_empty() {
                results.push(self.fail(
                    "31-025",
                    "embedded symbolic TrueType program has no cmap subtable",
                ));
            } else if subtables.len() > 1 && !has_30 {
                results.push(self.fail(
                "31-026",
                &format!(
                        "embedded symbolic TrueType program has {} cmap subtables but none is a (3,0) Microsoft Symbol cmap",
                        subtables.len()
                    ),
                ));
            }
            return;
        }

        // Non-symbolic
        if !self.rendered() {
            return;
        }
        // 31-017: at least one non-symbolic cmap ((3,0) does not count)
        let non_symbolic_cmaps = subtables
            .iter()
            .filter(|s| !(s.platform_id == 3 && s.encoding_id == 0))
            .count();
        if non_symbolic_cmaps == 0 {
            results.push(self.fail(
                "31-017",
                "non-symbolic TrueType font used for rendering has no non-symbolic cmap subtable in the embedded program",
                ));
        }
        // 31-023: Differences requires a (3,1) cmap
        let has_differences = encoding
            .and_then(|o| resolve(self.doc, o))
            .and_then(|o| o.as_dict().ok())
            .is_some_and(|d| d.get(b"Differences").is_ok());
        if has_differences && !has_31 {
            results.push(self.fail(
                "31-023",
                "non-symbolic TrueType font has a /Differences array but the embedded program has no (3,1) Microsoft Unicode cmap",
                ));
        }
    }

    // ------------------------------------------------------------------
    // 31-011, 31-016, 31-018, 31-030: per-glyph checks over used codes
    // ------------------------------------------------------------------
    #[allow(clippy::too_many_lines)]
    fn check_glyph_coverage_and_widths(
        &self,
        program: &FontProgram<'_>,
        results: &mut Vec<CheckResult>,
    ) {
        let rendered = self.rendered_codes();
        if rendered.is_empty() {
            return;
        }

        let simple_encoding = if self.is_type0() {
            None
        } else {
            Some(SimpleEncoding::load(
                self.doc,
                self.font,
                self.is_symbolic(),
                self.is_truetype(),
            ))
        };
        let cmap = if self.is_type0() {
            self.load_encoding_cmap()
        } else {
            None
        };
        let cid_to_gid = self.descendant.map(|d| CidToGid::load(self.doc, d));
        let widths = if self.is_type0() {
            self.descendant.map(|d| Widths::cid(self.doc, d))
        } else {
            Some(Widths::simple(self.doc, self.font, self.descriptor))
        };

        let mut missing_glyphs: Vec<String> = Vec::new();
        let mut notdef_codes: Vec<String> = Vec::new();
        let mut cmap_lookup_failures: Vec<String> = Vec::new();
        let mut width_mismatches: Vec<String> = Vec::new();

        for (code, num_bytes) in &rendered {
            let glyph = if self.is_type0() {
                let Some(cmap) = &cmap else { continue };
                let Some(cid) = cmap.to_cid(*code, *num_bytes) else {
                    missing_glyphs.push(format!("code {code:#06x} (no CID mapping)"));
                    continue;
                };
                if cid == 0 {
                    notdef_codes.push(format!("code {code:#06x}"));
                    continue;
                }
                let gid = match &cid_to_gid {
                    Some(CidToGid::Map(map)) => map.get(cid as usize).map(|g| usize::from(*g)),
                    _ => program.gid_for_cid(cid),
                };
                match gid {
                    Some(0) => {
                        notdef_codes.push(format!("code {code:#06x} (CID {cid})"));
                        continue;
                    }
                    Some(gid) if program.gid_exists(gid) => GlyphRef::Gid(gid),
                    _ => {
                        missing_glyphs.push(format!("CID {cid}"));
                        continue;
                    }
                }
            } else {
                let enc = simple_encoding.as_ref().expect("simple font encoding");
                let code8 = u8::try_from(*code).unwrap_or(0);
                match enc.resolve(program, code8) {
                    Resolved::Glyph(g) => g,
                    Resolved::NotDef => {
                        notdef_codes.push(format!("code {code}"));
                        continue;
                    }
                    Resolved::CmapMiss => {
                        cmap_lookup_failures.push(format!("code {code}"));
                        continue;
                    }
                    Resolved::Unknown => continue,
                    Resolved::Missing(name) => {
                        missing_glyphs.push(format!("code {code} (/{name})"));
                        continue;
                    }
                }
            };

            // 31-016: widths
            if let Some(widths) = &widths {
                let dict_width = if self.is_type0() {
                    cmap.as_ref()
                        .and_then(|c| c.to_cid(*code, *num_bytes))
                        .and_then(|cid| widths.get(cid))
                } else {
                    widths.get(*code)
                };
                let program_width = match &glyph {
                    GlyphRef::Gid(gid) => program.advance_by_gid(*gid),
                    GlyphRef::Name(name) => program.advance_by_name(name),
                };
                if let (Some(dw), Some(pw)) = (dict_width, program_width) {
                    if (dw - pw).abs() > 1.0 + 1e-6 {
                        width_mismatches.push(format!(
                            "code {code}: dictionary {dw:.1}, font program {pw:.1}"
                        ));
                    }
                }
            }
        }

        if !missing_glyphs.is_empty() {
            results.push(self.fail(
                "31-011",
                &format!(
                    "{} rendered glyph(s) are not present in the embedded font program: {}",
                    missing_glyphs.len(),
                    missing_glyphs.iter().take(5).cloned().collect::<Vec<_>>().join(", ")
                ),
            ));
        }
        if !cmap_lookup_failures.is_empty() {
            results.push(self.fail(
                "31-018",
                &format!(
                    "{} rendered character code(s) cannot be looked up through the non-symbolic cmap subtables of the embedded TrueType program: {}",
                    cmap_lookup_failures.len(),
                    cmap_lookup_failures.iter().take(5).cloned().collect::<Vec<_>>().join(", ")
                ),
            ));
        }
        if !notdef_codes.is_empty() {
            results.push(self.fail(
                "31-030",
                &format!(
                    "{} text-showing character code(s) reference the .notdef glyph: {}",
                    notdef_codes.len(),
                    notdef_codes.iter().take(5).cloned().collect::<Vec<_>>().join(", ")
                ),
            ));
        }
        if !width_mismatches.is_empty() {
            results.push(self.fail(
                "31-016",
                &format!(
                    "{} glyph width(s) in the font dictionary differ from the embedded font program by more than 1/1000: {}",
                    width_mismatches.len(),
                    width_mismatches.iter().take(3).cloned().collect::<Vec<_>>().join("; ")
                ),
            ));
        } else if widths.is_some() {
            results.push(self.pass("31-016", "glyph widths match the embedded font program"));
        }
        if missing_glyphs.is_empty() && cmap_lookup_failures.is_empty() {
            results.push(self.pass(
                "31-011",
                "all rendered glyphs are present in the embedded font program",
            ));
        }
    }

    fn load_encoding_cmap(&self) -> Option<EncodingCMap> {
        let enc = self.font.get(b"Encoding").ok()?;
        if let Ok(name) = enc.as_name() {
            return match name {
                b"Identity-H" | b"Identity-V" => Some(EncodingCMap::identity()),
                _ => None,
            };
        }
        let stream = resolve(self.doc, enc)?.as_stream().ok()?;
        let data = stream.decompressed_content().ok()?;
        let cmap = EncodingCMap::parse(&data);
        cmap.can_split().then_some(cmap)
    }

    // ------------------------------------------------------------------
    // 10-001, 31-027, 31-028, 31-029: Unicode mapping
    // ------------------------------------------------------------------
    fn check_unicode_mapping(&self, results: &mut Vec<CheckResult>) {
        let to_unicode = self
            .font
            .get(b"ToUnicode")
            .ok()
            .and_then(|o| resolve(self.doc, o))
            .and_then(|o| o.as_stream().ok())
            .and_then(|s| s.decompressed_content().ok())
            .map(|data| ToUnicodeCMap::parse(&data));

        if let Some(cmap) = &to_unicode {
            // 31-028 / 31-029
            let mut zero = 0usize;
            let mut nonchar = 0usize;
            for dst in cmap.all_destinations() {
                if dst.is_empty() || dst.iter().all(|u| *u == 0) {
                    zero += 1;
                } else if dst.iter().any(|u| matches!(*u, 0xFEFF | 0xFFFE)) {
                    nonchar += 1;
                }
            }
            if zero > 0 {
                results.push(self.fail(
                "31-028",
                &format!("ToUnicode CMap maps {zero} character code(s) to U+0000 — glyphs have no Unicode representation"),
                ));
            }
            if nonchar > 0 {
                results.push(self.fail(
                    "31-029",
                    &format!("ToUnicode CMap maps {nonchar} character code(s) to U+FEFF or U+FFFE"),
                ));
            }
            if zero == 0 && nonchar == 0 {
                results.push(self.pass(
                    "31-028",
                    "ToUnicode CMap contains no forbidden Unicode values",
                ));
            }

            // 10-001: every used code must be mapped
            let unmapped: Vec<String> = self
                .used_codes()
                .into_iter()
                .filter(|(code, n)| cmap.lookup(*code, *n).is_none())
                .map(|(code, _)| format!("{code:#x}"))
                .collect();
            if !unmapped.is_empty() && !self.has_implicit_unicode_mapping() {
                results.push(self.fail(
                    "10-001",
                    &format!(
                        "{} used character code(s) have no ToUnicode mapping: {}",
                        unmapped.len(),
                        unmapped.iter().take(8).cloned().collect::<Vec<_>>().join(", ")
                    ),
                ));
            }
            return;
        }

        // 31-027: no ToUnicode — one of the permitted alternatives must apply
        if self.has_implicit_unicode_mapping() {
            results.push(self.pass(
                "31-027",
                "Unicode mapping is provided without a ToUnicode CMap (permitted encoding)",
            ));
        } else {
            let reason = if self.is_type0() {
                "composite font has no ToUnicode CMap and its CIDFont does not use an Adobe-GB1/CNS1/Japan1/Korea1 character collection"
            } else if self.is_truetype() {
                "symbolic TrueType font has no ToUnicode CMap"
            } else {
                "font has no ToUnicode CMap and its encoding is not MacRomanEncoding, MacExpertEncoding or WinAnsiEncoding, and not all used glyph names are Adobe Glyph List names"
            };
            results.push(self.fail("31-027", reason));
        }
    }

    /// ISO 14289-1 7.21.7 alternatives to a `ToUnicode` `CMap`.
    fn has_implicit_unicode_mapping(&self) -> bool {
        if self.is_type0() {
            return self
                .descendant
                .and_then(|d| d.get_deref(b"CIDSystemInfo", self.doc).ok())
                .and_then(|o| o.as_dict().ok())
                .is_some_and(|csi| {
                    let reg = csi
                        .get_deref(b"Registry", self.doc)
                        .ok()
                        .and_then(|o| o.as_str().ok())
                        .unwrap_or(b"");
                    let ord = csi
                        .get_deref(b"Ordering", self.doc)
                        .ok()
                        .and_then(|o| o.as_str().ok())
                        .unwrap_or(b"");
                    reg == b"Adobe" && matches!(ord, b"Japan1" | b"GB1" | b"CNS1" | b"Korea1")
                });
        }
        // Predefined encodings
        let encoding = self.font.get_deref(b"Encoding", self.doc).ok();
        let base_name = encoding.and_then(|e| {
            e.as_name().ok().map(<[u8]>::to_vec).or_else(|| {
                e.as_dict()
                    .ok()
                    .and_then(|d| d.get_deref(b"BaseEncoding", self.doc).ok())
                    .and_then(|o| o.as_name().ok())
                    .map(<[u8]>::to_vec)
            })
        });
        let has_differences = encoding
            .and_then(|e| e.as_dict().ok())
            .is_some_and(|d| d.get(b"Differences").is_ok());
        if !has_differences
            && matches!(
                base_name.as_deref(),
                Some(b"MacRomanEncoding" | b"MacExpertEncoding" | b"WinAnsiEncoding")
            )
        {
            return true;
        }
        // Non-symbolic TrueType fonts map through the standard encodings
        if self.is_truetype() && !self.is_symbolic() {
            return true;
        }
        // Type 1 / Type 3 fonts whose used glyph names are all AGL (or Symbol) names
        if self.subtype == b"Type1" || self.subtype == b"MMType1" || self.is_type3() {
            let program = self
                .program_data
                .as_ref()
                .and_then(|(kind, data)| FontProgram::parse(*kind, data));
            let enc = SimpleEncoding::load(self.doc, self.font, self.is_symbolic(), false);
            let codes = self.used_codes();
            if codes.is_empty() {
                return false;
            }
            return codes.iter().all(|(code, _)| {
                let byte = u8::try_from(*code).unwrap_or(0);
                enc.glyph_name(program.as_ref(), byte).is_some_and(|n| {
                    agl::glyph_name_to_unicode(&n).is_some() || is_symbol_font_name(&n)
                })
            });
        }
        false
    }
}

/// Names of the Symbol font glyph set (ISO 32000-1 Annex D.5) are accepted as
/// Unicode-mappable by 7.21.7.
fn is_symbol_font_name(name: &[u8]) -> bool {
    encodings::SYMBOL
        .iter()
        .flatten()
        .any(|n| n.as_bytes() == name)
}

enum GlyphRef {
    Gid(usize),
    Name(Vec<u8>),
}

enum Resolved {
    Glyph(GlyphRef),
    NotDef,
    /// The encoding does not determine a glyph for this code (no built-in
    /// encoding could be read); nothing can be concluded.
    Unknown,
    /// Non-symbolic TrueType: no cmap subtable maps the code (31-018).
    CmapMiss,
    /// Named glyph absent from the program.
    Missing(String),
}

/// Encoding of a simple font: how a byte code maps to a glyph.
struct SimpleEncoding {
    base: Option<&'static encodings::GlyphNameTable>,
    differences: HashMap<u8, Vec<u8>>,
    has_encoding_entry: bool,
    symbolic: bool,
    truetype: bool,
}

impl SimpleEncoding {
    fn load(doc: &Document, font: &Dictionary, symbolic: bool, truetype: bool) -> Self {
        let mut enc = Self {
            base: None,
            differences: HashMap::new(),
            has_encoding_entry: false,
            symbolic,
            truetype,
        };
        let Ok(encoding) = font.get_deref(b"Encoding", doc) else {
            return enc;
        };
        enc.has_encoding_entry = true;
        if let Ok(name) = encoding.as_name() {
            enc.base = encodings::predefined(name);
            return enc;
        }
        let Ok(dict) = encoding.as_dict() else {
            return enc;
        };
        if let Some(name) = dict
            .get_deref(b"BaseEncoding", doc)
            .ok()
            .and_then(|o| o.as_name().ok())
        {
            enc.base = encodings::predefined(name);
        }
        if let Some(diffs) = dict
            .get_deref(b"Differences", doc)
            .ok()
            .and_then(|o| o.as_array().ok())
        {
            let mut code: i64 = 0;
            for item in diffs {
                match item {
                    Object::Integer(i) => code = *i,
                    Object::Real(r) => code = *r as i64,
                    Object::Name(n) => {
                        if let Ok(c) = u8::try_from(code) {
                            enc.differences.insert(c, n.clone());
                        }
                        code += 1;
                    }
                    _ => {}
                }
            }
        }
        enc
    }

    /// Glyph name for a code (Differences → base → built-in / Standard).
    fn glyph_name(&self, program: Option<&FontProgram<'_>>, code: u8) -> Option<Vec<u8>> {
        if let Some(n) = self.differences.get(&code) {
            return Some(n.clone());
        }
        if let Some(base) = self.base {
            return base[usize::from(code)].map(|s| s.as_bytes().to_vec());
        }
        if let Some(n) = program.and_then(|p| p.builtin_glyph_name(code)) {
            return Some(n.into_bytes());
        }
        if self.symbolic && !self.has_encoding_entry {
            return None;
        }
        encodings::STANDARD[usize::from(code)].map(|s| s.as_bytes().to_vec())
    }

    /// Resolve a code to a glyph in the program, following ISO 32000-1 9.6.6.
    fn resolve(&self, program: &FontProgram<'_>, code: u8) -> Resolved {
        if self.truetype {
            return self.resolve_truetype(program, code);
        }
        // Type 1 / CFF: by glyph name
        let Some(name) = self.glyph_name(Some(program), code) else {
            return Resolved::Unknown;
        };
        if name == b".notdef" {
            return Resolved::NotDef;
        }
        if program.has_glyph_named(&name) {
            return Resolved::Glyph(GlyphRef::Name(name));
        }
        // CFF fonts: a `uniXXXX` name may be present under its AGL name and vice versa
        if let Some(u) = agl::glyph_name_to_unicode(&name) {
            let alt = format!("uni{u:04X}");
            if program.has_glyph_named(alt.as_bytes()) {
                return Resolved::Glyph(GlyphRef::Name(alt.into_bytes()));
            }
        }
        Resolved::Missing(String::from_utf8_lossy(&name).into_owned())
    }

    /// ISO 32000-1 9.6.6.4: mapping codes to TrueType glyph indices.
    fn resolve_truetype(&self, program: &FontProgram<'_>, code: u8) -> Resolved {
        let subtables = program.cmap_subtables();
        let c = u32::from(code);
        if subtables.is_empty() {
            // No cmap: the code is the glyph index (via post table if names exist)
            if let Some(name) = self.glyph_name(Some(program), code) {
                if let Some(gid) = program.post_lookup(&name) {
                    return Resolved::Glyph(GlyphRef::Gid(usize::from(gid)));
                }
            }
            return if program.gid_exists(usize::from(code)) {
                Resolved::Glyph(GlyphRef::Gid(usize::from(code)))
            } else {
                Resolved::Missing(format!("gid {code}"))
            };
        }

        // (3,0) with the code and the 0xF0xx private-use aliases, then (1,0).
        // A subtable that explicitly maps the code to glyph 0 is remembered so
        // that .notdef references can be told apart from missing mappings.
        let symbolic_lookup = || -> (Option<u16>, bool) {
            let mut saw_notdef = false;
            for base in [0u32, 0xF000, 0xF100, 0xF200] {
                match program.cmap_lookup(3, 0, base + c) {
                    Some(0) => saw_notdef = true,
                    Some(g) => return (Some(g), false),
                    None => {}
                }
            }
            match program.cmap_lookup(1, 0, c) {
                Some(0) => (None, true),
                Some(g) => (Some(g), false),
                None => (None, saw_notdef),
            }
        };

        if self.symbolic && !self.has_encoding_entry {
            return match symbolic_lookup() {
                (Some(g), _) => Resolved::Glyph(GlyphRef::Gid(usize::from(g))),
                (None, saw_notdef) => {
                    // Fall back to any unicode cmap with the raw code
                    match program.unicode_lookup(c) {
                        Some(0) => Resolved::NotDef,
                        Some(g) => Resolved::Glyph(GlyphRef::Gid(usize::from(g))),
                        None if saw_notdef => Resolved::NotDef,
                        None => Resolved::Missing(format!("code {code} via (3,0)/(1,0) cmap")),
                    }
                }
            };
        }

        // Non-symbolic (or symbolic with an Encoding): name → Unicode → (3,1); name → Mac Roman code → (1,0); post table
        let name = self.glyph_name(Some(program), code);
        if let Some(name) = &name {
            if name == b".notdef" {
                return Resolved::NotDef;
            }
            if let Some(u) = agl::glyph_name_to_unicode(name) {
                match program.cmap_lookup(3, 1, u) {
                    Some(0) => return Resolved::NotDef,
                    Some(g) => return Resolved::Glyph(GlyphRef::Gid(usize::from(g))),
                    None => {}
                }
            }
            if let Some(mac_code) = encodings::MAC_ROMAN
                .iter()
                .position(|n| n.is_some_and(|n| n.as_bytes() == name.as_slice()))
            {
                match program.cmap_lookup(1, 0, mac_code as u32) {
                    Some(0) => return Resolved::NotDef,
                    Some(g) => return Resolved::Glyph(GlyphRef::Gid(usize::from(g))),
                    None => {}
                }
            }
            if let Some(g) = program.post_lookup(name) {
                return Resolved::Glyph(GlyphRef::Gid(usize::from(g)));
            }
            // `gNN` / `glyphNN` / `index NN` names address glyph indices directly
            if let Some(gid) = glyph_index_name(name) {
                if program.gid_exists(gid) {
                    return Resolved::Glyph(GlyphRef::Gid(gid));
                }
            }
        }
        // Symbolic fonts that nevertheless carry an Encoding: try the symbolic route too
        if self.symbolic {
            if let (Some(g), _) = symbolic_lookup() {
                return Resolved::Glyph(GlyphRef::Gid(usize::from(g)));
            }
        }
        // As a last resort, (3,0) with the raw code (common in the wild)
        match program
            .cmap_lookup(3, 0, 0xF000 + c)
            .or_else(|| program.cmap_lookup(3, 0, c))
        {
            Some(0) => Resolved::NotDef,
            Some(g) => Resolved::Glyph(GlyphRef::Gid(usize::from(g))),
            None => Resolved::CmapMiss,
        }
    }
}

/// `gNN`, `glyphNN`, `GNN`, `indexNN` glyph names.
fn glyph_index_name(name: &[u8]) -> Option<usize> {
    for prefix in [b"glyph" as &[u8], b"index", b"g", b"G"] {
        if let Some(rest) = name.strip_prefix(prefix) {
            if !rest.is_empty() && rest.iter().all(u8::is_ascii_digit) {
                return std::str::from_utf8(rest).ok()?.parse().ok();
            }
        }
    }
    None
}

/// `/CIDToGIDMap` of a `CIDFontType2`.
enum CidToGid {
    Identity,
    Map(Vec<u16>),
}

impl CidToGid {
    fn load(doc: &Document, cid_font: &Dictionary) -> Self {
        let Some(stream) = cid_font
            .get(b"CIDToGIDMap")
            .ok()
            .and_then(|o| resolve(doc, o))
            .and_then(|o| o.as_stream().ok())
        else {
            return Self::Identity;
        };
        let Ok(data) = stream.decompressed_content() else {
            return Self::Identity;
        };
        Self::Map(
            data.chunks(2)
                .map(|c| u16::from_be_bytes([c[0], *c.get(1).unwrap_or(&0)]))
                .collect(),
        )
    }
}

/// Glyph widths from a font dictionary (simple `/Widths` or CID `/W`).
struct Widths {
    map: HashMap<u32, f64>,
    default: Option<f64>,
}

impl Widths {
    fn simple(doc: &Document, font: &Dictionary, descriptor: Option<&Dictionary>) -> Self {
        let mut map = HashMap::new();
        let first = font
            .get_deref(b"FirstChar", doc)
            .ok()
            .and_then(|o| o.as_i64().ok())
            .unwrap_or(0);
        if let Some(widths) = font
            .get_deref(b"Widths", doc)
            .ok()
            .and_then(|o| o.as_array().ok())
        {
            for (i, w) in widths.iter().enumerate() {
                let w = match resolve(doc, w) {
                    Some(Object::Integer(v)) => f64::from(i32::try_from(*v).unwrap_or(0)),
                    Some(Object::Real(v)) => f64::from(*v),
                    _ => continue,
                };
                let Some(code) = first
                    .checked_add(i as i64)
                    .and_then(|c| u32::try_from(c).ok())
                else {
                    break;
                };
                map.insert(code, w);
            }
        }
        let default = descriptor
            .and_then(|d| d.get_deref(b"MissingWidth", doc).ok())
            .and_then(num);
        Self { map, default }
    }

    fn cid(doc: &Document, cid_font: &Dictionary) -> Self {
        let mut map = HashMap::new();
        let default = cid_font
            .get_deref(b"DW", doc)
            .ok()
            .and_then(num)
            .or(Some(1000.0));
        if let Some(w) = cid_font
            .get_deref(b"W", doc)
            .ok()
            .and_then(|o| o.as_array().ok())
        {
            let items: Vec<&Object> = w.iter().filter_map(|o| resolve(doc, o)).collect();
            let mut i = 0;
            while i < items.len() {
                let Some(first) = num(items[i]) else {
                    i += 1;
                    continue;
                };
                match items.get(i + 1) {
                    Some(Object::Array(arr)) => {
                        for (k, w) in arr.iter().enumerate() {
                            let Some(cid) = (first as u32).checked_add(k as u32) else {
                                break;
                            };
                            if let Some(w) = resolve(doc, w).and_then(num) {
                                map.insert(cid, w);
                            }
                        }
                        i += 2;
                    }
                    Some(last) => {
                        let (Some(last), Some(w)) =
                            (num(last), items.get(i + 2).and_then(|o| num(o)))
                        else {
                            i += 3;
                            continue;
                        };
                        let (lo, hi) = (first as u32, last as u32);
                        if hi >= lo && hi - lo < 65536 {
                            for cid in lo..=hi {
                                map.insert(cid, w);
                            }
                        }
                        i += 3;
                    }
                    None => break,
                }
            }
        }
        Self { map, default }
    }

    fn get(&self, code: u32) -> Option<f64> {
        self.map.get(&code).copied().or(self.default)
    }
}

/// Numeric value of an Integer or Real object.
fn num(obj: &Object) -> Option<f64> {
    match obj {
        #[allow(clippy::cast_precision_loss)]
        Object::Integer(i) => Some(*i as f64),
        Object::Real(r) => Some(f64::from(*r)),
        _ => None,
    }
}

fn join_ids(ids: &[u32]) -> String {
    let shown: Vec<String> = ids.iter().take(8).map(u32::to_string).collect();
    if ids.len() > 8 {
        format!("{}, …", shown.join(", "))
    } else {
        shown.join(", ")
    }
}

fn resolve<'a>(doc: &'a Document, obj: &'a Object) -> Option<&'a Object> {
    match obj {
        Object::Reference(id) => doc.get_object(*id).ok(),
        other => Some(other),
    }
}

fn resolve_dict<'a>(doc: &'a Document, obj: &'a Object) -> Option<&'a Dictionary> {
    resolve(doc, obj)?.as_dict().ok()
}

/// Remove duplicate results (same rule and description).
fn dedup(results: &mut Vec<CheckResult>) {
    let mut seen = std::collections::HashSet::new();
    results.retain(|r| seen.insert(format!("{}:{}", r.rule_id, r.description)));
}
