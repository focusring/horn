//! PDF/UA-2 logical-structure requirements (ISO 14289-2, 8.2 and 8.4.3).
//!
//! PDF 2.0 structure elements may live in namespaces (`/NS`) and be role
//! mapped through `/RoleMapNS`; every type test below works on the type
//! *after* role mapping, resolved by [`crate::checks::namespaces`].
//!
//! | Rule | Condition |
//! |------|-----------|
//! | `ua2:8.2.4-2` | circular `RoleMapNS` mapping |
//! | `ua2:8.2.4-3` | role mapping within the same namespace |
//! | `ua2:8.2.5.2-1` | structure tree root has exactly one child, a `Document` |
//! | `ua2:8.2.5.2-2` | that `Document` is in the PDF 2.0 namespace |
//! | `ua2:8.2.5.8-1` | `TOCI` without `/Ref` |
//! | `ua2:8.2.5.12-1` | generic `H` heading used |
//! | `ua2:8.2.5.14-1` | deprecated `Note` used |
//! | `ua2:8.2.5.14-4` | `FENote` with an invalid `NoteType` |
//! | `ua2:8.2.5.25-1` | list with `Lbl` items but no `ListNumbering` (or `None`) |
//! | `ua2:8.2.5.25-2` | `LI` with content outside `Lbl`/`LBody` |
//! | `ua2:8.2.5.29-1` | MathML element not inside `Formula` |
//! | `ua2:8.4.3-2` / `-3` | private-use code points in `ActualText` / `Alt` |

use super::{
    attribute, contains_private_use, decode_text_string, fail, kids, pass, resolve_dict,
    struct_tree_root, type_name,
};
use crate::checks::Check;
use crate::checks::namespaces::{self, NS_PDF2, NS_PDF17, QualifiedType};
use crate::document::HornDocument;
use crate::model::{CheckResult, Standard};
use anyhow::Result;
use lopdf::{Dictionary, Document, Object};

pub struct Ua2StructureChecks;

impl Check for Ua2StructureChecks {
    fn id(&self) -> &'static str {
        "ua2-structure"
    }

    fn checkpoint(&self) -> u8 {
        9
    }

    fn rules(&self) -> &'static [&'static str] {
        &[
            "ua2:8.2.4-2",
            "ua2:8.2.4-3",
            "ua2:8.2.5.2-1",
            "ua2:8.2.5.2-2",
            "ua2:8.2.5.8-1",
            "ua2:8.2.5.12-1",
            "ua2:8.2.5.14-1",
            "ua2:8.2.5.14-4",
            "ua2:8.2.5.25-1",
            "ua2:8.2.5.25-2",
            "ua2:8.2.5.29-1",
            "ua2:8.4.3-2",
            "ua2:8.4.3-3",
        ]
    }

    fn description(&self) -> &'static str {
        "PDF/UA-2: PDF 2.0 namespaces, Document root, headings, lists, notes, MathML, private-use text"
    }

    fn supports(&self, standard: Standard) -> bool {
        standard == Standard::Ua2
    }

    fn run(&self, doc: &mut HornDocument) -> Result<Vec<CheckResult>> {
        let mut results = Vec::new();
        let lopdf = doc.lopdf();
        let Ok(catalog) = lopdf.catalog() else {
            return Ok(results);
        };
        let Some(root) = struct_tree_root(lopdf, catalog) else {
            return Ok(results);
        };
        let role_map = namespaces::role_map(lopdf, root);

        check_document_root(lopdf, root, role_map, &mut results);
        check_namespace_role_maps(lopdf, root, &mut results);

        let mut state = TreeState::default();
        for kid in kids(lopdf, root) {
            if let Some(elem) = resolve_dict(lopdf, kid) {
                walk(lopdf, elem, None, role_map, &mut state, 0);
            }
        }
        state.emit(&mut results);

        Ok(results)
    }
}

