//! PDF/UA-2 annotation and form requirements (ISO 14289-2, 8.2.5.20, 8.9, 8.10).
//!
//! Annotation ↔ structure association (Link/Form/Annot parents, Contents on
//! markup annotations, `TrapNet`, `PrinterMark`, …) is shared with PDF/UA-1 and is
//! reported under Matterhorn ids by `annot_struct`. This module adds the
//! PDF/UA-2-only conditions:
//!
//! | Rule | Condition |
//! |------|-----------|
//! | `ua2:8.2.5.20-2` | links in one `Link`/`Reference` element target different locations |
//! | `ua2:8.9.2.2-1` / `-2` | Invisible / NoView annotations in the structure tree that are not artifacts |
//! | `ua2:8.9.2.4.7-1` | `Stamp` without `/Name` or `/Contents` |
//! | `ua2:8.9.2.4.8-1`, `.12-1`, `.19-1`, `.19-2` | `Ink`, `Screen`, `3D`, `RichMedia` without `/Contents` |
//! | `ua2:8.9.2.4.9-1` | `Popup` in the structure tree |
//! | `ua2:8.9.2.4.10-1` | `FileAttachment` file specification without `/AFRelationship` |
//! | `ua2:8.9.2.4.11-1` / `-2` | `Sound` / `Movie` annotations |
//! | `ua2:8.9.2.4.13-1` | zero-size `Widget` in the structure tree that is not an artifact |
//! | `ua2:8.9.4.2-1` | `/Contents` differs from the element's `/Alt` |
//! | `ua2:8.10.1-2` | `Form` element with more than one widget |
//! | `ua2:8.10.2.3-1` / `-2` | field widget without `Lbl`/`Contents`; widget with `/AA` but no `/Contents` |

use super::{
    decode_text_string, fail, kids, nonempty_text, page_location, pass, resolve, resolve_dict,
    struct_tree_root, type_name,
};
use crate::checks::Check;
use crate::checks::namespaces;
use crate::document::HornDocument;
use crate::model::{CheckResult, Standard};
use anyhow::Result;
use lopdf::{Dictionary, Document, Object, ObjectId};
use std::collections::{BTreeSet, HashMap};

pub struct Ua2AnnotationChecks;

