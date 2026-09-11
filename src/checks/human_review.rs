//! Manual-review coverage for the Matterhorn Protocol's human-judgment
//! failure conditions.
//!
//! 48 of the 137 Matterhorn 1.1 failure conditions "usually require human
//! judgment" (`How::Human`). Software cannot decide them, but a conformance
//! report is only complete when it lists them, so that a reviewer knows what
//! still has to be checked by hand — the same idea as PAC's "manual checks".
//!
//! This check inspects the document for the features each condition applies
//! to (tables, lists, annotations, multimedia, JavaScript, …) and emits a
//! `NeedsReview` result for every applicable human condition and
//! `NotApplicable` for the rest, so every one of the 137 conditions appears in
//! the report exactly once (machine conditions are covered by the other checks).

use crate::checks::Check;
use crate::document::HornDocument;
use crate::fontprog::{FontFileKind, FontProgram};
use crate::matterhorn::{CONDITIONS, How};
use crate::model::{CheckOutcome, CheckResult, Severity};
use anyhow::Result;
use lopdf::{Dictionary, Document, Object};
use std::collections::BTreeSet;

pub struct HumanReviewChecks;

impl Check for HumanReviewChecks {
    fn id(&self) -> &'static str {
        "manual-review"
    }

    fn checkpoint(&self) -> u8 {
        0
    }

    fn description(&self) -> &'static str {
        "Manual review: human-judgment Matterhorn conditions applicable to this document"
    }

    fn is_machine_checkable(&self) -> bool {
        false
    }

    fn rules(&self) -> &'static [&'static str] {
        &HUMAN_RULES
    }

    fn run(&self, doc: &mut HornDocument) -> Result<Vec<CheckResult>> {
        let features = DocFeatures::scan(doc);
        let mut results = Vec::new();

        for cond in &CONDITIONS {
            if cond.how == How::Machine {
                continue;
            }
            let outcome = match cond.how {
                How::None => CheckOutcome::NotApplicable,
                _ => match features.applicability(cond.id) {
                    Some(reason) => CheckOutcome::NeedsReview { reason },
                    None => CheckOutcome::NotApplicable,
                },
            };
            let severity = if matches!(outcome, CheckOutcome::NeedsReview { .. })
                && features.review_is_warning(cond.id)
            {
                Severity::Warning
            } else {
                Severity::Info
            };
            results.push(CheckResult {
                rule_id: cond.id.to_string(),
                checkpoint: cond.checkpoint,
                description: cond.description.to_string(),
                severity,
                outcome,
            });
        }

        Ok(results)
    }
}

/// The 48 human-judgment conditions plus the two conditions without a test
/// (23-001, 27-001); every one of them appears in each report.
static HUMAN_RULES: [&str; 50] = [
    "23-001", "27-001", "01-001", "01-002", "01-006", "02-002", "03-001", "03-002", "03-003",
    "04-001", "05-001", "05-002", "05-003", "06-004", "08-001", "08-002", "09-001", "09-002",
    "09-003", "11-007", "12-001", "13-001", "13-002", "13-003", "13-005", "13-006", "13-007",
    "13-008", "14-001", "14-004", "14-005", "15-001", "15-002", "15-004", "15-005", "16-001",
    "16-002", "16-003", "17-001", "18-001", "18-002", "19-001", "19-002", "22-001", "24-001",
    "28-001", "28-003", "28-013", "29-001", "31-010",
];

/// Document features that decide which human conditions apply.
#[derive(Debug, Default)]
#[allow(clippy::struct_excessive_bools)]
struct DocFeatures {
    struct_types: BTreeSet<Vec<u8>>,
    has_text: bool,
    has_invisible_text: bool,
    has_role_map: bool,
    has_actions: bool,
    has_javascript: bool,
    has_multimedia: bool,
    has_dc_title: bool,
    has_lang: bool,
    has_annotations: bool,
    has_links: bool,
    has_ismap_link: bool,
    has_widgets: bool,
    has_threads: bool,
    has_artifacts: bool,
    has_embedded_fonts: bool,
    /// Embedded fonts whose OS/2 `fsType` forbids installable embedding.
    restricted_fonts: Vec<String>,
    heading_depth: u8,
    has_custom_heading_types: bool,
    has_ordered_list_hint: bool,
    has_invalid_list_numbering: bool,
}

