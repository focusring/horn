//! Code → glyph-name tables for the predefined simple-font encodings of
//! ISO 32000-1 Annex D (`StandardEncoding`, `WinAnsiEncoding`, `MacRomanEncoding`,
//! `MacExpertEncoding` and the built-in encoding of the Symbol font).
//!
//! Generated from the encoding tables shipped with `lopdf` (MIT), expressed as
//! Adobe Glyph List names so they can be resolved against embedded Type 1 / CFF
//! font programs and the AGL for Unicode mapping.

/// A 256-entry code → glyph name table; `None` marks unassigned codes.
pub type GlyphNameTable = [Option<&'static str>; 256];

/// Look up a predefined encoding by its PDF name.
pub fn predefined(name: &[u8]) -> Option<&'static GlyphNameTable> {
    match name {
        b"WinAnsiEncoding" => Some(&WIN_ANSI),
        b"MacRomanEncoding" => Some(&MAC_ROMAN),
        b"MacExpertEncoding" => Some(&MAC_EXPERT),
        b"StandardEncoding" => Some(&STANDARD),
        _ => None,
    }
}

/// Standard code → glyph name.
#[rustfmt::skip]
pub static STANDARD: GlyphNameTable = [
    None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
    None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
    Some("space"), Some("exclam"), Some("quotedbl"), Some("numbersign"), Some("dollar"),
    Some("percent"), Some("ampersand"), Some("quoteright"), Some("parenleft"), Some("parenright"),
    Some("asterisk"), Some("plus"), Some("comma"), Some("hyphen"), Some("period"), Some("slash"),
    Some("zero"), Some("one"), Some("two"), Some("three"), Some("four"), Some("five"), Some("six"),
    Some("seven"), Some("eight"), Some("nine"), Some("colon"), Some("semicolon"), Some("less"),
    Some("equal"), Some("greater"), Some("question"), Some("at"), Some("A"), Some("B"), Some("C"),
    Some("D"), Some("E"), Some("F"), Some("G"), Some("H"), Some("I"), Some("J"), Some("K"),
    Some("L"), Some("M"), Some("N"), Some("O"), Some("P"), Some("Q"), Some("R"), Some("S"),
    Some("T"), Some("U"), Some("V"), Some("W"), Some("X"), Some("Y"), Some("Z"),
    Some("bracketleft"), Some("backslash"), Some("bracketright"), Some("asciicircum"),
    Some("underscore"), Some("quoteleft"), Some("a"), Some("b"), Some("c"), Some("d"), Some("e"),
    Some("f"), Some("g"), Some("h"), Some("i"), Some("j"), Some("k"), Some("l"), Some("m"),
    Some("n"), Some("o"), Some("p"), Some("q"), Some("r"), Some("s"), Some("t"), Some("u"),
    Some("v"), Some("w"), Some("x"), Some("y"), Some("z"), Some("braceleft"), Some("bar"),
    Some("braceright"), Some("asciitilde"), None, None, None, None, None, None, None, None, None,
    None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
    None, None, None, None, None, None, None, None, None, Some("exclamdown"), Some("cent"),
    Some("sterling"), Some("fraction"), Some("yen"), Some("florin"), Some("section"),
    Some("currency"), Some("quotesingle"), Some("quotedblleft"), Some("guillemotleft"),
    Some("guilsinglleft"), Some("guilsinglright"), Some("fi"), Some("fl"), None, Some("endash"),
    Some("dagger"), Some("daggerdbl"), Some("periodcentered"), None, Some("paragraph"),
    Some("bullet"), Some("quotesinglbase"), Some("quotedblbase"), Some("quotedblright"),
    Some("guillemotright"), Some("ellipsis"), Some("perthousand"), None, Some("questiondown"), None,
    Some("grave"), Some("acute"), Some("circumflex"), Some("tilde"), Some("macron"), Some("breve"),
    Some("dotaccent"), Some("dieresis"), None, Some("ring"), Some("cedilla"), None,
    Some("hungarumlaut"), Some("ogonek"), Some("caron"), Some("emdash"), None, None, None, None,
    None, None, None, None, None, None, None, None, None, None, None, None, Some("AE"), None,
    Some("ordfeminine"), None, None, None, None, Some("Lslash"), Some("Oslash"), Some("OE"),
    Some("ordmasculine"), None, None, None, None, None, Some("ae"), None, None, None,
    Some("dotlessi"), None, None, Some("lslash"), Some("oslash"), Some("oe"), Some("germandbls"),
    None, None, None, None,
];