/// `ua2:8.2.5.2-1` / `ua2:8.2.5.2-2`: a single `Document` root in the PDF 2.0 namespace.
fn check_document_root(
    doc: &Document,
    root: &Dictionary,
    role_map: Option<&Dictionary>,
    results: &mut Vec<CheckResult>,
) {
    let elems: Vec<&Dictionary> = kids(doc, root)
        .into_iter()
        .filter_map(|k| resolve_dict(doc, k))
        .filter(|d| d.get(b"S").is_ok())
        .collect();

    if elems.len() != 1 {
        results.push(fail(
            "ua2:8.2.5.2-1",
            format!(
                "The structure tree root has {} structure element children — it must contain exactly one Document element",
                elems.len()
            ),
            None,
        ));
        return;
    }

    let qt = namespaces::resolve_qualified_type(doc, role_map, elems[0]);
    if !(qt.is_standard_namespace() && qt.name == b"Document") {
        results.push(fail(
            "ua2:8.2.5.2-1",
            format!(
                "The only child of the structure tree root is /{} — it must be a Document element",
                String::from_utf8_lossy(&qt.name)
            ),
            None,
        ));
        return;
    }
    results.push(pass(
        "ua2:8.2.5.2-1",
        "The structure tree root contains a single Document element",
    ));

    if qt.namespace.as_deref() == Some(NS_PDF2) {
        results.push(pass(
            "ua2:8.2.5.2-2",
            "The Document element is in the PDF 2.0 namespace",
        ));
    } else {
        results.push(fail(
            "ua2:8.2.5.2-2",
            format!(
                "The Document element is in the {} namespace — PDF/UA-2 requires the PDF 2.0 namespace ({NS_PDF2}) via /NS",
                qt.namespace
                    .as_deref()
                    .map_or_else(|| "default (PDF 1.7)".to_string(), str::to_string)
            ),
            None,
        ));
    }
}

/// `ua2:8.2.4-2` / `ua2:8.2.4-3`: `RoleMapNS` entries must map across namespaces and
/// must not form cycles.
fn check_namespace_role_maps(doc: &Document, root: &Dictionary, results: &mut Vec<CheckResult>) {
    let Some(namespaces_arr) = root
        .get_deref(b"Namespaces", doc)
        .ok()
        .and_then(|o| o.as_array().ok())
    else {
        return;
    };

    let mut same_ns = 0usize;
    let mut cycles = 0usize;
    let mut entries = 0usize;

    for ns_obj in namespaces_arr {
        let Some(ns) = resolve_dict(doc, ns_obj) else {
            continue;
        };
        let uri = namespaces::namespace_uri(ns).unwrap_or_default();
        let Some(role_map_ns) = ns
            .get_deref(b"RoleMapNS", doc)
            .ok()
            .and_then(|o| o.as_dict().ok())
        else {
            continue;
        };

        for (name, target) in role_map_ns {
            entries += 1;
            let Some((target_qt, _)) = namespaces::role_map_ns_target(doc, target) else {
                continue;
            };
            let target_ns = target_qt
                .namespace
                .clone()
                .unwrap_or_else(|| NS_PDF17.to_string());
            if target_ns == uri {
                same_ns += 1;
                results.push(fail(
                    "ua2:8.2.4-3",
                    format!(
                        "Namespace {uri}: structure type /{} is role mapped to /{} within the same namespace",
                        String::from_utf8_lossy(name),
                        String::from_utf8_lossy(&target_qt.name)
                    ),
                    None,
                ));
            }
            if let Some(cycle) = role_map_ns_cycle(doc, name, ns, &uri) {
                cycles += 1;
                results.push(fail(
                    "ua2:8.2.4-2",
                    format!("Circular role mapping in RoleMapNS: {cycle}"),
                    None,
                ));
            }
        }
    }

    if entries > 0 {
        if same_ns == 0 {
            results.push(pass(
                "ua2:8.2.4-3",
                "All RoleMapNS entries map to types in other namespaces",
            ));
        }
        if cycles == 0 {
            results.push(pass("ua2:8.2.4-2", "No circular RoleMapNS mappings"));
        }
    }
}