impl DocFeatures {
    #[allow(clippy::too_many_lines)]
    fn scan(doc: &mut HornDocument) -> Self {
        let mut f = Self::default();
        let usage = doc.content_usage();
        f.has_text = usage.fonts.values().any(|u| u.show_ops > 0);
        f.has_invisible_text = usage.fonts.values().any(|u| u.show_ops > 0 && !u.rendered);
        f.has_artifacts = usage.artifact_sequences > 0;

        let lopdf = doc.lopdf();
        let Ok(catalog) = lopdf.catalog() else {
            return f;
        };

        // Structure tree
        if let Some(tree) = catalog
            .get(b"StructTreeRoot")
            .ok()
            .and_then(|o| resolve(lopdf, o))
            .and_then(|o| o.as_dict().ok())
        {
            f.has_role_map = tree.get(b"RoleMap").is_ok();
            let role_map = tree
                .get_deref(b"RoleMap", lopdf)
                .ok()
                .and_then(|o| o.as_dict().ok());
            if let Some(rm) = role_map {
                for (_, target) in rm {
                    if let Ok(t) = target.as_name() {
                        if t.len() >= 2 && t[0] == b'H' && t[1..].iter().all(u8::is_ascii_digit) {
                            f.has_custom_heading_types = true;
                        }
                    }
                }
            }
            let mut visited = std::collections::HashSet::new();
            walk_struct(lopdf, tree, &mut f, &mut visited, 0);
        }

        // Catalog-level features
        f.has_lang = catalog.get(b"Lang").is_ok();
        f.has_threads = catalog.get(b"Threads").is_ok();
        f.has_actions |= catalog.get(b"OpenAction").is_ok() || catalog.get(b"AA").is_ok();
        if let Some(names) = catalog
            .get_deref(b"Names", lopdf)
            .ok()
            .and_then(|o| o.as_dict().ok())
        {
            f.has_javascript |= names.get(b"JavaScript").is_ok();
        }
        if let Some(meta) = catalog
            .get(b"Metadata")
            .ok()
            .and_then(|o| resolve(lopdf, o))
            .and_then(|o| o.as_stream().ok())
            .and_then(|s| s.get_plain_content().ok())
        {
            f.has_dc_title = String::from_utf8_lossy(&meta).contains("dc:title");
        }

        // Pages: annotations, actions, multimedia
        for (_, page_id) in lopdf.get_pages() {
            let Ok(page) = lopdf.get_dictionary(page_id) else {
                continue;
            };
            f.has_actions |= page.get(b"AA").is_ok();
            let Some(annots) = page
                .get(b"Annots")
                .ok()
                .and_then(|o| resolve(lopdf, o))
                .and_then(|o| o.as_array().ok())
            else {
                continue;
            };
            for a in annots {
                let Some(annot) = resolve(lopdf, a).and_then(|o| o.as_dict().ok()) else {
                    continue;
                };
                let subtype = annot
                    .get(b"Subtype")
                    .ok()
                    .and_then(|o| o.as_name().ok())
                    .unwrap_or(b"");
                if matches!(subtype, b"Popup" | b"PrinterMark" | b"TrapNet") {
                    continue;
                }
                f.has_annotations = true;
                match subtype {
                    b"Link" => {
                        f.has_links = true;
                        if annot.get(b"IsMap").ok().and_then(|o| o.as_bool().ok()) == Some(true) {
                            f.has_ismap_link = true;
                        }
                    }
                    b"Widget" => f.has_widgets = true,
                    b"Screen" | b"Movie" | b"Sound" | b"RichMedia" | b"3D" => {
                        f.has_multimedia = true;
                    }
                    _ => {}
                }
                if annot.get(b"A").is_ok() || annot.get(b"AA").is_ok() {
                    f.has_actions = true;
                }
                if let Some(action) = annot
                    .get(b"A")
                    .ok()
                    .and_then(|o| resolve(lopdf, o))
                    .and_then(|o| o.as_dict().ok())
                {
                    let s = action
                        .get(b"S")
                        .ok()
                        .and_then(|o| o.as_name().ok())
                        .unwrap_or(b"");
                    if s == b"JavaScript" {
                        f.has_javascript = true;
                    }
                    if matches!(s, b"Rendition" | b"Movie" | b"Sound") {
                        f.has_multimedia = true;
                    }
                }
            }
        }

        // Fonts: embedding restrictions (31-010)
        for obj in lopdf.objects.values() {
            let Ok(dict) = obj.as_dict() else { continue };
            if dict.get(b"Type").ok().and_then(|o| o.as_name().ok()) != Some(b"FontDescriptor") {
                continue;
            }
            let data = dict
                .get(b"FontFile2")
                .ok()
                .or_else(|| dict.get(b"FontFile3").ok())
                .or_else(|| dict.get(b"FontFile").ok())
                .and_then(|o| resolve(lopdf, o))
                .and_then(|o| o.as_stream().ok())
                .and_then(|s| s.decompressed_content().ok());
            let Some(data) = data else { continue };
            f.has_embedded_fonts = true;
            let kind = if dict.get(b"FontFile2").is_ok() {
                FontFileKind::TrueType
            } else if dict.get(b"FontFile3").is_ok() {
                FontFileKind::FontFile3
            } else {
                FontFileKind::Type1
            };
            if let Some(FontProgram::TrueType(face)) = FontProgram::parse(kind, &data) {
                if let Some(os2) = face.tables().os2 {
                    if matches!(os2.permissions(), Some(ttf_parser::Permissions::Restricted)) {
                        let name = dict
                            .get(b"FontName")
                            .ok()
                            .and_then(|o| o.as_name().ok())
                            .map(|n| String::from_utf8_lossy(n).into_owned())
                            .unwrap_or_default();
                        f.restricted_fonts.push(name);
                    }
                }
            }
        }

        f
    }