/// Winansi code → glyph name.
#[rustfmt::skip]
pub static WIN_ANSI: GlyphNameTable = [
    None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
    None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
    Some("space"), Some("exclam"), Some("quotedbl"), Some("numbersign"), Some("dollar"),
    Some("percent"), Some("ampersand"), Some("quotesingle"), Some("parenleft"), Some("parenright"),
    Some("asterisk"), Some("plus"), Some("comma"), Some("hyphen"), Some("period"), Some("slash"),
    Some("zero"), Some("one"), Some("two"), Some("three"), Some("four"), Some("five"), Some("six"),
    Some("seven"), Some("eight"), Some("nine"), Some("colon"), Some("semicolon"), Some("less"),
    Some("equal"), Some("greater"), Some("question"), Some("at"), Some("A"), Some("B"), Some("C"),
    Some("D"), Some("E"), Some("F"), Some("G"), Some("H"), Some("I"), Some("J"), Some("K"),
    Some("L"), Some("M"), Some("N"), Some("O"), Some("P"), Some("Q"), Some("R"), Some("S"),
    Some("T"), Some("U"), Some("V"), Some("W"), Some("X"), Some("Y"), Some("Z"),
    Some("bracketleft"), Some("backslash"), Some("bracketright"), Some("asciicircum"),
    Some("underscore"), Some("grave"), Some("a"), Some("b"), Some("c"), Some("d"), Some("e"),
    Some("f"), Some("g"), Some("h"), Some("i"), Some("j"), Some("k"), Some("l"), Some("m"),
    Some("n"), Some("o"), Some("p"), Some("q"), Some("r"), Some("s"), Some("t"), Some("u"),
    Some("v"), Some("w"), Some("x"), Some("y"), Some("z"), Some("braceleft"), Some("bar"),
    Some("braceright"), Some("asciitilde"), Some("bullet"), Some("Euro"), Some("bullet"),
    Some("quotesinglbase"), Some("florin"), Some("quotedblbase"), Some("ellipsis"), Some("dagger"),
    Some("daggerdbl"), Some("circumflex"), Some("perthousand"), Some("Scaron"),
    Some("guilsinglleft"), Some("OE"), Some("bullet"), Some("Zcaron"), Some("bullet"),
    Some("bullet"), Some("quoteleft"), Some("quoteright"), Some("quotedblleft"),
    Some("quotedblright"), Some("bullet"), Some("endash"), Some("emdash"), Some("tilde"),
    Some("trademark"), Some("scaron"), Some("guilsinglright"), Some("oe"), Some("bullet"),
    Some("zcaron"), Some("Ydieresis"), Some("space"), Some("exclamdown"), Some("cent"),
    Some("sterling"), Some("currency"), Some("yen"), Some("brokenbar"), Some("section"),
    Some("dieresis"), Some("copyright"), Some("ordfeminine"), Some("guillemotleft"),
    Some("logicalnot"), Some("hyphen"), Some("registered"), Some("macron"), Some("degree"),
    Some("plusminus"), Some("twosuperior"), Some("threesuperior"), Some("acute"), Some("mu"),
    Some("paragraph"), Some("periodcentered"), Some("cedilla"), Some("onesuperior"),
    Some("ordmasculine"), Some("guillemotright"), Some("onequarter"), Some("onehalf"),
    Some("threequarters"), Some("questiondown"), Some("Agrave"), Some("Aacute"),
    Some("Acircumflex"), Some("Atilde"), Some("Adieresis"), Some("Aring"), Some("AE"),
    Some("Ccedilla"), Some("Egrave"), Some("Eacute"), Some("Ecircumflex"), Some("Edieresis"),
    Some("Igrave"), Some("Iacute"), Some("Icircumflex"), Some("Idieresis"), Some("Eth"),
    Some("Ntilde"), Some("Ograve"), Some("Oacute"), Some("Ocircumflex"), Some("Otilde"),
    Some("Odieresis"), Some("multiply"), Some("Oslash"), Some("Ugrave"), Some("Uacute"),
    Some("Ucircumflex"), Some("Udieresis"), Some("Yacute"), Some("Thorn"), Some("germandbls"),
    Some("agrave"), Some("aacute"), Some("acircumflex"), Some("atilde"), Some("adieresis"),
    Some("aring"), Some("ae"), Some("ccedilla"), Some("egrave"), Some("eacute"),
    Some("ecircumflex"), Some("edieresis"), Some("igrave"), Some("iacute"), Some("icircumflex"),
    Some("idieresis"), Some("eth"), Some("ntilde"), Some("ograve"), Some("oacute"),
    Some("ocircumflex"), Some("otilde"), Some("odieresis"), Some("divide"), Some("oslash"),
    Some("ugrave"), Some("uacute"), Some("ucircumflex"), Some("udieresis"), Some("yacute"),
    Some("thorn"), Some("ydieresis"),
];

