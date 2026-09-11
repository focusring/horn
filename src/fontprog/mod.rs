//! Font-program level analysis used by the checkpoint 31 (Fonts) checks.
//!
//! PDF/UA-1 clause 7.21 places requirements not only on the font *dictionaries*
//! in a PDF but on the embedded font *programs* themselves (glyph coverage,
//! CharSet/CIDSet consistency, glyph widths, TrueType cmap subtables). This
//! module contains small, dependency-free parsers for the three embedded font
//! formats — TrueType/OpenType (`FontFile2`, via `ttf-parser`), CFF/Type1C
//! (`FontFile3`) and Type 1 (`FontFile`) — and exposes them through a common
//! [`FontProgram`] interface.

pub mod agl;
pub mod cff;
pub mod encodings;
pub mod type1;

use ttf_parser::GlyphId;

/// Which `FontDescriptor` entry a font program came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontFileKind {
    /// `/FontFile` — Type 1.
    Type1,
    /// `/FontFile2` — TrueType.
    TrueType,
    /// `/FontFile3` — CFF (`/Type1C`, `/CIDFontType0C`) or OpenType.
    FontFile3,
}

/// A parsed embedded font program.
pub enum FontProgram<'a> {
    /// TrueType or OpenType (`ttf-parser`).
    TrueType(ttf_parser::Face<'a>),
    /// Bare CFF.
    Cff(cff::CffFont),
    /// Type 1.
    Type1(type1::Type1Font),
}

/// One `cmap` subtable of a TrueType font program.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CmapSubtable {
    pub platform_id: u16,
    pub encoding_id: u16,
}

impl<'a> FontProgram<'a> {
    /// Parse a font program according to the descriptor entry it was found in.
    /// OpenType data in `/FontFile3` (`OTTO`) and TrueType collections are also
    /// handled. Returns `None` when the data cannot be parsed.
    pub fn parse(kind: FontFileKind, data: &'a [u8]) -> Option<Self> {
        match kind {
            FontFileKind::TrueType => ttf_parser::Face::parse(data, 0).ok().map(Self::TrueType),
            FontFileKind::FontFile3 => {
                if data.starts_with(b"OTTO") || data.starts_with(&[0, 1, 0, 0]) || data.starts_with(b"true") {
                    ttf_parser::Face::parse(data, 0).ok().map(Self::TrueType)
                } else {
                    cff::CffFont::parse(data).map(Self::Cff)
                }
            }
            FontFileKind::Type1 => {
                if data.starts_with(b"%!") || data.first() == Some(&0x80) || data.windows(5).take(1024).any(|w| w == b"eexec") {
                    type1::Type1Font::parse(data).map(Self::Type1)
                } else if data.first() == Some(&1) {
                    // Some producers put bare CFF in /FontFile
                    cff::CffFont::parse(data).map(Self::Cff)
                } else {
                    type1::Type1Font::parse(data).map(Self::Type1)
                }
            }
        }
    }

    /// Number of glyphs in the program.
    pub fn glyph_count(&self) -> usize {
        match self {
            Self::TrueType(face) => usize::from(face.number_of_glyphs()),
            Self::Cff(cff) => cff.glyph_count,
            Self::Type1(t1) => t1.glyph_names().len(),
        }
    }

    /// True if a glyph with the given name exists (Type 1 / name-keyed CFF /
    /// OpenType with a `post` or CFF table).
    pub fn has_glyph_named(&self, name: &[u8]) -> bool {
        match self {
            Self::TrueType(face) => std::str::from_utf8(name)
                .ok()
                .and_then(|n| face.glyph_index_by_name(n))
                .is_some(),
            Self::Cff(cff) => cff.gid_by_name(name).is_some(),
            Self::Type1(t1) => t1.has_glyph(name),
        }
    }

    /// Whether glyph names can be enumerated for this program.
    pub fn glyph_names(&self) -> Option<Vec<String>> {
        match self {
            Self::TrueType(face) => {
                if !face.tables().post.is_some_and(|p| p.names().next().is_some()) {
                    return None;
                }
                Some(
                    (0..face.number_of_glyphs())
                        .filter_map(|g| face.glyph_name(GlyphId(g)).map(str::to_string))
                        .collect(),
                )
            }
            Self::Cff(cff) => cff.glyph_names(),
            Self::Type1(t1) => Some(t1.glyph_names().iter().map(|s| (*s).to_string()).collect()),
        }
    }

    /// True when the program is a CID-keyed CFF font.
    pub fn is_cid_keyed_cff(&self) -> bool {
        matches!(self, Self::Cff(c) if c.is_cid)
    }

    /// GID for a CID: identity for TrueType, charset lookup for CID-keyed CFF.
    pub fn gid_for_cid(&self, cid: u32) -> Option<usize> {
        match self {
            Self::Cff(cff) if cff.is_cid => cff.gid_of_cid(u16::try_from(cid).ok()?),
            _ => Some(cid as usize),
        }
    }