impl Check for Ua2AnnotationChecks {
    fn id(&self) -> &'static str {
        "ua2-annotations"
    }

    fn checkpoint(&self) -> u8 {
        28
    }

    fn rules(&self) -> &'static [&'static str] {
        &[
            "ua2:8.2.5.20-2",
            "ua2:8.9.2.2-1",
            "ua2:8.9.2.2-2",
            "ua2:8.9.2.4.7-1",
            "ua2:8.9.2.4.8-1",
            "ua2:8.9.2.4.9-1",
            "ua2:8.9.2.4.10-1",
            "ua2:8.9.2.4.11-1",
            "ua2:8.9.2.4.11-2",
            "ua2:8.9.2.4.12-1",
            "ua2:8.9.2.4.13-1",
            "ua2:8.9.2.4.19-1",
            "ua2:8.9.2.4.19-2",
            "ua2:8.9.4.2-1",
            "ua2:8.10.1-2",
            "ua2:8.10.2.3-1",
            "ua2:8.10.2.3-2",
        ]
    }

    fn description(&self) -> &'static str {
        "PDF/UA-2: annotation artifacts, annotation types, Contents/Alt, form widgets"
    }

    fn supports(&self, standard: Standard) -> bool {
        standard == Standard::Ua2
    }

    #[allow(clippy::too_many_lines)]
    fn run(&self, doc: &mut HornDocument) -> Result<Vec<CheckResult>> {
        let mut results = Vec::new();
        let lopdf = doc.lopdf();
        let Ok(catalog) = lopdf.catalog() else {
            return Ok(results);
        };

        let mut tree = Tree::default();
        if let Some(root) = struct_tree_root(lopdf, catalog) {
            let role_map = namespaces::role_map(lopdf, root);
            collect(lopdf, root, role_map, &mut tree, 0);
        }

        let mut widget_counts: HashMap<usize, usize> = HashMap::new();
        let mut link_targets: HashMap<usize, BTreeSet<String>> = HashMap::new();
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        let mut failed: BTreeSet<String> = BTreeSet::new();

        for (page_num, page_id) in lopdf.get_pages() {
            let Ok(page) = lopdf.get_dictionary(page_id) else {
                continue;
            };
            let Some(annots) = page
                .get(b"Annots")
                .ok()
                .and_then(|o| resolve(lopdf, o))
                .and_then(|o| o.as_array().ok())
            else {
                continue;
            };

            for annot_ref in annots {
                let Ok(annot_id) = annot_ref.as_reference() else {
                    continue;
                };
                let Some(annot) = resolve_dict(lopdf, annot_ref) else {
                    continue;
                };
                let subtype = annot
                    .get(b"Subtype")
                    .ok()
                    .and_then(|o| o.as_name().ok())
                    .unwrap_or(b"");
                let subtype_str = String::from_utf8_lossy(subtype).into_owned();
                let flags = annot
                    .get(b"F")
                    .ok()
                    .and_then(|o| o.as_i64().ok())
                    .unwrap_or(0);
                let elem_idx = tree.objr.get(&annot_id).copied();
                let parent = elem_idx.map(|i| &tree.elems[i]);
                let in_structure = parent.is_some();
                let is_artifact = parent.is_some_and(|p| p.is_artifact);
                let element = format!("/{subtype_str}");
                let label = format!(
                    "Page {page_num}: /{subtype_str} annotation (obj {}.{})",
                    annot_id.0, annot_id.1
                );
                let contents = nonempty_text(lopdf, annot, b"Contents");

                let mut emit = |rule: &'static str, message: String| {
                    failed.insert(rule.to_string());
                    results.push(fail(rule, message, Some(page_location(page_num, &element))));
                };

                // 8.9.2.2: annotations that are not visible must be artifacts
                if in_structure && !is_artifact {
                    seen.insert("ua2:8.9.2.2-1");
                    seen.insert("ua2:8.9.2.2-2");
                    if flags & 1 != 0 {
                        emit(
                            "ua2:8.9.2.2-1",
                            format!(
                                "{label} has the Invisible flag but is included in the logical structure — it must be an artifact"
                            ),
                        );
                    }
                    if flags & 32 != 0 && flags & 256 == 0 {
                        emit(
                            "ua2:8.9.2.2-2",
                            format!(
                                "{label} has the NoView flag (without ToggleNoView) but is included in the logical structure — it must be an artifact"
                            ),
                        );
                    }
                }

                match subtype {
                    b"Popup" => {
                        seen.insert("ua2:8.9.2.4.9-1");
                        if in_structure {
                            emit(
                                "ua2:8.9.2.4.9-1",
                                format!(
                                    "{label} is included in the logical structure — Popup annotations must not be tagged"
                                ),
                            );
                        }
                    }
                    b"Sound" => emit(
                        "ua2:8.9.2.4.11-1",
                        format!("{label}: Sound annotations are not permitted in PDF/UA-2"),
                    ),
                    b"Movie" => emit(
                        "ua2:8.9.2.4.11-2",
                        format!("{label}: Movie annotations are not permitted in PDF/UA-2"),
                    ),
                    b"Stamp" => {
                        seen.insert("ua2:8.9.2.4.7-1");
                        if annot.get(b"Name").is_err() && contents.is_none() {
                            emit(
                                "ua2:8.9.2.4.7-1",
                                format!("{label} has neither /Name nor /Contents"),
                            );
                        }
                    }
                    b"Ink" | b"Screen" | b"3D" | b"RichMedia" => {
                        let rule = match subtype {
                            b"Ink" => "ua2:8.9.2.4.8-1",
                            b"Screen" => "ua2:8.9.2.4.12-1",
                            b"3D" => "ua2:8.9.2.4.19-1",
                            _ => "ua2:8.9.2.4.19-2",
                        };
                        seen.insert(rule);
                        if contents.is_none() {
                            emit(rule, format!("{label} has no /Contents entry"));
                        }
                    }
                    b"FileAttachment" => {
                        if let Some(fs) = annot.get(b"FS").ok().and_then(|o| resolve_dict(lopdf, o))
                        {
                            seen.insert("ua2:8.9.2.4.10-1");
                            if fs.get(b"AFRelationship").is_err() {
                                emit(
                                    "ua2:8.9.2.4.10-1",
                                    format!(
                                        "{label}: file specification has no /AFRelationship entry"
                                    ),
                                );
                            }
                        }
                    }
                    b"Widget" => {
                        if in_structure && !is_artifact {
                            seen.insert("ua2:8.9.2.4.13-1");
                            if is_zero_size(annot) {
                                emit(
                                    "ua2:8.9.2.4.13-1",
                                    format!(
                                        "{label} has a zero-size /Rect but is included in the logical structure — it must be an artifact"
                                    ),
                                );
                            }
                            if let Some(idx) = elem_idx {
                                if tree.elems[idx].std_type == b"Form" {
                                    *widget_counts.entry(idx).or_insert(0) += 1;
                                }
                            }
                            if is_field_widget(lopdf, annot) {
                                seen.insert("ua2:8.10.2.3-1");
                                seen.insert("ua2:8.10.2.3-2");
                                let has_lbl = parent.is_some_and(|p| p.has_lbl);
                                if !has_lbl && contents.is_none() {
                                    emit(
                                        "ua2:8.10.2.3-1",
                                        format!(
                                            "{label}: form field has neither a Lbl element in its Form structure element nor a /Contents entry"
                                        ),
                                    );
                                }
                                if annot.get(b"AA").is_ok() && contents.is_none() {
                                    emit(
                                        "ua2:8.10.2.3-2",
                                        format!(
                                            "{label}: form field has additional actions (/AA) but no /Contents entry describing them"
                                        ),
                                    );
                                }
                            }
                        }
                    }
                    b"Link" => {
                        if let Some(idx) = elem_idx {
                            if matches!(tree.elems[idx].std_type.as_slice(), b"Link" | b"Reference")
                            {
                                link_targets
                                    .entry(idx)
                                    .or_default()
                                    .insert(link_target_key(lopdf, annot));
                            }
                        }
                    }
                    _ => {}
                }

                // 8.9.4.2: Contents and Alt must agree when both are present
                if let (Some(c), Some(alt)) = (contents, parent.and_then(|p| p.alt.as_deref())) {
                    seen.insert("ua2:8.9.4.2-1");
                    let c = decode_text_string(c);
                    let a = decode_text_string(alt);
                    if c.trim() != a.trim() {
                        emit(
                            "ua2:8.9.4.2-1",
                            format!(
                                "{label}: /Contents (\"{}\") differs from the /Alt of its structure element (\"{}\")",
                                truncate(&c),
                                truncate(&a)
                            ),
                        );
                    }
                }
            }
        }

        for (idx, count) in &widget_counts {
            seen.insert("ua2:8.10.1-2");
            if *count > 1 {
                failed.insert("ua2:8.10.1-2".to_string());
                results.push(fail(
                    "ua2:8.10.1-2",
                    format!(
                        "Form structure element {} contains {count} widget annotations — a Form element may contain at most one",
                        tree.elems[*idx].describe()
                    ),
                    None,
                ));
            }
        }
        for (idx, targets) in &link_targets {
            seen.insert("ua2:8.2.5.20-2");
            if targets.len() > 1 {
                failed.insert("ua2:8.2.5.20-2".to_string());
                results.push(fail(
                    "ua2:8.2.5.20-2",
                    format!(
                        "{} element {} encloses {} link annotations that target different locations — use one Link element per target",
                        String::from_utf8_lossy(&tree.elems[*idx].std_type),
                        tree.elems[*idx].describe(),
                        targets.len()
                    ),
                    None,
                ));
            }
        }

        for rule in seen {
            if !failed.contains(rule) {
                results.push(pass(rule, "No violation found"));
            }
        }

        Ok(results)
    }
}