/// Macroman code → glyph name.
#[rustfmt::skip]
pub static MAC_ROMAN: GlyphNameTable = [
    None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
    None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
    Some("space"), Some("exclam"), Some("quotedbl"), Some("numbersign"), Some("dollar"),
    Some("percent"), Some("ampersand"), Some("quotesingle"), Some("parenleft"), Some("parenright"),
    Some("asterisk"), Some("plus"), Some("comma"), Some("hyphen"), Some("period"), Some("slash"),
    Some("zero"), Some("one"), Some("two"), Some("three"), Some("four"), Some("five"), Some("six"),
    Some("seven"), Some("eight"), Some("nine"), Some("colon"), Some("semicolon"), Some("less"),
    Some("equal"), Some("greater"), Some("question"), Some("at"), Some("A"), Some("B"), Some("C"),
    Some("D"), Some("E"), Some("F"), Some("G"), Some("H"), Some("I"), Some("J"), Some("K"),
    Some("L"), Some("M"), Some("N"), Some("O"), Some("P"), Some("Q"), Some("R"), Some("S"),
    Some("T"), Some("U"), Some("V"), Some("W"), Some("X"), Some("Y"), Some("Z"),
    Some("bracketleft"), Some("backslash"), Some("bracketright"), Some("asciicircum"),
    Some("underscore"), Some("grave"), Some("a"), Some("b"), Some("c"), Some("d"), Some("e"),
    Some("f"), Some("g"), Some("h"), Some("i"), Some("j"), Some("k"), Some("l"), Some("m"),
    Some("n"), Some("o"), Some("p"), Some("q"), Some("r"), Some("s"), Some("t"), Some("u"),
    Some("v"), Some("w"), Some("x"), Some("y"), Some("z"), Some("braceleft"), Some("bar"),
    Some("braceright"), Some("asciitilde"), None, Some("Adieresis"), Some("Aring"),
    Some("Ccedilla"), Some("Eacute"), Some("Ntilde"), Some("Odieresis"), Some("Udieresis"),
    Some("aacute"), Some("agrave"), Some("acircumflex"), Some("adieresis"), Some("atilde"),
    Some("aring"), Some("ccedilla"), Some("eacute"), Some("egrave"), Some("ecircumflex"),
    Some("edieresis"), Some("iacute"), Some("igrave"), Some("icircumflex"), Some("idieresis"),
    Some("ntilde"), Some("oacute"), Some("ograve"), Some("ocircumflex"), Some("odieresis"),
    Some("otilde"), Some("uacute"), Some("ugrave"), Some("ucircumflex"), Some("udieresis"),
    Some("dagger"), Some("degree"), Some("cent"), Some("sterling"), Some("section"), Some("bullet"),
    Some("paragraph"), Some("germandbls"), Some("registered"), Some("copyright"), Some("trademark"),
    Some("acute"), Some("dieresis"), Some("notequal"), Some("AE"), Some("Oslash"), Some("infinity"),
    Some("plusminus"), Some("lessequal"), Some("greaterequal"), Some("yen"), Some("mu"),
    Some("partialdiff"), Some("summation"), Some("product"), Some("pi"), Some("integral"),
    Some("ordfeminine"), Some("ordmasculine"), Some("Omega"), Some("ae"), Some("oslash"),
    Some("questiondown"), Some("exclamdown"), Some("logicalnot"), Some("radical"), Some("florin"),
    Some("approxequal"), Some("Delta"), Some("guillemotleft"), Some("guillemotright"),
    Some("ellipsis"), Some("space"), Some("Agrave"), Some("Atilde"), Some("Otilde"), Some("OE"),
    Some("oe"), Some("endash"), Some("emdash"), Some("quotedblleft"), Some("quotedblright"),
    Some("quoteleft"), Some("quoteright"), Some("divide"), Some("lozenge"), Some("ydieresis"),
    Some("Ydieresis"), Some("fraction"), Some("currency"), Some("guilsinglleft"),
    Some("guilsinglright"), Some("fi"), Some("fl"), Some("daggerdbl"), Some("periodcentered"),
    Some("quotesinglbase"), Some("quotedblbase"), Some("perthousand"), Some("Acircumflex"),
    Some("Ecircumflex"), Some("Aacute"), Some("Edieresis"), Some("Egrave"), Some("Iacute"),
    Some("Icircumflex"), Some("Idieresis"), Some("Igrave"), Some("Oacute"), Some("Ocircumflex"),
    Some("apple"), Some("Ograve"), Some("Uacute"), Some("Ucircumflex"), Some("Ugrave"),
    Some("dotlessi"), Some("circumflex"), Some("tilde"), Some("macron"), Some("breve"),
    Some("dotaccent"), Some("ring"), Some("cedilla"), Some("hungarumlaut"), Some("ogonek"),
    Some("caron"),
];