    fn has(&self, ty: &[u8]) -> bool {
        self.struct_types.contains(ty)
    }

    fn has_figures(&self) -> bool {
        self.has(b"Figure")
    }

    fn has_tables(&self) -> bool {
        self.has(b"Table")
    }

    fn has_lists(&self) -> bool {
        self.has(b"L")
    }

    /// Whether a human condition applies, with the reason shown to the reviewer.
    #[allow(clippy::too_many_lines)]
    fn applicability(&self, id: &str) -> Option<String> {
        let text = self.has_text;
        let r = |s: &str| Some(s.to_string());
        match id {
            "01-001" | "01-002" if text || self.has_artifacts => r(
                "Verify that artifacts (page furniture, decoration) are not tagged as real content and that no real content is marked as an artifact",
            ),
            "01-006" if text => r(
                "Verify that every structure element's type and attributes match the semantics of its content",
            ),
            "02-002" if self.has_role_map => r(
                "Verify that each RoleMap entry maps to the semantically nearest standard structure type",
            ),
            "03-001" if self.has_actions => {
                r("Actions are present: verify none of them produce flickering or flashing content")
            }
            "03-002" if self.has_multimedia => {
                r("Multimedia is present: verify it contains no flickering content")
            }
            "03-003" | "05-003" | "29-001" if self.has_javascript => match id {
                "03-003" => r("JavaScript is present: verify no script produces flickering"),
                "05-003" => r(
                    "JavaScript is present: verify no script relies on beep() without another means of notification",
                ),
                _ => r(
                    "JavaScript is present: verify no script requires specific timing for individual keystrokes",
                ),
            },
            "04-001" if text || self.has_figures() => r(
                "Verify that information conveyed by colour, contrast, format or layout is also available through the tag structure",
            ),
            "05-001" | "05-002" if self.has_multimedia => r(
                "Media/audio annotations are present: verify their audio content is also available in another form (e.g. a transcript)",
            ),
            "06-004" if self.has_dc_title => {
                r("Verify that the XMP dc:title clearly identifies the document")
            }
            "08-001" | "08-002" if self.has_invisible_text => match id {
                "08-001" => r(
                    "Invisible (OCR) text is present: verify it contains no significant recognition errors",
                ),
                _ => r("Invisible (OCR) text is present: verify it is tagged as real content"),
            },
            "09-001" if text => {
                r("Verify that the structure tree follows the logical reading order")
            }
            "09-002" if text => r(
                "Verify that structure elements are nested in a semantically appropriate way (e.g. no table inside a heading)",
            ),
            "09-003" if text => r(
                "Verify that the structure type of every element (after role mapping) is semantically appropriate",
            ),
            "11-007" if self.has_lang => r(
                "Verify that the declared natural language(s) match the actual language of the text",
            ),
            "12-001" if text => r(
                "Verify that stretched characters (e.g. large braces or brackets) are represented as a single character in ActualText",
            ),
            "13-001" if text || self.has_figures() => r(
                "Verify that all graphics that carry information are tagged as <Figure> (and decorative graphics as artifacts)",
            ),
            "13-002" if self.has_links && self.has_figures() => r(
                "Links and figures are present: verify links with a meaningful graphical background describe both the link and the graphic",
            ),
            "13-003" if self.has_figures() || self.has_tables() => {
                r("Verify that captions of figures and tables are tagged as <Caption>")
            }
            "13-005" | "13-006" | "13-007" | "13-008" if self.has_figures() => match id {
                "13-005" => r(
                    "Figures are present: verify ActualText is not used where alternative text (Alt) would be appropriate",
                ),
                "13-006" => r(
                    "Figures are present: verify graphics that only have meaning as a group are tagged as one <Figure>",
                ),
                "13-007" => r(
                    "Figures are present: verify a more accessible representation (e.g. a table instead of a chart image) is used where possible",
                ),
                _ => r(
                    "Figures are present: verify figures meant to be read as text carry ActualText",
                ),
            },
            "14-001" if text => {
                r("Verify that all headings in the content are tagged as heading elements")
            }
            "14-004" if self.has_custom_heading_types => r(
                "Custom heading types are role-mapped: verify they map to H1–H6 (Arabic numerals) or Hn",
            ),
            "14-005" if self.heading_depth >= 6 => {
                r("H6 is used: verify content at a 7th or deeper heading level uses <H7> or higher")
            }
            "15-001" | "15-002" | "15-005" if self.has_tables() => match id {
                "15-001" => r("Tables are present: verify every row header cell is tagged as <TH>"),
                "15-002" => {
                    r("Tables are present: verify every column header cell is tagged as <TH>")
                }
                _ => r(
                    "Tables are present: verify every data cell's header can be unambiguously determined (Scope / Headers)",
                ),
            },
            "15-004" if self.has_tables() => {
                r("Tables are present: verify each is a real table (rows and columns), not layout")
            }
            "16-001" | "16-002" if self.has_lists() => {
                if self.has_invalid_list_numbering {
                    r(
                        "A list has a ListNumbering value outside Decimal/UpperRoman/LowerRoman/UpperAlpha/LowerAlpha — verify it is not an ordered list",
                    )
                } else if self.has_ordered_list_hint {
                    r(
                        "Ordered lists are present: verify every ordered list carries a valid ListNumbering attribute",
                    )
                } else {
                    r(
                        "Lists are present: verify ordered lists carry a ListNumbering attribute with a valid value",
                    )
                }
            }
            "16-003" if text => r("Verify that all list-like content is tagged as <L>/<LI>"),
            "17-001" if text => r("Verify that mathematical expressions are tagged as <Formula>"),
            "18-001" | "18-002" if text => match id {
                "18-001" => {
                    r("Verify that running headers and footers are marked as pagination artifacts")
                }
                _ => r("Verify that header/footer artifacts use the /Header or /Footer subtype"),
            },
            "19-001" | "19-002" if text => match id {
                "19-001" => r("Verify that footnotes and endnotes are tagged as <Note>"),
                _ => r("Verify that references to notes are tagged as <Reference>"),
            },
            "22-001" if self.has_threads => {
                r("Article threads are present: verify they reflect the logical reading order")
            }
            "24-001" if text && !self.has_widgets => r(
                "No interactive form fields: if the document contains a form meant to be printed, verify its fields are tagged with the PrintField attribute",
            ),
            "28-001" if self.has_annotations => r(
                "Annotations are present: verify their position in the structure tree matches the reading order",
            ),
            "28-003" if self.has_annotations => r(
                "Annotations are present: verify annotations used for visual formatting are tagged according to their semantic function",
            ),
            "28-013" if self.has_ismap_link => r(
                "A link annotation has /IsMap true: verify the image-map functionality is provided in some other way",
            ),
            "31-010" if self.has_embedded_fonts => {
                if self.restricted_fonts.is_empty() {
                    r(
                        "Embedded fonts are present: verify their licences permit embedding for unlimited, universal rendering",
                    )
                } else {
                    Some(format!(
                        "Embedded TrueType font(s) declare 'Restricted License embedding' (OS/2 fsType): {} — verify the licence permits embedding",
                        self.restricted_fonts.join(", ")
                    ))
                }
            }
            _ => None,
        }
    }

