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