/// Follow a `RoleMapNS` chain from `name` in namespace `ns`; returns the path if it loops.
fn role_map_ns_cycle(doc: &Document, name: &[u8], ns: &Dictionary, uri: &str) -> Option<String> {
    let mut visited: Vec<(Vec<u8>, String)> = vec![(name.to_vec(), uri.to_string())];
    let mut current_name = name.to_vec();
    let mut current_ns: Option<&Dictionary> = Some(ns);

    for _ in 0..32 {
        let nsd = current_ns?;
        let role_map_ns = nsd
            .get_deref(b"RoleMapNS", doc)
            .ok()
            .and_then(|o| o.as_dict().ok())?;
        let target = role_map_ns.get(&current_name).ok()?;
        let (qt, next_ns) = namespaces::role_map_ns_target(doc, target)?;
        let key = (
            qt.name.clone(),
            qt.namespace.clone().unwrap_or_else(|| NS_PDF17.to_string()),
        );
        if visited.contains(&key) {
            visited.push(key);
            return Some(
                visited
                    .iter()
                    .map(|(n, ns)| format!("/{} ({ns})", String::from_utf8_lossy(n)))
                    .collect::<Vec<_>>()
                    .join(" -> "),
            );
        }
        if qt.is_standard() {
            return None;
        }
        visited.push(key);
        current_name = qt.name;
        current_ns = next_ns;
    }
    None
}

#[derive(Default)]
struct TreeState {
    generic_h: usize,
    notes: usize,
    bad_note_types: Vec<String>,
    toci_without_ref: usize,
    toci_count: usize,
    math_outside_formula: usize,
    math_count: usize,
    lists_without_numbering: usize,
    lists_with_labels: usize,
    li_with_direct_content: usize,
    li_count: usize,
    pua_actual_text: usize,
    pua_alt: usize,
    text_attrs: usize,
}

impl TreeState {
    #[allow(clippy::too_many_lines)]
    fn emit(&self, results: &mut Vec<CheckResult>) {
        if self.generic_h > 0 {
            results.push(fail(
                "ua2:8.2.5.12-1",
                format!(
                    "{} generic H heading element(s) found — PDF/UA-2 requires numbered headings (H1, H2, …)",
                    self.generic_h
                ),
                None,
            ));
        }
        if self.notes > 0 {
            results.push(fail(
                "ua2:8.2.5.14-1",
                format!(
                    "{} Note structure element(s) found — Note is deprecated in PDF 2.0; use FENote",
                    self.notes
                ),
                None,
            ));
        }
        for note_type in &self.bad_note_types {
            results.push(fail(
                "ua2:8.2.5.14-4",
                format!(
                    "FENote element has NoteType /{note_type} — must be Footnote, Endnote or None"
                ),
                None,
            ));
        }
        if self.toci_without_ref > 0 {
            results.push(fail(
                "ua2:8.2.5.8-1",
                format!(
                    "{} of {} TOCI element(s) have no /Ref entry identifying the referenced content",
                    self.toci_without_ref, self.toci_count
                ),
                None,
            ));
        } else if self.toci_count > 0 {
            results.push(pass(
                "ua2:8.2.5.8-1",
                format!("All {} TOCI element(s) have a /Ref entry", self.toci_count),
            ));
        }
        if self.math_outside_formula > 0 {
            results.push(fail(
                "ua2:8.2.5.29-1",
                format!(
                    "{} MathML structure element(s) are not children of a Formula element",
                    self.math_outside_formula
                ),
                None,
            ));
        } else if self.math_count > 0 {
            results.push(pass(
                "ua2:8.2.5.29-1",
                "All MathML structure elements are enclosed in Formula elements",
            ));
        }
        if self.lists_without_numbering > 0 {
            results.push(fail(
                "ua2:8.2.5.25-1",
                format!(
                    "{} list(s) with Lbl items have no ListNumbering attribute or ListNumbering /None",
                    self.lists_without_numbering
                ),
                None,
            ));
        } else if self.lists_with_labels > 0 {
            results.push(pass(
                "ua2:8.2.5.25-1",
                format!(
                    "All {} list(s) with Lbl items have a ListNumbering attribute",
                    self.lists_with_labels
                ),
            ));
        }
        if self.li_with_direct_content > 0 {
            results.push(fail(
                "ua2:8.2.5.25-2",
                format!(
                    "{} LI element(s) contain content that is not enclosed in a Lbl or LBody element",
                    self.li_with_direct_content
                ),
                None,
            ));
        } else if self.li_count > 0 {
            results.push(pass(
                "ua2:8.2.5.25-2",
                "All LI content is enclosed in Lbl or LBody elements",
            ));
        }
        if self.pua_actual_text > 0 {
            results.push(fail(
                "ua2:8.4.3-2",
                format!(
                    "{} ActualText value(s) contain Unicode private-use-area code points",
                    self.pua_actual_text
                ),
                None,
            ));
        }
        if self.pua_alt > 0 {
            results.push(fail(
                "ua2:8.4.3-3",
                format!(
                    "{} Alt value(s) contain Unicode private-use-area code points",
                    self.pua_alt
                ),
                None,
            ));
        }
        if self.text_attrs > 0 && self.pua_actual_text == 0 && self.pua_alt == 0 {
            results.push(pass(
                "ua2:8.4.3-2",
                "No Alt or ActualText entry uses private-use-area code points",
            ));
        }
    }
}