/// A structure element as seen by the annotation checks.
struct Elem {
    std_type: Vec<u8>,
    is_artifact: bool,
    alt: Option<Vec<u8>>,
    has_lbl: bool,
    object_id: Option<ObjectId>,
}

impl Elem {
    fn describe(&self) -> String {
        self.object_id.map_or_else(
            || "(direct object)".to_string(),
            |id| format!("(obj {}.{})", id.0, id.1),
        )
    }
}

#[derive(Default)]
struct Tree {
    elems: Vec<Elem>,
    /// Annotation object id → index of the enclosing element in `elems`.
    objr: HashMap<ObjectId, usize>,
}

fn collect(
    doc: &Document,
    dict: &Dictionary,
    role_map: Option<&Dictionary>,
    tree: &mut Tree,
    depth: usize,
) {
    if depth > 100 {
        return;
    }
    let qt = namespaces::resolve_qualified_type(doc, role_map, dict);
    let idx = tree.elems.len();
    tree.elems.push(Elem {
        is_artifact: qt.is_standard_namespace() && qt.name == b"Artifact",
        std_type: qt.name,
        alt: nonempty_text(doc, dict, b"Alt").map(<[u8]>::to_vec),
        has_lbl: false,
        object_id: None,
    });

    for kid in kids(doc, dict) {
        let object_id = kid.as_reference().ok();
        let Some(child) = resolve_dict(doc, kid) else {
            continue;
        };
        match type_name(child) {
            Some(b"OBJR") => {
                if let Some(id) = child.get(b"Obj").ok().and_then(|o| o.as_reference().ok()) {
                    tree.objr.insert(id, idx);
                }
            }
            Some(b"MCR") => {}
            _ if child.get(b"S").is_ok() => {
                let child_idx = tree.elems.len();
                collect(doc, child, role_map, tree, depth + 1);
                let mut child_is_lbl = false;
                if let Some(c) = tree.elems.get_mut(child_idx) {
                    c.object_id = object_id;
                    child_is_lbl = c.std_type == b"Lbl";
                }
                if child_is_lbl {
                    tree.elems[idx].has_lbl = true;
                }
            }
            _ => {}
        }
    }
}

