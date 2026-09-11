//! Walks every content stream of a document (pages, Form XObjects and
//! annotation appearance streams) and records, per font dictionary, the
//! character codes shown with it and whether any of them are rendered
//! (text rendering mode other than 3, "invisible").
//!
//! This is the basis for the font-program checks of checkpoint 31 that only
//! apply to glyphs "referenced for rendering" (31-009, 31-011, 31-016, 31-018,
//! 31-030) and for the Unicode-mapping check (31-027 / 10-001).

use crate::content::cmap::EncodingCMap;
use lopdf::content::Content;
use lopdf::{Dictionary, Document, Object, ObjectId};
use std::collections::{BTreeSet, HashMap, HashSet};

/// Character codes used with one font dictionary.
#[derive(Debug, Default, Clone)]
pub struct FontUsage {
    /// Every (code, byte length) shown with the font, in any rendering mode.
    pub codes: BTreeSet<(u32, usize)>,
    /// Codes shown in a rendering mode other than 3 (i.e. actually rendered).
    pub rendered_codes: BTreeSet<(u32, usize)>,
    /// True when the font's `/Encoding` is a predefined CMap we cannot split
    /// byte strings with; `codes` is then unreliable and callers should skip
    /// code-level checks.
    pub codes_unreliable: bool,
    /// Number of text-showing operations that used the font.
    pub show_ops: u32,
    /// True if at least one of the text-showing operations used a rendering mode
    /// other than 3 (invisible text, e.g. OCR layers).
    pub rendered: bool,
}

/// Font usage keyed by the font dictionary's object id. Fonts referenced only
/// as direct dictionaries (no indirect object) are keyed by a synthetic id
/// derived from the resource name and are rare in practice.
pub type FontUsageMap = HashMap<ObjectId, FontUsage>;

/// Collect font usage over all pages, their Form XObjects and annotation
/// appearance streams.
pub fn collect_font_usage(doc: &Document) -> FontUsageMap {
    let mut walker = Walker {
        doc,
        usage: HashMap::new(),
        cmaps: HashMap::new(),
        visited_forms: HashSet::new(),
    };

    for (_, page_id) in doc.get_pages() {
        let Ok(page) = doc.get_dictionary(page_id) else {
            continue;
        };
        let resources = resolve_dict(doc, page.get(b"Resources").ok());
        let content = doc.get_page_content(page_id);
        walker.walk_stream(&content, resources, 0);

        // Annotation appearance streams
        if let Some(annots) = page
            .get(b"Annots")
            .ok()
            .and_then(|o| resolve(doc, o))
            .and_then(|o| o.as_array().ok())
        {
            for annot in annots {
                let Some(annot) = resolve(doc, annot).and_then(|o| o.as_dict().ok()) else {
                    continue;
                };
                let Some(ap) = annot.get(b"AP").ok().and_then(|o| resolve(doc, o)).and_then(|o| o.as_dict().ok()) else {
                    continue;
                };
                for (_, entry) in ap {
                    walker.walk_appearance(entry, resources, 0);
                }
            }
        }
    }

    walker.usage
}

struct Walker<'a> {
    doc: &'a Document,
    usage: FontUsageMap,
    /// Parsed encoding CMaps per Type 0 font (None = cannot split reliably).
    cmaps: HashMap<ObjectId, Option<EncodingCMap>>,
    visited_forms: HashSet<ObjectId>,
}

#[derive(Clone)]
struct TextState {
    font: Option<ObjectId>,
    render_mode: i64,
}