/// Symbol code → glyph name.
#[rustfmt::skip]
pub static SYMBOL: GlyphNameTable = [
    None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
    None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
    Some("space"), Some("exclam"), Some("universal"), Some("numbersign"), Some("existential"),
    Some("percent"), Some("ampersand"), Some("suchthat"), Some("parenleft"), Some("parenright"),
    Some("asteriskmath"), Some("plus"), Some("comma"), Some("minus"), Some("period"), Some("slash"),
    Some("zero"), Some("one"), Some("two"), Some("three"), Some("four"), Some("five"), Some("six"),
    Some("seven"), Some("eight"), Some("nine"), Some("colon"), Some("semicolon"), Some("less"),
    Some("equal"), Some("greater"), Some("question"), Some("congruent"), Some("Alpha"),
    Some("Beta"), Some("Chi"), Some("Delta"), Some("Epsilon"), Some("Phi"), Some("Gamma"),
    Some("Eta"), Some("Iota"), Some("theta1"), Some("Kappa"), Some("Lambda"), Some("Mu"),
    Some("Nu"), Some("Omicron"), Some("Pi"), Some("Theta"), Some("Rho"), Some("Sigma"), Some("Tau"),
    Some("Upsilon"), Some("sigma1"), Some("Omega"), Some("Xi"), Some("Psi"), Some("Zeta"),
    Some("bracketleft"), Some("therefore"), Some("bracketright"), Some("perpendicular"),
    Some("underscore"), Some("radicalex"), Some("alpha"), Some("beta"), Some("chi"), Some("delta"),
    Some("epsilon"), Some("phi"), Some("gamma"), Some("eta"), Some("iota"), Some("phi2"),
    Some("kappa"), Some("lambda"), Some("mu"), Some("nu"), Some("omicron"), Some("pi"),
    Some("theta"), Some("rho"), Some("sigma"), Some("tau"), Some("upsilon"), Some("omega1"),
    Some("omega"), Some("xi"), Some("psi"), Some("zeta"), Some("braceleft"), Some("bar"),
    Some("braceright"), Some("similar"), None, None, None, None, None, None, None, None, None, None,
    None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
    None, None, None, None, None, None, None, None, Some("Upsilon1"), Some("minute"),
    Some("lessequal"), Some("fraction"), Some("infinity"), Some("florin"), Some("club"),
    Some("diamond"), Some("heart"), Some("spade"), Some("arrowboth"), Some("arrowleft"),
    Some("arrowup"), Some("arrowright"), Some("arrowdown"), Some("degree"), Some("plusminus"),
    Some("second"), Some("greaterequal"), Some("multiply"), Some("proportional"),
    Some("partialdiff"), Some("bullet"), Some("divide"), Some("notequal"), Some("equivalence"),
    Some("approxequal"), Some("ellipsis"), Some("arrowvertex"), Some("arrowhorizex"),
    Some("carriagereturn"), Some("aleph"), Some("Ifraktur"), Some("Rfraktur"), Some("weierstrass"),
    Some("circlemultiply"), Some("circleplus"), Some("emptyset"), Some("intersection"),
    Some("union"), Some("propersuperset"), Some("reflexsuperset"), Some("notsubset"),
    Some("propersubset"), Some("reflexsubset"), Some("element"), Some("notelement"), Some("angle"),
    Some("gradient"), Some("registerserif"), Some("copyrightserif"), Some("trademarkserif"),
    Some("product"), Some("radical"), Some("dotmath"), Some("logicalnot"), Some("logicaland"),
    Some("logicalor"), Some("arrowdblboth"), Some("arrowdblleft"), Some("arrowdblup"),
    Some("arrowdblright"), Some("arrowdbldown"), Some("lozenge"), Some("angleleft"),
    Some("registersans"), Some("copyrightsans"), Some("trademarksans"), Some("summation"),
    Some("parenlefttp"), Some("parenleftex"), Some("parenleftbt"), Some("bracketlefttp"),
    Some("bracketleftex"), Some("bracketleftbt"), Some("bracelefttp"), Some("braceleftmid"),
    Some("braceleftbt"), Some("braceex"), None, Some("angleright"), Some("integral"),
    Some("integraltp"), Some("integralex"), Some("integralbt"), Some("parenrighttp"),
    Some("parenrightex"), Some("parenrightbt"), Some("bracketrighttp"), Some("bracketrightex"),
    Some("bracketrightbt"), Some("bracerighttp"), Some("bracerightmid"), Some("bracerightbt"), None,
];