fn walk(
    doc: &Document,
    dict: &Dictionary,
    parent: Option<&QualifiedType>,
    role_map: Option<&Dictionary>,
    state: &mut TreeState,
    depth: usize,
) {
    if depth > 100 {
        return;
    }

    let qt = namespaces::resolve_qualified_type(doc, role_map, dict);
    let standard_ns = qt.is_standard_namespace();
    let name = qt.name.as_slice();

    if standard_ns {
        match name {
            b"H" => state.generic_h += 1,
            b"Note" => state.notes += 1,
            b"FENote" => {
                if let Some(nt) = attribute(doc, dict, b"NoteType").and_then(|o| o.as_name().ok()) {
                    if !matches!(nt, b"Footnote" | b"Endnote" | b"None") {
                        state
                            .bad_note_types
                            .push(String::from_utf8_lossy(nt).into_owned());
                    }
                }
            }
            b"TOCI" => {
                state.toci_count += 1;
                if dict.get(b"Ref").is_err() {
                    state.toci_without_ref += 1;
                }
            }
            b"L" => check_list(doc, dict, role_map, state),
            b"LI" => {
                state.li_count += 1;
                if has_direct_content(doc, dict) {
                    state.li_with_direct_content += 1;
                }
            }
            _ => {}
        }
    }

    if qt.is_mathml() {
        state.math_count += 1;
        let enclosed = parent
            .is_some_and(|p| p.is_mathml() || (p.is_standard_namespace() && p.name == b"Formula"));
        if !enclosed {
            state.math_outside_formula += 1;
        }
    }

    for key in [b"ActualText" as &[u8], b"Alt"] {
        if let Some(text) = super::nonempty_text(doc, dict, key) {
            state.text_attrs += 1;
            if contains_private_use(&decode_text_string(text)) {
                if key == b"Alt" {
                    state.pua_alt += 1;
                } else {
                    state.pua_actual_text += 1;
                }
            }
        }
    }

    for kid in kids(doc, dict) {
        let Some(child) = resolve_dict(doc, kid) else {
            continue;
        };
        if matches!(type_name(child), Some(b"OBJR" | b"MCR")) {
            continue;
        }
        if child.get(b"S").is_ok() {
            walk(doc, child, Some(&qt), role_map, state, depth + 1);
        }
    }
}

/// `ua2:8.2.5.25-1`: lists whose items carry `Lbl` need `ListNumbering` other than `None`.
fn check_list(
    doc: &Document,
    list: &Dictionary,
    role_map: Option<&Dictionary>,
    state: &mut TreeState,
) {
    let has_labels = kids(doc, list)
        .into_iter()
        .filter_map(|k| resolve_dict(doc, k))
        .filter(|li| namespaces::resolve_qualified_type(doc, role_map, li).name == b"LI")
        .any(|li| {
            kids(doc, li)
                .into_iter()
                .filter_map(|k| resolve_dict(doc, k))
                .any(|c| namespaces::resolve_qualified_type(doc, role_map, c).name == b"Lbl")
        });
    if !has_labels {
        return;
    }
    state.lists_with_labels += 1;
    let numbering = attribute(doc, list, b"ListNumbering").and_then(|o| o.as_name().ok());
    if numbering.is_none_or(|n| n == b"None") {
        state.lists_without_numbering += 1;
    }
}

/// Marked content (MCID or MCR) directly inside an element.
fn has_direct_content(doc: &Document, dict: &Dictionary) -> bool {
    kids(doc, dict).into_iter().any(|k| match k {
        Object::Integer(_) => true,
        other => resolve_dict(doc, other).is_some_and(|d| type_name(d) == Some(b"MCR")),
    })
}