    /// True if the glyph id exists in the program *and* carries glyph data
    /// (TrueType: non-empty `glyf`/CFF outline; CFF/Type 1: a charstring).
    /// Glyph 0 (.notdef) is always considered present when it exists.
    pub fn gid_is_present(&self, gid: usize) -> bool {
        match self {
            Self::TrueType(face) => {
                let Ok(g) = u16::try_from(gid) else { return false };
                if g >= face.number_of_glyphs() {
                    return false;
                }
                if g == 0 {
                    return true;
                }
                face.glyph_bounding_box(GlyphId(g)).is_some()
            }
            Self::Cff(cff) => gid < cff.glyph_count,
            Self::Type1(t1) => gid < t1.glyph_names().len(),
        }
    }

    /// True if the glyph id exists at all (even if it has no outline).
    pub fn gid_exists(&self, gid: usize) -> bool {
        gid < self.glyph_count()
    }

    /// Advance width of a glyph id in 1/1000 text-space units.
    pub fn advance_by_gid(&self, gid: usize) -> Option<f64> {
        match self {
            Self::TrueType(face) => {
                let g = u16::try_from(gid).ok()?;
                let adv = face.glyph_hor_advance(GlyphId(g))?;
                let upem = f64::from(face.units_per_em());
                if upem <= 0.0 {
                    return None;
                }
                Some(f64::from(adv) * 1000.0 / upem)
            }
            Self::Cff(cff) => cff.advance_width(gid),
            Self::Type1(t1) => {
                let name = t1.glyph_names().get(gid).copied()?;
                t1.advance_width(name.as_bytes())
            }
        }
    }

    /// Advance width of a named glyph in 1/1000 text-space units.
    pub fn advance_by_name(&self, name: &[u8]) -> Option<f64> {
        match self {
            Self::TrueType(face) => {
                let n = std::str::from_utf8(name).ok()?;
                let gid = face.glyph_index_by_name(n)?;
                self.advance_by_gid(usize::from(gid.0))
            }
            Self::Cff(cff) => cff.advance_width(cff.gid_by_name(name)?),
            Self::Type1(t1) => t1.advance_width(name),
        }
    }

    /// The TrueType `cmap` subtables (platform, encoding) present in the program.
    pub fn cmap_subtables(&self) -> Vec<CmapSubtable> {
        let Self::TrueType(face) = self else {
            return Vec::new();
        };
        let Some(cmap) = face.tables().cmap else {
            return Vec::new();
        };
        cmap.subtables
            .into_iter()
            .map(|s| CmapSubtable {
                platform_id: s.platform_id.to_u16(),
                encoding_id: s.encoding_id,
            })
            .collect()
    }

    /// Look a code up in a specific TrueType `cmap` subtable.
    pub fn cmap_lookup(&self, platform_id: u16, encoding_id: u16, code: u32) -> Option<u16> {
        let Self::TrueType(face) = self else {
            return None;
        };
        let cmap = face.tables().cmap?;
        for s in cmap.subtables {
            if s.platform_id.to_u16() == platform_id && s.encoding_id == encoding_id {
                if let Some(g) = s.glyph_index(code) {
                    if g.0 != 0 {
                        return Some(g.0);
                    }
                }
            }
        }
        None
    }

    /// Look a Unicode code point up in any Unicode `cmap` subtable.
    pub fn unicode_lookup(&self, code_point: u32) -> Option<u16> {
        let Self::TrueType(face) = self else {
            return None;
        };
        let cmap = face.tables().cmap?;
        for s in cmap.subtables {
            if s.is_unicode() {
                if let Some(g) = s.glyph_index(code_point) {
                    if g.0 != 0 {
                        return Some(g.0);
                    }
                }
            }
        }
        None
    }

    /// Look a glyph name up via the TrueType `post` table (or CFF charset).
    pub fn post_lookup(&self, name: &[u8]) -> Option<u16> {
        let Self::TrueType(face) = self else {
            return None;
        };
        let n = std::str::from_utf8(name).ok()?;
        face.glyph_index_by_name(n).map(|g| g.0)
    }

    /// The built-in encoding of a Type 1 / CFF program (code → glyph name), if any.
    pub fn builtin_glyph_name(&self, code: u8) -> Option<String> {
        match self {
            Self::Type1(t1) => t1.builtin_glyph_name(code).map(str::to_string),
            Self::Cff(cff) => cff.builtin_glyph_name(code),
            Self::TrueType(_) => None,
        }
    }
}

/// PlatformId helper: `ttf_parser::PlatformId` has no direct numeric accessor.
trait PlatformIdExt {
    fn to_u16(self) -> u16;
}

impl PlatformIdExt for ttf_parser::PlatformId {
    fn to_u16(self) -> u16 {
        match self {
            ttf_parser::PlatformId::Unicode => 0,
            ttf_parser::PlatformId::Macintosh => 1,
            ttf_parser::PlatformId::Iso => 2,
            ttf_parser::PlatformId::Windows => 3,
            ttf_parser::PlatformId::Custom => 4,
        }
    }
}