/// A widget that belongs to a form field (has `/FT` itself or through its `/Parent` chain).
fn is_field_widget(doc: &Document, annot: &Dictionary) -> bool {
    let mut current = annot;
    for _ in 0..16 {
        if current.get(b"FT").is_ok() {
            return true;
        }
        match current
            .get(b"Parent")
            .ok()
            .and_then(|o| resolve_dict(doc, o))
        {
            Some(parent) => current = parent,
            None => return false,
        }
    }
    false
}

fn is_zero_size(annot: &Dictionary) -> bool {
    let Some(rect) = annot.get(b"Rect").ok().and_then(|o| o.as_array().ok()) else {
        return false;
    };
    let coords: Vec<f64> = rect
        .iter()
        .filter_map(|o| match o {
            Object::Real(f) => Some(f64::from(*f)),
            #[allow(clippy::cast_precision_loss)]
            Object::Integer(i) => Some(*i as f64),
            _ => None,
        })
        .collect();
    coords.len() == 4
        && ((coords[2] - coords[0]).abs() < 1e-6 || (coords[3] - coords[1]).abs() < 1e-6)
}

/// A key identifying where a link annotation leads (its action or destination).
fn link_target_key(doc: &Document, annot: &Dictionary) -> String {
    if let Some(action) = annot.get(b"A").ok().and_then(|o| resolve_dict(doc, o)) {
        let kind = action
            .get(b"S")
            .ok()
            .and_then(|o| o.as_name().ok())
            .map(|n| String::from_utf8_lossy(n).into_owned())
            .unwrap_or_default();
        let target = match kind.as_str() {
            "GoTo" => action.get(b"SD").or_else(|_| action.get(b"D")).ok(),
            "URI" => action.get(b"URI").ok(),
            _ => None,
        };
        return match target {
            Some(t) => format!("{kind}:{t:?}"),
            None => format!("{kind}:{action:?}"),
        };
    }
    if let Ok(dest) = annot.get(b"SD").or_else(|_| annot.get(b"Dest")) {
        return format!("Dest:{dest:?}");
    }
    "none".to_string()
}

fn truncate(s: &str) -> String {
    if s.chars().count() > 40 {
        format!("{}…", s.chars().take(40).collect::<String>())
    } else {
        s.to_string()
    }
}