/// Macexpert code → glyph name.
#[rustfmt::skip]
pub static MAC_EXPERT: GlyphNameTable = [
    None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
    None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
    Some("space"), Some("exclamsmall"), Some("Hungarumlautsmall"), Some("centoldstyle"),
    Some("dollaroldstyle"), Some("dollarsuperior"), Some("ampersandsmall"), Some("Acutesmall"),
    Some("parenleftsuperior"), Some("parenrightsuperior"), Some("twodotenleader"),
    Some("onedotenleader"), Some("comma"), Some("hyphen"), Some("period"), Some("fraction"),
    Some("zerooldstyle"), Some("oneoldstyle"), Some("twooldstyle"), Some("threeoldstyle"),
    Some("fouroldstyle"), Some("fiveoldstyle"), Some("sixoldstyle"), Some("sevenoldstyle"),
    Some("eightoldstyle"), Some("nineoldstyle"), Some("colon"), Some("semicolon"), None,
    Some("threequartersemdash"), None, Some("questionsmall"), None, None, None, None,
    Some("Ethsmall"), None, None, Some("onequarter"), Some("onehalf"), Some("threequarters"),
    Some("oneeighth"), Some("threeeighths"), Some("fiveeighths"), Some("seveneighths"),
    Some("onethird"), Some("twothirds"), None, None, None, None, None, None, Some("ff"), Some("fi"),
    Some("fl"), Some("ffi"), Some("ffl"), Some("parenleftinferior"), None,
    Some("parenrightinferior"), Some("Circumflexsmall"), Some("hypheninferior"), Some("Gravesmall"),
    Some("Asmall"), Some("Bsmall"), Some("Csmall"), Some("Dsmall"), Some("Esmall"), Some("Fsmall"),
    Some("Gsmall"), Some("Hsmall"), Some("Ismall"), Some("Jsmall"), Some("Ksmall"), Some("Lsmall"),
    Some("Msmall"), Some("Nsmall"), Some("Osmall"), Some("Psmall"), Some("Qsmall"), Some("Rsmall"),
    Some("Ssmall"), Some("Tsmall"), Some("Usmall"), Some("Vsmall"), Some("Wsmall"), Some("Xsmall"),
    Some("Ysmall"), Some("Zsmall"), Some("colonmonetary"), Some("onefitted"), Some("rupiah"),
    Some("Tildesmall"), None, None, Some("asuperior"), Some("centsuperior"), None, None, None, None,
    Some("Aacutesmall"), Some("Agravesmall"), Some("Acircumflexsmall"), Some("Adieresissmall"),
    Some("Atildesmall"), Some("Aringsmall"), Some("Ccedillasmall"), Some("Eacutesmall"),
    Some("Egravesmall"), Some("Ecircumflexsmall"), Some("Edieresissmall"), Some("Iacutesmall"),
    Some("Igravesmall"), Some("Icircumflexsmall"), Some("Idieresissmall"), Some("Ntildesmall"),
    Some("Oacutesmall"), Some("Ogravesmall"), Some("Ocircumflexsmall"), Some("Odieresissmall"),
    Some("Otildesmall"), Some("Uacutesmall"), Some("Ugravesmall"), Some("Ucircumflexsmall"),
    Some("Udieresissmall"), None, Some("eightsuperior"), Some("fourinferior"),
    Some("threeinferior"), Some("sixinferior"), Some("eightinferior"), Some("seveninferior"),
    Some("Scaronsmall"), None, Some("centinferior"), Some("twoinferior"), None,
    Some("Dieresissmall"), None, Some("Caronsmall"), Some("osuperior"), Some("fiveinferior"), None,
    Some("commainferior"), Some("periodinferior"), Some("Yacutesmall"), None,
    Some("dollarinferior"), None, None, Some("Thornsmall"), None, Some("nineinferior"),
    Some("zeroinferior"), Some("Zcaronsmall"), Some("AEsmall"), Some("Oslashsmall"),
    Some("questiondownsmall"), Some("oneinferior"), Some("Lslashsmall"), None, None, None, None,
    None, None, Some("Cedillasmall"), None, None, None, None, None, Some("OEsmall"),
    Some("figuredash"), Some("hyphensuperior"), None, None, None, None, Some("exclamdownsmall"),
    None, Some("Ydieresissmall"), None, Some("onesuperior"), Some("twosuperior"),
    Some("threesuperior"), Some("foursuperior"), Some("fivesuperior"), Some("sixsuperior"),
    Some("sevensuperior"), Some("ninesuperior"), Some("zerosuperior"), None, Some("esuperior"),
    Some("rsuperior"), Some("tsuperior"), None, None, Some("isuperior"), Some("ssuperior"),
    Some("dsuperior"), None, None, None, None, None, Some("lsuperior"), Some("Ogoneksmall"),
    Some("Brevesmall"), Some("Macronsmall"), Some("bsuperior"), Some("nsuperior"),
    Some("msuperior"), Some("commasuperior"), Some("periodsuperior"), Some("Dotaccentsmall"),
    Some("Ringsmall"), None, None, None, None,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spot_checks() {
        assert_eq!(WIN_ANSI[0x41], Some("A"));
        assert_eq!(WIN_ANSI[0x20], Some("space"));
        assert_eq!(WIN_ANSI[0x60], Some("grave"));
        assert_eq!(MAC_ROMAN[0xCA], Some("space"));
        assert_eq!(STANDARD[0x27], Some("quoteright"));
        assert_eq!(SYMBOL[0x61], Some("alpha"));
        assert!(predefined(b"WinAnsiEncoding").is_some());
        assert!(predefined(b"Identity-H").is_none());
    }
}