    /// Conditions where the document shows a concrete warning sign.
    fn review_is_warning(&self, id: &str) -> bool {
        match id {
            "31-010" => !self.restricted_fonts.is_empty(),
            "16-002" => self.has_invalid_list_numbering,
            "28-013" => true,
            _ => false,
        }
    }
}

fn walk_struct(
    doc: &Document,
    dict: &Dictionary,
    f: &mut DocFeatures,
    visited: &mut std::collections::HashSet<lopdf::ObjectId>,
    depth: usize,
) {
    if depth > 100 {
        return;
    }
    if let Some(s) = dict.get(b"S").ok().and_then(|o| o.as_name().ok()) {
        f.struct_types.insert(s.to_vec());
        if s.len() == 2 && s[0] == b'H' && s[1].is_ascii_digit() {
            f.heading_depth = f.heading_depth.max(s[1] - b'0');
        }
        if s == b"L" {
            if let Some(numbering) = list_numbering(doc, dict) {
                match numbering.as_slice() {
                    b"Decimal" | b"UpperRoman" | b"LowerRoman" | b"UpperAlpha" | b"LowerAlpha" => {
                        f.has_ordered_list_hint = true;
                    }
                    b"None" | b"Disc" | b"Circle" | b"Square" | b"Unordered" | b"Ordered"
                    | b"Description" => {}
                    _ => f.has_invalid_list_numbering = true,
                }
            }
        }
    }
    let Ok(kids) = dict.get(b"K") else { return };
    let mut visit = |o: &Object| {
        // Shared /K references are processed at most once
        if let Object::Reference(id) = o {
            if !visited.insert(*id) {
                return;
            }
        }
        if let Some(d) = resolve(doc, o).and_then(|o| o.as_dict().ok()) {
            walk_struct(doc, d, f, visited, depth + 1);
        }
    };
    match kids {
        Object::Array(arr) => arr.iter().for_each(&mut visit),
        other => visit(other),
    }
}

fn list_numbering(doc: &Document, dict: &Dictionary) -> Option<Vec<u8>> {
    let attrs = dict.get(b"A").ok()?;
    let from_dict = |d: &Dictionary| {
        d.get(b"ListNumbering")
            .ok()
            .and_then(|o| o.as_name().ok())
            .map(<[u8]>::to_vec)
    };
    match resolve(doc, attrs)? {
        Object::Dictionary(d) => from_dict(d),
        Object::Array(arr) => arr
            .iter()
            .filter_map(|o| resolve(doc, o))
            .filter_map(|o| o.as_dict().ok())
            .find_map(from_dict),
        _ => None,
    }
}

fn resolve<'a>(doc: &'a Document, obj: &'a Object) -> Option<&'a Object> {
    match obj {
        Object::Reference(id) => doc.get_object(*id).ok(),
        other => Some(other),
    }
}