impl Walker<'_> {
    /// An appearance entry is a stream or a sub-dictionary of state → stream.
    fn walk_appearance(&mut self, entry: &Object, parent_resources: Option<&Dictionary>, depth: usize) {
        let Some(resolved) = resolve(self.doc, entry) else { return };
        if let Ok(stream) = resolved.as_stream() {
            let id = entry.as_reference().ok();
            self.walk_form(stream, id, parent_resources, depth);
        } else if let Ok(states) = resolved.as_dict() {
            for (_, sub) in states {
                if let Some(s) = resolve(self.doc, sub).and_then(|o| o.as_stream().ok()) {
                    let id = sub.as_reference().ok();
                    self.walk_form(s, id, parent_resources, depth);
                }
            }
        }
    }

    fn walk_form(
        &mut self,
        stream: &lopdf::Stream,
        id: Option<ObjectId>,
        parent_resources: Option<&Dictionary>,
        depth: usize,
    ) {
        if depth > 12 {
            return;
        }
        if let Some(id) = id {
            if !self.visited_forms.insert(id) {
                return;
            }
        }
        let resources = resolve_dict(self.doc, stream.dict.get(b"Resources").ok()).or(parent_resources);
        let Ok(data) = stream.decompressed_content() else {
            return;
        };
        self.walk_stream(&data, resources, depth + 1);
    }

    fn walk_stream(&mut self, data: &[u8], resources: Option<&Dictionary>, depth: usize) {
        let Ok(content) = Content::decode(data) else {
            return;
        };
        let fonts = resources
            .and_then(|r| r.get(b"Font").ok())
            .and_then(|o| resolve(self.doc, o))
            .and_then(|o| o.as_dict().ok());
        let xobjects = resources
            .and_then(|r| r.get(b"XObject").ok())
            .and_then(|o| resolve(self.doc, o))
            .and_then(|o| o.as_dict().ok());

        let mut state = TextState {
            font: None,
            render_mode: 0,
        };
        let mut stack: Vec<TextState> = Vec::new();

        for op in &content.operations {
            match op.operator.as_str() {
                "q" => stack.push(state.clone()),
                "Q" => {
                    if let Some(s) = stack.pop() {
                        state = s;
                    }
                }
                "Tf" => {
                    state.font = op
                        .operands
                        .first()
                        .and_then(|o| o.as_name().ok())
                        .and_then(|name| fonts.and_then(|f| f.get(name).ok()))
                        .and_then(|o| o.as_reference().ok());
                }
                "Tr" => {
                    state.render_mode = op.operands.first().and_then(|o| o.as_i64().ok()).unwrap_or(0);
                }
                "Tj" | "'" | "\"" => {
                    if let Some(Object::String(bytes, _)) = op.operands.last() {
                        let bytes = bytes.clone();
                        self.record(state.font, &bytes, state.render_mode);
                    }
                }
                "TJ" => {
                    if let Some(Object::Array(items)) = op.operands.first() {
                        let strings: Vec<Vec<u8>> = items
                            .iter()
                            .filter_map(|it| match it {
                                Object::String(b, _) => Some(b.clone()),
                                _ => None,
                            })
                            .collect();
                        for s in strings {
                            self.record(state.font, &s, state.render_mode);
                        }
                    }
                }
                "Do" => {
                    let target = op
                        .operands
                        .first()
                        .and_then(|o| o.as_name().ok())
                        .and_then(|name| xobjects.and_then(|x| x.get(name).ok()));
                    if let Some(obj) = target {
                        let id = obj.as_reference().ok();
                        if let Some(stream) = resolve(self.doc, obj).and_then(|o| o.as_stream().ok()) {
                            let is_form = stream
                                .dict
                                .get(b"Subtype")
                                .ok()
                                .and_then(|o| o.as_name().ok())
                                == Some(b"Form");
                            if is_form {
                                self.walk_form(stream, id, resources, depth);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn record(&mut self, font: Option<ObjectId>, bytes: &[u8], render_mode: i64) {
        let Some(font_id) = font else { return };
        let codes = self.split_codes(font_id, bytes);
        let entry = self.usage.entry(font_id).or_default();
        entry.show_ops += 1;
        if render_mode != 3 {
            entry.rendered = true;
        }
        match codes {
            Some(codes) => {
                for c in codes {
                    entry.codes.insert(c);
                    if render_mode != 3 {
                        entry.rendered_codes.insert(c);
                    }
                }
            }
            None => entry.codes_unreliable = true,
        }
    }

    /// Split a string into character codes according to the font type.
    fn split_codes(&mut self, font_id: ObjectId, bytes: &[u8]) -> Option<Vec<(u32, usize)>> {
        let Ok(font) = self.doc.get_dictionary(font_id) else {
            return None;
        };
        let subtype = font.get(b"Subtype").ok().and_then(|o| o.as_name().ok()).unwrap_or(b"");
        if subtype != b"Type0" {
            return Some(bytes.iter().map(|b| (u32::from(*b), 1)).collect());
        }
        if !self.cmaps.contains_key(&font_id) {
            let cmap = self.load_cmap(font);
            self.cmaps.insert(font_id, cmap);
        }
        let cmap = self.cmaps.get(&font_id)?.as_ref()?;
        Some(cmap.split_codes(bytes))
    }

    fn load_cmap(&self, font: &Dictionary) -> Option<EncodingCMap> {
        let enc = font.get(b"Encoding").ok()?;
        if let Ok(name) = enc.as_name() {
            return match name {
                b"Identity-H" | b"Identity-V" => Some(EncodingCMap::identity()),
                _ => None, // predefined CJK CMap: byte layout unknown to us
            };
        }
        let stream = resolve(self.doc, enc)?.as_stream().ok()?;
        let data = stream.decompressed_content().ok()?;
        let cmap = EncodingCMap::parse(&data);
        cmap.can_split().then_some(cmap)
    }
}

/// Resolve a reference (or return the object itself).
fn resolve<'a>(doc: &'a Document, obj: &'a Object) -> Option<&'a Object> {
    match obj {
        Object::Reference(id) => doc.get_object(*id).ok(),
        other => Some(other),
    }
}

fn resolve_dict<'a>(doc: &'a Document, obj: Option<&'a Object>) -> Option<&'a Dictionary> {
    resolve(doc, obj?)?.as_dict().ok()
}
